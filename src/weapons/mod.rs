use crate::combat::DamageEvent;
use crate::components::{AreaMultiplier, CooldownReduction, DamageMultiplier, Enemy, Player};
use crate::death::MarkedForDeath;

use crate::resources::GameState;
use crate::weapons::magick_circle::{
    apply_magick_circle_weapon_upgrades, spawn_magick_circle, spawn_magick_circle_attack,
    MagickCircle, PatternType,
};
use crate::weapons::overlap::{overlap_detection_system, ActiveOverlaps, EnemyCircleHits};
use crate::weapons::weapon_upgrade::{
    apply_common_weapon_upgrades, update_weapon_level, WeaponUpgradeConfig,
};
use crate::GameplaySets;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use std::fmt::Formatter;
use std::time::Duration;
use strum::EnumIter;

pub mod magick_circle;
pub mod overlap;
pub mod weapon_upgrade;

/// Plugin to register all weapon-related systems
pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WeaponUpgradeConfig>()
            .init_resource::<EnemyCircleHits>()
            .init_resource::<ActiveOverlaps>()
            .insert_resource(WeaponPickupCounter(0))
            .add_message::<AddWeaponEvent>()
            .add_message::<AreaEffectEvent>()
            .add_message::<BindingEvent>()
            .add_systems(
                Update,
                (
                    update_weapon_level,
                    (
                        apply_common_weapon_upgrades,
                        apply_magick_circle_weapon_upgrades,
                    )
                        .after(update_weapon_level),
                )
                    .before(GameplaySets::Weapons),
            )
            .add_systems(
                Update,
                (
                    setup_player_inventory,
                    handle_new_weapons,
                    weapon_firing_system,
                    update_weapon_positions,
                    invocation_cleanup_system,
                    area_effect_system,
                    overlap_detection_system,
                    handle_binding_events,
                    binding_expiration_system,
                    protection_system,
                    attack_lifetime_system,
                    attack_rotation_system,
                    orbital_movement_system,
                )
                    .chain()
                    .in_set(GameplaySets::Weapons)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Core weapon type identifier
#[derive(Debug, Component)]
pub struct WeaponMeta {
    pub weapon_type: WeaponType,
    pub level: u32,
    pub pickup_id: u8,
}

#[derive(Resource)]
pub struct WeaponPickupCounter(u8);

// Component to define what weapon a player starts with
#[derive(Component)]
pub struct StartingWeapon(pub WeaponType);

// Event for when a new weapon should be added
#[derive(Message)]
pub struct AddWeaponEvent {
    pub player: Entity,
    pub weapon_type: WeaponType,
}

// Different types of area effects
#[derive(Message)]
pub enum AreaEffectEvent {
    Damage(DamageEvent),
    Binding(BindingEvent),
    // Protection(ProtectionEvent),
    // etc.
}

#[derive(Message)]
pub struct BindingEvent {
    pub target: Entity,
    pub strength: f32, // How strongly they're held
    pub source: Entity,
}

// This runs once on player spawn and handles the starting weapon
pub fn setup_player_inventory(
    query: Query<(Entity, &StartingWeapon), Added<Player>>,
    mut add_weapon_event: MessageWriter<AddWeaponEvent>,
) {
    for (player_entity, starting_weapon) in query.iter() {
        add_weapon_event.write(AddWeaponEvent {
            player: player_entity,
            weapon_type: starting_weapon.0,
        });
    }
}

// This system only runs when new weapons are added
pub fn handle_new_weapons(
    mut commands: Commands,
    mut events: MessageReader<AddWeaponEvent>,
    mut weapon_pickup_counter: ResMut<WeaponPickupCounter>,
) {
    for event in events.read() {
        let pickup_id = weapon_pickup_counter.0;
        spawn_weapon(&mut commands, event.player, event.weapon_type, pickup_id);

        weapon_pickup_counter.0 += 1;
    }
}

#[derive(Debug, Component, Copy, Clone, Eq, PartialEq, Hash, EnumIter)]
pub enum WeaponType {
    MagickCircle,
    // Future weapon types...
}

impl std::fmt::Display for WeaponType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
pub struct Attack; // Formerly held an AttackType enum

#[derive(Component)]
pub struct Lifetime {
    pub timer: Timer,
}

#[derive(Component)]
pub struct Rotates {
    pub speed: f32,
    pub current_angle: f32,
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
pub fn spawn_weapon(
    commands: &mut Commands,
    player_entity: Entity,
    weapon_type: WeaponType,
    pickup_id: u8,
) {
    match weapon_type {
        WeaponType::MagickCircle => spawn_magick_circle(commands, player_entity, pickup_id),
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
        &ChildOf,
        &mut WeaponCooldown,
        &WeaponDamage,
        &WeaponArea,
        &WeaponMeta,
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
        )) = player_query.get(parent.parent())
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
            let effective_damage =
                (damage.base_amount as f32 * damage_percent * damage_multiplier.factor).floor()
                    as i32;

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
    let player_pos = if let Ok(player_transform) = param_set.p1().single() {
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

/// System to manage area effects for weapons that have them.
/// Populates `EnemyCircleHits` as a side effect so `overlap_detection_system`
/// can identify enemies inside multiple circles.
pub fn area_effect_system(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    mut effect_query: Query<
        (
            Entity,
            &mut AreaEffect,
            &Damage,
            &Area,
            &PatternType,
            &Transform,
        ),
        With<Attack>,
    >,
    mut damage_events: MessageWriter<DamageEvent>,
    mut binding_events: MessageWriter<BindingEvent>,
    context_query: ReadRapierContext,
    enemy_query: Query<Entity, (With<Enemy>, Without<MarkedForDeath>)>,
    mut enemy_circle_hits: ResMut<EnemyCircleHits>,
) {
    let rapier_context = context_query
        .single()
        .expect("Expected one initialized Rapier context");

    // Clear previous frame's data; this resource is also read by overlap_detection_system
    enemy_circle_hits.hits.clear();
    let mut ticking_circles: HashSet<Entity> = HashSet::default();

    // First pass: collect all circle effects affecting each enemy
    for (circle_entity, mut area_effect, _, _, pattern, circle_transform) in effect_query.iter_mut()
    {
        if time.elapsed_secs() - area_effect.last_tick >= area_effect.tick_rate {
            area_effect.last_tick = time.elapsed_secs();
            ticking_circles.insert(circle_entity);
        }
        let circle_pos = circle_transform.translation.truncate();

        // Spatial membership is continuous; only effect application is timed.
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
                enemy_circle_hits
                    .hits
                    .entry(enemy_entity)
                    .or_default()
                    .push((circle_entity, *pattern, circle_pos));
            }
        }
    }

    // Second pass: apply per-pattern effects for each enemy
    for (enemy_entity, affecting_circles) in enemy_circle_hits.hits.iter() {
        for (circle_entity, pattern, circle_pos) in affecting_circles {
            // Pulling is continuous while inside a circle, independent of damage ticks.
            if *pattern == PatternType::Invocation {
                commands.entity(*enemy_entity).insert(InvocationPull {
                    pull_target: *circle_pos,
                    strength: 120.0,
                });
                continue;
            }
            if !ticking_circles.contains(circle_entity) {
                continue;
            }
            match pattern {
                PatternType::Banishment => {
                    if let Ok((_, _, damage, _, _, _)) = effect_query.get(*circle_entity) {
                        damage_events.write(DamageEvent {
                            target: *enemy_entity,
                            amount: damage.amount,
                            source: Some(*circle_entity),
                        });
                    }
                }
                PatternType::Binding => {
                    binding_events.write(BindingEvent {
                        target: *enemy_entity,
                        strength: 1.0,
                        source: *circle_entity,
                    });
                }
                PatternType::Invocation => {
                    // Applied continuously above.
                }
                PatternType::Manifestation => {
                    // Periodic damage burst — deals 1.5x the circle's base damage
                    if let Ok((_, _, damage, _, _, _)) = effect_query.get(*circle_entity) {
                        damage_events.write(DamageEvent {
                            target: *enemy_entity,
                            amount: (damage.amount as f32 * 1.5) as i32,
                            source: Some(*circle_entity),
                        });
                    }
                }
                PatternType::Protection => {
                    // Protection effect on player is handled by protection_system;
                    // no enemy-side effect needed here.
                }
            }
        }
    }
}

#[derive(Component)]
pub struct BindingEffect {
    pub strength: f32,
    pub source: Entity,
    pub timer: Timer,
}

pub fn handle_binding_events(
    mut commands: Commands,
    mut binding_events: MessageReader<BindingEvent>,
) {
    for event in binding_events.read() {
        commands.entity(event.target).insert(BindingEffect {
            strength: event.strength,
            source: event.source,
            // Slightly longer than area_effect tick_rate (0.5s) so the effect
            // lingers briefly after leaving the circle, then wears off.
            timer: Timer::from_seconds(1.0, TimerMode::Once),
        });
    }
}

/// Ticks binding timers and removes expired binding effects so enemies
/// regain movement after leaving a Binding circle.
pub fn binding_expiration_system(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    mut query: Query<(Entity, &mut BindingEffect)>,
) {
    for (entity, mut binding) in query.iter_mut() {
        binding.timer.tick(time.delta());
        if binding.timer.is_finished() {
            commands.entity(entity).remove::<BindingEffect>();
        }
    }
}

/// Applied to enemies inside an Invocation circle. Cleared every frame
/// by `invocation_cleanup_system` and re-applied by `area_effect_system`
/// while the enemy remains inside the circle.
#[derive(Component)]
pub struct InvocationPull {
    pub pull_target: Vec2,
    pub strength: f32,
}

/// Clears all `InvocationPull` components so stale pulls are removed.
/// Must run BEFORE `area_effect_system` which re-inserts them for
/// enemies still inside Invocation circles.
pub fn invocation_cleanup_system(
    mut commands: Commands,
    query: Query<Entity, With<InvocationPull>>,
) {
    for entity in query.iter() {
        commands.entity(entity).remove::<InvocationPull>();
    }
}

/// Applied to the player when they're standing inside a Protection circle.
/// Reduces incoming damage by `damage_reduction` (0.0–1.0).
#[derive(Component)]
pub struct ProtectionBuff {
    pub damage_reduction: f32,
}

/// Checks whether the player is inside any active Protection circle and
/// inserts or removes `ProtectionBuff` accordingly.
/// Uses Rapier intersection pairs (circles are GROUP_3 filtering for
/// GROUP_1 | GROUP_2, player is GROUP_1 filtering for GROUP_3).
pub fn protection_system(
    mut commands: Commands,
    context_query: ReadRapierContext,
    player_query: Query<Entity, With<Player>>,
    protection_circles: Query<(Entity, &PatternType), With<Attack>>,
) {
    let Ok(player_entity) = player_query.single() else {
        return;
    };
    let rapier_context = context_query
        .single()
        .expect("Expected one initialized Rapier context");

    let mut inside_protection = false;

    for (circle_entity, pattern) in protection_circles.iter() {
        if *pattern != PatternType::Protection {
            continue;
        }
        for (collider1, collider2, intersecting) in
            rapier_context.intersection_pairs_with(circle_entity)
        {
            if !intersecting {
                continue;
            }
            let other = if collider1 == circle_entity {
                collider2
            } else {
                collider1
            };
            if other == player_entity {
                inside_protection = true;
                break;
            }
        }
        if inside_protection {
            break;
        }
    }

    if inside_protection {
        commands.entity(player_entity).insert(ProtectionBuff {
            damage_reduction: 0.5, // 50% damage reduction
        });
    } else {
        commands.entity(player_entity).remove::<ProtectionBuff>();
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
        if lifetime.timer.is_finished() {
            // First check if the entity still exists
            if commands.get_entity(entity).is_ok() {
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
