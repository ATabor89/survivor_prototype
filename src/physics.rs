use crate::components::{Enemy, Health, Player, Projectile};
use crate::resources::GameState;
use bevy::prelude::*;
use bevy::utils::HashSet;
use bevy_rapier2d::prelude::*;
use bevy_rapier2d::rapier::geometry::CollisionEventFlags;

pub struct PhysicsPlugin;

// A component to mark our damage sensor
#[derive(Component)]
pub struct DamageSensor;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        // Base physics setup with custom configuration
        app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
            // Configure physics with no gravity
            .insert_resource(RapierConfiguration {
                gravity: Vec2::ZERO,
                physics_pipeline_active: true,
                query_pipeline_active: true,
                timestep_mode: TimestepMode::Variable {
                    max_dt: 1.0 / 60.0,
                    time_scale: 1.0,
                    substeps: 1,
                },
                scaled_shape_subdivision: 10,
                force_update_from_transform_changes: false,
            });

        #[cfg(debug_assertions)]
        {
            app.add_plugins(RapierDebugRenderPlugin::default());
        }

        app.add_systems(
            Update,
            (setup_physics_bodies, handle_collision_events)
                .chain()
                .run_if(in_state(GameState::Playing)),
        );
    }
}

pub fn setup_physics_bodies(
    mut commands: Commands,
    new_players: Query<Entity, (Added<Player>, Without<RigidBody>)>,
    new_enemies: Query<Entity, (Added<Enemy>, Without<RigidBody>)>,
    new_projectiles: Query<Entity, (Added<Projectile>, Without<RigidBody>)>,
) {
    // Define membership groups
    let player_group = Group::GROUP_1;
    let enemy_group = Group::GROUP_2;
    let projectile_group = Group::GROUP_3;
    let sensor_group = Group::GROUP_4;

    // info!("Setting up physics bodies");

    // Player setup
    for entity in new_players.iter() {
        info!("Setting up player entity: {:?}", entity);
        if commands.get_entity(entity).is_some() {
            commands.entity(entity)
                .insert((
                    RigidBody::KinematicPositionBased,
                    Collider::ball(12.0),
                    Velocity::zero(),
                    LockedAxes::ROTATION_LOCKED,
                    ActiveEvents::COLLISION_EVENTS,
                    CollisionGroups::new(player_group, enemy_group),
                    Friction::coefficient(0.0),
                    Restitution::coefficient(0.5),
                    Dominance::group(10),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        DamageSensor,
                        Collider::ball(16.0),
                        Sensor,
                        ActiveEvents::COLLISION_EVENTS,
                        // Key change: Make sure sensor can detect enemies and enemies can detect sensor
                        CollisionGroups {
                            memberships: sensor_group,
                            filters: enemy_group,
                        },
                        TransformBundle::default(),
                    ));
                });
        }
    }

    // Enemy setup
    for entity in new_enemies.iter() {
        info!("Setting up enemy entity: {:?}", entity);
        if commands.get_entity(entity).is_some() {
            commands.entity(entity).insert((
                RigidBody::Dynamic,
                Collider::ball(12.0),
                Velocity::zero(),
                LockedAxes::ROTATION_LOCKED,
                Damping {
                    linear_damping: 2.0,
                    angular_damping: 1.0,
                },
                ActiveEvents::COLLISION_EVENTS,
                // Key change: Make sure enemies can detect sensors
                CollisionGroups {
                    memberships: enemy_group,
                    filters: player_group | enemy_group | projectile_group | sensor_group,
                },
                ColliderMassProperties::Density(1.0),
                Friction::coefficient(0.0),
                Restitution::coefficient(0.5),
                Dominance::group(0),
            ));
        }
    }

    // Projectile setup - now as solid bodies
    for entity in new_projectiles.iter() {
        info!("Setting up projectile entity: {:?}", entity);
        if commands.get_entity(entity).is_some() {
            commands.entity(entity).insert((
                RigidBody::Dynamic,
                Collider::ball(8.0),
                LockedAxes::ROTATION_LOCKED,
                ActiveEvents::COLLISION_EVENTS,
                CollisionGroups::new(projectile_group, enemy_group),
                // No Sensor anymore
                Velocity::zero(),
                Dominance::group(5), // Higher than enemies but lower than player
            ));
        }
    }
}

pub fn handle_collision_events(
    mut collision_events: EventReader<CollisionEvent>,
    mut commands: Commands,
    player_query: Query<Entity, With<Player>>,
    mut health_query: Query<&mut Health>,
    enemy_query: Query<Entity, With<Enemy>>,
    damage_sensor_query: Query<(Entity, &Parent), With<DamageSensor>>,
    projectile_query: Query<(Entity, &Projectile)>,
    time: Res<Time<Virtual>>,
) {
    let mut enemy_collisions = HashSet::new();
    let mut processed_projectiles = HashSet::new();

    // Get the player entity
    let player_entity = if let Ok(entity) = player_query.get_single() {
        entity
    } else {
        return;
    };

    info!("Processing {} collision events", collision_events.len());

    // Process all collision events
    for event in collision_events.read() {
        match event {
            CollisionEvent::Started(e1, e2, flags) => {
                info!(
                    "Collision started between {:?} and {:?} with flags {:?}",
                    e1, e2, flags
                );

                // Handle damage sensor collisions
                if flags.contains(CollisionEventFlags::SENSOR) {
                    info!("Sensor collision detected");
                    // Check if either entity is our damage sensor
                    if let Some((sensor_entity, parent)) = damage_sensor_query
                        .iter()
                        .find(|(sensor, _)| *sensor == *e1 || *sensor == *e2)
                    {
                        info!("Found damage sensor collision");

                        // If the parent is our player and the other entity is an enemy
                        if parent.get() == player_entity {
                            let other_entity = if *e1 == sensor_entity { *e2 } else { *e1 };
                            if enemy_query.contains(other_entity) {
                                info!("Adding enemy to collision set");
                                enemy_collisions.insert(other_entity);
                            }
                        }
                    }
                } else {
                    // Handle projectile collisions
                    let proj_entity = if projectile_query.contains(*e1) {
                        Some(*e1)
                    } else if projectile_query.contains(*e2) {
                        Some(*e2)
                    } else {
                        None
                    };

                    let enemy_entity = if enemy_query.contains(*e1) {
                        Some(*e1)
                    } else if enemy_query.contains(*e2) {
                        Some(*e2)
                    } else {
                        None
                    };

                    if let (Some(proj), Some(enemy)) = (proj_entity, enemy_entity) {
                        if !processed_projectiles.contains(&proj) {
                            processed_projectiles.insert(proj);

                            if let Ok(mut enemy_health) = health_query.get_mut(enemy) {
                                enemy_health.current -= 25.0;
                                info!("Enemy hit! Health remaining: {}", enemy_health.current);

                                if enemy_health.current <= 0.0 {
                                    commands.entity(enemy).despawn();
                                    info!("Enemy killed!");
                                }
                            }

                            commands.entity(proj).despawn();
                            info!("Projectile despawned");
                        }
                    }
                }
            }
            CollisionEvent::Stopped(e1, e2, flags) => {
                info!(
                    "Collision stopped between {:?} and {:?} with flags {:?}",
                    e1, e2, flags
                );
            }
        }
    }

    // Debug: Print collision set size
    if !enemy_collisions.is_empty() {
        info!(
            "Found {} enemy collisions this frame",
            enemy_collisions.len()
        );
    }

    // Apply damage if we have enemy collisions
    if !enemy_collisions.is_empty() {
        if let Ok(mut health) = health_query.get_mut(player_entity) {
            let time_since_last_tick = time.elapsed_seconds() % 0.25;
            if time_since_last_tick < time.delta_seconds() {
                let damage = 5.0 * enemy_collisions.len() as f32;
                health.current -= damage;
                info!(
                    "Player took {} damage from {} enemies! Health: {}",
                    damage,
                    enemy_collisions.len(),
                    health.current
                );
            }
        }
    }
}

// Helper function just for projectiles now
fn is_projectile_enemy_collision(
    entity1: Entity,
    entity2: Entity,
    projectile_query: &Query<(Entity, &Projectile)>,
    enemy_query: &Query<Entity, With<Enemy>>,
) -> Option<(Entity, Entity)> {
    if projectile_query.contains(entity1) && enemy_query.contains(entity2) {
        Some((entity1, entity2))
    } else if projectile_query.contains(entity2) && enemy_query.contains(entity1) {
        Some((entity2, entity1))
    } else {
        None
    }
}

fn is_player_enemy_collision(
    entity1: Entity,
    entity2: Entity,
    player_query: &Query<(&mut Health, &Transform), With<Player>>,
    enemy_query: &Query<(Entity, &Transform), With<Enemy>>,
) -> Option<(Entity, Entity)> {
    if player_query.contains(entity1) && enemy_query.contains(entity2) {
        Some((entity1, entity2))
    } else if player_query.contains(entity2) && enemy_query.contains(entity1) {
        Some((entity2, entity1))
    } else {
        None
    }
}

fn is_enemy_enemy_collision(
    entity1: Entity,
    entity2: Entity,
    enemy_query: &Query<Entity, With<Enemy>>,
) -> bool {
    enemy_query.contains(entity1) && enemy_query.contains(entity2)
}

pub fn enemy_pushback(mut enemy_query: Query<(Entity, &Transform, &mut Velocity), With<Enemy>>) {
    // Collect enemy positions first to avoid borrow checker issues
    let enemies: Vec<(Entity, Vec3)> = enemy_query
        .iter()
        .map(|(entity, transform, _)| (entity, transform.translation))
        .collect();

    for (entity_a, transform_a, mut velocity_a) in enemy_query.iter_mut() {
        let mut push = Vec2::ZERO;

        for (entity_b, transform_b) in &enemies {
            // Skip self comparison
            if entity_a == *entity_b {
                continue;
            }

            let diff = transform_a.translation - *transform_b;
            let dist = diff.length();

            if dist < 32.0 && dist > 0.0 {
                // 32.0 is double the enemy radius
                push += diff.truncate().normalize() * (1.0 - dist / 32.0) * 50.0;
            }
        }

        velocity_a.linvel += push;
    }
}
