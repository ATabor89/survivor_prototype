use bevy::math::FloatOrd;
use crate::components::{Combat, Enemy, Health, Player, Projectile};
use crate::death::MarkedForDeath;
use bevy::prelude::*;
use bevy_rapier2d::dynamics::{Dominance, LockedAxes, RigidBody, Velocity};
use bevy_rapier2d::geometry::{ActiveEvents, Collider, CollisionGroups, Group};
use bevy_rapier2d::math::Vect;
use crate::resources::GameTextures;

#[derive(Event)]
pub struct DamageEvent {
    pub target: Entity,
    pub amount: f32,
    pub source: Option<Entity>,
}

#[derive(Component)]
pub struct LastDamageTime {
    pub time: f32,
    pub cooldown: f32,
}

impl Default for LastDamageTime {
    fn default() -> Self {
        Self {
            time: 0.0,
            cooldown: 0.25, // Default to 0.25s between damage
        }
    }
}

#[derive(Component)]
pub struct ProjectileStats {
    pub damage: f32,
    pub pierce_remaining: u32,
    pub pierce_cooldown: f32,
    pub last_hit_time: f32,
}

impl ProjectileStats {
    pub fn new(damage: f32, pierce: u32) -> Self {
        Self {
            damage,
            pierce_remaining: pierce,
            pierce_cooldown: 0.1,
            last_hit_time: 0.0,
        }
    }
}

pub fn combat_system(
    mut commands: Commands,
    game_textures: Res<GameTextures>,
    time: Res<Time<Virtual>>,
    mut player_query: Query<(&Transform, &mut Combat), With<Player>>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
) {
    for (player_transform, mut combat) in player_query.iter_mut() {
        if time.elapsed_seconds() - combat.last_attack >= 1.0 / combat.attack_speed {
            if let Some((_, enemy_transform)) = enemy_query.iter().min_by_key(|(_, transform)| {
                FloatOrd((transform.translation - player_transform.translation).length())
            }) {
                let direction =
                    (enemy_transform.translation - player_transform.translation).normalize();
                let velocity = Vect::new(direction.x, direction.y);

                commands.spawn((
                    Projectile {
                        speed: 300.0,
                    },
                    ProjectileStats::new(combat.attack_damage, 1), // Store damage here only
                    SpriteBundle {
                        texture: game_textures.projectiles.clone(),
                        sprite: Sprite {
                            custom_size: Some(Vec2::new(16.0, 16.0)),
                            ..default()
                        },
                        transform: Transform::from_translation(player_transform.translation),
                        ..default()
                    },
                    TextureAtlas {
                        layout: game_textures.projectiles_layout.clone(),
                        index: 0,
                    },
                    RigidBody::Dynamic,
                    Collider::ball(8.0),
                    LockedAxes::ROTATION_LOCKED,
                    ActiveEvents::COLLISION_EVENTS,
                    CollisionGroups::new(Group::GROUP_3, Group::GROUP_2),
                    Velocity::linear(velocity * 300.0),
                    Dominance::group(5),
                ));

                combat.last_attack = time.elapsed_seconds();
            }
        }
    }
}

pub fn handle_damage(
    time: Res<Time>,
    mut commands: Commands,
    mut damage_events: EventReader<DamageEvent>,
    mut health_query: Query<&mut Health>,
    mut last_damage_query: Query<&mut LastDamageTime>,
) {
    for event in damage_events.read() {
        // Check damage cooldown
        if let Ok(mut last_damage) = last_damage_query.get_mut(event.target) {
            let current_time = time.elapsed_seconds();
            if current_time - last_damage.time < last_damage.cooldown {
                continue;
            }
            last_damage.time = current_time;
        } else {
            // If entity doesn't have LastDamageTime, add it
            commands
                .entity(event.target)
                .insert(LastDamageTime::default());
        }

        // Apply damage
        if let Ok(mut health) = health_query.get_mut(event.target) {
            health.current -= event.amount;

            if health.current <= 0.0 {
                commands.entity(event.target).insert(MarkedForDeath);
            }
        }
    }
}
