use crate::combat::DamageEvent;
use crate::components::{AreaMultiplier, CooldownReduction, DamageMultiplier, Enemy, Player};
use crate::death::MarkedForDeath;
use crate::menu::UpgradeConfirmedEvent;
use crate::physics::handle_rapier_context_error;
use crate::resources::GameState;
use crate::upgrade::UpgradeType;
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_rapier2d::prelude::*;
use std::fmt::Formatter;
use std::time::Duration;
use strum_macros::EnumIter;
use crate::weapons::magick_circle::{spawn_magick_circle, spawn_magick_circle_attack, MagickCircle, MagickCircleUpgrade, PatternType};

mod magick_circle;

pub static MAX_WEAPON_LEVEL: u8 = 8;

/// Plugin to register all weapon-related systems
pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<AddWeaponEvent>()
            .add_event::<AreaEffectEvent>()
            .add_event::<BindingEvent>()
            .add_systems(
                Update,
                (
                    setup_player_inventory,
                    handle_new_weapons,
                    handle_weapon_upgrade,
                    weapon_firing_system,
                    update_weapon_positions,
                    area_effect_system,
                    handle_binding_events,
                    attack_lifetime_system,
                    attack_rotation_system,
                    orbital_movement_system,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Core weapon type identifier
#[derive(Component)]
pub struct Weapon {
    pub weapon_type: WeaponType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponStat {
    Damage,
    Area,
    Duration,
    Cooldown,
}

impl std::fmt::Display for WeaponStat {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            WeaponStat::Damage => write!(f, "Damage"),
            WeaponStat::Area => write!(f, "Area"),
            WeaponStat::Duration => write!(f, "Duration"),
            WeaponStat::Cooldown => write!(f, "Cooldown"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WeaponConfig {
    pub(crate) weapon_type: WeaponType,
    pub(crate) level: u32,
    // Could add other configuration here like variants/modifiers
}

// Component to define what weapon a player starts with
#[derive(Component)]
pub struct StartingWeapon(pub WeaponType);

// Event for when a new weapon should be added
#[derive(Event)]
pub struct AddWeaponEvent {
    pub player: Entity,
    pub weapon_type: WeaponType,
}

// Different types of area effects
#[derive(Event)]
pub enum AreaEffectEvent {
    Damage(DamageEvent),
    Binding(BindingEvent),
    // Protection(ProtectionEvent),
    // etc.
}

#[derive(Event)]
pub struct BindingEvent {
    pub target: Entity,
    pub strength: f32, // How strongly they're held
    pub source: Entity,
}

#[derive(Debug, Component)]
pub struct WeaponInventory {
    pub(crate) weapons: Vec<WeaponConfig>,
}

// This runs once on player spawn and handles the starting weapon
pub fn setup_player_inventory(
    mut commands: Commands,
    query: Query<(Entity, &StartingWeapon), (Added<Player>, Without<WeaponInventory>)>,
) {
    for (player_entity, starting_weapon) in query.iter() {
        let inventory = WeaponInventory {
            weapons: vec![WeaponConfig {
                weapon_type: starting_weapon.0,
                level: 1,
            }],
        };

        commands.entity(player_entity).insert(inventory);
        spawn_weapon(&mut commands, player_entity, starting_weapon.0);
    }
}

// This system only runs when new weapons are added
pub fn handle_new_weapons(
    mut commands: Commands,
    mut events: EventReader<AddWeaponEvent>,
    mut inventories: Query<&mut WeaponInventory>,
) {
    for event in events.read() {
        if let Ok(mut inventory) = inventories.get_mut(event.player) {
            inventory.weapons.push(WeaponConfig {
                weapon_type: event.weapon_type,
                level: 1,
            });
            spawn_weapon(&mut commands, event.player, event.weapon_type);
        }
    }
}

pub fn handle_weapon_upgrade(
    mut upgrade_events: EventReader<UpgradeConfirmedEvent>,
    mut weapon_query: Query<(
        &Parent,
        &mut MagickCircle,
        &mut WeaponCooldown,
        &mut WeaponDamage,
        &mut WeaponArea,
    )>,
    mut player_query: Query<(Entity, &mut WeaponInventory)>,
) {
    for event in upgrade_events.read() {
        if let UpgradeType::Weapon(weapon_type, upgrade) = &event.upgrade.upgrade_type {
            if let Ok((player_entity, mut inventory)) = player_query.get_single_mut() {
                // Update the weapon config in inventory
                if let Some(weapon_config) = inventory
                    .weapons
                    .iter_mut()
                    .find(|w| w.weapon_type == *weapon_type)
                {
                    // Increment the level
                    weapon_config.level += 1;

                    // Find and update the actual weapon component
                    for (parent, mut circle, mut cooldown, mut damage, mut area) in
                        weapon_query.iter_mut()
                    {
                        if parent.get() == player_entity {
                            match upgrade {
                                WeaponUpgrade::MagickCircle(upgrade) => {
                                    let upgrades: Vec<&MagickCircleUpgrade> = {
                                        if let MagickCircleUpgrade::Combined(upgrades) = upgrade {
                                            upgrades.iter().collect()
                                        } else {
                                            vec![upgrade]
                                        }
                                    };

                                    for upgrade in upgrades {
                                        match upgrade {
                                            MagickCircleUpgrade::AddCircle { pattern, .. } => {
                                                info!(
                                                    "Adding new circle pattern: {:?} at level {}",
                                                    pattern, weapon_config.level
                                                );
                                                circle.patterns.push(*pattern);
                                            }
                                            MagickCircleUpgrade::StatUpgrade {
                                                stat,
                                                bonus,
                                                ..
                                            } => {
                                                match stat {
                                                    WeaponStat::Damage => {
                                                        damage.damage_bonus += bonus;
                                                    }
                                                    WeaponStat::Area => {
                                                        area.area_bonus += bonus;
                                                    }
                                                    WeaponStat::Cooldown => {
                                                        // The bonus already comes in negative
                                                        cooldown.cooldown_bonus += bonus;
                                                    }
                                                    WeaponStat::Duration => {
                                                        // We'll need to add this component/field
                                                    }
                                                }
                                                info!(
                                                    "Upgraded {:?} by {} at level {}",
                                                    stat, bonus, weapon_config.level
                                                );
                                            }
                                            MagickCircleUpgrade::Combined(_) => {
                                                unreachable!("You should not be here. You have violated the laws of nature.")
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                info!("Updated weapon inventory: {:?}", inventory);
            }
        }
    }
}

#[derive(Debug, Component, Copy, Clone, Eq, PartialEq, Hash, EnumIter)]
pub enum WeaponType {
    MagickCircle,
    // Future weapon types...
}

impl std::fmt::Display for WeaponType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MagickCircle => write!(f, "Magick Circle"),
        }
    }
}

/// Base weapon statistics
#[derive(Component)]
pub struct WeaponCooldown {
    pub timer: Timer,
    pub base_duration: f32,
    pub cooldown_bonus: i32, // Negative numbers speed it up
}

impl Default for WeaponCooldown {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
            base_duration: 1.0,
            cooldown_bonus: 0,
        }
    }
}

#[derive(Component)]
pub enum WeaponMovement {
    /// Weapon stays at spawn location
    Stationary,
    /// Weapon follows player position
    FollowPlayer,
    // Could add more variants like:
    // OrbitalRotation(f32), // Rotates around player at given radius
    // ReturnToPlayer,       // Boomerang-style
    // LeashToPlayer(f32),   // Follows but with max distance
}

#[derive(Component)]
pub struct WeaponDamage {
    pub base_amount: i32,
    pub damage_bonus: i32, // Positive numbers increase damage
}

#[derive(Component)]
pub struct Damage {
    pub amount: i32,
}

#[derive(Component)]
pub struct WeaponArea {
    pub base_radius: f32,
    pub area_bonus: i32, // Positive numbers increase area
}

#[derive(Component)]
pub struct Area {
    pub radius: f32,
}

/// Attack-specific components
#[derive(Component)]
pub struct Attack {
    pub attack_type: AttackType,
}

#[derive(Component, Clone)]
pub enum AttackType {
    MagickCircle,
    // Future attack types...
}

#[derive(Component)]
pub struct Lifetime {
    pub timer: Timer,
}

#[derive(Component)]
pub struct Rotates {
    pub speed: f32,
    pub current_angle: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WeaponUpgrade {
    MagickCircle(MagickCircleUpgrade),
}

impl std::fmt::Display for WeaponUpgrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WeaponUpgrade::MagickCircle(magick_circle_upgrade) => {
                write!(f, "{}", magick_circle_upgrade)
            }
        }
    }
}

impl WeaponType {
    pub fn get_progression(&self) -> Vec<WeaponUpgrade> {
        match self {
            WeaponType::MagickCircle => vec![
                // Level 2: Initial power boost
                WeaponUpgrade::MagickCircle(MagickCircleUpgrade::Combined(vec![
                    MagickCircleUpgrade::StatUpgrade {
                        stat: WeaponStat::Damage,
                        bonus: 2,
                        is_limit_break: false,
                    },
                    MagickCircleUpgrade::StatUpgrade {
                        stat: WeaponStat::Area,
                        bonus: 1,
                        is_limit_break: false,
                    },
                ])),
                // Level 3: First additional circle
                WeaponUpgrade::MagickCircle(MagickCircleUpgrade::AddCircle {
                    pattern: PatternType::Banishment,
                    offset_angle: std::f32::consts::PI,
                }),
                // Level 4: Second circle + minor boost
                WeaponUpgrade::MagickCircle(MagickCircleUpgrade::Combined(vec![
                    MagickCircleUpgrade::AddCircle {
                        pattern: PatternType::Banishment,
                        offset_angle: std::f32::consts::PI * 0.5,
                    },
                    MagickCircleUpgrade::StatUpgrade {
                        stat: WeaponStat::Damage,
                        bonus: 1,
                        is_limit_break: false,
                    },
                    MagickCircleUpgrade::StatUpgrade {
                        stat: WeaponStat::Area,
                        bonus: 1,
                        is_limit_break: false,
                    },
                ])),
                // Level 5: Third circle
                WeaponUpgrade::MagickCircle(MagickCircleUpgrade::AddCircle {
                    pattern: PatternType::Banishment,
                    offset_angle: std::f32::consts::PI * 1.5,
                }),
                // Level 6: Significant power boost + fourth circle
                WeaponUpgrade::MagickCircle(MagickCircleUpgrade::Combined(vec![
                    MagickCircleUpgrade::AddCircle {
                        pattern: PatternType::Banishment,
                        offset_angle: std::f32::consts::PI * 2.0,
                    },
                    MagickCircleUpgrade::StatUpgrade {
                        stat: WeaponStat::Damage,
                        bonus: 2,
                        is_limit_break: false,
                    },
                    MagickCircleUpgrade::StatUpgrade {
                        stat: WeaponStat::Area,
                        bonus: 1,
                        is_limit_break: false,
                    },
                ])),
                // Level 7: Fifth circle + minor boost
                WeaponUpgrade::MagickCircle(MagickCircleUpgrade::Combined(vec![
                    MagickCircleUpgrade::AddCircle {
                        pattern: PatternType::Banishment,
                        offset_angle: std::f32::consts::PI * 2.5,
                    },
                    MagickCircleUpgrade::StatUpgrade {
                        stat: WeaponStat::Damage,
                        bonus: 1,
                        is_limit_break: false,
                    },
                    MagickCircleUpgrade::StatUpgrade {
                        stat: WeaponStat::Area,
                        bonus: 1,
                        is_limit_break: false,
                    },
                ])),
                // Level 8: Final circle + major power spike
                WeaponUpgrade::MagickCircle(MagickCircleUpgrade::Combined(vec![
                    MagickCircleUpgrade::AddCircle {
                        pattern: PatternType::Banishment,
                        offset_angle: std::f32::consts::PI * 3.0,
                    },
                    MagickCircleUpgrade::StatUpgrade {
                        stat: WeaponStat::Damage,
                        bonus: 3,
                        is_limit_break: false,
                    },
                    MagickCircleUpgrade::StatUpgrade {
                        stat: WeaponStat::Area,
                        bonus: 2,
                        is_limit_break: false,
                    },
                ])),
            ],
        }
    }

    pub fn get_limit_break_options(&self) -> Vec<WeaponUpgrade> {
        match self {
            WeaponType::MagickCircle => vec![
                // Raw damage
                WeaponUpgrade::MagickCircle(MagickCircleUpgrade::StatUpgrade {
                    stat: WeaponStat::Damage,
                    bonus: 2,
                    is_limit_break: true,
                }),
                // Area of effect
                WeaponUpgrade::MagickCircle(MagickCircleUpgrade::StatUpgrade {
                    stat: WeaponStat::Area,
                    bonus: 2,
                    is_limit_break: true,
                }),
                // Duration focus (circles last longer)
                WeaponUpgrade::MagickCircle(MagickCircleUpgrade::StatUpgrade {
                    stat: WeaponStat::Duration,
                    bonus: 2,
                    is_limit_break: true,
                }),
                // Speed focus (spawn more frequently)
                WeaponUpgrade::MagickCircle(MagickCircleUpgrade::StatUpgrade {
                    stat: WeaponStat::Cooldown,
                    bonus: -2,
                    is_limit_break: true,
                }),
            ],
        }
    }
}

#[derive(Component)]
pub struct Sigil {
    pub index: u32,
    pub base_size: f32,
}

#[derive(Component)]
pub struct Orbits {
    pub radius: f32,
    pub speed: f32,
    pub current_angle: f32,
}



/// Optional weapon modifiers
#[derive(Component)]
pub struct PiercingAttack {
    pub pierce_count: u32,
}

#[derive(Component)]
pub struct AreaEffect {
    pub duration: f32,
    pub tick_rate: f32,
    pub last_tick: f32,
}

#[derive(Component)]
pub struct Knockback {
    pub force: f32,
}

/// Spawns a weapon for the player based on weapon type
pub fn spawn_weapon(commands: &mut Commands, player_entity: Entity, weapon_type: WeaponType) {
    match weapon_type {
        WeaponType::MagickCircle => spawn_magick_circle(commands, player_entity),
        // Add other weapon types here
    }
}

/// System to handle weapon firing logic
pub fn weapon_firing_system(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    // Query player stats
    player_query: Query<
        (
            Entity,
            &CooldownReduction,
            &DamageMultiplier,
            &AreaMultiplier,
            &Transform,
        ),
        With<Player>,
    >,
    // Query weapons and their stats
    mut weapon_query: Query<(
        Entity,
        &Parent,
        &mut WeaponCooldown,
        &WeaponDamage,
        &WeaponArea,
        &Weapon,
    )>,
    // Query specific weapon types for their unique properties
    magick_circle_query: Query<&MagickCircle>,
) {
    // info!("Checking weapons - found {} weapons", weapon_query.iter().count());

    for (weapon_entity, parent, mut cooldown, damage, area, weapon) in weapon_query.iter_mut() {
        // info!("Processing weapon: {:?}", weapon_entity);

        if let Ok((
                      _player_entity,
                      cooldown_reduction,
                      damage_multiplier,
                      area_multiplier,
                      player_transform,
                  )) = player_query.get(parent.get())
        {
            // info!("Found player stats - CD reduction: {}, damage mult: {}, area mult: {}",
            //     cooldown_reduction.percent,
            //     damage_multiplier.factor,
            //     area_multiplier.factor
            // );

            let cooldown_percent = (100 + cooldown.cooldown_bonus) as f32 / 100.0;
            let effective_cooldown =
                cooldown.base_duration * cooldown_percent * (1.0 - cooldown_reduction.percent); // Player's cooldown reduction

            cooldown
                .timer
                .set_duration(Duration::from_secs_f32(effective_cooldown));
            cooldown.timer.tick(time.delta());

            let damage_percent = (100 + damage.damage_bonus) as f32 / 100.0;
            let effective_damage = (damage.base_amount as f32 * damage_percent * damage_multiplier.factor).floor() as i32;

            let area_percent = (100 + area.area_bonus) as f32 / 100.0;
            let effective_radius = area.base_radius * area_percent * area_multiplier.factor;

            // info!("Timer progress: {}/{}",
            //     cooldown.timer.elapsed_secs(),
            //     cooldown.timer.duration().as_secs_f32()
            // );

            if cooldown.timer.just_finished() {
                info!(
                    "Timer finished! Current time: {}, Duration: {}",
                    time.elapsed_secs(),
                    cooldown.timer.duration().as_secs_f32()
                );
                // info!("Cooldown finished!");
                match weapon.weapon_type {
                    WeaponType::MagickCircle => {
                        // info!("Attempting to spawn MagickCircle attack");
                        if let Ok(magick_circle) = magick_circle_query.get(weapon_entity) {
                            // First circle always spawns centered
                            spawn_magick_circle_attack(
                                &mut commands,
                                player_transform.translation,
                                effective_damage,
                                effective_radius,
                                magick_circle.patterns[0],
                                magick_circle.num_sigils,
                                None, // No offset for first circle
                            );

                            // info!("Spawning MagickCircle attack at position: {:?}", player_transform.translation);
                            // Additional circles are evenly spaced
                            if magick_circle.patterns.len() > 1 {
                                let angle_step = std::f32::consts::TAU
                                    / (magick_circle.patterns.len() - 1) as f32;
                                for (i, pattern) in magick_circle.patterns[1..].iter().enumerate() {
                                    let angle = angle_step * i as f32;
                                    spawn_magick_circle_attack(
                                        &mut commands,
                                        player_transform.translation,
                                        effective_damage,
                                        effective_radius,
                                        *pattern,
                                        magick_circle.num_sigils,
                                        Some(angle),
                                    );
                                }
                            }
                        } else {
                            info!("Failed to get MagickCircle component from weapon entity");
                        }
                    }
                }
            }
        } else {
            info!("Failed to get player stats for weapon: {:?}", weapon_entity);
        }
    }
}

fn update_weapon_positions(
    mut param_set: ParamSet<(
        Query<(&mut Transform, &WeaponMovement), With<Attack>>,
        Query<&Transform, With<Player>>,
    )>,
) {
    // First get the player position
    let player_pos = if let Ok(player_transform) = param_set.p1().get_single() {
        player_transform.translation
    } else {
        return;
    };

    // Then update weapon positions
    for (mut weapon_transform, movement) in &mut param_set.p0() {
        match movement {
            WeaponMovement::Stationary => (), // Do nothing
            WeaponMovement::FollowPlayer => {
                weapon_transform.translation = player_pos;
            }
        }
    }
}

/// System to manage area effects for weapons that have them
pub fn area_effect_system(
    time: Res<Time<Virtual>>,
    mut effect_query: Query<(Entity, &mut AreaEffect, &Damage, &Area, &PatternType), With<Attack>>,
    mut damage_events: EventWriter<DamageEvent>,
    mut binding_events: EventWriter<BindingEvent>,
    context_query: Query<&RapierContext>,
    enemy_query: Query<Entity, (With<Enemy>, Without<MarkedForDeath>)>,
) {
    let rapier_context = context_query
        .get_single()
        .unwrap_or_else(|e| handle_rapier_context_error(e));

    // Track which enemies are affected by which circles
    let mut enemy_effects: HashMap<Entity, Vec<(Entity, PatternType)>> = HashMap::new();

    // First pass: collect all circle effects affecting each enemy
    for (circle_entity, mut area_effect, _, _, pattern) in effect_query.iter_mut() {
        if time.elapsed_secs() - area_effect.last_tick >= area_effect.tick_rate {
            area_effect.last_tick = time.elapsed_secs();

            for (collider1, collider2, intersecting) in
                rapier_context.intersection_pairs_with(circle_entity)
            {
                if !intersecting {
                    continue;
                }

                let enemy_entity = if collider1 == circle_entity {
                    collider2
                } else {
                    collider1
                };

                if enemy_query.contains(enemy_entity) {
                    enemy_effects
                        .entry(enemy_entity)
                        .or_default()
                        .push((circle_entity, *pattern));
                }
            }
        }
    }

    // Second pass: apply effects for each enemy
    for (enemy_entity, affecting_circles) in enemy_effects.iter() {
        for (circle_entity, pattern) in affecting_circles {
            match pattern {
                PatternType::Banishment => {
                    if let Ok((_, _, damage, _, _)) = effect_query.get(*circle_entity) {
                        damage_events.send(DamageEvent {
                            target: *enemy_entity,
                            amount: damage.amount,
                            source: Some(*circle_entity),
                        });
                    }
                }
                PatternType::Binding => {
                    binding_events.send(BindingEvent {
                        target: *enemy_entity,
                        strength: 1.0, // We can make this configurable later
                        source: *circle_entity,
                    });
                }
                // Add other pattern types here as we implement them
                _ => {
                    // Log unhandled pattern types in debug builds
                    #[cfg(debug_assertions)]
                    info!("Unhandled pattern type: {:?}", pattern);
                }
            }
        }
    }
}

#[derive(Component)]
pub struct BindingEffect {
    pub strength: f32,
    pub source: Entity,
}

pub fn handle_binding_events(
    mut commands: Commands,
    mut binding_events: EventReader<BindingEvent>,
) {
    for event in binding_events.read() {
        commands.entity(event.target).insert(BindingEffect {
            strength: event.strength,
            source: event.source,
        });
    }
}

/// Handles lifetime of attacks and marks them for death when expired
pub fn attack_lifetime_system(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    mut query: Query<(Entity, &mut Lifetime), (With<Attack>, Without<MarkedForDeath>)>,
) {
    for (entity, mut lifetime) in query.iter_mut() {
        lifetime.timer.tick(time.delta());
        if lifetime.timer.finished() {
            // First check if the entity still exists
            if commands.get_entity(entity).is_some() {
                commands.entity(entity).insert(MarkedForDeath);
            }
        }
    }
}

/// Updates rotation for rotating attacks
pub fn attack_rotation_system(time: Res<Time<Virtual>>, mut query: Query<&mut Rotates>) {
    for mut rotates in query.iter_mut() {
        rotates.current_angle += rotates.speed * time.delta_secs();

        // Normalize angle to prevent potential float overflow in very long sessions
        if rotates.current_angle > std::f32::consts::TAU {
            rotates.current_angle -= std::f32::consts::TAU;
        }
    }
}

fn orbital_movement_system(
    time: Res<Time<Virtual>>,
    mut query: Query<(&mut Transform, &mut Orbits)>,
) {
    for (mut transform, mut orbits) in &mut query {
        orbits.current_angle += orbits.speed * time.delta_secs();

        // Normalize angle
        if orbits.current_angle > std::f32::consts::TAU {
            orbits.current_angle -= std::f32::consts::TAU;
        }

        let offset = Vec2::new(
            orbits.current_angle.cos() * orbits.radius,
            orbits.current_angle.sin() * orbits.radius,
        );

        // Since we're using parent-relative transforms, this will work
        // whether the entity is parented to the player or to a circle
        transform.translation = Vec3::new(offset.x, offset.y, transform.translation.z);
    }
}

