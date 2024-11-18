use crate::components::{Enemy, Health, MarkedForDespawn, Player, Projectile};
use crate::resources::{GameState, GameStats, LastDamageTime};
use bevy::prelude::*;
use bevy::utils::HashSet;
use bevy_rapier2d::prelude::*;

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
    let player_group = Group::GROUP_1;
    let enemy_group = Group::GROUP_2;
    let projectile_group = Group::GROUP_3;
    let experience_group = Group::GROUP_4;

    // Player setup
    for entity in new_players.iter() {
        if commands.get_entity(entity).is_some() {
            commands
                .entity(entity)
                .insert((
                    RigidBody::KinematicPositionBased,
                    Collider::ball(12.0),
                    ActiveEvents::COLLISION_EVENTS,
                    CollisionGroups::new(player_group, enemy_group | experience_group),
                    Velocity::zero(),
                    LockedAxes::ROTATION_LOCKED,
                ))
                .with_children(|children| {
                    // Simple sensor setup
                    children.spawn((
                        Collider::ball(16.0),
                        Sensor,
                        ActiveEvents::COLLISION_EVENTS,
                        DamageSensor,
                        TransformBundle::default(),
                    ));
                });
        }
    }

    // Enemy setup
    for entity in new_enemies.iter() {
        if commands.get_entity(entity).is_some() {
            commands.entity(entity).insert((
                RigidBody::Dynamic,
                Collider::ball(12.0),
                Velocity::zero(),
                LockedAxes::ROTATION_LOCKED,
                ActiveEvents::COLLISION_EVENTS,
                CollisionGroups::new(enemy_group, player_group | enemy_group | projectile_group),
                Damping {
                    linear_damping: 2.0,
                    angular_damping: 1.0,
                },
            ));
        }
    }

    // Projectiles setup
    for entity in new_projectiles.iter() {
        if commands.get_entity(entity).is_some() {
            commands.entity(entity).insert((
                RigidBody::Dynamic,
                Collider::ball(8.0),
                LockedAxes::ROTATION_LOCKED,
                ActiveEvents::COLLISION_EVENTS,
                CollisionGroups::new(projectile_group, enemy_group),
            ));
        }
    }
}

pub fn handle_collision_events(
    mut collision_events: EventReader<CollisionEvent>,
    rapier_context: Res<RapierContext>,
    mut commands: Commands,
    player_query: Query<Entity, With<Player>>,
    mut health_query: Query<&mut Health>,
    enemy_query: Query<Entity, (With<Enemy>, Without<MarkedForDespawn>)>,
    damage_sensor_query: Query<(Entity, &Parent), With<DamageSensor>>,
    projectile_query: Query<(Entity, &Projectile)>,
    time: Res<Time<Virtual>>,
    mut last_damage: ResMut<LastDamageTime>,
) {
    // Get player entity and their damage sensor
    let (player_entity, sensor_entity) = if let Ok(player_entity) = player_query.get_single() {
        if let Some((sensor_entity, _)) = damage_sensor_query
            .iter()
            .find(|(_, parent)| parent.get() == player_entity)
        {
            (player_entity, sensor_entity)
        } else {
            return;
        }
    } else {
        return;
    };

    // Handle projectile collisions
    let mut processed_projectiles = HashSet::new();
    let mut marked_enemies = HashSet::new(); // Track enemies marked in this frame

    for event in collision_events.read() {
        if let CollisionEvent::Started(e1, e2, _) = event {
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
                // Skip if we've already marked this enemy for death in this frame
                if marked_enemies.contains(&enemy) {
                    continue;
                }

                if !processed_projectiles.contains(&proj) {
                    processed_projectiles.insert(proj);

                    if let Ok(mut enemy_health) = health_query.get_mut(enemy) {
                        enemy_health.current -= 25.0;
                        info!("Enemy hit! Health: {}", enemy_health.current);

                        if enemy_health.current <= 0.0 {
                            // Mark for death instead of immediate despawn
                            commands.entity(enemy).insert(MarkedForDespawn);
                            marked_enemies.insert(enemy);
                            info!("Enemy marked for death!");
                        }
                    }

                    commands.entity(proj).despawn();
                    info!("Projectile despawned");
                }
            }
        }
    }

    // Count how many enemies are currently intersecting with our sensor
    let mut intersecting_enemies = 0;
    for (collider1, collider2, intersecting) in
        rapier_context.intersection_pairs_with(sensor_entity)
    {
        if intersecting {
            let other_entity = if collider1 == sensor_entity {
                collider2
            } else {
                collider1
            };

            if enemy_query.contains(other_entity) {
                intersecting_enemies += 1;
            }
        }
    }

    // Apply damage if we have intersecting enemies
    if intersecting_enemies > 0 {
        let current_time = time.elapsed_seconds();
        if current_time - last_damage.0 >= 0.25 {
            if let Ok(mut health) = health_query.get_mut(player_entity) {
                let damage = 1.0 * intersecting_enemies as f32;
                health.current -= damage;
                last_damage.0 = current_time;
                info!(
                    "Player took {} damage from {} enemies! Health: {}",
                    damage, intersecting_enemies, health.current
                );
            }
        }
    }
}
