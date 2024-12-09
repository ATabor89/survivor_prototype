use crate::combat::DamageEvent;
use crate::components::{AreaMultiplier, CooldownReduction, DamageMultiplier, Enemy, Player};
use crate::death::MarkedForDeath;
use crate::resources::GameState;
use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::prelude::*;
use std::time::Duration;
use bevy::ecs::query::QuerySingleError;
use crate::physics::handle_rapier_context_error;

/// Plugin to register all weapon-related systems
pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), setup_player_weapons)
            .add_systems(
                Update,
                (
                    weapon_firing_system,
                    area_effect_system,
                    attack_lifetime_system,
                    attack_rotation_system,
                    sigil_position_system,
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

#[derive(Component, Clone)]
pub enum WeaponType {
    MagickCircle,
    // Future weapon types...
}

/// Base weapon statistics
#[derive(Component)]
pub struct WeaponCooldown {
    pub timer: Timer,
    pub base_duration: f32,
}

impl Default for WeaponCooldown {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
            base_duration: 1.0,
        }
    }
}

#[derive(Component)]
pub struct WeaponDamage {
    pub base_amount: f32,
}

#[derive(Component)]
pub struct WeaponArea {
    pub base_radius: f32,
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

/// Specialized MagickCircle components
#[derive(Component)]
pub struct MagickCircle {
    pub pattern_type: PatternType,
    pub num_sigils: u32,
}

#[derive(Component)]
pub struct Sigil {
    pub index: u32,
    pub base_size: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternType {
    Protection,    // Basic defensive circle
    Binding,       // Slows/holds enemies
    Banishment,    // Damages/pushes enemies
    Invocation,    // Attracts/pulls enemies
    Manifestation, // Creates effects over time
}

#[derive(Debug, Clone, PartialEq)]
pub enum SigilIntent {
    Protect,
    Attack,
    Control,
    Transform,
    Reveal,
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

/// System to give newly spawned players their starting weapon
pub fn setup_player_weapons(
    mut commands: Commands,
    query: Query<Entity, (With<Player>, Added<Player>)>,
) {
    info!("Setup player weapons system running");
    for player_entity in query.iter() {
        info!("Setting up weapons for player: {:?}", player_entity);
        spawn_weapon(&mut commands, player_entity, WeaponType::MagickCircle);
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
            player_entity,
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

            let effective_cooldown = cooldown.base_duration * (1.0 - cooldown_reduction.percent);
            // info!("Effective cooldown: {} (base: {})", effective_cooldown, cooldown.base_duration);

            cooldown
                .timer
                .set_duration(Duration::from_secs_f32(effective_cooldown));
            cooldown.timer.tick(time.delta());

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
                            // info!("Spawning MagickCircle attack at position: {:?}", player_transform.translation);
                            spawn_magick_circle_attack(
                                &mut commands,
                                player_entity,
                                player_transform.translation,
                                damage.base_amount * damage_multiplier.factor,
                                area.base_radius * area_multiplier.factor,
                                magick_circle.pattern_type.clone(),
                                magick_circle.num_sigils,
                            );
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

/// System to manage area effects for weapons that have them
pub fn area_effect_system(
    time: Res<Time<Virtual>>,
    mut effect_query: Query<(Entity, &mut AreaEffect, &WeaponDamage, &WeaponArea), With<Attack>>,
    mut damage_events: EventWriter<DamageEvent>,
    context_query: Query<&RapierContext>,
    enemy_query: Query<Entity, With<Enemy>>,
) {
    // info!("Processing {} area effects", effect_query.iter().count());

    let rapier_context = context_query.get_single()
        .unwrap_or_else(|e| handle_rapier_context_error(e));

    for (entity, mut area_effect, damage, area) in effect_query.iter_mut() {
        if time.elapsed_secs() - area_effect.last_tick >= area_effect.tick_rate {
            let radius = area.base_radius;
            // info!(
            //     "Checking collisions for effect {:?} with radius {}",
            //     entity, radius
            // );

            let mut hits = 0;
            for (collider1, collider2, intersecting) in
                rapier_context.intersection_pairs_with(entity)
            {
                if !intersecting {
                    continue;
                }

                let enemy_entity = if collider1 == entity {
                    collider2
                } else {
                    collider1
                };

                if enemy_query.contains(enemy_entity) {
                    hits += 1;
                    info!(
                        "Sending damage event: {:?} -> {:?}, amount: {}",
                        entity, enemy_entity, damage.base_amount
                    );
                    damage_events.send(DamageEvent {
                        target: enemy_entity,
                        amount: damage.base_amount,
                        source: Some(entity),
                    });
                }
            }
            // info!("Found {} enemies in area", hits);

            area_effect.last_tick = time.elapsed_secs();
        }
    }
}

/// Handles lifetime of attacks and marks them for death when expired
pub fn attack_lifetime_system(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    mut query: Query<(Entity, &mut Lifetime), With<Attack>>,
) {
    for (entity, mut lifetime) in query.iter_mut() {
        lifetime.timer.tick(time.delta());
        if lifetime.timer.finished() {
            commands.entity(entity).insert(MarkedForDeath);
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

/// Positions sigils around their parent attacks
pub fn sigil_position_system(
    mut sigil_query: Query<(&Sigil, &mut Transform, &Parent)>,
    attack_query: Query<(&Transform, &Rotates, &WeaponArea), Without<Sigil>>,
) {
    for (sigil, mut sigil_transform, parent) in sigil_query.iter_mut() {
        if let Ok((attack_transform, rotates, area)) = attack_query.get(parent.get()) {
            let num_sigils = 4;
            let angle = rotates.current_angle
                + (sigil.index as f32) * std::f32::consts::TAU / num_sigils as f32;
            let offset = Vec2::new(angle.cos(), angle.sin()) * area.base_radius;

            sigil_transform.translation =
                attack_transform.translation + Vec3::new(offset.x, offset.y, 0.1);
        }
    }
}

/// Spawns a magick circle weapon with default configuration
fn spawn_magick_circle(commands: &mut Commands, player_entity: Entity) {
    info!("Spawning magick circle for player: {:?}", player_entity);
    commands.entity(player_entity).with_children(|parent| {
        // Spawn the base weapon entity
        parent.spawn((
            // Core weapon components
            Weapon {
                weapon_type: WeaponType::MagickCircle,
            },
            WeaponCooldown {
                timer: Timer::from_seconds(3.5, TimerMode::Repeating),
                base_duration: 3.5,
            },
            WeaponDamage { base_amount: 10.0 },
            WeaponArea { base_radius: 64.0 },
            // MagickCircle specific components
            MagickCircle {
                pattern_type: PatternType::Banishment,
                num_sigils: 4,
            },
            // Optional modifiers
            AreaEffect {
                duration: 0.5,
                tick_rate: 0.5,
                last_tick: 0.0,
            },
            // Could add other modifiers like PiercingAttack or Knockback
            // based on configuration
        ));
    });
}

/// Helper function to spawn a magick circle attack
pub fn spawn_magick_circle_attack(
    commands: &mut Commands,
    parent_entity: Entity,
    position: Vec3,
    damage: f32,
    radius: f32,
    pattern_type: PatternType,
    num_sigils: u32,
) -> Entity {
    info!("Spawning attack at position: {:?}", position);
    // First spawn the attack entity
    let attack_entity = commands
        .spawn((
            Attack {
                attack_type: AttackType::MagickCircle,
            },
            Lifetime {
                timer: Timer::from_seconds(3.0, TimerMode::Once),
            },
            Rotates {
                speed: 1.0,
                current_angle: 0.0,
            },
            WeaponDamage {
                base_amount: damage,
            },
            WeaponArea {
                base_radius: radius,
            },
            AreaEffect {
                duration: 0.5,
                tick_rate: 0.5,
                last_tick: 0.0,
            },
            ShapeBundle {
                path: GeometryBuilder::new()
                    .add(&shapes::Circle {
                        radius,
                        center: Vec2::ZERO,
                    })
                    .build(),
                transform: Transform::from_translation(position),
                ..default()
            },
            Fill::color(Color::srgba(0.5, 0.5, 1.0, 0.3)),
            Sensor,
            Collider::ball(radius),
            ActiveEvents::COLLISION_EVENTS,
            CollisionGroups::new(Group::GROUP_3, Group::GROUP_2),
        ))
        .id();

    // Add it as a child of the parent entity
    commands.entity(parent_entity).add_child(attack_entity);

    // Then spawn sigils as children of the attack
    let sigil_size = radius * 0.25;
    for i in 0..num_sigils {
        let sigil_entity = commands
            .spawn((
                Sigil {
                    index: i,
                    base_size: sigil_size,
                },
                ShapeBundle {
                    path: GeometryBuilder::new()
                        .add(&shapes::Rectangle {
                            extents: Vec2::splat(sigil_size),
                            origin: RectangleOrigin::Center,
                            ..default()
                        })
                        .build(),
                    transform: Transform::default(),
                    ..default()
                },
                Fill::color(Color::rgba(0.7, 0.7, 1.0, 0.8)),
            ))
            .id();

        commands.entity(attack_entity).add_child(sigil_entity);
    }

    attack_entity
}
