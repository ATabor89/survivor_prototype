//! Small authored attack cycles. Rapier owns movement, wall blocking, and projectile hits.
use crate::{
    arena::WORLD_GROUP,
    combat::DamageEvent,
    components::{Enemy, Player},
    content::{EnemyBehavior, RangedConfig},
    death::{MarkedForDeath, MarkedForDespawn},
    resources::GameState,
    run::RunEntity,
    GameplaySets,
};
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

#[derive(Clone, Copy, Debug)]
enum Phase {
    Approach,
    Windup { remaining: f32, direction: Vec2 },
    Dash { remaining: f32, direction: Vec2 },
    Recover { remaining: f32 },
}

#[derive(Component)]
pub struct EnemyBrain {
    behavior: EnemyBehavior,
    phase: Phase,
    ranged_turn: bool,
    telegraph: Option<Entity>,
    pub(crate) desired_velocity: Vec2,
}

impl EnemyBrain {
    pub fn new(behavior: EnemyBehavior) -> Self {
        Self {
            ranged_turn: behavior.ranged().is_some(),
            behavior,
            phase: Phase::Approach,
            telegraph: None,
            desired_velocity: Vec2::ZERO,
        }
    }
}

/// Separate children let hit flashes tint the enemy without hiding its warning.
#[derive(Component)]
pub(crate) struct EnemyTelegraph;

#[derive(Component)]
pub(crate) struct HostileProjectile {
    previous_position: Vec2,
    remaining: f32,
    damage: i32,
}

pub struct EnemyBehaviorPlugin;
impl Plugin for EnemyBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            steer_enemies
                .in_set(GameplaySets::Movement)
                .before(crate::systems::enemy_movement)
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            Update,
            resolve_hostile_projectiles
                .in_set(GameplaySets::Weapons)
                .run_if(in_state(GameState::Playing)),
        );
    }
}

#[expect(
    clippy::type_complexity,
    reason = "Only living free-moving enemies may advance attack cycles; ring formations retain their own steering."
)]
fn steer_enemies(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    player: Query<
        &Transform,
        (
            With<Player>,
            Without<MarkedForDeath>,
            Without<MarkedForDespawn>,
        ),
    >,
    mut enemies: Query<
        (Entity, &Transform, &Enemy, &mut EnemyBrain),
        (
            Without<crate::waves::RingMember>,
            Without<MarkedForDeath>,
            Without<MarkedForDespawn>,
        ),
    >,
) {
    let Ok(player) = player.single() else {
        return;
    };
    let delta = time.delta_secs();
    for (entity, transform, enemy, mut brain) in &mut enemies {
        if brain.is_added() && brain.behavior.charge().is_some() {
            commands.entity(entity).insert(Ccd::enabled());
        }
        let origin = transform.translation.truncate();
        let offset = player.translation.truncate() - origin;
        let direction = offset.normalize_or(Vec2::X);
        let pursuit = offset.normalize_or_zero() * enemy.speed * 0.8;
        let charge = (!brain.ranged_turn)
            .then(|| brain.behavior.charge())
            .flatten();
        let ranged = brain.ranged_turn.then(|| brain.behavior.ranged()).flatten();
        brain.desired_velocity = Vec2::ZERO;
        match brain.phase {
            Phase::Approach => {
                let range = charge
                    .map(|c| c.trigger_range)
                    .or_else(|| ranged.map(|r| r.preferred_range));
                if range.is_none_or(|range| offset.length() > range) {
                    brain.desired_velocity = pursuit;
                } else {
                    let (windup, length, count, spread, color) = if let Some(c) = charge {
                        (
                            c.windup,
                            (c.dash_speed * c.dash_seconds).min(600.0),
                            1,
                            0.0,
                            Color::srgba(1.0, 0.18, 0.13, 0.7),
                        )
                    } else if let Some(r) = ranged {
                        (
                            r.windup,
                            r.preferred_range.min(600.0),
                            r.projectile_count,
                            r.spread_radians,
                            Color::srgba(1.0, 0.73, 0.12, 0.65),
                        )
                    } else {
                        continue;
                    };
                    brain.telegraph = Some(spawn_telegraph(
                        &mut commands,
                        entity,
                        direction,
                        length,
                        count,
                        spread,
                        color,
                    ));
                    brain.phase = Phase::Windup {
                        remaining: windup,
                        direction,
                    };
                }
            }
            Phase::Windup {
                remaining,
                direction,
            } => {
                if remaining > delta {
                    brain.phase = Phase::Windup {
                        remaining: remaining - delta,
                        direction,
                    };
                    continue;
                }
                if let Some(telegraph) = brain.telegraph.take() {
                    commands.entity(telegraph).despawn();
                }
                if let Some(c) = charge {
                    brain.desired_velocity = direction * c.dash_speed;
                    brain.phase = Phase::Dash {
                        remaining: c.dash_seconds,
                        direction,
                    };
                } else if let Some(r) = ranged {
                    for index in 0..r.projectile_count {
                        let aim = spread_direction(
                            direction,
                            index,
                            r.projectile_count,
                            r.spread_radians,
                        );
                        spawn_hostile_projectile(&mut commands, origin, aim, &r);
                    }
                    brain.phase = Phase::Recover {
                        remaining: r.cooldown,
                    };
                }
            }
            Phase::Dash {
                remaining,
                direction,
            } => {
                if let Some(c) = charge {
                    if remaining > delta {
                        brain.desired_velocity = direction * c.dash_speed;
                        brain.phase = Phase::Dash {
                            remaining: remaining - delta,
                            direction,
                        };
                    } else {
                        brain.phase = Phase::Recover {
                            remaining: c.recovery,
                        };
                    }
                }
            }
            Phase::Recover { remaining } => {
                if remaining > delta {
                    brain.phase = Phase::Recover {
                        remaining: remaining - delta,
                    };
                } else {
                    if matches!(brain.behavior, EnemyBehavior::Warden { .. }) {
                        brain.ranged_turn = !brain.ranged_turn;
                    }
                    brain.phase = Phase::Approach;
                }
            }
        }
    }
}

fn spread_direction(direction: Vec2, index: u8, count: u8, spread: f32) -> Vec2 {
    let angle = if count <= 1 {
        0.0
    } else {
        (f32::from(index) / f32::from(count - 1) - 0.5) * spread
    };
    Vec2::from_angle(direction.to_angle() + angle)
}

fn spawn_telegraph(
    commands: &mut Commands,
    enemy: Entity,
    direction: Vec2,
    length: f32,
    count: u8,
    spread: f32,
    color: Color,
) -> Entity {
    let root = commands
        .spawn((EnemyTelegraph, Transform::default(), Visibility::default()))
        .with_children(|children| {
            for index in 0..count {
                let aim = spread_direction(direction, index, count, spread);
                children.spawn((
                    Sprite::from_color(color, Vec2::new(length, 3.0)),
                    Transform::from_translation((aim * (length * 0.5 + 16.0)).extend(1.0))
                        .with_rotation(Quat::from_rotation_z(aim.to_angle())),
                ));
                children.spawn((
                    Sprite::from_color(color, Vec2::splat(8.0)),
                    Transform::from_translation((aim * (length + 16.0)).extend(1.0)),
                ));
            }
        })
        .id();
    commands.entity(enemy).add_child(root);
    root
}

pub(crate) fn spawn_hostile_projectile(
    commands: &mut Commands,
    origin: Vec2,
    direction: Vec2,
    config: &RangedConfig,
) -> Entity {
    commands
        .spawn((
            Name::new("Hostile bolt"),
            RunEntity,
            HostileProjectile {
                previous_position: origin,
                remaining: config.projectile_lifetime,
                damage: config.damage,
            },
            Sprite::from_color(Color::srgb(1.0, 0.43, 0.09), Vec2::splat(13.0)),
            Transform::from_translation(origin.extend(4.0))
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
            RigidBody::Dynamic,
            Velocity::linear(direction.normalize_or(Vec2::X) * config.projectile_speed),
            GravityScale(0.0),
            Collider::ball(6.0),
            Sensor,
            Ccd::enabled(),
            LockedAxes::ROTATION_LOCKED,
            CollisionGroups::new(Group::GROUP_3, Group::GROUP_1 | WORLD_GROUP),
        ))
        .id()
}

#[expect(
    clippy::type_complexity,
    reason = "Sweeps include only a living player's main collider and authored world geometry, never contact-sensor children."
)]
fn resolve_hostile_projectiles(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    context: ReadRapierContext,
    mut projectiles: Query<
        (Entity, &mut HostileProjectile, &Transform, &Collider),
        Without<MarkedForDespawn>,
    >,
    players: Query<
        (),
        (
            With<Player>,
            Without<MarkedForDeath>,
            Without<MarkedForDespawn>,
        ),
    >,
    groups: Query<&CollisionGroups>,
    mut damage: MessageWriter<DamageEvent>,
) {
    let Ok(context) = context.single() else {
        return;
    };
    for (entity, mut projectile, transform, collider) in &mut projectiles {
        let previous = projectile.previous_position;
        let current = transform.translation.truncate();
        // Clamp the last sweep to the bolt's remaining life, even on a long frame.
        let fraction = (projectile.remaining / time.delta_secs().max(f32::EPSILON)).min(1.0);
        let movement = (current - previous) * fraction;
        let include = |other, _: &_| {
            players.contains(other)
                || groups
                    .get(other)
                    .is_ok_and(|g| g.memberships.intersects(WORLD_GROUP))
        };
        let filter = QueryFilter::default()
            .exclude_collider(entity)
            .groups(CollisionGroups::new(
                Group::GROUP_3,
                Group::GROUP_1 | WORLD_GROUP,
            ))
            .predicate(&include);
        let hit = context.cast_shape(
            previous,
            0.0,
            movement,
            collider.raw.as_ref(),
            ShapeCastOptions {
                max_time_of_impact: 1.0,
                stop_at_penetration: true,
                ..default()
            },
            filter,
        );
        if let Some((target, _)) = hit {
            if players.contains(target) {
                damage.write(DamageEvent {
                    target,
                    amount: projectile.damage,
                    source: Some(entity),
                });
            }
        }
        projectile.remaining -= time.delta_secs();
        projectile.previous_position = current;
        if hit.is_some() || projectile.remaining <= 0.0 {
            // Combat can still inspect this source; cleanup runs after damage resolution.
            commands.entity(entity).insert(MarkedForDespawn);
        }
    }
}
