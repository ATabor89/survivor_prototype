//! Authored estate layout and dialogue, validated like the stage data.
use crate::inventory::{PassiveType, MAX_PASSIVES};
use crate::profile::{Profile, Stockpile};
use crate::setup::PassiveLoadout;
use crate::weapons::WeaponType;
use bevy::prelude::*;
use serde::Deserialize;
use std::collections::HashSet;

pub const EMBEDDED_ESTATE: &str = include_str!("../../assets/data/estate.ron");

/// Clearance kept between walkable points and walls or buildings.
const MARGIN: f32 = 24.0;

#[derive(Resource, Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EstateContent {
    pub half_size: [f32; 2],
    pub spawn: [f32; 2],
    pub expedition_weapons: Vec<WeaponType>,
    #[serde(default)]
    pub buildings: Vec<BuildingDefinition>,
    pub stations: Vec<StationDefinition>,
    #[serde(default)]
    pub npcs: Vec<NpcDefinition>,
    /// Improvements the workshop can build, in the order they are listed.
    #[serde(default)]
    pub upgrades: Vec<UpgradeDefinition>,
}

/// A one-time estate improvement bought with expedition resources.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpgradeDefinition {
    /// Stable identifier stored in the saved profile.
    pub id: String,
    pub name: String,
    pub description: String,
    pub cost: Stockpile,
    pub effect: EstateEffect,
}

/// How a built upgrade changes later expeditions. Every effect is expressed
/// through the run recipe, so expeditions need no knowledge of the estate.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum EstateEffect {
    /// Adds a starting-weapon choice at the expedition table.
    UnlockWeapon(WeaponType),
    /// Every estate expedition begins with this passive at level one.
    StartingPassive(PassiveType),
}

/// A solid, labelled block. Interiors are not modelled in the prototype.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildingDefinition {
    pub name: String,
    pub center: [f32; 2],
    pub half_size: [f32; 2],
    pub color: [f32; 3],
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash)]
pub enum StationKind {
    /// Opens the menu that plans and launches an expedition.
    ExpeditionTable,
    /// Opens the menu that builds estate upgrades.
    Workshop,
}

/// A fixed spot the player can use.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StationDefinition {
    pub kind: StationKind,
    pub name: String,
    pub position: [f32; 2],
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcDefinition {
    pub id: String,
    pub name: String,
    pub position: [f32; 2],
    pub color: [f32; 3],
    /// Spoken before the first expedition.
    pub lines: Vec<String>,
    /// Spoken once at least one expedition has returned; falls back to `lines`.
    #[serde(default)]
    pub returning_lines: Vec<String>,
}

impl NpcDefinition {
    pub fn lines_for(&self, expeditions: u32) -> &[String] {
        if expeditions > 0 && !self.returning_lines.is_empty() {
            &self.returning_lines
        } else {
            &self.lines
        }
    }
}

impl Default for EstateContent {
    fn default() -> Self {
        Self::load().unwrap_or_else(|error| panic!("Cannot load estate content: {error}"))
    }
}

impl EstateContent {
    /// Runtime edits to assets/data/estate.ron take effect on the next launch.
    pub fn load() -> Result<Self, String> {
        let Some(path) = crate::content::data_file("estate.ron") else {
            warn!("assets/data/estate.ron was not found; using the bundled estate");
            return Self::parse(EMBEDDED_ESTATE);
        };
        let text = std::fs::read_to_string(&path).map_err(|error| {
            format!(
                "{}: {error}. Restore this asset or correct BEVY_ASSET_ROOT.",
                path.display()
            )
        })?;
        Self::parse(&text).map_err(|error| format!("{}: {error}", path.display()))
    }

    /// Starting weapons on offer: the basics plus every unlocked one.
    pub fn available_weapons(&self, profile: &Profile) -> Vec<WeaponType> {
        let unlocked = self
            .upgrades
            .iter()
            .filter(|upgrade| profile.owns(&upgrade.id))
            .filter_map(|upgrade| match upgrade.effect {
                EstateEffect::UnlockWeapon(weapon) => Some(weapon),
                EstateEffect::StartingPassive(_) => None,
            });
        self.expedition_weapons
            .iter()
            .copied()
            .chain(unlocked)
            .collect()
    }

    /// Passives every estate expedition starts with, from built upgrades.
    pub fn starting_passives(&self, profile: &Profile) -> Vec<PassiveLoadout> {
        self.upgrades
            .iter()
            .filter(|upgrade| profile.owns(&upgrade.id))
            .filter_map(|upgrade| match upgrade.effect {
                EstateEffect::StartingPassive(kind) => Some(PassiveLoadout { kind, level: 1 }),
                EstateEffect::UnlockWeapon(_) => None,
            })
            .collect()
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let content: Self = ron::from_str(text).map_err(|error| error.to_string())?;
        content.validate()?;
        Ok(content)
    }

    fn validate(&self) -> Result<(), String> {
        let half = Vec2::from_array(self.half_size);
        ensure(
            half.is_finite() && half.min_element() >= 200.0,
            "estate half_size must be finite and at least 200 on each axis",
        )?;
        ensure(
            !self.expedition_weapons.is_empty(),
            "estate needs at least one expedition weapon",
        )?;
        ensure(
            self.expedition_weapons.iter().collect::<HashSet<_>>().len()
                == self.expedition_weapons.len(),
            "estate expedition weapons must be unique",
        )?;
        for building in &self.buildings {
            let center = Vec2::from_array(building.center);
            let size = Vec2::from_array(building.half_size);
            ensure(
                !building.name.trim().is_empty()
                    && center.is_finite()
                    && size.is_finite()
                    && size.min_element() > 0.0
                    && (center.abs() + size).cmple(half).all()
                    && building.color.iter().all(|value| value.is_finite()),
                &format!(
                    "building '{}' needs a name, a finite colour, and a positive size inside the estate",
                    building.name
                ),
            )?;
        }
        self.ensure_walkable("spawn", self.spawn)?;
        ensure(
            self.stations
                .iter()
                .any(|station| station.kind == StationKind::ExpeditionTable),
            "estate needs an ExpeditionTable station",
        )?;
        for station in &self.stations {
            ensure(
                !station.name.trim().is_empty(),
                "every station needs a name",
            )?;
            self.ensure_walkable(&format!("station '{}'", station.name), station.position)?;
        }
        let mut ids = HashSet::new();
        for npc in &self.npcs {
            ensure(
                !npc.id.trim().is_empty() && ids.insert(npc.id.as_str()),
                &format!("npc id '{}' must be non-empty and unique", npc.id),
            )?;
            ensure(
                !npc.name.trim().is_empty() && npc.color.iter().all(|value| value.is_finite()),
                &format!("npc '{}' needs a name and a finite colour", npc.id),
            )?;
            ensure(
                !npc.lines.is_empty()
                    && npc
                        .lines
                        .iter()
                        .chain(&npc.returning_lines)
                        .all(|line| !line.trim().is_empty()),
                &format!(
                    "npc '{}' needs at least one line and no blank lines",
                    npc.id
                ),
            )?;
            self.ensure_walkable(&format!("npc '{}'", npc.id), npc.position)?;
        }
        self.validate_upgrades()
    }

    fn validate_upgrades(&self) -> Result<(), String> {
        ensure(
            self.upgrades.is_empty()
                || self
                    .stations
                    .iter()
                    .any(|station| station.kind == StationKind::Workshop),
            "estate upgrades need a Workshop station to build them",
        )?;
        let mut ids = HashSet::new();
        let mut weapons: HashSet<_> = self.expedition_weapons.iter().copied().collect();
        let mut passives = HashSet::new();
        for upgrade in &self.upgrades {
            ensure(
                !upgrade.id.trim().is_empty() && ids.insert(upgrade.id.as_str()),
                &format!("upgrade id '{}' must be non-empty and unique", upgrade.id),
            )?;
            ensure(
                !upgrade.name.trim().is_empty() && !upgrade.description.trim().is_empty(),
                &format!("upgrade '{}' needs a name and a description", upgrade.id),
            )?;
            ensure(
                upgrade.cost != Stockpile::default(),
                &format!("upgrade '{}' must cost something", upgrade.id),
            )?;
            let fresh = match upgrade.effect {
                EstateEffect::UnlockWeapon(weapon) => weapons.insert(weapon),
                EstateEffect::StartingPassive(kind) => passives.insert(kind),
            };
            ensure(
                fresh,
                &format!(
                    "upgrade '{}' repeats a weapon or passive that is already granted",
                    upgrade.id
                ),
            )?;
        }
        ensure(
            passives.len() <= MAX_PASSIVES,
            "estate upgrades grant more starting passives than a run has slots",
        )
    }

    /// The point is inside the walls and clear of every building, so the
    /// player can stand on it or reach it.
    fn ensure_walkable(&self, label: &str, point: [f32; 2]) -> Result<(), String> {
        let point = Vec2::from_array(point);
        let half = Vec2::from_array(self.half_size);
        ensure(
            point.is_finite() && point.abs().cmplt(half - MARGIN).all(),
            &format!("{label} must be inside the estate walls"),
        )?;
        for building in &self.buildings {
            let offset = (point - Vec2::from_array(building.center)).abs();
            let reach = Vec2::from_array(building.half_size) + MARGIN;
            ensure(
                !offset.cmplt(reach).all(),
                &format!("{label} is inside or against building '{}'", building.name),
            )?;
        }
        Ok(())
    }
}

fn ensure(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}
