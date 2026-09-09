//! Stage props and tangible rewards. Rapier supplies every hit and pickup overlap.
use crate::arena::{camera_half_view, ArenaBounds, GameCamera, WORLD_GROUP};
use crate::components::{CooldownReduction, Health, Player};
use crate::content::{ActiveStage, GameContent};
use crate::death::{MarkedForDeath, MarkedForDespawn};
use crate::events::EntityDeathEvent;
use crate::inventory::{PassiveCatalog, PassiveInventory, PassiveUpgradeEvent};
use crate::menu::{GenericUpgradeConfirmedEvent, WeaponUpgradeConfirmedEvent};
use crate::resources::{GameState, GameStats};
use crate::run::RunEntity;
use crate::setup::RunRng;
use crate::upgrade::{GenericUpgrade, UpgradeType};
use crate::weapons::magick_circle::PatternType;
use crate::weapons::weapon_upgrade::{WeaponUpgradeChange, WeaponUpgradeConfig};
use crate::weapons::{WeaponCooldown, WeaponMeta, WeaponType};
use crate::GameplaySets;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::{Rng, RngExt};
use strum::IntoEnumIterator;

pub const BREAKABLE_GROUP: Group = Group::GROUP_6;
const MAX_LOOSE_PICKUPS: usize = 96;

#[derive(Component)]
pub struct ChestDrop;

#[derive(Component)]
pub struct Breakable {
    pub drop_chance: f32,
    pub drop: LootKind,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum LootKind {
    Healing,
    Gold,
    Chest,
}

#[derive(Resource)]
struct LootDirector {
    seeded: bool,
    timer: Timer,
}

impl Default for LootDirector {
    fn default() -> Self {
        Self {
            seeded: false,
            timer: Timer::from_seconds(4.0, TimerMode::Repeating),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TreasureReward {
    pub upgrade: UpgradeType,
    pub description: String,
}

#[derive(Resource, Default)]
pub struct PendingTreasure {
    pub rewards: Vec<TreasureReward>,
    revealed: usize,
    elapsed: f32,
    input_ready: bool,
}

#[derive(Component)]
struct TreasureOverlay;
#[derive(Component)]
struct TreasureButton;
#[derive(Component)]
struct TreasureButtonText;
#[derive(Component)]
struct TreasureLine(usize);

pub struct LootPlugin;
impl Plugin for LootPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LootDirector>()
            .init_resource::<PendingTreasure>()
            .add_systems(
                Update,
                replenish_props
                    .in_set(GameplaySets::Spawning)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (drop_loot, collect_loot)
                    .chain()
                    .in_set(GameplaySets::Experience)
                    .after(crate::experience::check_level_up)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnEnter(GameState::Treasure), spawn_treasure_overlay)
            .add_systems(
                Update,
                (treasure_input, update_treasure_overlay)
                    .chain()
                    .run_if(in_state(GameState::Treasure)),
            )
            .add_systems(OnExit(GameState::Treasure), cleanup_treasure_overlay);
    }
}

pub fn reset_loot(world: &mut World) {
    *world.resource_mut::<LootDirector>() = LootDirector::default();
    *world.resource_mut::<PendingTreasure>() = PendingTreasure::default();
}

/// Props are non-blocking fixed sensors: they can be damaged but are never enemies.
pub(crate) fn spawn_breakable(
    commands: &mut Commands,
    position: Vec2,
    health: i32,
    drop_chance: f32,
    name: &str,
    drop: LootKind,
) -> Entity {
    let lower = name.to_lowercase();
    let (color, size) = if lower.contains("urn") || lower.contains("pot") {
        (Color::srgb(0.65, 0.36, 0.21), Vec2::new(25.0, 30.0))
    } else if lower.contains("candle") || lower.contains("lantern") {
        (Color::srgb(0.76, 0.62, 0.24), Vec2::new(16.0, 35.0))
    } else {
        (Color::srgb(0.30, 0.72, 0.76), Vec2::new(22.0, 30.0))
    };
    commands
        .spawn((
            RunEntity,
            Name::new(name.to_owned()),
            Breakable { drop_chance, drop },
            Health {
                current: health.max(1),
                maximum: health.max(1),
            },
            Sprite::from_color(color, size),
            crate::art::ItemIcon(if lower.contains("candle") || lower.contains("lantern") {
                11
            } else {
                10
            }),
            Transform::from_translation(position.extend(1.0)),
            RigidBody::Fixed,
            Collider::ball(14.0),
            Sensor,
            ActiveCollisionTypes::STATIC_STATIC,
            CollisionGroups::new(BREAKABLE_GROUP, Group::GROUP_3),
        ))
        .id()
}

pub(crate) fn spawn_loot(commands: &mut Commands, position: Vec2, kind: LootKind) -> Entity {
    let (color, size, name) = match kind {
        LootKind::Healing => (
            Color::srgb(0.85, 0.15, 0.24),
            Vec2::new(15.0, 21.0),
            "Healing flask",
        ),
        LootKind::Gold => (Color::srgb(1.0, 0.78, 0.15), Vec2::splat(13.0), "Gold coin"),
        LootKind::Chest => (
            Color::srgb(0.55, 0.30, 0.12),
            Vec2::new(32.0, 24.0),
            "Treasure chest",
        ),
    };
    commands
        .spawn((
            RunEntity,
            kind,
            Name::new(name),
            Sprite::from_color(color, size),
            crate::art::ItemIcon(match kind {
                LootKind::Healing => 7,
                LootKind::Chest => 8,
                LootKind::Gold => 9,
            }),
            Transform::from_translation(position.extend(2.0)),
            RigidBody::Fixed,
            Collider::ball(12.0),
            Sensor,
            ActiveCollisionTypes::KINEMATIC_STATIC,
            CollisionGroups::new(Group::GROUP_4, Group::GROUP_1),
        ))
        .id()
}

#[expect(
    clippy::too_many_arguments,
    reason = "Bevy injects independent stage, spatial-query, timing, and entity access."
)]
fn replenish_props(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    content: Res<GameContent>,
    active: Res<ActiveStage>,
    bounds: Res<ArenaBounds>,
    mut director: ResMut<LootDirector>,
    camera: Query<(&Transform, &Projection), With<GameCamera>>,
    props: Query<(Entity, &Transform), With<Breakable>>,
    context: ReadRapierContext,
    mut rng: ResMut<RunRng>,
) {
    let stage = content.stage(&active);
    if !director.seeded {
        for pickup in &stage.pickups {
            let kind = match pickup.kind.as_str() {
                "healing" => LootKind::Healing,
                "gold" => LootKind::Gold,
                "chest" => LootKind::Chest,
                _ => continue, // Content validation rejects unsupported kinds.
            };
            spawn_loot(&mut commands, Vec2::from_array(pickup.position), kind);
        }
        director.seeded = true;
    }
    let config = &stage.breakables;
    director
        .timer
        .set_duration(std::time::Duration::from_secs_f32(config.respawn_seconds));
    if !director.timer.tick(time.delta()).just_finished() {
        return;
    }
    let Ok((camera, projection)) = camera.single() else {
        return;
    };
    let Ok(context) = context.single() else {
        return;
    };
    let center = camera.translation.truncate();
    let view = camera_half_view(projection);
    let mut count = 0;
    for (entity, transform) in &props {
        if (transform.translation.truncate() - center)
            .abs()
            .cmpgt(view + Vec2::splat(650.0))
            .any()
        {
            commands.entity(entity).despawn();
        } else {
            count += 1;
        }
    }
    let collider = Collider::ball(24.0);
    for _ in count..config.count {
        let mut candidate = None;
        for _ in 0..24 {
            let side = rng.loot.random_range(0..4);
            let along = rng.loot.random_range(-1.0..1.0);
            let distance = rng.loot.random_range(80.0..320.0);
            let offset = match side {
                0 => Vec2::new(-view.x - distance, along * (view.y + 120.0)),
                1 => Vec2::new(view.x + distance, along * (view.y + 120.0)),
                2 => Vec2::new(along * (view.x + 120.0), -view.y - distance),
                _ => Vec2::new(along * (view.x + 120.0), view.y + distance),
            };
            let position = center + offset;
            if position
                .abs()
                .cmpgt(bounds.half_size - Vec2::splat(35.0))
                .any()
            {
                continue;
            }
            let mut occupied = false;
            // Query Rapier's world geometry and existing props. Sensor props
            // don't block movement, but should not be placed inside one another.
            context.intersect_shape(
                position,
                0.0,
                collider.raw.as_ref(),
                QueryFilter::default().groups(CollisionGroups::new(
                    Group::GROUP_1 | Group::GROUP_3,
                    WORLD_GROUP | BREAKABLE_GROUP,
                )),
                |_| {
                    occupied = true;
                    false
                },
            );
            if !occupied {
                candidate = Some(position);
                break;
            }
        }
        let Some(position) = candidate else {
            continue;
        };
        let name = config
            .names
            .get(rng.loot.random_range(0..config.names.len().max(1)))
            .map_or("Clay urn", String::as_str);
        let drop = if rng.loot.random::<f32>() < 0.35 {
            LootKind::Healing
        } else {
            LootKind::Gold
        };
        spawn_breakable(
            &mut commands,
            position,
            config.health,
            config.drop_chance,
            name,
            drop,
        );
    }
}

fn drop_loot(
    mut commands: Commands,
    mut deaths: MessageReader<EntityDeathEvent>,
    props: Query<&Breakable>,
    bosses: Query<(), With<ChestDrop>>,
    pickups: Query<&LootKind>,
    mut rng: ResMut<RunRng>,
) {
    let mut loose_count = pickups
        .iter()
        .filter(|kind| **kind != LootKind::Chest)
        .count();
    for event in deaths.read() {
        if bosses.contains(event.entity) {
            spawn_loot(&mut commands, event.position, LootKind::Chest);
        } else if let Ok(prop) = props.get(event.entity) {
            if loose_count < MAX_LOOSE_PICKUPS && rng.loot.random::<f32>() < prop.drop_chance {
                spawn_loot(&mut commands, event.position, prop.drop);
                loose_count += 1;
            }
        }
    }
}

struct SimulatedWeapon {
    weapon: WeaponType,
    level: u32,
    cooldown_bonus: i32,
    base_duration: f32,
}

fn roll_rewards(
    config: &WeaponUpgradeConfig,
    catalog: &PassiveCatalog,
    weapons: &mut [SimulatedWeapon],
    passives: &mut PassiveInventory,
    count: usize,
    player_reduction: f32,
    rng: &mut impl Rng,
) -> Vec<TreasureReward> {
    let patterns: Vec<_> = PatternType::iter().collect();
    let mut rewards = Vec::new();
    for _ in 0..count {
        let mut offers = Vec::new();
        for weapon in weapons.iter() {
            let cooldown = WeaponCooldown {
                base_duration: weapon.base_duration,
                cooldown_bonus: weapon.cooldown_bonus,
                ..default()
            };
            for upgrade in config.get_next_upgrades(weapon.weapon, weapon.level) {
                if !upgrade
                    .changes
                    .iter()
                    .all(|change| matches!(change, WeaponUpgradeChange::Cooldown(_)))
                    || cooldown.effective_seconds(player_reduction) > WeaponCooldown::MIN_SECONDS
                {
                    offers.push(UpgradeType::Weapon(weapon.weapon, upgrade));
                }
            }
        }
        for item in &passives.items {
            if item.level < catalog.get(item.kind).max_level() {
                offers.push(UpgradeType::Passive(item.kind, item.level + 1));
            }
        }
        let mut upgrade = if offers.is_empty() {
            UpgradeType::Generic(GenericUpgrade::GoldPickup(25))
        } else {
            offers.swap_remove(rng.random_range(0..offers.len()))
        };
        // Simulate each awarded level before rolling again. A multi-reward chest
        // follows real progression and cannot award the same capped level twice.
        let description = match &mut upgrade {
            UpgradeType::Weapon(kind, spec) => {
                let weapon = weapons
                    .iter_mut()
                    .find(|weapon| weapon.weapon == *kind)
                    .expect("owned weapon reward");
                for change in &mut spec.changes {
                    match change {
                        WeaponUpgradeChange::AddCircle { pattern } if pattern.is_none() => {
                            *pattern = Some(patterns[rng.random_range(0..patterns.len())])
                        }
                        WeaponUpgradeChange::Cooldown(bonus) => {
                            weapon.cooldown_bonus = weapon.cooldown_bonus.saturating_add(*bonus)
                        }
                        _ => {}
                    }
                }
                weapon.level += 1;
                let changes = spec
                    .changes
                    .iter()
                    .map(|change| match change {
                        WeaponUpgradeChange::AddCircle {
                            pattern: Some(pattern),
                        } => format!("Add {pattern} circle"),
                        _ => change.to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                format!("{} level {}: {changes}", weapon.weapon, weapon.level)
            }
            UpgradeType::Passive(kind, level) => {
                passives
                    .items
                    .iter_mut()
                    .find(|item| item.kind == *kind)
                    .expect("owned passive reward")
                    .level = *level;
                let definition = catalog.get(*kind);
                format!(
                    "{} - {}",
                    definition.name,
                    definition.level_description(*level)
                )
            }
            UpgradeType::Generic(GenericUpgrade::GoldPickup(amount)) => {
                format!("Gold purse: +{amount} gold")
            }
            _ => unreachable!("chests upgrade owned items or award gold"),
        };
        rewards.push(TreasureReward {
            upgrade,
            description,
        });
    }
    rewards
}

#[expect(
    clippy::too_many_arguments,
    reason = "Bevy tracks pickup, progression, player, and Rapier state independently."
)]
#[expect(
    clippy::type_complexity,
    reason = "The player query excludes entities already killed or scheduled for removal."
)]
fn collect_loot(
    mut commands: Commands,
    mut players: Query<
        (Entity, &mut Health, &CooldownReduction),
        (
            With<Player>,
            Without<MarkedForDeath>,
            Without<MarkedForDespawn>,
        ),
    >,
    pickups: Query<(Entity, &LootKind), Without<MarkedForDespawn>>,
    weapons: Query<(&WeaponMeta, &WeaponCooldown)>,
    config: Res<WeaponUpgradeConfig>,
    mut stats: ResMut<GameStats>,
    mut pending: ResMut<PendingTreasure>,
    mut next: ResMut<NextState<GameState>>,
    context: ReadRapierContext,
    catalog: Res<PassiveCatalog>,
    passives: Query<&PassiveInventory, With<Player>>,
    mut rng: ResMut<RunRng>,
) {
    let Ok((player, mut health, reduction)) = players.single_mut() else {
        return;
    };
    if health.current <= 0 {
        return;
    }
    let Ok(context) = context.single() else {
        return;
    };
    let mut touching: Vec<_> = context
        .intersection_pairs_with(player)
        .filter_map(|(a, b, touching)| {
            if touching {
                Some(if a == player { b } else { a })
            } else {
                None
            }
        })
        .collect();
    touching.sort_unstable();
    touching.dedup();
    for entity in touching {
        let Ok((entity, kind)) = pickups.get(entity) else {
            continue;
        };
        match kind {
            LootKind::Healing => {
                health.current = health.current.saturating_add(20).min(health.maximum)
            }
            LootKind::Gold => stats.gold = stats.gold.saturating_add(1),
            LootKind::Chest => {
                if !matches!(*next, NextState::Unchanged) || !pending.rewards.is_empty() {
                    continue;
                }
                let mut candidates: Vec<_> = weapons
                    .iter()
                    .map(|(meta, cooldown)| SimulatedWeapon {
                        weapon: meta.weapon_type,
                        level: meta.level,
                        cooldown_bonus: cooldown.cooldown_bonus,
                        base_duration: cooldown.base_duration,
                    })
                    .collect();
                let roll = rng.upgrades.random::<f32>();
                let count = if roll < 0.05 {
                    5
                } else if roll < 0.30 {
                    3
                } else {
                    1
                };
                let mut simulated_passives = passives.get(player).cloned().unwrap_or_default();
                let rewards = roll_rewards(
                    &config,
                    &catalog,
                    &mut candidates,
                    &mut simulated_passives,
                    count,
                    reduction.percent,
                    &mut rng.upgrades,
                );
                if rewards.is_empty() {
                    continue;
                }
                *pending = PendingTreasure {
                    rewards,
                    ..default()
                };
                next.set(GameState::Treasure);
            }
        }
        commands.entity(entity).insert(MarkedForDespawn);
    }
}

fn spawn_treasure_overlay(mut commands: Commands, pending: Res<PendingTreasure>) {
    commands
        .spawn((
            TreasureOverlay,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.018, 0.045, 0.93)),
            GlobalZIndex(20),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: px(680),
                        max_width: percent(95),
                        padding: UiRect::all(px(30)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(16),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.12, 0.08, 0.16)),
                    BorderColor::all(Color::srgb(0.85, 0.65, 0.22)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("TREASURE RECLAIMED"),
                        TextFont {
                            font_size: FontSize::Px(32.0),
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.82, 0.34)),
                    ));
                    panel.spawn((
                        Text::new(format!(
                            "{} treasure reward{}",
                            pending.rewards.len(),
                            if pending.rewards.len() == 1 { "" } else { "s" }
                        )),
                        TextFont {
                            font_size: FontSize::Px(20.0),
                            ..default()
                        },
                    ));
                    for index in 0..pending.rewards.len() {
                        panel.spawn((
                            TreasureLine(index),
                            Text::new("Sealed reward..."),
                            TextFont {
                                font_size: FontSize::Px(20.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.89, 0.85, 0.95)),
                        ));
                    }
                    panel
                        .spawn((
                            TreasureButton,
                            Button,
                            Node {
                                padding: UiRect::axes(px(20), px(14)),
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.44, 0.31, 0.10)),
                        ))
                        .with_children(|button| {
                            button.spawn((
                                TreasureButtonText,
                                Text::new("Reveal all  [Enter / A]"),
                                TextFont {
                                    font_size: FontSize::Px(22.0),
                                    ..default()
                                },
                            ));
                        });
                    panel.spawn((
                        Text::new("The run is paused. Reveal, then claim your upgrades."),
                        TextFont {
                            font_size: FontSize::Px(16.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.67, 0.64, 0.74)),
                    ));
                });
        });
}

#[expect(
    clippy::too_many_arguments,
    reason = "Treasure confirmation routes typed reward messages to their owning systems."
)]
fn treasure_input(
    mut pending: ResMut<PendingTreasure>,
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    button: Query<&Interaction, (With<TreasureButton>, Changed<Interaction>)>,
    mut upgrades: MessageWriter<WeaponUpgradeConfirmedEvent>,
    mut next: ResMut<NextState<GameState>>,
    players: Query<Entity, With<Player>>,
    mut passives: MessageWriter<PassiveUpgradeEvent>,
    mut generics: MessageWriter<GenericUpgradeConfirmedEvent>,
    #[cfg(feature = "dev-tools")] debug: Res<crate::dev_tools::DebugSession>,
) {
    if !pending.input_ready {
        pending.input_ready = true;
        return;
    }
    #[cfg(feature = "dev-tools")]
    let (keyboard_allowed, pointer_allowed) = (!debug.blocks_keyboard(), !debug.blocks_pointer());
    #[cfg(not(feature = "dev-tools"))]
    let (keyboard_allowed, pointer_allowed) = (true, true);
    let activate = (keyboard_allowed
        && (keyboard.just_pressed(KeyCode::Enter)
            || keyboard.just_pressed(KeyCode::Space)
            || gamepads
                .iter()
                .any(|pad| pad.just_pressed(GamepadButton::South))))
        || (pointer_allowed
            && button
                .iter()
                .any(|interaction| *interaction == Interaction::Pressed));
    if !activate {
        return;
    }
    if pending.revealed < pending.rewards.len() {
        pending.revealed = pending.rewards.len();
    } else {
        for reward in pending.rewards.drain(..) {
            match reward.upgrade {
                UpgradeType::Weapon(weapon_type, upgrade_spec) => {
                    upgrades.write(WeaponUpgradeConfirmedEvent {
                        weapon_type,
                        upgrade_spec,
                    });
                }
                UpgradeType::Passive(kind, level) => {
                    if let Ok(player) = players.single() {
                        passives.write(PassiveUpgradeEvent {
                            player,
                            kind,
                            level,
                        });
                    }
                }
                UpgradeType::Generic(generic_upgrade_type) => {
                    generics.write(GenericUpgradeConfirmedEvent {
                        generic_upgrade_type,
                    });
                }
                UpgradeType::AcquireWeapon(_) => unreachable!("treasure only upgrades owned items"),
            }
        }
        next.set(GameState::Playing);
    }
}

fn update_treasure_overlay(
    time: Res<Time<bevy::time::Real>>,
    mut pending: ResMut<PendingTreasure>,
    mut lines: Query<(&TreasureLine, &mut Text), Without<TreasureButtonText>>,
    mut button: Query<&mut Text, With<TreasureButtonText>>,
) {
    pending.elapsed += time.delta_secs();
    pending.revealed = pending
        .revealed
        .max((pending.elapsed / 0.45) as usize)
        .min(pending.rewards.len());
    for (line, mut text) in &mut lines {
        if line.0 < pending.revealed {
            **text = pending.rewards[line.0].description.clone();
        }
    }
    for mut text in &mut button {
        **text = if pending.revealed >= pending.rewards.len() {
            "Claim rewards  [Enter / A]"
        } else {
            "Reveal all  [Enter / A]"
        }
        .into();
    }
}

fn cleanup_treasure_overlay(
    mut commands: Commands,
    overlays: Query<Entity, With<TreasureOverlay>>,
    mut pending: ResMut<PendingTreasure>,
) {
    for entity in &overlays {
        commands.entity(entity).despawn();
    }
    *pending = PendingTreasure::default();
}
