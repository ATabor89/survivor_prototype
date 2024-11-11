use crate::components::{Enemy, Health, Player, Projectile};
use crate::resources::GameState;
use bevy::prelude::*;
use bevy::utils::HashSet;
use bevy_rapier2d::prelude::*;
use bevy_rapier2d::rapier::geometry::CollisionEventFlags;

pub struct PhysicsPlugin;

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

        app.add_systems(Update, (
            setup_physics_bodies,
            handle_collision_events,
        )
            .chain()
            .run_if(in_state(GameState::Playing)));
    }
}

pub fn maintain_separation(
    mut query_set: ParamSet<(
        Query<&Transform, With<Player>>,
        Query<&mut Transform, With<Enemy>>
    )>,
) {
    // First get the player position
    let player_pos = if let Ok(player_transform) = query_set.p0().get_single() {
        player_transform.translation
    } else {
        return;
    };

    // Then handle enemy positions
    for mut enemy_transform in query_set.p1().iter_mut() {
        let diff = enemy_transform.translation - player_pos;
        let distance = diff.length();

        // Get the combined radius of both colliders (assuming they're circles)
        let min_distance = 32.0; // Combined radius of player and enemy

        if distance < min_distance && distance > 0.0 {
            // Calculate separation vector
            let separation = diff.normalize() * (min_distance - distance);
            // Move enemy away to maintain minimum distance
            enemy_transform.translation += separation;
        }
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

    // info!("Setting up physics bodies");

    // Player setup
    for entity in new_players.iter() {
        info!("Setting up player entity: {:?}", entity);
        if commands.get_entity(entity).is_some() {
            commands.entity(entity).insert((
                RigidBody::KinematicPositionBased,
                Collider::ball(16.0),
                Velocity::zero(),
                LockedAxes::ROTATION_LOCKED,
                ActiveEvents::COLLISION_EVENTS,
                CollisionGroups::new(player_group, enemy_group),
                Friction::coefficient(0.0),
                Restitution::coefficient(0.5),
                Dominance::group(10),
            ));
        }
    }

    // Enemy setup
    for entity in new_enemies.iter() {
        info!("Setting up enemy entity: {:?}", entity);
        if commands.get_entity(entity).is_some() {
            commands.entity(entity).insert((
                RigidBody::Dynamic,
                Collider::ball(16.0),
                Velocity::zero(),
                LockedAxes::ROTATION_LOCKED,
                Damping {
                    linear_damping: 2.0,
                    angular_damping: 1.0,
                },
                ActiveEvents::COLLISION_EVENTS,
                CollisionGroups::new(
                    enemy_group,
                    player_group | enemy_group | projectile_group,
                ),
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
    projectile_query: Query<(Entity, &Projectile)>,
    time: Res<Time>,
) {
    let mut enemy_collisions = HashSet::new();
    let mut processed_projectiles = HashSet::new();

    // Get player entity
    let player_entity = match player_query.get_single() {
        Ok(entity) => entity,
        Err(_) => return,
    };

    // Process all collisions first
    for event in collision_events.read() {
        match event {
            CollisionEvent::Started(e1, e2, _) => {
                // Handle player-enemy collisions
                if (*e1 == player_entity && enemy_query.contains(*e2)) {
                    enemy_collisions.insert(*e2);
                } else if (*e2 == player_entity && enemy_query.contains(*e1)) {
                    enemy_collisions.insert(*e1);
                }

                // Handle projectile-enemy collisions
                let proj_entity = if projectile_query.contains(*e1) { Some(*e1) }
                else if projectile_query.contains(*e2) { Some(*e2) }
                else { None };

                let enemy_entity = if enemy_query.contains(*e1) { Some(*e1) }
                else if enemy_query.contains(*e2) { Some(*e2) }
                else { None };

                if let (Some(proj), Some(enemy)) = (proj_entity, enemy_entity) {
                    if !processed_projectiles.contains(&proj) {
                        processed_projectiles.insert(proj);

                        // Get enemy health and apply damage
                        if let Ok(mut enemy_health) = health_query.get_mut(enemy) {
                            enemy_health.current -= 15.0;
                            info!("Enemy hit! Health remaining: {}", enemy_health.current);

                            // If enemy dies, despawn it
                            if enemy_health.current <= 0.0 {
                                commands.entity(enemy).despawn();
                                info!("Enemy killed!");
                            }
                        }

                        // Always despawn the projectile
                        commands.entity(proj).despawn();
                        info!("Projectile despawned");
                    }
                }
            }
            CollisionEvent::Stopped(_, _, _) => {},
        }
    }

    // Apply player damage if we have enemy collisions
    if !enemy_collisions.is_empty() {
        if let Ok(mut health) = health_query.get_mut(player_entity) {
            let time_since_last_tick = time.elapsed_seconds() % 0.25;
            if time_since_last_tick < time.delta_seconds() {
                let damage = 5.0 * enemy_collisions.len() as f32;
                health.current -= damage;
                info!("Player took {} damage from {} enemies! Health: {}", 
                      damage, enemy_collisions.len(), health.current);
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

pub fn enemy_pushback(
    mut enemy_query: Query<(Entity, &Transform, &mut Velocity), With<Enemy>>,
) {
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

            if dist < 32.0 && dist > 0.0 {  // 32.0 is double the enemy radius
                push += diff.truncate().normalize() * (1.0 - dist / 32.0) * 50.0;
            }
        }

        velocity_a.linvel += push;
    }
}