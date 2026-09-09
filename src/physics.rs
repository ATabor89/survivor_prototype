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

        // Rapier propagates transforms before its simulation. Bevy 0.19's
        // hierarchy optimization needs dirty trees marked before that pass,
        // otherwise movement of a body with children can be overwritten.
        app.add_systems(
            PostUpdate,
            bevy::transform::systems::mark_dirty_trees.before(RapierTransformPropagateSet),
        );

        // Since RapierConfiguration is now a component, we'll need a startup system
        // to configure the physics world
        app.add_systems(Startup, configure_physics);
        app.add_systems(Startup, verify_physics_config.after(configure_physics));

        #[cfg(debug_assertions)]
        {
            app.add_plugins(RapierDebugRenderPlugin {
                enabled: false,
                ..default()
            });
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

fn configure_physics(mut configurations: Query<&mut RapierConfiguration>) {
    for mut configuration in &mut configurations {
        configuration.gravity = Vec2::ZERO;
    }
}
fn verify_physics_config(
    config_query: Query<(Entity, &RapierConfiguration)>,
    context_query: Query<Entity, With<DefaultRapierContext>>,
) {
    // Log RapierConfiguration status
    match config_query.single() {
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
        if commands.get_entity(entity).is_ok() {
            commands
                .entity(entity)
                .insert((
                    RigidBody::KinematicPositionBased,
                    KinematicCharacterController {
                        autostep: None,
                        snap_to_ground: None,
                        // Top-down movement has no uphill/downhill restrictions.
                        max_slope_climb_angle: std::f32::consts::PI,
                        min_slope_slide_angle: -std::f32::consts::PI,
                        apply_impulse_to_dynamic_bodies: false,
                        filter_groups: Some(CollisionGroups::new(
                            player_group,
                            crate::arena::WORLD_GROUP,
                        )),
                        ..default()
                    },
                    Collider::ball(12.0),
                    ActiveEvents::COLLISION_EVENTS,
                    CollisionGroups::new(
                        player_group,
                        enemy_group
                            | projectile_group
                            | experience_group
                            | crate::arena::WORLD_GROUP,
                    ),
                    Velocity::zero(),
                    LockedAxes::ROTATION_LOCKED,
                ))
                .with_children(|children| {
                    // Simple sensor setup
                    children.spawn((
                        Collider::ball(16.0),
                        Sensor,
                        ActiveEvents::COLLISION_EVENTS,
                        CollisionGroups::new(player_group, enemy_group),
                        DamageSensor,
                        Transform::default(),
                    ));
                });
        }
    }

    // Enemy setup
    for entity in new_enemies.iter() {
        if commands.get_entity(entity).is_ok() {
            commands.entity(entity).insert((
                RigidBody::Dynamic,
                Collider::ball(12.0),
                Velocity::zero(),
                LockedAxes::ROTATION_LOCKED,
                ActiveEvents::COLLISION_EVENTS,
                CollisionGroups::new(
                    enemy_group,
                    player_group | enemy_group | projectile_group | crate::arena::WORLD_GROUP,
                ),
                Damping {
                    linear_damping: 2.0,
                    angular_damping: 1.0,
                },
            ));
        }
    }
}

#[expect(
    clippy::type_complexity,
    reason = "Explicit ECS filters exclude dead enemies from contact damage."
)]
pub fn handle_player_enemy_collision(
    context_query: ReadRapierContext,
    player_query: Query<(Entity, &Transform), With<Player>>,
    enemy_query: Query<
        Entity,
        (
            With<Enemy>,
            Without<MarkedForDespawn>,
            Without<MarkedForDeath>,
        ),
    >,
    damage_sensor_query: Query<(Entity, &ChildOf), With<DamageSensor>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    // Get player entity and their damage sensor
    let (player_entity, sensor_entity) = if let Ok((player_entity, _)) = player_query.single() {
        if let Some((sensor_entity, _)) = damage_sensor_query
            .iter()
            .find(|(_, parent)| parent.parent() == player_entity)
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
        .single()
        .expect("Expected one initialized Rapier context");

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
        damage_events.write(DamageEvent {
            target: player_entity,
            amount: intersecting_enemies,
            source: None,
        });
    }
}
