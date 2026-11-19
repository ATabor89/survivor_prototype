//! Authored estate layout and dialogue, validated like the stage data.
use crate::inventory::{PassiveCatalog, PassiveType, MAX_PASSIVES};
use crate::profile::{Profile, ResourceKind, Stockpile};
use crate::run_modifiers::RunModifiers;
use crate::setup::{PassiveLoadout, WeaponLoadout};
use crate::weapons::WeaponType;
use bevy::prelude::*;
use serde::Deserialize;
use std::collections::HashSet;

pub const EMBEDDED_ESTATE: &str = include_str!("../../assets/data/estate.ron");

/// Clearance kept between walkable points and walls or buildings.
const MARGIN: f32 = 24.0;

/// How many provisions can be packed for one expedition.
pub const PROVISION_SLOTS: usize = 2;

/// Largest total haul bonus one resource can reach from upgrades, in percent.
const MAX_UPGRADE_HAUL_BONUS: u32 = 200;

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
    /// Single-expedition supplies sold at the provisioner.
    #[serde(default)]
    pub provisions: Vec<ProvisionDefinition>,
    /// Rites performed at the altar before an expedition.
    #[serde(default)]
    pub rituals: Vec<RitualDefinition>,
}

/// A lasting estate improvement bought with expedition resources, one rank at a time.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpgradeDefinition {
    /// Stable identifier stored in the saved profile.
    pub id: String,
    pub name: String,
    pub description: String,
    /// The price of each rank, in order. A one-time upgrade lists one price.
    pub ranks: Vec<Stockpile>,
    /// Upgrades that must be built first. They must be listed earlier.
    #[serde(default)]
    pub requires: Vec<String>,
    pub effect: EstateEffect,
}

impl UpgradeDefinition {
    pub fn max_rank(&self) -> u32 {
        self.ranks.len() as u32
    }

    /// The price of the rank after `built`, or `None` when fully built.
    pub fn next_cost(&self, built: u32) -> Option<&Stockpile> {
        self.ranks.get(built as usize)
    }
}

/// How a built upgrade changes later expeditions. Every effect is expressed
/// through the run recipe or the haul bonus, so expeditions need no knowledge
/// of the estate.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum EstateEffect {
    /// Adds a starting-weapon choice at the expedition table. One rank only.
    UnlockWeapon(WeaponType),
    /// Every estate expedition begins with this passive, one level per rank.
    StartingPassive(PassiveType),
    /// Estate expeditions bring home this much more of a resource per rank, in percent.
    HaulBonus(ResourceKind, u32),
}

/// A supply bought for the next estate expedition only.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvisionDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub cost: Stockpile,
    pub effect: ProvisionEffect,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum ProvisionEffect {
    /// One more level of this passive, on top of any estate upgrade.
    Passive(PassiveType),
    /// Carry this weapon as well, or one level higher if it is already the
    /// chosen starting weapon.
    Weapon(WeaponType),
}

/// A rite for the next estate expedition: it changes how hard the run is and
/// how much it brings home.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RitualDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub cost: Stockpile,
    /// Upgrades that must be built before the rite is known.
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub modifiers: RunModifiers,
    /// Extra haul in percent per resource, added to upgrade bonuses.
    #[serde(default)]
    pub haul_bonus: Stockpile,
}

/// What an estate expedition starts with, gathered from upgrades, packed
/// provisions, and the prepared ritual.
#[derive(Clone, Debug, PartialEq)]
pub struct Outfit {
    pub weapons: Vec<WeaponLoadout>,
    pub passives: Vec<PassiveLoadout>,
    pub modifiers: RunModifiers,
    /// Haul bonus in percent per resource.
    pub haul_bonus: Stockpile,
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
    /// Opens the shop that sells provisions for the next expedition.
    Provisioner,
    /// Opens the altar where a ritual is prepared for the next expedition.
    Altar,
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
                _ => None,
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
            .filter_map(|upgrade| match upgrade.effect {
                EstateEffect::StartingPassive(kind) if profile.owns(&upgrade.id) => {
                    Some(PassiveLoadout {
                        kind,
                        level: u8::try_from(profile.rank(&upgrade.id)).unwrap_or(u8::MAX),
                    })
                }
                _ => None,
            })
            .collect()
    }

    /// The haul bonus from built upgrades alone, in percent per resource.
    pub fn upgrade_haul_bonus(&self, profile: &Profile) -> Stockpile {
        let mut bonus = Stockpile::default();
        for upgrade in &self.upgrades {
            if let EstateEffect::HaulBonus(kind, percent) = upgrade.effect {
                bonus.add(&Stockpile::of(
                    kind,
                    percent.saturating_mul(profile.rank(&upgrade.id)),
                ));
            }
        }
        bonus
    }

    pub fn provision(&self, id: &str) -> Option<&ProvisionDefinition> {
        self.provisions.iter().find(|provision| provision.id == id)
    }

    pub fn ritual(&self, id: &str) -> Option<&RitualDefinition> {
        self.rituals.iter().find(|ritual| ritual.id == id)
    }

    /// The ritual prepared for the next expedition, if it still exists.
    pub fn prepared_ritual(&self, profile: &Profile) -> Option<&RitualDefinition> {
        profile.ritual.as_deref().and_then(|id| self.ritual(id))
    }

    /// Names of the upgrades in `requires` that are not built yet.
    pub fn missing_requirements<'a>(
        &'a self,
        requires: &'a [String],
        profile: &Profile,
    ) -> Vec<&'a str> {
        requires
            .iter()
            .filter(|id| !profile.owns(id))
            .map(|id| {
                self.upgrades
                    .iter()
                    .find(|upgrade| upgrade.id == **id)
                    .map_or(id.as_str(), |upgrade| upgrade.name.as_str())
            })
            .collect()
    }

    /// Everything an estate expedition with `weapon` starts with. Levels are
    /// capped at what the passive or weapon allows.
    pub fn outfit(
        &self,
        profile: &Profile,
        weapon: WeaponType,
        catalog: &PassiveCatalog,
        max_weapon_level: impl Fn(WeaponType) -> u32,
    ) -> Outfit {
        let mut weapons = vec![WeaponLoadout {
            weapon,
            level: 1,
            patterns: vec![],
        }];
        let mut passives = self.starting_passives(profile);
        for provision in profile
            .provisions
            .iter()
            .filter_map(|id| self.provision(id))
        {
            match provision.effect {
                ProvisionEffect::Passive(kind) => {
                    match passives.iter_mut().find(|passive| passive.kind == kind) {
                        Some(passive) => passive.level = passive.level.saturating_add(1),
                        None => passives.push(PassiveLoadout { kind, level: 1 }),
                    }
                }
                ProvisionEffect::Weapon(extra) => {
                    match weapons.iter_mut().find(|loadout| loadout.weapon == extra) {
                        Some(loadout) => loadout.level += 1,
                        None => weapons.push(WeaponLoadout {
                            weapon: extra,
                            level: 1,
                            patterns: vec![],
                        }),
                    }
                }
            }
        }
        for passive in &mut passives {
            passive.level = passive.level.min(catalog.get(passive.kind).max_level());
        }
        for loadout in &mut weapons {
            loadout.level = loadout.level.min(max_weapon_level(loadout.weapon));
        }
        let ritual = self.prepared_ritual(profile);
        let mut haul_bonus = self.upgrade_haul_bonus(profile);
        if let Some(ritual) = ritual {
            haul_bonus.add(&ritual.haul_bonus);
        }
        Outfit {
            weapons,
            passives,
            modifiers: ritual.map(|ritual| ritual.modifiers).unwrap_or_default(),
            haul_bonus,
        }
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
        self.validate_upgrades()?;
        self.validate_provisions()?;
        self.validate_rituals()
    }

    fn has_station(&self, kind: StationKind) -> bool {
        self.stations.iter().any(|station| station.kind == kind)
    }

    fn validate_upgrades(&self) -> Result<(), String> {
        ensure(
            self.upgrades.is_empty() || self.has_station(StationKind::Workshop),
            "estate upgrades need a Workshop station to build them",
        )?;
        let mut ids = HashSet::new();
        let mut weapons: HashSet<_> = self.expedition_weapons.iter().copied().collect();
        let mut passives = HashSet::new();
        let mut haul_bonus = Stockpile::default();
        for upgrade in &self.upgrades {
            ensure(
                !upgrade.id.trim().is_empty() && !ids.contains(upgrade.id.as_str()),
                &format!("upgrade id '{}' must be non-empty and unique", upgrade.id),
            )?;
            ensure(
                !upgrade.name.trim().is_empty() && !upgrade.description.trim().is_empty(),
                &format!("upgrade '{}' needs a name and a description", upgrade.id),
            )?;
            ensure(
                !upgrade.ranks.is_empty()
                    && upgrade
                        .ranks
                        .iter()
                        .all(|cost| *cost != Stockpile::default()),
                &format!(
                    "upgrade '{}' needs at least one rank, and every rank must cost something",
                    upgrade.id
                ),
            )?;
            for required in &upgrade.requires {
                ensure(
                    ids.contains(required.as_str()),
                    &format!(
                        "upgrade '{}' requires '{required}', which must be an upgrade listed before it",
                        upgrade.id
                    ),
                )?;
            }
            let fresh = match upgrade.effect {
                EstateEffect::UnlockWeapon(weapon) => {
                    ensure(
                        upgrade.ranks.len() == 1,
                        &format!(
                            "upgrade '{}' unlocks a weapon and can only have one rank",
                            upgrade.id
                        ),
                    )?;
                    weapons.insert(weapon)
                }
                EstateEffect::StartingPassive(kind) => passives.insert(kind),
                EstateEffect::HaulBonus(kind, percent) => {
                    ensure(
                        percent > 0,
                        &format!("upgrade '{}' needs a positive haul bonus", upgrade.id),
                    )?;
                    haul_bonus.add(&Stockpile::of(
                        kind,
                        percent.saturating_mul(upgrade.max_rank()),
                    ));
                    true
                }
            };
            ensure(
                fresh,
                &format!(
                    "upgrade '{}' repeats a weapon or passive that is already granted",
                    upgrade.id
                ),
            )?;
            ids.insert(upgrade.id.as_str());
        }
        ensure(
            haul_bonus.max() <= MAX_UPGRADE_HAUL_BONUS,
            &format!("estate upgrades add more than {MAX_UPGRADE_HAUL_BONUS}% to one resource"),
        )?;
        for provision in &self.provisions {
            if let ProvisionEffect::Passive(kind) = provision.effect {
                passives.insert(kind);
            }
        }
        ensure(
            passives.len() <= MAX_PASSIVES,
            "estate upgrades and provisions grant more starting passives than a run has slots",
        )
    }

    fn validate_provisions(&self) -> Result<(), String> {
        ensure(
            self.provisions.is_empty() || self.has_station(StationKind::Provisioner),
            "estate provisions need a Provisioner station to sell them",
        )?;
        let mut ids = HashSet::new();
        for provision in &self.provisions {
            ensure(
                !provision.id.trim().is_empty() && ids.insert(provision.id.as_str()),
                &format!(
                    "provision id '{}' must be non-empty and unique",
                    provision.id
                ),
            )?;
            ensure(
                !provision.name.trim().is_empty() && !provision.description.trim().is_empty(),
                &format!(
                    "provision '{}' needs a name and a description",
                    provision.id
                ),
            )?;
            ensure(
                provision.cost != Stockpile::default(),
                &format!("provision '{}' must cost something", provision.id),
            )?;
        }
        Ok(())
    }

    fn validate_rituals(&self) -> Result<(), String> {
        ensure(
            self.rituals.is_empty() || self.has_station(StationKind::Altar),
            "estate rituals need an Altar station to perform them",
        )?;
        let mut ids = HashSet::new();
        for ritual in &self.rituals {
            ensure(
                !ritual.id.trim().is_empty() && ids.insert(ritual.id.as_str()),
                &format!("ritual id '{}' must be non-empty and unique", ritual.id),
            )?;
            ensure(
                !ritual.name.trim().is_empty() && !ritual.description.trim().is_empty(),
                &format!("ritual '{}' needs a name and a description", ritual.id),
            )?;
            ensure(
                ritual.cost != Stockpile::default(),
                &format!("ritual '{}' must cost something", ritual.id),
            )?;
            ritual
                .modifiers
                .validate()
                .map_err(|error| format!("ritual '{}': {error}", ritual.id))?;
            ensure(
                !ritual.modifiers.is_neutral() || ritual.haul_bonus != Stockpile::default(),
                &format!("ritual '{}' must change the run or its haul", ritual.id),
            )?;
            for required in &ritual.requires {
                ensure(
                    self.upgrades.iter().any(|upgrade| &upgrade.id == required),
                    &format!(
                        "ritual '{}' requires unknown upgrade '{required}'",
                        ritual.id
                    ),
                )?;
            }
        }
        Ok(())
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
