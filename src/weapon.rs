use crate::combat::DamageEvent;
use crate::components::Enemy;
use crate::death::{MarkedForDeath, MarkedForDespawn};
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

// Core trait for weapon behavior
trait WeaponEffect: Send + Sync + 'static {
    fn update(&mut self, commands: &mut Commands, owner: Entity, transform: &Transform, time: f32);
    fn on_hit(&mut self, commands: &mut Commands, entity: Entity, target: Entity);
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

#[derive(Component)]
pub struct CircleMagick {
    // Core properties
    pub radius: f32,
    pub duration: f32, // How long the circle lasts
    pub power: f32,    // Base effect strength

    // Rotation mechanics
    pub rotation_speed: f32,
    pub current_angle: f32,

    // Visual and effect properties
    pub pattern_type: PatternType,
    pub num_sigils: u32,   // Number of sigils around the circle
    pub sigil_radius: f32, // Size of individual sigils
    pub active_time: f32,  // How long it's been active

    // Effect state
    pub effect_interval: f32,  // How often to apply effects
    pub last_effect_time: f32, // Track last effect application
}

impl Default for CircleMagick {
    fn default() -> Self {
        Self {
            radius: 64.0,
            duration: 5.0,
            power: 10.0,
            rotation_speed: 1.0,
            current_angle: 0.0,
            pattern_type: PatternType::Protection,
            num_sigils: 4,
            sigil_radius: 8.0,
            active_time: 0.0,
            effect_interval: 0.5,
            last_effect_time: 0.0,
        }
    }
}

// Marker component for sigils that are part of a circle
#[derive(Component)]
pub struct CircleSigil {
    pub parent_circle: Entity,
    pub index: u32,
}

// Bundle for spawning a complete circle
#[derive(Bundle)]
pub struct CircleMagickBundle {
    pub circle: CircleMagick,
    pub sprite: SpriteBundle,
    // Add any other components needed for visualization/collision
}

// Components for different effects
#[derive(Component)]
struct PatternEffect {
    pattern_type: PatternType,
    completion_progress: f32,
}

#[derive(Component)]
struct SigilEffect {
    intent: SigilIntent,
    drawing_progress: f32,
}

pub fn spawn_circle_magick(
    commands: &mut Commands,
    position: Vec3,
    pattern_type: PatternType,
) -> Entity {
    let circle_radius = 64.0;
    let circle_group = Group::GROUP_3;
    let enemy_group = Group::GROUP_2;

    // Create a basic circle texture (a filled circle with transparency)
    let circle = commands
        .spawn((
            CircleMagickBundle {
                circle: CircleMagick {
                    pattern_type,
                    ..default()
                },
                sprite: SpriteBundle {
                    sprite: Sprite {
                        color: Color::srgba(0.5, 0.5, 1.0, 0.3),
                        // Make sure sprite size matches the collider diameter
                        custom_size: Some(Vec2::new(circle_radius * 2.0, circle_radius * 2.0)),
                        ..default()
                    },
                    transform: Transform::from_translation(position),
                    ..default()
                },
            },
            Sensor,
            Collider::ball(circle_radius), // Radius matches sprite size
            ActiveEvents::COLLISION_EVENTS,
            CollisionGroups::new(circle_group, enemy_group),
        ))
        .id();

    // Sigil size relative to circle size
    let sigil_size = circle_radius * 0.25; // 1/4 the radius of the circle

    // Spawn rotating sigils
    for i in 0..4 {
        let angle = (i as f32) * std::f32::consts::PI * 0.5;
        let offset = Vec2::new(angle.cos(), angle.sin()) * circle_radius;

        commands.spawn((
            CircleSigil {
                parent_circle: circle,
                index: i,
            },
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgba(0.7, 0.7, 1.0, 0.8),
                    custom_size: Some(Vec2::new(sigil_size, sigil_size)),
                    ..default()
                },
                transform: Transform::from_translation(
                    position + Vec3::new(offset.x, offset.y, 0.1),
                ),
                ..default()
            },
        ));
    }

    circle
}

pub fn update_circle_magick(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    mut param_set: ParamSet<(
        Query<(Entity, &mut CircleMagick, &Transform), Without<MarkedForDeath>>,
        Query<(Entity, &CircleSigil, &mut Transform)>,
    )>,
    enemy_query: Query<&Enemy, (Without<MarkedForDeath>, Without<MarkedForDespawn>)>,
    rapier_context: Res<RapierContext>,
    mut damage_events: EventWriter<DamageEvent>,
) {
    let current_time = time.elapsed_seconds();

    // First, gather circle updates and find expired circles
    let mut circles_to_update = Vec::new();
    let mut sigils_to_mark = Vec::new();

    {
        let mut circle_query = param_set.p0();
        for (circle_entity, mut circle, circle_transform) in circle_query.iter_mut() {
            // Update circle lifetime
            circle.active_time += time.delta_seconds();
            if circle.active_time >= circle.duration {
                // Mark the circle for death
                commands.entity(circle_entity).insert(MarkedForDeath);

                // Store circle entity to find its sigils later
                sigils_to_mark.push(circle_entity);
                continue;
            }

            // Apply damage effect if it's time
            if current_time - circle.last_effect_time >= circle.effect_interval {
                let mut enemies_in_circle = 0;

                for (collider1, collider2, intersecting) in
                    rapier_context.intersection_pairs_with(circle_entity)
                {
                    if !intersecting {
                        continue;
                    }

                    let other_entity = if collider1 == circle_entity {
                        collider2
                    } else {
                        collider1
                    };

                    if enemy_query.contains(other_entity) {
                        enemies_in_circle += 1;
                        info!("Spawning damage event for enemy: {:?}", other_entity);
                        damage_events.send(DamageEvent {
                            target: other_entity,
                            amount: circle.power,
                            source: Some(circle_entity),
                        });
                    }
                }

                circle.last_effect_time = current_time;
            }

            // Update rotation
            circle.current_angle += circle.rotation_speed * time.delta_seconds();

            circles_to_update.push((
                circle_entity,
                circle.current_angle,
                circle.radius,
                circle_transform.translation,
            ));
        }
    }

    // Mark sigils of expired circles for death
    {
        let sigil_query = param_set.p1();
        for circle_entity in sigils_to_mark {
            for (sigil_entity, sigil, _) in sigil_query
                .iter()
                .filter(|(_, s, _)| s.parent_circle == circle_entity)
            {
                commands.entity(sigil_entity).insert(MarkedForDeath);
            }
        }
    }

    // Update sigil positions
    {
        let mut sigil_query = param_set.p1();
        for (circle_entity, current_angle, radius, circle_pos) in circles_to_update {
            for (_, sigil, mut sigil_transform) in sigil_query.iter_mut() {
                if sigil.parent_circle == circle_entity {
                    let angle = current_angle + (sigil.index as f32) * std::f32::consts::PI * 0.5;
                    let offset = Vec2::new(angle.cos(), angle.sin()) * radius;

                    sigil_transform.translation = circle_pos + Vec3::new(offset.x, offset.y, 0.1);
                }
            }
        }
    }
}
