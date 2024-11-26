use crate::combat::{DamageEvent, ProjectileStats};
use crate::components::{Enemy, Player, Projectile};
use crate::death::{MarkedForDeath, MarkedForDespawn};
use crate::resources::GameState;
use crate::GameplaySets;
use bevy::prelude::*;
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
            (
                setup_physics_bodies,
                handle_player_enemy_collision,
                handle_projectile_collision,
            )
                .chain()
                .in_set(GameplaySets::Physics)
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

pub fn handle_player_enemy_collision(
    rapier_context: Res<RapierContext>,
    time: Res<Time<Virtual>>,
    player_query: Query<(Entity, &Transform), With<Player>>,
    enemy_query: Query<
        Entity,
        (
            With<Enemy>,
            Without<MarkedForDespawn>,
            Without<MarkedForDeath>,
        ),
    >,
    damage_sensor_query: Query<(Entity, &Parent), With<DamageSensor>>,
    mut damage_events: EventWriter<DamageEvent>,
) {
    // Get player entity and their damage sensor
    let (player_entity, sensor_entity) = if let Ok((player_entity, _)) = player_query.get_single() {
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

    // Count intersecting enemies that aren't marked for death/despawn
    let mut intersecting_enemies = 0;
    for (collider1, collider2, intersecting) in
        rapier_context.intersection_pairs_with(sensor_entity)
    {
        if !intersecting {
            continue;
        }

        let other_entity = if collider1 == sensor_entity {
            collider2
        } else {
            collider1
        };

        if enemy_query.contains(other_entity) {
            intersecting_enemies += 1;
        }
    }

    // Send damage event if there are intersecting enemies
    if intersecting_enemies > 0 {
        damage_events.send(DamageEvent {
            target: player_entity,
            amount: 1.0 * intersecting_enemies as f32,
            source: None,
        });
    }
}

pub fn handle_projectile_collision(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    mut collision_events: EventReader<CollisionEvent>,
    mut projectile_query: Query<(Entity, &mut ProjectileStats), With<Projectile>>,
    enemy_query: Query<
        Entity,
        (
            With<Enemy>,
            Without<MarkedForDespawn>,
            Without<MarkedForDeath>,
        ),
    >,
    mut damage_events: EventWriter<DamageEvent>,
) {
    for event in collision_events.read() {
        if let CollisionEvent::Started(e1, e2, _) = event {
            // Find which entity is the projectile and which is the enemy
            let (projectile_entity, enemy_entity, mut projectile_stats) =
                if let Ok((proj_entity, stats)) = projectile_query.get_mut(*e1) {
                    if enemy_query.contains(*e2) {
                        (proj_entity, *e2, stats)
                    } else {
                        continue;
                    }
                } else if let Ok((proj_entity, stats)) = projectile_query.get_mut(*e2) {
                    if enemy_query.contains(*e1) {
                        (proj_entity, *e1, stats)
                    } else {
                        continue;
                    }
                } else {
                    continue;
                };

            let current_time = time.elapsed_seconds();

            // Check pierce cooldown
            if current_time - projectile_stats.last_hit_time < projectile_stats.pierce_cooldown {
                continue;
            }

            // Send damage event
            damage_events.send(DamageEvent {
                target: enemy_entity,
                amount: projectile_stats.damage,
                source: Some(projectile_entity),
            });

            projectile_stats.last_hit_time = current_time;

            // Handle piercing
            if projectile_stats.pierce_remaining > 0 {
                projectile_stats.pierce_remaining -= 1;
                if projectile_stats.pierce_remaining == 0 {
                    commands.entity(projectile_entity).insert(MarkedForDeath);
                }
            } else {
                commands.entity(projectile_entity).insert(MarkedForDeath);
            }
        }
    }
}
