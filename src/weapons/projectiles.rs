//! Rapier moves projectiles and owns swept collision and burst queries.
use super::{
    Attack, AttackSource, Damage, Lifetime, WeaponArea, WeaponCooldown, WeaponDamage, WeaponMeta,
    WeaponType,
};
use crate::arena::WORLD_GROUP;
use crate::combat::DamageEvent;
use crate::components::Enemy;
use crate::death::{MarkedForDeath, MarkedForDespawn};
use crate::loot::{Breakable, BREAKABLE_GROUP};
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy_prototype_lyon::{
    geometry::{ShapeBuilder, ShapeBuilderBase},
    shapes,
};
use bevy_rapier2d::prelude::*;

#[derive(Component)]
pub struct Projectile {
    previous_position: Vec2,
    hit_entities: HashSet<Entity>,
    kind: ProjectileKind,
}

enum ProjectileKind {
    Needle {
        remaining_hits: u8,
    },
    Vessel {
        distance_left: f32,
        burst_radius: f32,
    },
}

#[derive(Component)]
pub struct VesselBurst;

pub fn spawn_projectile_weapon(commands: &mut Commands, player: Entity, weapon: WeaponType) {
    let (damage, cooldown, radius) = match weapon {
        WeaponType::InscribedNeedles => (12, 0.8, 4.0),
        WeaponType::AlchemicalVessel => (24, 2.4, 72.0),
        WeaponType::MagickCircle => return,
    };
    commands.entity(player).with_children(|parent| {
        parent.spawn((
            Name::new(weapon.to_string()),
            WeaponMeta {
                weapon_type: weapon,
                level: 1,
            },
            WeaponDamage {
                base_amount: damage,
                damage_bonus: 0,
            },
            WeaponArea {
                base_radius: radius,
                area_bonus: 0,
            },
            WeaponCooldown {
                timer: Timer::from_seconds(cooldown, TimerMode::Repeating),
                base_duration: cooldown,
                cooldown_bonus: 0,
            },
        ));
    });
}

pub fn spawn_projectile(
    commands: &mut Commands,
    origin: Vec2,
    target: Vec2,
    weapon: WeaponType,
    damage: i32,
    area: f32,
) -> Entity {
    let offset = target - origin;
    let direction = offset.normalize_or(Vec2::X);
    let (kind, speed, radius, color, size, lifetime) = match weapon {
        WeaponType::InscribedNeedles => (
            ProjectileKind::Needle { remaining_hits: 3 },
            720.0,
            area.max(1.0),
            Color::srgb(0.87, 0.94, 1.0),
            Vec2::new(24.0, (area * 1.25).max(3.0)),
            2.0,
        ),
        WeaponType::AlchemicalVessel => (
            ProjectileKind::Vessel {
                distance_left: offset.length(),
                burst_radius: area.max(1.0),
            },
            320.0,
            7.0,
            Color::srgb(0.95, 0.56, 0.16),
            Vec2::splat(23.0),
            3.0,
        ),
        WeaponType::MagickCircle => unreachable!("Circles have a separate attack constructor"),
    };
    let entity = commands
        .spawn((
            Attack,
            AttackSource(weapon),
            Name::new(format!("{weapon} projectile")),
            Damage { amount: damage },
            Projectile {
                previous_position: origin,
                hit_entities: HashSet::default(),
                kind,
            },
            Lifetime {
                timer: Timer::from_seconds(lifetime, TimerMode::Once),
            },
            Sprite::from_color(color, size),
            Transform::from_translation(origin.extend(3.0))
                .with_rotation(Quat::from_rotation_z(direction.to_angle())),
            (
                RigidBody::Dynamic,
                Velocity::linear(direction * speed),
                GravityScale(0.0),
                Collider::ball(radius),
                Sensor,
                Ccd::enabled(),
                LockedAxes::ROTATION_LOCKED,
                CollisionGroups::new(
                    Group::GROUP_3,
                    Group::GROUP_2 | WORLD_GROUP | BREAKABLE_GROUP,
                ),
            ),
        ))
        .id();
    if weapon == WeaponType::AlchemicalVessel {
        commands.entity(entity).insert(crate::art::ItemIcon(2));
    }
    entity
}

#[expect(
    clippy::type_complexity,
    reason = "The ECS query excludes finished projectiles and identifies the only damageable target classes."
)]
pub fn resolve_projectiles(
    mut commands: Commands,
    context: ReadRapierContext,
    mut projectiles: Query<
        (Entity, &mut Projectile, &Transform, &Collider, &Damage),
        Without<MarkedForDeath>,
    >,
    targets: Query<
        (),
        (
            Or<(With<Enemy>, With<Breakable>)>,
            Without<MarkedForDeath>,
            Without<MarkedForDespawn>,
        ),
    >,
    collider_groups: Query<&CollisionGroups>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let Ok(context) = context.single() else {
        return;
    };
    for (entity, mut projectile, transform, collider, damage) in &mut projectiles {
        let previous = projectile.previous_position;
        let current = transform.translation.truncate();
        let movement = current - previous;
        let travel = movement.length();
        let sweep = match projectile.kind {
            ProjectileKind::Vessel { distance_left, .. } if travel > distance_left => {
                movement.normalize_or_zero() * distance_left
            }
            _ => movement,
        };
        let mut finished = false;
        loop {
            // CCD is enabled on the physical body, and this Rapier shape cast
            // additionally covers sensor crossings. No hand-written hit geometry.
            let include = |other| {
                !projectile.hit_entities.contains(&other)
                    && (targets.contains(other)
                        || collider_groups
                            .get(other)
                            .is_ok_and(|groups| groups.memberships.intersects(WORLD_GROUP)))
            };
            let filter = QueryFilter::default()
                .exclude_collider(entity)
                .groups(CollisionGroups::new(
                    Group::GROUP_3,
                    Group::GROUP_2 | WORLD_GROUP | BREAKABLE_GROUP,
                ))
                .predicate(&include);
            let hit = context.cast_shape(
                previous,
                0.0,
                sweep,
                collider.raw.as_ref(),
                ShapeCastOptions {
                    max_time_of_impact: 1.0,
                    stop_at_penetration: true,
                    ..default()
                },
                filter,
            );
            let Some((target, hit)) = hit else {
                break;
            };
            let position = previous + sweep * hit.time_of_impact;
            if let ProjectileKind::Vessel { burst_radius, .. } = projectile.kind {
                burst(
                    &mut commands,
                    &context,
                    &targets,
                    &mut damage_events,
                    entity,
                    position,
                    burst_radius,
                    damage.amount,
                );
                finished = true;
                break;
            }
            if !targets.contains(target) {
                // A filtered, non-damageable collider is authored world geometry.
                finished = true;
                break;
            }
            projectile.hit_entities.insert(target);
            damage_events.write(DamageEvent {
                target,
                amount: damage.amount,
                source: Some(entity),
            });
            if let ProjectileKind::Needle {
                ref mut remaining_hits,
            } = projectile.kind
            {
                *remaining_hits -= 1;
                if *remaining_hits == 0 {
                    finished = true;
                    break;
                }
            }
        }
        if !finished {
            if let ProjectileKind::Vessel {
                ref mut distance_left,
                burst_radius,
            } = projectile.kind
            {
                *distance_left = (*distance_left - travel).max(0.0);
                if *distance_left <= f32::EPSILON {
                    burst(
                        &mut commands,
                        &context,
                        &targets,
                        &mut damage_events,
                        entity,
                        previous + sweep,
                        burst_radius,
                        damage.amount,
                    );
                    finished = true;
                }
            }
        }
        projectile.previous_position = current;
        if finished {
            commands.entity(entity).insert(MarkedForDeath);
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "The helper receives existing ECS system parameters without duplicating world access."
)]
#[expect(
    clippy::type_complexity,
    reason = "The target filter allows live enemies and breakable props, never players."
)]
fn burst(
    commands: &mut Commands,
    context: &RapierContext<'_>,
    targets: &Query<
        (),
        (
            Or<(With<Enemy>, With<Breakable>)>,
            Without<MarkedForDeath>,
            Without<MarkedForDespawn>,
        ),
    >,
    damage_events: &mut MessageWriter<DamageEvent>,
    source: Entity,
    position: Vec2,
    radius: f32,
    damage: i32,
) {
    let shape = Collider::ball(radius);
    let mut hits = Vec::new();
    context.intersect_shape(
        position,
        0.0,
        shape.raw.as_ref(),
        QueryFilter::default().groups(CollisionGroups::new(
            Group::GROUP_3,
            Group::GROUP_2 | BREAKABLE_GROUP,
        )),
        |target| {
            if targets.contains(target) {
                hits.push(target);
            }
            true
        },
    );
    hits.sort_unstable();
    hits.dedup();
    for target in hits {
        damage_events.write(DamageEvent {
            target,
            amount: damage,
            source: Some(source),
        });
    }
    commands.spawn((
        Attack,
        AttackSource(WeaponType::AlchemicalVessel),
        VesselBurst,
        Lifetime {
            timer: Timer::from_seconds(0.3, TimerMode::Once),
        },
        ShapeBuilder::with(&shapes::Circle {
            radius,
            center: Vec2::ZERO,
        })
        .fill(Color::srgba(0.95, 0.45, 0.13, 0.25))
        .stroke((Color::srgba(1.0, 0.78, 0.32, 0.85), 3.0))
        .build(),
        Transform::from_translation(position.extend(2.0)),
    ));
}
