use crate::combat::DamageEvent;
use crate::components::{AreaMultiplier, CooldownReduction, DamageMultiplier, Enemy, Player};
use crate::death::MarkedForDeath;
use crate::physics::handle_rapier_context_error;
use crate::resources::GameState;
use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::prelude::*;
use std::time::Duration;
use crate::systems::spawn_player;

/// Plugin to register all weapon-related systems
pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<AddWeaponEvent>()
            .add_systems(
            Update,
            (
                setup_player_inventory,
                handle_new_weapons,
                weapon_firing_system,
                update_weapon_positions,
                area_effect_system,
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

#[derive(Clone)]
pub struct WeaponConfig {
    weapon_type: WeaponType,
    level: u32,
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

#[derive(Component)]
pub struct WeaponInventory {
    weapons: Vec<WeaponConfig>,
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

#[derive(Component, Copy, Clone)]
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

#[derive(Clone)]
pub enum WeaponUpgrade {
    Damage(f32),
    Area(f32),
    Cooldown(f32),
    Count(u32),
    Special(SpecialUpgrade),
}

#[derive(Clone)]
pub enum SpecialUpgrade {
    MagickCircleBinding,
    MagickCircleBanishment,
    // etc
}

pub struct WeaponProgression {
    upgrades: Vec<WeaponUpgrade>,
    max_level: u32,
    limit_break_options: Vec<WeaponUpgrade>,
    limit_break_weights: Vec<f32>,
}

impl WeaponType {
    pub fn get_progression(&self) -> WeaponProgression {
        match self {
            WeaponType::MagickCircle => WeaponProgression {
                upgrades: vec![
                    WeaponUpgrade::Damage(1.2),  // Level 2
                    WeaponUpgrade::Count(1),     // Level 3
                    WeaponUpgrade::Area(1.3),    // Level 4
                    WeaponUpgrade::Special(SpecialUpgrade::MagickCircleBinding), // Level 5
                ],
                max_level: 5,
                limit_break_options: vec![
                    WeaponUpgrade::Damage(1.1),
                    WeaponUpgrade::Area(1.1),
                    WeaponUpgrade::Cooldown(0.9),
                ],
                limit_break_weights: vec![0.4, 0.3, 0.3],
            },
            // Add other weapon types here
        }
    }
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

#[derive(Component)]
pub struct Orbits {
    pub radius: f32,
    pub speed: f32,
    pub current_angle: f32,
}

#[derive(Component, Debug, Clone, PartialEq)]
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
    mut effect_query: Query<(Entity, &mut AreaEffect, &WeaponDamage, &WeaponArea), With<Attack>>,
    mut damage_events: EventWriter<DamageEvent>,
    context_query: Query<&RapierContext>,
    enemy_query: Query<Entity, With<Enemy>>,
) {
    // info!("Processing {} area effects", effect_query.iter().count());

    let rapier_context = context_query
        .get_single()
        .unwrap_or_else(|e| handle_rapier_context_error(e));

    for (entity, mut area_effect, damage, _area) in effect_query.iter_mut() {
        if time.elapsed_secs() - area_effect.last_tick >= area_effect.tick_rate {
            // let mut hits = 0;
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
                    // hits += 1;
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
            pattern_type,
            WeaponMovement::Stationary,
        ))
        .id();

    // Then spawn sigils as children of the attack
    let sigil_size = radius * 0.25;
    for i in 0..num_sigils {
        let sigil_entity = commands
            .spawn((
                Sigil {
                    index: i,
                    base_size: sigil_size,
                },
                Orbits {
                    radius,
                    speed: 1.0,
                    current_angle: (i as f32) * std::f32::consts::TAU / num_sigils as f32,
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
                Fill::color(Color::srgba(0.7, 0.7, 1.0, 0.8)),
            ))
            .id();

        commands.entity(attack_entity).add_child(sigil_entity);
    }

    attack_entity
}
