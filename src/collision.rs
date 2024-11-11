use crate::components::{Enemy, Experience, Health, IntendedMovement, PhysicsBody, Player, Projectile, Velocity};
use crate::resources::GameState;
use bevy::prelude::*;

#[derive(Event)]
pub struct CollisionEvent {
    pub entity_a: Entity,
    pub entity_b: Entity,
    pub collision_type: CollisionType,
}

#[derive(Debug)]
pub enum CollisionType {
    PlayerEnemy,
    ProjectileEnemy,
    EnemyEnemy,
}

// Track entities that will be despawned at end of frame
#[derive(Component)]
pub struct DespawnMarker;

const PUSH_FORCE: f32 = 150.0; // Force applied when entities overlap
const MIN_SEPARATION: f32 = 2.0; // Minimum separation to maintain
const COLLISION_SUBSTEPS: u32 = 4; // Number of substeps for projectile movement

// System to set up physics bodies (run during entity spawn)
pub fn setup_physics_body(
    mut commands: Commands,
    new_entities: Query<Entity, (Added<Enemy>, Without<PhysicsBody>)>,
    player_query: Query<Entity, (Added<Player>, Without<PhysicsBody>)>,
    projectile_query: Query<Entity, (Added<Projectile>, Without<PhysicsBody>)>,
) {
    // Setup enemies
    for entity in new_entities.iter() {
        commands.entity(entity).insert(PhysicsBody {
            radius: 14.0,
            mass: 1.0,
        });
    }

    // Setup player
    for entity in player_query.iter() {
        commands.entity(entity).insert(PhysicsBody {
            radius: 14.0,
            mass: 1.0,
        });
    }

    // Setup projectiles
    for entity in projectile_query.iter() {
        commands.entity(entity).insert(PhysicsBody {
            radius: 8.0,
            mass: 0.1,
        });
    }
}

// System for enemies to declare their intended movement
pub fn enemy_movement_intent(
    mut commands: Commands,
    time: Res<Time>,
    player_query: Query<(&Transform, &PhysicsBody), With<Player>>,
    enemy_query: Query<(Entity, &Enemy, &Transform, &PhysicsBody), Without<DespawnMarker>>,
) {
    let (player_transform, player_body) = player_query.single();

    for (entity, enemy, transform, enemy_body) in enemy_query.iter() {
        let to_player = player_transform.translation - transform.translation;
        let distance = to_player.length();
        let min_distance = player_body.radius + enemy_body.radius + MIN_SEPARATION;

        let direction = if distance > min_distance {
            to_player.normalize()
        } else {
            // If too close, move away slightly
            -to_player.normalize() * 0.5
        };

        let intended_move = direction * enemy.speed * time.delta_seconds();
        commands.entity(entity).insert(IntendedMovement(intended_move));
    }
}

// Projectile movement and collision system with substeps
pub fn projectile_physics_system(
    mut commands: Commands,
    time: Res<Time>,
    mut collision_events: EventWriter<CollisionEvent>,
    projectile_query: Query<(Entity, &Transform, &PhysicsBody, &Projectile), Without<DespawnMarker>>,
    enemy_query: Query<(Entity, &Transform, &PhysicsBody), (With<Enemy>, Without<DespawnMarker>)>,
) {
    for (projectile_entity, projectile_transform, projectile_body, projectile) in projectile_query.iter() {
        let start_pos = projectile_transform.translation;
        let velocity = Vec3::new(
            projectile_transform.right().x,
            projectile_transform.right().y,
            0.0,
        ) * projectile.speed;

        let step_velocity = velocity * time.delta_seconds() / COLLISION_SUBSTEPS as f32;
        let mut current_pos = start_pos;

        // Check collision at multiple points along the path
        for step in 0..COLLISION_SUBSTEPS {
            let next_pos = current_pos + step_velocity;

            // Check for collisions at this substep
            for (enemy_entity, enemy_transform, enemy_body) in enemy_query.iter() {
                let to_enemy = enemy_transform.translation - current_pos;
                let distance = to_enemy.length();
                let combined_radius = projectile_body.radius + enemy_body.radius;

                if distance <= combined_radius {
                    collision_events.send(CollisionEvent {
                        entity_a: projectile_entity,
                        entity_b: enemy_entity,
                        collision_type: CollisionType::ProjectileEnemy,
                    });

                    #[cfg(debug_assertions)]
                    info!("Projectile collision at step {}/{}, distance: {}, angle: {}",
                        step,
                        COLLISION_SUBSTEPS,
                        distance,
                        to_enemy.normalize().dot(velocity.normalize()).acos().to_degrees()
                    );

                    return; // Exit early on collision
                }

                // Also check if the path segment intersects the enemy
                let closest_point = closest_point_on_line_segment(
                    current_pos,
                    next_pos,
                    enemy_transform.translation,
                );
                let closest_distance = (closest_point - enemy_transform.translation).length();

                if closest_distance <= combined_radius {
                    collision_events.send(CollisionEvent {
                        entity_a: projectile_entity,
                        entity_b: enemy_entity,
                        collision_type: CollisionType::ProjectileEnemy,
                    });

                    #[cfg(debug_assertions)]
                    info!("Projectile intersection at step {}/{}, distance: {}",
                        step,
                        COLLISION_SUBSTEPS,
                        closest_distance
                    );

                    return; // Exit early on collision
                }
            }

            current_pos = next_pos;
        }
    }
}

// Helper function to find closest point on a line segment to a point
fn closest_point_on_line_segment(start: Vec3, end: Vec3, point: Vec3) -> Vec3 {
    let line_vec = end - start;
    let point_vec = point - start;
    let line_length = line_vec.length();

    if line_length == 0.0 {
        return start;
    }

    let t = (point_vec.dot(line_vec) / line_length).clamp(0.0, line_length) / line_length;
    start + line_vec * t
}

// Debug visualization system (only compiled in debug mode)
#[cfg(debug_assertions)]
pub fn debug_visualization_system(
    mut gizmos: Gizmos,
    projectile_query: Query<(&Transform, &PhysicsBody), With<Projectile>>,
    enemy_query: Query<(&Transform, &PhysicsBody), With<Enemy>>,
) {
    // Draw projectile paths
    for (transform, body) in projectile_query.iter() {
        gizmos.circle_2d(
            transform.translation.truncate(),
            body.radius,
            Color::srgb(1.0, 1.0, 0.0),
        );
    }

    // Draw enemy collision circles
    for (transform, body) in enemy_query.iter() {
        gizmos.circle_2d(
            transform.translation.truncate(),
            body.radius,
            Color::srgb(1.0, 0.0, 0.0),
        );
    }
}

// Helper function for continuous collision detection
fn ray_circle_intersection(ray_start: Vec3, ray_end: Vec3, circle_center: Vec3, radius: f32) -> Option<f32> {
    let ray_dir = ray_end - ray_start;
    let to_circle = circle_center - ray_start;

    let a = ray_dir.dot(ray_dir);
    let b = -2.0 * to_circle.dot(ray_dir);
    let c = to_circle.dot(to_circle) - radius * radius;

    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        return None;
    }

    let t = (-b - discriminant.sqrt()) / (2.0 * a);
    if t >= 0.0 && t <= 1.0 {
        Some(t)
    } else {
        None
    }
}

// Regular physics collision system for non-projectile entities
pub fn physics_collision_system(
    mut collision_events: EventWriter<CollisionEvent>,
    mut commands: Commands,
    time: Res<Time>,
    physics_bodies: Query<(Entity, &PhysicsBody, &Transform), Without<Projectile>>,
    mut intended_movements: Query<(Entity, &mut IntendedMovement)>,
    player_query: Query<Entity, With<Player>>,
    enemy_query: Query<Entity, With<Enemy>>,
) {
    let player_entity = player_query.single();
    let mut movement_adjustments: Vec<(Entity, Vec3)> = Vec::new();

    let bodies: Vec<_> = physics_bodies.iter().collect();
    for (i, (entity_a, body_a, transform_a)) in bodies.iter().enumerate() {
        for (entity_b, body_b, transform_b) in bodies[i + 1..].iter() {
            let delta = transform_b.translation - transform_a.translation;
            let distance = delta.length();
            let min_distance = body_a.radius + body_b.radius + MIN_SEPARATION;

            if distance < min_distance {
                let push_direction = if distance > 0.0 {
                    delta.normalize()
                } else {
                    Vec3::new(rand::random::<f32>() - 0.5, rand::random::<f32>() - 0.5, 0.0).normalize()
                };

                let push_strength = (min_distance - distance) / min_distance * PUSH_FORCE * time.delta_seconds();
                let mass_ratio_a = body_b.mass / (body_a.mass + body_b.mass);
                let mass_ratio_b = body_a.mass / (body_a.mass + body_b.mass);

                let push_a = -push_direction * push_strength * mass_ratio_a;
                let push_b = push_direction * push_strength * mass_ratio_b;

                movement_adjustments.push((*entity_a, push_a));
                movement_adjustments.push((*entity_b, push_b));

                if enemy_query.contains(*entity_a) && enemy_query.contains(*entity_b) {
                    collision_events.send(CollisionEvent {
                        entity_a: *entity_a,
                        entity_b: *entity_b,
                        collision_type: CollisionType::EnemyEnemy,
                    });
                } else if (player_entity == *entity_a && enemy_query.contains(*entity_b)) ||
                    (player_entity == *entity_b && enemy_query.contains(*entity_a)) {
                    collision_events.send(CollisionEvent {
                        entity_a: *entity_a,
                        entity_b: *entity_b,
                        collision_type: CollisionType::PlayerEnemy,
                    });
                }
            }
        }
    }

    for (entity, mut intended_movement) in intended_movements.iter_mut() {
        for (adj_entity, adjustment) in movement_adjustments.iter() {
            if entity == *adj_entity {
                intended_movement.0 += *adjustment;
            }
        }
    }
}

// Rest of the systems remain mostly the same...
pub fn damage_system(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    mut health_query: ParamSet<(
        Query<&mut Health, With<Player>>,
        Query<(&mut Health, &Enemy), Without<DespawnMarker>>,
    )>,
    projectile_query: Query<&Projectile>,
    time: Res<Time>,
) {
    for collision in collision_events.read() {
        match collision.collision_type {
            CollisionType::PlayerEnemy => {
                if let Ok(mut player_health) = health_query.p0().get_single_mut() {
                    player_health.current -= 10.0 * time.delta_seconds();
                }
            }
            CollisionType::ProjectileEnemy => {
                if let Ok(projectile) = projectile_query.get(collision.entity_a) {
                    let mut enemy_query = health_query.p1();
                    if let Ok((mut enemy_health, _)) = enemy_query.get_mut(collision.entity_b) {
                        enemy_health.current -= projectile.damage;

                        commands.entity(collision.entity_a).insert(DespawnMarker);

                        if enemy_health.current <= 0.0 {
                            commands.entity(collision.entity_b).insert(DespawnMarker);
                        }
                    }
                }
            }
            CollisionType::EnemyEnemy => (), // No damage for enemy-enemy collisions
        }
    }
}

// System to apply final movements
pub fn apply_movement_system(
    mut commands: Commands,
    mut query: Query<(Entity, &IntendedMovement, &mut Transform)>,
) {
    for (entity, movement, mut transform) in query.iter_mut() {
        transform.translation += movement.0;
        commands.entity(entity).remove::<IntendedMovement>();
    }
}

pub fn death_system(
    mut commands: Commands,
    mut entity_query: ParamSet<(
        Query<(Entity, &Health), With<Player>>,
        Query<(Entity, &Health, &Enemy), With<DespawnMarker>>,
    )>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exp_query: Query<&mut Experience>,
) {
    // Check player death
    if let Ok((player_entity, player_health)) = entity_query.p0().get_single() {
        if player_health.current <= 0.0 {
            info!("Player died!");
            commands.entity(player_entity).despawn();
            next_state.set(GameState::GameOver);
            return;
        }
    }

    // Handle enemy deaths
    for (enemy_entity, _, enemy) in entity_query.p1().iter() {
        // Grant experience to player
        if let Ok(mut player_exp) = exp_query.get_single_mut() {
            player_exp.current += enemy.experience_value;

            // Level up check
            let exp_needed = player_exp.level * 100;
            if player_exp.current >= exp_needed {
                info!("Level up! Current level: {}", player_exp.level + 1);
                player_exp.current -= exp_needed;
                player_exp.level += 1;
                next_state.set(GameState::LevelUp);
            }
        }

        commands.entity(enemy_entity).despawn();
    }
}

// Add a cleanup system for marked entities
pub fn cleanup_system(
    mut commands: Commands,
    query: Query<Entity, With<DespawnMarker>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}