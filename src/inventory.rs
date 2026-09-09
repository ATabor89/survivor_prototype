//! Slot-limited passive inventory and calculated global player stats.
use crate::{components::*, GameplaySets};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;
use strum::EnumIter;

pub const MAX_WEAPONS: usize = 6;
pub const MAX_PASSIVES: usize = 6;
const EMBEDDED_ITEMS: &str = include_str!("../assets/data/items.ron");

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash, EnumIter)]
#[serde(rename_all = "snake_case")]
pub enum PassiveType {
    MoveSpeed,
    PickupRange,
    MaxHealth,
    Cooldown,
}

impl PassiveType {
    #[cfg(any(test, feature = "dev-tools"))]
    pub fn id(self) -> &'static str {
        match self {
            Self::MoveSpeed => "pilgrims_boots",
            Self::PickupRange => "lodestone",
            Self::MaxHealth => "vital_relic",
            Self::Cooldown => "hourglass",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PassiveItem {
    pub kind: PassiveType,
    pub level: u8,
}

#[derive(Component, Clone, Debug, Default)]
pub struct PassiveInventory {
    pub items: Vec<PassiveItem>,
}

impl PassiveInventory {
    pub fn level(&self, kind: PassiveType) -> u8 {
        self.items
            .iter()
            .find(|item| item.kind == kind)
            .map_or(0, |item| item.level)
    }

    pub fn can_upgrade(&self, kind: PassiveType, catalog: &PassiveCatalog) -> bool {
        let level = self.level(kind);
        (level > 0 || self.items.len() < MAX_PASSIVES) && level < catalog.get(kind).max_level()
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassiveDefinition {
    pub kind: PassiveType,
    pub name: String,
    pub description: String,
    /// Cumulative bonus at each level; fractions except MaxHealth (flat HP).
    pub levels: Vec<f32>,
}

impl PassiveDefinition {
    pub fn max_level(&self) -> u8 {
        self.levels.len() as u8
    }
    pub fn bonus(&self, level: u8) -> f32 {
        level
            .checked_sub(1)
            .and_then(|index| self.levels.get(index as usize))
            .copied()
            .unwrap_or(0.0)
    }
    pub fn level_description(&self, level: u8) -> String {
        let value = self.bonus(level);
        let effect = match self.kind {
            PassiveType::MoveSpeed => format!("+{:.0}% movement speed", value * 100.0),
            PassiveType::PickupRange => format!("+{:.0}% pickup range", value * 100.0),
            PassiveType::MaxHealth => format!("+{value:.0} maximum health"),
            PassiveType::Cooldown => format!("{:.0}% shorter weapon cooldowns", value * 100.0),
        };
        format!("Level {level}/{}: {effect}", self.max_level())
    }
}

#[derive(Resource, Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassiveCatalog {
    pub items: Vec<PassiveDefinition>,
}

impl Default for PassiveCatalog {
    fn default() -> Self {
        let explicit = std::env::var_os("BEVY_ASSET_ROOT")
            .map(|root| Path::new(&root).join("assets/data/items.ron"));
        let deployed = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|dir| dir.join("assets/data/items.ron")))
            .filter(|path| path.exists());
        let path = explicit
            .or(deployed)
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/data/items.ron"));
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error)
                if std::env::var_os("BEVY_ASSET_ROOT").is_some()
                    || error.kind() != std::io::ErrorKind::NotFound =>
            {
                panic!("Cannot load {}: {error}", path.display())
            }
            Err(_) => {
                warn!(
                    "{} missing; using bundled passive definitions",
                    path.display()
                );
                EMBEDDED_ITEMS.to_owned()
            }
        };
        Self::parse(&text).unwrap_or_else(|error| panic!("Invalid {}: {error}", path.display()))
    }
}

impl PassiveCatalog {
    pub fn parse(text: &str) -> Result<Self, String> {
        use strum::IntoEnumIterator;
        let catalog: Self = ron::from_str(text).map_err(|error| error.to_string())?;
        for kind in PassiveType::iter() {
            if catalog
                .items
                .iter()
                .filter(|item| item.kind == kind)
                .count()
                != 1
            {
                return Err(format!("passive {kind:?} must have exactly one definition"));
            }
        }
        for item in &catalog.items {
            if item.name.trim().is_empty()
                || item.description.trim().is_empty()
                || item.levels.is_empty()
                || item.levels.len() > u8::MAX as usize
            {
                return Err(format!(
                    "passive {:?} needs a name, description and 1..=255 levels",
                    item.kind
                ));
            }
            let mut previous = 0.0;
            for value in &item.levels {
                if !value.is_finite()
                    || *value <= previous
                    || (item.kind == PassiveType::Cooldown && *value > 0.8)
                {
                    return Err(format!("passive {:?}: bonuses must increase, be finite and positive; cooldown is capped at 80%", item.kind));
                }
                previous = *value;
            }
        }
        Ok(catalog)
    }

    pub fn get(&self, kind: PassiveType) -> &PassiveDefinition {
        self.items
            .iter()
            .find(|item| item.kind == kind)
            .expect("validated passive definition")
    }
}

#[derive(Component, Clone, Copy)]
pub struct PlayerBaseStats(pub PlayerStats);

#[derive(Message, Clone, Copy)]
pub struct PassiveUpgradeEvent {
    pub player: Entity,
    pub kind: PassiveType,
    pub level: u8,
}

pub struct InventoryPlugin;
impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PassiveCatalog>()
            .add_message::<PassiveUpgradeEvent>()
            .add_systems(
                Update,
                (initialize_inventories, apply_passive_events, refresh_stats)
                    .chain()
                    .before(GameplaySets::Weapons),
            );
    }
}

fn initialize_inventories(world: &mut World) {
    let players: Vec<_> = world
        .query_filtered::<Entity, (With<Player>, Without<PassiveInventory>)>()
        .iter(world)
        .collect();
    for player in players {
        ensure_inventory(world, player);
    }
}

fn ensure_inventory(world: &mut World, entity: Entity) {
    if world.get::<PassiveInventory>(entity).is_some() {
        return;
    }
    let Some(player) = world.get::<Player>(entity) else {
        return;
    };
    let Some(health) = world.get::<Health>(entity) else {
        return;
    };
    let stats = PlayerStats {
        move_speed: player.speed,
        pickup_range: player.magnet_strength,
        max_health: health.maximum,
        cooldown_reduction: world
            .get::<CooldownReduction>(entity)
            .map_or(0.0, |value| value.percent),
        damage_multiplier: world
            .get::<DamageMultiplier>(entity)
            .map_or(1.0, |value| value.factor),
        area_multiplier: world
            .get::<AreaMultiplier>(entity)
            .map_or(1.0, |value| value.factor),
        luck: world.get::<Luck>(entity).map_or(0, |luck| luck.0),
        armor: 0.0,
        regeneration: 0.0,
    };
    world
        .entity_mut(entity)
        .insert((PassiveInventory::default(), PlayerBaseStats(stats), stats));
}

/// Grants exactly the next passive level. Used by configured starting loadouts.
pub fn grant_passive(world: &mut World, player: Entity, kind: PassiveType) -> Result<(), String> {
    ensure_inventory(world, player);
    let Some(inventory) = world.get::<PassiveInventory>(player) else {
        return Err("Player is missing inventory/health".into());
    };
    let catalog = world.resource::<PassiveCatalog>();
    if !inventory.can_upgrade(kind, catalog) {
        return Err("Passive is capped or no slot is available".into());
    }
    let next_level = inventory.level(kind) + 1;
    let mut inventory = world
        .get_mut::<PassiveInventory>(player)
        .expect("inventory exists");
    if let Some(item) = inventory.items.iter_mut().find(|item| item.kind == kind) {
        item.level = next_level;
    } else {
        inventory.items.push(PassiveItem {
            kind,
            level: next_level,
        });
    }
    recalculate_player(world, player);
    Ok(())
}

fn apply_passive_events(mut events: MessageReader<PassiveUpgradeEvent>, mut commands: Commands) {
    for event in events.read().copied() {
        commands.queue(move |world: &mut World| {
            ensure_inventory(world, event.player);
            // Reject stale/duplicate offers instead of accidentally granting two levels.
            if world
                .get::<PassiveInventory>(event.player)
                .is_some_and(|inventory| {
                    inventory.level(event.kind).checked_add(1) == Some(event.level)
                })
            {
                if let Err(error) = grant_passive(world, event.player, event.kind) {
                    debug!("Passive upgrade rejected: {error}");
                }
            }
        });
    }
}

fn refresh_stats(world: &mut World) {
    let players: Vec<_> = world
        .query_filtered::<Entity, Changed<PassiveInventory>>()
        .iter(world)
        .collect();
    for player in players {
        recalculate_player(world, player);
    }
}

fn recalculate_player(world: &mut World, player: Entity) {
    let Some(base) = world.get::<PlayerBaseStats>(player) else {
        return;
    };
    let Some(inventory) = world.get::<PassiveInventory>(player) else {
        return;
    };
    let catalog = world.resource::<PassiveCatalog>();
    let mut stats = base.0;
    for item in &inventory.items {
        let bonus = catalog.get(item.kind).bonus(item.level);
        match item.kind {
            PassiveType::MoveSpeed => stats.move_speed *= 1.0 + bonus,
            PassiveType::PickupRange => stats.pickup_range *= 1.0 + bonus,
            PassiveType::MaxHealth => stats.max_health += bonus.round() as i32,
            PassiveType::Cooldown => {
                stats.cooldown_reduction = 1.0 - (1.0 - stats.cooldown_reduction) * (1.0 - bonus)
            }
        }
    }
    if let Some(mut data) = world.get_mut::<Player>(player) {
        data.speed = stats.move_speed;
        data.magnet_strength = stats.pickup_range;
    }
    if let Some(mut health) = world.get_mut::<Health>(player) {
        let increase = (stats.max_health - health.maximum).max(0);
        health.maximum = stats.max_health;
        health.current = (health.current + increase).min(health.maximum);
    }
    world.entity_mut(player).insert((
        stats,
        CooldownReduction {
            percent: stats.cooldown_reduction,
        },
        DamageMultiplier {
            factor: stats.damage_multiplier,
        },
        AreaMultiplier {
            factor: stats.area_multiplier,
        },
        Luck(stats.luck),
    ));
}
