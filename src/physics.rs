use crate::combat::DamageEvent;
use crate::components::{Enemy, Player};
use crate::death::{MarkedForDeath, MarkedForDespawn};
use crate::resources::GameState;
use crate::GameplaySets;
use bevy::ecs::query::QuerySingleError;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

pub struct PhysicsPlugin;

// A component to mark our damage sensor
#[derive(Component)]
pub struct DamageSensor;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        // Base physics setup
        app.add_plugins(RapierPhysicsPlugin::<NoUserData>::default());

        // Since RapierConfiguration is now a component, we'll need a startup system
        // to configure the physics world
        app.add_systems(Startup, configure_physics);
        app.add_systems(Startup, verify_physics_config.after(configure_physics));

        #[cfg(debug_assertions)]
        {
            app.add_plugins(RapierDebugRenderPlugin::default());
        }

        app.add_systems(
            Update,
            (setup_physics_bodies, handle_player_enemy_collision)
                .chain()
                .in_set(GameplaySets::Physics)
                .run_if(in_state(GameState::Playing)),
        );
    }
}

fn configure_physics(
    mut commands: Commands,
    rapier_query: Query<(Entity, Option<&RapierConfiguration>), With<RapierContext>>,
) {
    match rapier_query.get_single() {
        Ok((entity, maybe_config)) => {
            if maybe_config.is_none() {
                info!(
                    "Adding RapierConfiguration to existing RapierContext entity {:?}",
                    entity
                );
                commands.entity(entity).insert(RapierConfiguration {
                    gravity: Vec2::ZERO,
                    physics_pipeline_active: true,
                    query_pipeline_active: true,
                    scaled_shape_subdivision: 10,
                    force_update_from_transform_changes: false,
                });
            }
        }
        Err(QuerySingleError::NoEntities(_)) => {
            error!("No RapierContext found! Physics systems may not initialize correctly.");
        }
        Err(QuerySingleError::MultipleEntities(_)) => {
            error!("Multiple RapierContext components found! This may cause physics issues.");
        }
    }
}

fn verify_physics_config(
    config_query: Query<(Entity, &RapierConfiguration)>,
    context_query: Query<Entity, With<RapierContext>>,
) {
    // Log RapierConfiguration status
    match config_query.get_single() {
        Ok((entity, config)) => {
            info!(
                "Physics world found on entity {:?} with gravity: {:?}",
                entity, config.gravity
            );
            if config.gravity != Vec2::ZERO {
                warn!("Physics world has non-zero gravity!");
            }
        }
        Err(QuerySingleError::NoEntities(_)) => {
            error!("No RapierConfiguration found in world!");
        }
        Err(QuerySingleError::MultipleEntities(_)) => {
            error!("Multiple RapierConfiguration components found!");
        }
    }

    // Log RapierContext status
    let context_count = context_query.iter().count();
    match context_count {
        0 => error!("No RapierContext found in world!"),
        1 => {
            let context_entity = context_query.single();
            info!("Found single RapierContext on entity {:?}", context_entity);
        }
        n => {
            error!("Found {} RapierContext components!", n);
            for entity in context_query.iter() {
                error!("RapierContext found on entity {:?}", entity);
            }
        }
    }
}

pub(crate) fn handle_rapier_context_error(error: QuerySingleError) -> ! {
    match error {
        QuerySingleError::NoEntities(_) => {
            panic!("No RapierContext found in world! This suggests the physics world was not properly initialized.");
        }
        QuerySingleError::MultipleEntities(count) => {
            panic!("Found {} RapierContext components! Expected exactly one. This may be caused by multiple physics worlds being created.", count);
        }
    }
}

pub fn setup_physics_bodies(
    mut commands: Commands,
    new_players: Query<Entity, (Added<Player>, Without<RigidBody>)>,
    new_enemies: Query<Entity, (Added<Enemy>, Without<RigidBody>)>,
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
                        Transform::default(),
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
}

pub fn handle_player_enemy_collision(
    context_query: Query<&RapierContext>,
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
    let rapier_context = context_query
        .get_single()
        .unwrap_or_else(|e| handle_rapier_context_error(e));

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
            amount: 1 * intersecting_enemies,
            source: None,
        });
    }
}
