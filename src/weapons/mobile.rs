//! Moving alchemical streams, finite trail sigils, and a persistent familiar.
//! Steering describes the attack; Rapier determines every hit and wall crossing.
use super::arsenal::Cast;
use super::{Attack, AttackSource, WeaponType};
use crate::arena::WORLD_GROUP;
use crate::combat::DamageEvent;
use crate::components::{Enemy, Health, Player};
use crate::death::{MarkedForDeath, MarkedForDespawn};
use crate::loot::{Breakable, BREAKABLE_GROUP};
use crate::resources::GameState;
use crate::run::RunEntity;
use crate::GameplaySets;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy_prototype_lyon::{
    geometry::{ShapeBuilder, ShapeBuilderBase},
    shapes,
};
use bevy_rapier2d::prelude::*;

const SIGIL_FUSE: f32 = 4.0;
const SIGIL_FLASH: f32 = 0.28;
const STREAM_OUTBOUND: f32 = 0.8;
const STREAM_LIFETIME: f32 = 3.0;

type LiveTargets<'w, 's> = Query<
    'w,
    's,
    &'static Health,
    (
        Or<(With<Enemy>, With<Breakable>)>,
        Without<MarkedForDeath>,
        Without<MarkedForDespawn>,
    ),
>;

pub struct MobileWeaponsPlugin;

impl Plugin for MobileWeaponsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                receive_familiar_casts,
                move_streams,
                update_sigils,
                move_familiars,
            )
                .chain()
                .after(super::weapon_firing_system)
                .in_set(GameplaySets::Weapons)
                .run_if(in_state(GameState::Playing)),
        );
    }
}

#[derive(Component)]
struct Stream {
    cast: Cast,
    side: f32,
    direction: Vec2,
    age: f32,
    returning: bool,
    hits: HashSet<Entity>,
}

#[derive(Component)]
struct TrailSigil {
    cast: Cast,
    age: f32,
    armed: bool,
    flash_left: Option<f32>,
}

#[derive(Component)]
struct FamiliarCast(Cast);

#[derive(Component)]
struct Familiar {
    cast: Cast,
    target: Option<Entity>,
    charged: bool,
}

pub fn cast(commands: &mut Commands, cast: Cast) {
    match cast.kind {
        WeaponType::CirculatingPelican => {
            let direction = cast
                .target
                .map_or(cast.facing, |(_, position)| position - cast.origin)
                .normalize_or(Vec2::X);
            for side in [-1.0, 1.0] {
                commands.spawn((
                    Name::new("Circulating condensate"),
                    Attack,
                    AttackSource(cast.kind),
                    RunEntity,
                    Stream {
                        cast,
                        side,
                        direction,
                        age: 0.0,
                        returning: false,
                        hits: HashSet::default(),
                    },
                    Sprite::from_color(
                        Color::srgb(0.35, 0.95, 0.82),
                        Vec2::new(cast.radius.max(2.0) * 2.5, cast.radius.max(2.0)),
                    ),
                    Transform::from_translation(cast.origin.extend(4.0)),
                ));
            }
        }
        WeaponType::ForgottenSigil => {
            commands.spawn((
                Name::new("Forgotten Sigil"),
                Attack,
                AttackSource(cast.kind),
                RunEntity,
                TrailSigil {
                    cast,
                    age: 0.0,
                    armed: false,
                    flash_left: None,
                },
                crate::art::ItemIcon(16),
                Sprite::from_color(Color::srgb(0.45, 0.22, 0.65), Vec2::splat(24.0)),
                Transform::from_translation(cast.origin.extend(2.0))
                    .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
            ));
        }
        WeaponType::BoundIntention => {
            // Requests are coalesced per weapon before creating a familiar. This
            // also covers two casts arriving before deferred spawns are applied.
            commands.spawn((FamiliarCast(cast), RunEntity));
        }
        _ => {}
    }
}

#[expect(
    clippy::type_complexity,
    reason = "Only living enemies are familiar targets."
)]
fn receive_familiar_casts(
    mut commands: Commands,
    requests: Query<(Entity, &FamiliarCast)>,
    mut familiars: Query<(Entity, &mut Familiar)>,
    enemies: Query<
        (Entity, &Transform, &Health),
        (
            With<Enemy>,
            Without<MarkedForDeath>,
            Without<MarkedForDespawn>,
        ),
    >,
    owners: Query<(), With<Player>>,
    weapons: Query<()>,
) {
    let mut latest: HashMap<Entity, Cast> = HashMap::default();
    for (request, cast) in &requests {
        latest.insert(cast.0.weapon, cast.0);
        commands.entity(request).despawn();
    }
    let existing: HashMap<_, _> = familiars
        .iter()
        .map(|(entity, familiar)| (familiar.cast.weapon, entity))
        .collect();
    for (weapon, cast) in latest {
        if !owners.contains(cast.owner) || !weapons.contains(weapon) {
            continue;
        }
        let target = enemies
            .iter()
            .filter(|(_, transform, health)| {
                health.current > 0
                    && transform
                        .translation
                        .truncate()
                        .distance_squared(cast.origin)
                        <= cast.radius * cast.radius
            })
            .max_by(|(a, ta, ha), (b, tb, hb)| {
                ha.maximum
                    .cmp(&hb.maximum)
                    .then_with(|| {
                        tb.translation
                            .truncate()
                            .distance_squared(cast.origin)
                            .total_cmp(&ta.translation.truncate().distance_squared(cast.origin))
                    })
                    .then_with(|| b.cmp(a))
            })
            .map(|(entity, _, _)| entity);
        let state = Familiar {
            cast,
            target,
            charged: true,
        };
        if let Some(entity) = existing.get(&weapon) {
            if let Ok((_, mut familiar)) = familiars.get_mut(*entity) {
                *familiar = state;
            }
        } else {
            commands.spawn((
                Name::new("Bound Intention familiar"),
                Attack,
                AttackSource(cast.kind),
                RunEntity,
                state,
                crate::art::ItemIcon(17),
                Sprite::from_color(Color::srgb(0.98, 0.75, 0.46), Vec2::new(22.0, 30.0)),
                Transform::from_translation((cast.origin + Vec2::new(-35.0, 25.0)).extend(5.0)),
            ));
        }
    }
}

/// Sweep a segment through Rapier, visiting unique targets in impact order.
/// A wall always terminates travel before targets behind it can be damaged.
#[expect(
    clippy::too_many_arguments,
    reason = "Shared sweep receives the system's existing queries and damage writer."
)]
fn sweep(
    context: &RapierContext<'_>,
    targets: &LiveTargets,
    groups: &Query<&CollisionGroups>,
    source: Entity,
    start: Vec2,
    movement: Vec2,
    radius: f32,
    hits: &mut HashSet<Entity>,
    damage: i32,
    events: &mut MessageWriter<DamageEvent>,
) -> Option<Vec2> {
    let shape = Collider::ball(radius.max(1.0));
    loop {
        let include = |entity, _: &_| {
            !hits.contains(&entity)
                && (targets.get(entity).is_ok_and(|health| health.current > 0)
                    || groups
                        .get(entity)
                        .is_ok_and(|group| group.memberships.intersects(WORLD_GROUP)))
        };
        let result = context.cast_shape(
            start,
            0.0,
            movement,
            shape.raw.as_ref(),
            ShapeCastOptions {
                max_time_of_impact: 1.0,
                stop_at_penetration: true,
                ..default()
            },
            QueryFilter::default()
                .groups(CollisionGroups::new(
                    Group::GROUP_3,
                    Group::GROUP_2 | WORLD_GROUP | BREAKABLE_GROUP,
                ))
                .predicate(&include),
        );
        let (entity, impact) = result?;
        if !targets.contains(entity) {
            return Some(start + movement * impact.time_of_impact);
        }
        hits.insert(entity);
        events.write(DamageEvent {
            target: entity,
            amount: damage,
            source: Some(source),
        });
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Bevy explicitly injects physics, owners, targets, and damage access."
)]
fn move_streams(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    context: ReadRapierContext,
    mut streams: Query<(Entity, &mut Stream, &mut Transform), Without<MarkedForDespawn>>,
    owners: Query<&Transform, (With<Player>, Without<Stream>)>,
    weapons: Query<()>,
    targets: LiveTargets,
    groups: Query<&CollisionGroups>,
    mut events: MessageWriter<DamageEvent>,
) {
    let Ok(context) = context.single() else {
        return;
    };
    for (entity, mut stream, mut transform) in &mut streams {
        let Ok(owner) = owners.get(stream.cast.owner) else {
            commands.entity(entity).insert(MarkedForDespawn);
            continue;
        };
        if !weapons.contains(stream.cast.weapon) {
            commands.entity(entity).insert(MarkedForDespawn);
            continue;
        }
        let start = transform.translation.truncate();
        let was_returning = stream.returning;
        stream.age += time.delta_secs();
        let next = if was_returning {
            let toward = owner.translation.truncate() - start;
            start + toward.clamp_length_max(460.0 * time.delta_secs())
        } else {
            let progress = (stream.age / STREAM_OUTBOUND).min(1.0);
            let outward = stream.direction * (320.0 * progress);
            let sideways = stream.direction.perp()
                * (80.0 * stream.side * (std::f32::consts::PI * progress).sin());
            stream.cast.origin + outward + sideways
        };
        let radius = stream.cast.radius;
        let damage = stream.cast.damage;
        let blocked = sweep(
            &context,
            &targets,
            &groups,
            entity,
            start,
            next - start,
            radius,
            &mut stream.hits,
            damage,
            &mut events,
        );
        transform.translation = blocked.unwrap_or(next).extend(transform.translation.z);
        if next != start {
            transform.rotation = Quat::from_rotation_z((next - start).to_angle());
        }
        if !was_returning && stream.age >= STREAM_OUTBOUND {
            stream.returning = true;
            stream.hits.clear();
        }
        if blocked.is_some()
            || stream.age >= STREAM_LIFETIME
            || (was_returning && next.distance_squared(owner.translation.truncate()) < 16.0)
        {
            commands.entity(entity).insert(MarkedForDespawn);
        }
    }
}

fn overlap_targets(
    context: &RapierContext<'_>,
    targets: &LiveTargets,
    position: Vec2,
    radius: f32,
) -> Vec<Entity> {
    let shape = Collider::ball(radius.max(1.0));
    let mut hits = Vec::new();
    context.intersect_shape(
        position,
        0.0,
        shape.raw.as_ref(),
        QueryFilter::default().groups(CollisionGroups::new(
            Group::GROUP_3,
            Group::GROUP_2 | BREAKABLE_GROUP,
        )),
        |entity, _| {
            if targets.get(entity).is_ok_and(|health| health.current > 0) {
                hits.push(entity);
            }
            true
        },
    );
    hits.sort_unstable();
    hits.dedup();
    hits
}

#[expect(
    clippy::too_many_arguments,
    reason = "Sigils observe separate live owners, physics, and enemy membership."
)]
fn update_sigils(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    context: ReadRapierContext,
    mut sigils: Query<
        (Entity, &mut TrailSigil, &mut Sprite, &Transform),
        Without<MarkedForDespawn>,
    >,
    owners: Query<&Transform, With<Player>>,
    weapons: Query<()>,
    targets: LiveTargets,
    enemies: Query<(), With<Enemy>>,
    mut events: MessageWriter<DamageEvent>,
) {
    let Ok(context) = context.single() else {
        return;
    };
    for (entity, mut sigil, mut sprite, transform) in &mut sigils {
        let Ok(owner) = owners.get(sigil.cast.owner) else {
            commands.entity(entity).insert(MarkedForDespawn);
            continue;
        };
        if !weapons.contains(sigil.cast.weapon) {
            commands.entity(entity).insert(MarkedForDespawn);
            continue;
        }
        if let Some(left) = &mut sigil.flash_left {
            *left -= time.delta_secs();
            sprite
                .color
                .set_alpha((*left / SIGIL_FLASH).clamp(0.0, 1.0));
            if *left <= 0.0 {
                commands.entity(entity).insert(MarkedForDespawn);
            }
            continue;
        }
        sigil.age += time.delta_secs();
        let position = transform.translation.truncate();
        sigil.armed |= sigil.age >= 0.6
            || owner.translation.truncate().distance_squared(position) >= 60.0 * 60.0;
        let triggered = sigil.armed
            && overlap_targets(
                &context,
                &targets,
                position,
                (sigil.cast.radius * 0.4).min(45.0),
            )
            .iter()
            .any(|entity| enemies.contains(*entity));
        if triggered || sigil.age >= SIGIL_FUSE {
            for target in overlap_targets(&context, &targets, position, sigil.cast.radius) {
                events.write(DamageEvent {
                    target,
                    amount: sigil.cast.damage,
                    source: Some(entity),
                });
            }
            sigil.flash_left = Some(SIGIL_FLASH);
            sprite.color = Color::srgba(0.92, 0.45, 1.0, 0.85);
            let radius = sigil.cast.radius;
            commands.entity(entity).with_children(|parent| {
                parent.spawn((
                    ShapeBuilder::with(&shapes::Circle {
                        radius,
                        center: Vec2::ZERO,
                    })
                    .fill(Color::srgba(0.7, 0.2, 0.95, 0.16))
                    .stroke((Color::srgba(0.92, 0.45, 1.0, 0.8), 3.0))
                    .build(),
                    Transform::default(),
                ));
            });
        } else if sigil.armed {
            sprite.color = Color::srgb(0.8, 0.45, 0.95);
        }
    }
}

#[expect(
    clippy::type_complexity,
    clippy::too_many_arguments,
    reason = "Familiar movement separates mutable transforms from owners and live targets."
)]
fn move_familiars(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    context: ReadRapierContext,
    mut familiars: Query<
        (Entity, &mut Familiar, &mut Transform, &mut Sprite),
        Without<MarkedForDespawn>,
    >,
    owners: Query<&Transform, (With<Player>, Without<Familiar>)>,
    weapons: Query<()>,
    enemies: Query<
        &Transform,
        (
            With<Enemy>,
            Without<Familiar>,
            Without<Player>,
            Without<MarkedForDeath>,
            Without<MarkedForDespawn>,
        ),
    >,
    targets: LiveTargets,
    groups: Query<&CollisionGroups>,
    mut events: MessageWriter<DamageEvent>,
) {
    let Ok(context) = context.single() else {
        return;
    };
    for (entity, mut familiar, mut transform, mut sprite) in &mut familiars {
        let Ok(owner) = owners.get(familiar.cast.owner) else {
            commands.entity(entity).insert(MarkedForDespawn);
            continue;
        };
        if !weapons.contains(familiar.cast.weapon) {
            commands.entity(entity).insert(MarkedForDespawn);
            continue;
        }
        let home = owner.translation.truncate();
        let target_position = familiar
            .target
            .filter(|_| familiar.charged)
            .and_then(|target| enemies.get(target).ok())
            .map(|target| target.translation.truncate())
            .filter(|position| {
                position.distance_squared(home) <= (familiar.cast.radius * 1.25).powi(2)
            });
        let destination = target_position.unwrap_or(home + Vec2::new(-35.0, 25.0));
        let start = transform.translation.truncate();
        let movement = (destination - start).clamp_length_max(440.0 * time.delta_secs());
        // The familiar is a hovering magical agency, not a second character.
        // World geometry blocks its travel; target contact uses Rapier as well.
        let collider = Collider::ball(10.0);
        let wall = context.cast_shape(
            start,
            0.0,
            movement,
            collider.raw.as_ref(),
            ShapeCastOptions {
                max_time_of_impact: 1.0,
                stop_at_penetration: true,
                ..default()
            },
            QueryFilter::default()
                .groups(CollisionGroups::new(Group::GROUP_3, WORLD_GROUP))
                .predicate(&|other, _: &_| {
                    groups
                        .get(other)
                        .is_ok_and(|group| group.memberships.intersects(WORLD_GROUP))
                }),
        );
        transform.translation =
            (start + movement * wall.map_or(1.0, |(_, hit)| hit.time_of_impact)).extend(5.0);
        sprite.color = if familiar.charged {
            Color::srgb(1.0, 0.82, 0.5)
        } else {
            Color::srgb(0.62, 0.55, 0.44)
        };
        if let Some(target) = familiar
            .target
            .filter(|_| target_position.is_some() && wall.is_none())
        {
            if overlap_targets(&context, &targets, transform.translation.truncate(), 18.0)
                .contains(&target)
            {
                events.write(DamageEvent {
                    target,
                    amount: familiar.cast.damage,
                    source: Some(entity),
                });
                familiar.charged = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{count, game, key, player, step};
    use bevy::ecs::system::RunSystemOnce;

    fn arena() -> (App, Entity, Entity) {
        let mut app = game();
        let owner = player(&mut app);
        // These fixtures isolate attack behavior from the starting Circle and
        // authored terrain. Target colliders still use the real game/Rapier setup.
        let weapons: Vec<_> = app
            .world_mut()
            .query_filtered::<Entity, With<super::super::WeaponMeta>>()
            .iter(app.world())
            .collect();
        for weapon in weapons {
            app.world_mut().despawn(weapon);
        }
        let walls: Vec<_> = app
            .world_mut()
            .query::<(Entity, &CollisionGroups)>()
            .iter(app.world())
            .filter(|(_, groups)| groups.memberships.intersects(WORLD_GROUP))
            .map(|(entity, _)| entity)
            .collect();
        for wall in walls {
            app.world_mut().despawn(wall);
        }
        let weapon = app.world_mut().spawn_empty().id();
        (app, owner, weapon)
    }

    fn enemy(app: &mut App, position: Vec2, hp: i32) -> Entity {
        app.world_mut()
            .spawn((
                Enemy {
                    speed: 0.0,
                    experience_value: 0,
                },
                Health {
                    current: hp,
                    maximum: hp,
                },
                Transform::from_translation(position.extend(0.0)),
            ))
            .id()
    }

    fn request(app: &mut App, owner: Entity, weapon: Entity, kind: WeaponType) {
        let origin = app
            .world()
            .get::<Transform>(owner)
            .unwrap()
            .translation
            .truncate();
        app.world_mut()
            .run_system_once(move |mut commands: Commands| {
                cast(
                    &mut commands,
                    Cast {
                        owner,
                        weapon,
                        kind,
                        origin,
                        facing: Vec2::X,
                        target: None,
                        damage: 20,
                        radius: match kind {
                            WeaponType::CirculatingPelican => 14.0,
                            WeaponType::ForgottenSigil => 90.0,
                            _ => 360.0,
                        },
                        level: 1,
                    },
                );
            })
            .unwrap();
    }

    #[test]
    fn pelican_hits_once_per_stream_leg_and_returns_to_a_moving_owner() {
        let (mut app, owner, weapon) = arena();
        let target = enemy(&mut app, Vec2::new(300.0, 0.0), 1000);
        step(&mut app, 3);
        request(&mut app, owner, weapon, WeaponType::CirculatingPelican);
        step(&mut app, 58);
        assert_eq!(
            app.world().get::<Health>(target).unwrap().current,
            920,
            "Each of two streams may hit once outbound and once returning"
        );
        app.world_mut()
            .get_mut::<Transform>(owner)
            .unwrap()
            .translation = Vec3::new(-100.0, 150.0, 0.0);
        step(&mut app, 120);
        assert_eq!(count::<Stream>(&mut app), 0);
        assert_eq!(app.world().get::<Health>(target).unwrap().current, 920);
        assert_eq!(
            app.world()
                .resource::<crate::telemetry::RunTelemetry>()
                .damage_by_weapon[&WeaponType::CirculatingPelican],
            80
        );
    }

    #[test]
    fn pelican_and_familiar_do_not_deliver_damage_through_rapier_walls() {
        let (mut app, owner, weapon) = arena();
        let target = enemy(&mut app, Vec2::new(180.0, 0.0), 1000);
        app.world_mut().spawn((
            RigidBody::Fixed,
            Collider::cuboid(5.0, 200.0),
            CollisionGroups::new(WORLD_GROUP, Group::GROUP_3),
            Transform::from_xyz(150.0, 0.0, 0.0),
        ));
        step(&mut app, 3);
        request(&mut app, owner, weapon, WeaponType::CirculatingPelican);
        request(&mut app, owner, weapon, WeaponType::BoundIntention);
        step(&mut app, 150);
        assert_eq!(app.world().get::<Health>(target).unwrap().current, 1000);
        assert_eq!(count::<Stream>(&mut app), 0);
    }

    #[test]
    fn sigil_forces_eruption_without_proximity_and_its_fuse_pauses() {
        let (mut app, owner, weapon) = arena();
        let target = enemy(&mut app, Vec2::new(75.0, 0.0), 1000);
        step(&mut app, 3);
        request(&mut app, owner, weapon, WeaponType::ForgottenSigil);
        step(&mut app, 180);
        assert_eq!(app.world().get::<Health>(target).unwrap().current, 1000);
        key(&mut app, KeyCode::Escape);
        step(&mut app, 2);
        let age = app
            .world_mut()
            .query::<&TrailSigil>()
            .single(app.world())
            .unwrap()
            .age;
        step(&mut app, 180);
        assert_eq!(
            app.world_mut()
                .query::<&TrailSigil>()
                .single(app.world())
                .unwrap()
                .age,
            age
        );
        key(&mut app, KeyCode::Escape);
        step(&mut app, 100);
        assert_eq!(app.world().get::<Health>(target).unwrap().current, 980);
        assert_eq!(count::<TrailSigil>(&mut app), 0);
        assert_eq!(
            app.world()
                .resource::<crate::telemetry::RunTelemetry>()
                .damage_by_weapon[&WeaponType::ForgottenSigil],
            20
        );
    }

    #[test]
    fn sigil_waits_to_arm_then_detonates_on_rapier_proximity() {
        let (mut app, owner, weapon) = arena();
        let target = enemy(&mut app, Vec2::new(42.0, 0.0), 1000);
        step(&mut app, 3);
        request(&mut app, owner, weapon, WeaponType::ForgottenSigil);
        step(&mut app, 20);
        assert_eq!(app.world().get::<Health>(target).unwrap().current, 1000);
        step(&mut app, 20);
        assert_eq!(app.world().get::<Health>(target).unwrap().current, 980);
        step(&mut app, 30);
        assert_eq!(count::<TrailSigil>(&mut app), 0);
        assert_eq!(app.world().get::<Health>(target).unwrap().current, 980);
    }

    #[test]
    fn repeated_sigil_casts_have_a_bounded_population_without_targets() {
        let (mut app, owner, weapon) = arena();
        for _ in 0..70 {
            request(&mut app, owner, weapon, WeaponType::ForgottenSigil);
            step(&mut app, 6);
            assert!(count::<TrailSigil>(&mut app) <= 44);
        }
        step(&mut app, 270);
        assert_eq!(count::<TrailSigil>(&mut app), 0);
    }

    #[test]
    fn familiar_is_unique_prefers_durable_targets_and_attacks_only_on_cadence() {
        let (mut app, owner, weapon) = arena();
        let weak = enemy(&mut app, Vec2::new(60.0, 30.0), 100);
        let durable = enemy(&mut app, Vec2::new(150.0, 0.0), 1000);
        step(&mut app, 3);
        request(&mut app, owner, weapon, WeaponType::BoundIntention);
        request(&mut app, owner, weapon, WeaponType::BoundIntention);
        step(&mut app, 45);
        assert_eq!(count::<Familiar>(&mut app), 1);
        assert_eq!(app.world().get::<Health>(weak).unwrap().current, 100);
        assert_eq!(app.world().get::<Health>(durable).unwrap().current, 980);
        step(&mut app, 90);
        assert_eq!(app.world().get::<Health>(durable).unwrap().current, 980);
        request(&mut app, owner, weapon, WeaponType::BoundIntention);
        step(&mut app, 45);
        assert_eq!(count::<Familiar>(&mut app), 1);
        assert_eq!(app.world().get::<Health>(durable).unwrap().current, 960);
        app.world_mut().despawn(weapon);
        step(&mut app, 2);
        assert_eq!(count::<Familiar>(&mut app), 0);
    }

    #[test]
    fn owner_loss_cleans_all_mobile_effects() {
        let (mut app, owner, weapon) = arena();
        for kind in [
            WeaponType::CirculatingPelican,
            WeaponType::ForgottenSigil,
            WeaponType::BoundIntention,
        ] {
            request(&mut app, owner, weapon, kind);
        }
        step(&mut app, 2);
        app.world_mut().despawn(owner);
        step(&mut app, 2);
        assert_eq!(count::<Stream>(&mut app), 0);
        assert_eq!(count::<TrailSigil>(&mut app), 0);
        assert_eq!(count::<Familiar>(&mut app), 0);
    }
}
