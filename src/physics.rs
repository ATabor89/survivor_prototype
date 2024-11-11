use crate::components::{Enemy, Health, Player, Projectile};
use crate::resources::GameState;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

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

// When entities are created, set up their physics properties
pub fn setup_physics_bodies(
    mut commands: Commands,
    new_players: Query<Entity, (Added<Player>, Without<RigidBody>)>,  // Make sure entity exists and doesn't have physics yet
    new_enemies: Query<Entity, (Added<Enemy>, Without<RigidBody>)>,
    new_projectiles: Query<Entity, (Added<Projectile>, Without<RigidBody>)>,
) {
    // Set up collision groups
    let player_group = Group::GROUP_1;
    let enemy_group = Group::GROUP_2;
    let projectile_group = Group::GROUP_3;

    // Player setup
    for entity in new_players.iter() {
        if commands.get_entity(entity).is_some() {  // Double-check entity exists
            commands.entity(entity).insert((
                RigidBody::Dynamic,
                Collider::ball(16.0),
                Velocity::zero(),
                LockedAxes::ROTATION_LOCKED,
                Damping {
                    linear_damping: 10.0,
                    angular_damping: 1.0,
                },
                Sensor, // Player is still a sensor
                ActiveEvents::COLLISION_EVENTS,
                CollisionGroups::new(
                    player_group,
                    enemy_group, // Can collide with enemies
                ),
            ));
        }
    }

    // Enemies setup - now with solid collisions
    for entity in new_enemies.iter() {
        if commands.get_entity(entity).is_some() {
            commands.entity(entity).insert((
                RigidBody::Dynamic,
                Collider::ball(16.0),
                Velocity::zero(),
                LockedAxes::ROTATION_LOCKED,
                Damping {
                    linear_damping: 10.0,
                    angular_damping: 1.0,
                },
                // No Sensor component - enemies should be solid
                ActiveEvents::COLLISION_EVENTS,
                CollisionGroups::new(
                    enemy_group,
                    player_group | enemy_group | projectile_group, // Can collide with everything
                ),
            ));
        }
    }

    // Projectiles setup
    for entity in new_projectiles.iter() {
        if commands.get_entity(entity).is_some() {
            commands.entity(entity).insert((
                RigidBody::Dynamic,
                Collider::ball(8.0),
                Sensor,
                LockedAxes::ROTATION_LOCKED,
                ActiveEvents::COLLISION_EVENTS,
                CollisionGroups::new(
                    projectile_group,
                    enemy_group, // Can only collide with enemies
                ),
            ));
        }
    }
}

pub fn handle_collision_events(
    mut collision_events: EventReader<CollisionEvent>,
    mut commands: Commands,
    mut health_query: Query<&mut Health>,
    player_query: Query<Entity, With<Player>>,
    enemy_query: Query<Entity, With<Enemy>>,
    projectile_query: Query<(Entity, &Projectile)>,
    time: Res<Time>,
) {
    for event in collision_events.read() {
        if let CollisionEvent::Started(entity1, entity2, _flags) = event {
            // Check projectile-enemy collisions
            if let Some((proj_ent, enemy_ent)) = is_projectile_enemy_collision(
                *entity1, *entity2,
                &projectile_query, &enemy_query,
            ) {
                if let Ok((_, projectile)) = projectile_query.get(proj_ent) {
                    if let Ok(mut health) = health_query.get_mut(enemy_ent) {
                        health.current -= projectile.damage;
                        info!("Enemy hit! Health: {}", health.current);
                        commands.entity(proj_ent).despawn();
                    }
                }
                continue;
            }

            // Check player-enemy collisions
            if let Some((player_ent, _)) = is_player_enemy_collision(
                *entity1, *entity2,
                &player_query, &enemy_query,
            ) {
                if let Ok(mut health) = health_query.get_mut(player_ent) {
                    // Apply damage over time while touching
                    health.current -= 10.0 * time.delta_seconds();
                    info!("Player hit! Health: {}", health.current);
                }
                continue;
            }

            // Check enemy-enemy collisions (for physics pushback)
            if is_enemy_enemy_collision(*entity1, *entity2, &enemy_query) {
                // No damage, just physics pushback which Rapier handles automatically
                continue;
            }
        }
    }
}

// Helper functions to check collision types
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
    player_query: &Query<Entity, With<Player>>,
    enemy_query: &Query<Entity, With<Enemy>>,
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