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
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use std::fmt::Formatter;
use std::time::Duration;
use strum::EnumIter;

pub mod effects;
pub mod magick_circle;
use effects::{CircleEffect, PULL_SPEED_CAP, ROOT_LINGER_SECONDS};
pub mod overlap;
pub mod weapon_upgrade;

/// Plugin to register all weapon-related systems
pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WeaponUpgradeConfig>()
            .init_resource::<EnemyCircleHits>()
            .init_resource::<ActiveOverlaps>()
            .add_message::<AddWeaponEvent>()
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
                    invocation_cleanup_system,
                    area_effect_system,
                    magick_circle::animate_manifestation_charge,
                    overlap_detection_system,
                    handle_binding_events,
                    binding_expiration_system,
                    protection_system,
                    attack_lifetime_system,
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
}

// Component to define what weapon a player starts with
#[derive(Component)]
pub struct StartingWeapon(pub WeaponType);

// Event for when a new weapon should be added
#[derive(Message)]
pub struct AddWeaponEvent {
    pub player: Entity,
    pub weapon_type: WeaponType,
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
pub fn handle_new_weapons(mut commands: Commands, mut events: MessageReader<AddWeaponEvent>) {
    for event in events.read() {
        spawn_weapon(&mut commands, event.player, event.weapon_type);
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

impl WeaponCooldown {
    /// Limit breaks may exceed 100% reduction, but firing always has a cadence.
    pub const MIN_SECONDS: f32 = 0.1;

    pub(crate) fn effective_seconds(&self, player_reduction: f32) -> f32 {
        let weapon_factor = ((100.0 + self.cooldown_bonus as f32) / 100.0).max(0.0);
        (self.base_duration * weapon_factor * (1.0 - player_reduction.clamp(0.0, 1.0)))
            .max(Self::MIN_SECONDS)
    }
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

/// Attack-specific components
#[derive(Component)]
pub struct Attack; // Formerly held an AttackType enum

#[derive(Component)]
pub struct Lifetime {
    pub timer: Timer,
}

#[derive(Component)]
pub struct Sigil;

#[derive(Component)]
pub struct Orbits {
    pub radius: f32,
    pub speed: f32,
    pub current_angle: f32,
}

#[derive(Component)]
pub struct AreaEffect {
    /// Spawn-relative cadence: new circles always wait for their first effect.
    pub timer: Timer,
}

impl AreaEffect {
    pub fn new(pattern: PatternType) -> Self {
        Self {
            timer: Timer::from_seconds(pattern.effect().interval(), TimerMode::Repeating),
        }
    }
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
    for (weapon_entity, parent, mut cooldown, damage, area, weapon) in weapon_query.iter_mut() {
        if let Ok((cooldown_reduction, damage_multiplier, area_multiplier, player_transform)) =
            player_query.get(parent.parent())
        {
            let effective_cooldown = cooldown.effective_seconds(cooldown_reduction.percent);

            cooldown
                .timer
                .set_duration(Duration::from_secs_f32(effective_cooldown));
            cooldown.timer.tick(time.delta());

            let damage_percent = ((100.0 + damage.damage_bonus as f32) / 100.0).max(0.0);
            let effective_damage =
                (damage.base_amount as f32 * damage_percent * damage_multiplier.factor.max(0.0))
                    .max(0.0)
                    .floor() as i32;

            let area_percent = ((100.0 + area.area_bonus as f32) / 100.0).max(0.0);
            let effective_radius =
                (area.base_radius * area_percent * area_multiplier.factor.max(0.0)).max(0.0);

            if cooldown.timer.just_finished() {
                match weapon.weapon_type {
                    WeaponType::MagickCircle => {
                        if let Ok(magick_circle) = magick_circle_query.get(weapon_entity) {
                            let Some((&first, remaining)) = magick_circle.patterns.split_first()
                            else {
                                continue;
                            };
                            // First circle always spawns centered
                            spawn_magick_circle_attack(
                                &mut commands,
                                player_transform.translation,
                                effective_damage,
                                effective_radius,
                                first,
                                None, // No offset for first circle
                            );

                            // Additional circles are evenly spaced
                            if !remaining.is_empty() {
                                let angle_step = std::f32::consts::TAU / remaining.len() as f32;
                                for (i, pattern) in remaining.iter().enumerate() {
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
                            warn!("Missing MagickCircle component on weapon {weapon_entity:?}");
                        }
                    }
                }
            }
        } else {
            warn!("Missing player stats for weapon {weapon_entity:?}");
        }
    }
}

/// System to manage area effects for weapons that have them.
/// Populates `EnemyCircleHits` as a side effect so `overlap_detection_system`
/// can identify enemies inside multiple circles.
#[expect(
    clippy::too_many_arguments,
    reason = "Bevy injects separate queries, messages, and resources to track ECS access."
)]
pub fn area_effect_system(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    mut effect_query: Query<
        (Entity, &mut AreaEffect, &Damage, &PatternType, &Transform),
        With<Attack>,
    >,
    mut damage_events: MessageWriter<DamageEvent>,
    mut binding_events: MessageWriter<BindingEvent>,
    context_query: ReadRapierContext,
    enemy_query: Query<&Transform, (With<Enemy>, Without<MarkedForDeath>)>,
    breakables: Query<(), (With<crate::loot::Breakable>, Without<MarkedForDeath>)>,
    mut enemy_circle_hits: ResMut<EnemyCircleHits>,
) {
    let rapier_context = context_query
        .single()
        .expect("Expected one initialized Rapier context");

    // Clear previous frame's data; this resource is also read by overlap_detection_system
    enemy_circle_hits.hits.clear();
    let mut ticking_circles: HashMap<Entity, u32> = HashMap::default();

    // First pass: collect all circle effects affecting each enemy
    for (circle_entity, mut area_effect, damage, pattern, circle_transform) in
        effect_query.iter_mut()
    {
        area_effect.timer.tick(time.delta());
        let ticks = area_effect.timer.times_finished_this_tick();
        if ticks > 0 {
            ticking_circles.insert(circle_entity, ticks);
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
            } else if ticks > 0 && breakables.contains(enemy_entity) {
                // Props participate in Rapier intersections and damage, but never
                // in enemy-only root, pull, overlap, kill, or XP behavior.
                if let CircleEffect::Damage { multiplier, .. } = pattern.effect() {
                    damage_events.write(DamageEvent {
                        target: enemy_entity,
                        amount: ((damage.amount as f32 * multiplier).round() as i32)
                            .saturating_mul(i32::try_from(ticks).unwrap_or(i32::MAX)),
                        source: Some(circle_entity),
                    });
                }
            }
        }
    }

    // Second pass: apply per-pattern effects for each enemy
    for (enemy_entity, affecting_circles) in enemy_circle_hits.hits.iter_mut() {
        // Stable accumulation order, independent of query/archetype iteration.
        affecting_circles.sort_unstable_by_key(|(entity, _, _)| *entity);
        let enemy_pos = enemy_query
            .get(*enemy_entity)
            .unwrap()
            .translation
            .truncate();
        let mut pull_velocity = Vec2::ZERO;
        let mut has_pull = false;
        for (circle_entity, pattern, circle_pos) in affecting_circles {
            if let CircleEffect::Pull { speed } = pattern.effect() {
                let offset = *circle_pos - enemy_pos;
                // Slow near the center to avoid overshooting and oscillating.
                pull_velocity += offset.normalize_or_zero()
                    * speed.min(offset.length() / time.delta_secs().max(0.0001));
                has_pull = true;
                continue;
            }
            let Some(&ticks) = ticking_circles.get(circle_entity) else {
                continue;
            };
            match pattern.effect() {
                CircleEffect::Damage { multiplier, .. } => {
                    if let Ok((_, _, damage, _, _)) = effect_query.get(*circle_entity) {
                        damage_events.write(DamageEvent {
                            target: *enemy_entity,
                            amount: ((damage.amount as f32 * multiplier).round() as i32)
                                .saturating_mul(i32::try_from(ticks).unwrap_or(i32::MAX)),
                            source: Some(*circle_entity),
                        });
                    }
                }
                CircleEffect::Root { strength } => {
                    binding_events.write(BindingEvent {
                        target: *enemy_entity,
                        strength,
                        source: *circle_entity,
                    });
                }
                CircleEffect::Pull { .. } | CircleEffect::Ward { .. } => {}
            }
        }
        if has_pull {
            commands.entity(*enemy_entity).insert(InvocationPull {
                velocity: pull_velocity.clamp_length_max(PULL_SPEED_CAP),
            });
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
    mut bindings: Query<&mut BindingEffect>,
) {
    let mut roots: HashMap<Entity, (f32, Entity)> = HashMap::default();
    for event in binding_events.read() {
        let root = roots
            .entry(event.target)
            .or_insert((event.strength, event.source));
        if event.strength > root.0 || (event.strength == root.0 && event.source < root.1) {
            *root = (event.strength, event.source);
        }
    }
    for (target, (strength, source)) in roots {
        if let Ok(mut binding) = bindings.get_mut(target) {
            if strength >= binding.strength {
                binding.strength = strength;
                binding.source = source;
            }
            binding.timer.reset();
        } else {
            commands.entity(target).insert(BindingEffect {
                strength,
                source,
                timer: Timer::from_seconds(ROOT_LINGER_SECONDS, TimerMode::Once),
            });
        }
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
    pub velocity: Vec2,
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

    let mut reduction: f32 = 0.0;

    for (circle_entity, pattern) in protection_circles.iter() {
        let CircleEffect::Ward {
            reduction: circle_reduction,
        } = pattern.effect()
        else {
            continue;
        };
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
                reduction = reduction.max(circle_reduction);
                break;
            }
        }
    }

    if reduction > 0.0 {
        commands.entity(player_entity).insert(ProtectionBuff {
            damage_reduction: reduction,
        });
    } else {
        commands.entity(player_entity).remove::<ProtectionBuff>();
    }
}

/// Handles lifetime of attacks and marks them for death when expired
#[expect(
    clippy::type_complexity,
    reason = "The Bevy query filters expired attacks without revisiting entities already marked for death."
)]
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

fn orbital_movement_system(
    time: Res<Time<Virtual>>,
    mut query: Query<(&mut Transform, &mut Orbits)>,
) {
    for (mut transform, mut orbits) in &mut query {
        orbits.current_angle = (orbits.current_angle + orbits.speed * time.delta_secs())
            .rem_euclid(std::f32::consts::TAU);

        let offset = Vec2::new(
            orbits.current_angle.cos() * orbits.radius,
            orbits.current_angle.sin() * orbits.radius,
        );

        // Since we're using parent-relative transforms, this will work
        // whether the entity is parented to the player or to a circle
        transform.translation = Vec3::new(offset.x, offset.y, transform.translation.z);
    }
}
