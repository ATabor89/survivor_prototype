//! Boundaries, moving talismans, and rhythmic waves share the normal damage path.
use super::arsenal::{damage_shape, Cast, DamageTargets};
use super::{weapon_firing_system, Attack, AttackSource, WeaponType};
use crate::combat::DamageEvent;
use crate::components::Player;
use crate::death::MarkedForDespawn;
use crate::resources::GameState;
use crate::run::RunEntity;
use crate::GameplaySets;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy_prototype_lyon::geometry::{ShapeBuilder, ShapeBuilderBase};
use bevy_prototype_lyon::shapes;
use bevy_rapier2d::prelude::*;
use std::f32::consts::{PI, TAU};

pub struct RitualWeaponsPlugin;

impl Plugin for RitualWeaponsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (prepare_casts, advance_rituals, position_discs)
                .chain()
                .after(weapon_firing_system)
                .in_set(GameplaySets::Weapons)
                .run_if(in_state(GameState::Playing)),
        );
    }
}

#[derive(Component)]
struct RitualRequest(Cast);

#[derive(Component, Default)]
struct RecitationCadence(u8);

#[derive(Component)]
struct RitualEffect {
    age: f32,
    remaining: f32,
    damage: i32,
    radius: f32,
    form: RitualForm,
}

enum RitualForm {
    Knife {
        shape: Collider,
        struck: bool,
        angle: f32,
    },
    Orbit {
        owner: Entity,
        weapon: Entity,
        previous_origin: Vec2,
        count: usize,
        contacts: HashMap<Entity, f32>,
    },
    Pulse {
        struck: HashSet<Entity>,
    },
}

#[derive(Component)]
struct TalismanDisc(usize);

/// Requests are run-owned, consumed in the same weapon phase as normal firing.
pub fn cast(commands: &mut Commands, cast: Cast) {
    commands.spawn((RitualRequest(cast), RunEntity));
}

fn prepare_casts(
    mut commands: Commands,
    requests: Query<(Entity, &RitualRequest)>,
    mut effects: Query<&mut RitualEffect>,
    mut cadence: Query<&mut RecitationCadence>,
) {
    for (request, RitualRequest(cast)) in &requests {
        commands.entity(request).despawn();
        if cast.radius <= 0.0 || cast.damage <= 0 {
            continue;
        }
        let mut radius = cast.radius;
        let mut damage = cast.damage;
        let (form, duration, color) = match cast.kind {
            WeaponType::KnifeOfTheArt => {
                let points = knife_points(radius);
                let Some(shape) = Collider::convex_hull(&points) else {
                    continue;
                };
                (
                    RitualForm::Knife {
                        shape,
                        struck: false,
                        angle: cast.facing.normalize_or(Vec2::X).to_angle(),
                    },
                    0.22,
                    Color::srgba(0.75, 0.9, 1.0, 0.5),
                )
            }
            WeaponType::PlanetaryTalismans => {
                let count = (2 + cast.level.saturating_sub(1) as usize / 2).min(5);
                let mut refreshed = false;
                for mut effect in &mut effects {
                    if let RitualForm::Orbit {
                        weapon,
                        count: existing_count,
                        contacts,
                        ..
                    } = &mut effect.form
                    {
                        if *weapon == cast.weapon {
                            *existing_count = count;
                            // Each invocation recharges contact damage, so cooldown
                            // improves attack frequency without spawning more rings.
                            contacts.clear();
                            effect.damage = damage;
                            effect.radius = radius;
                            effect.remaining = 3.25;
                            refreshed = true;
                            break;
                        }
                    }
                }
                if refreshed {
                    continue;
                }
                (
                    RitualForm::Orbit {
                        owner: cast.owner,
                        weapon: cast.weapon,
                        previous_origin: cast.origin,
                        count,
                        contacts: HashMap::default(),
                    },
                    3.25,
                    Color::srgb(1.0, 0.8, 0.3),
                )
            }
            WeaponType::ResonantRecitation => {
                let beat = if let Ok(mut beat) = cadence.get_mut(cast.weapon) {
                    beat.0 = (beat.0 + 1) % 4;
                    beat.0
                } else {
                    commands.entity(cast.weapon).insert(RecitationCadence(1));
                    1
                };
                if beat == 0 {
                    damage = damage.saturating_mul(2);
                    radius *= 1.25;
                }
                (
                    RitualForm::Pulse {
                        struck: HashSet::default(),
                    },
                    0.7,
                    if beat == 0 {
                        Color::srgb(1.0, 0.9, 0.5)
                    } else {
                        Color::srgb(0.45, 0.9, 0.85)
                    },
                )
            }
            _ => continue,
        };
        let mut entity = commands.spawn((
            Name::new(format!("{} ritual", cast.kind)),
            Attack,
            AttackSource(cast.kind),
            RunEntity,
            Transform::from_translation(cast.origin.extend(3.0)),
            Visibility::default(),
        ));
        match &form {
            RitualForm::Knife { angle, .. } => {
                entity.insert((
                    ShapeBuilder::with(&shapes::Polygon {
                        points: knife_points(radius),
                        closed: true,
                    })
                    .fill(color)
                    .stroke((Color::srgb(0.8, 0.95, 1.0), 2.0))
                    .build(),
                    Transform::from_translation(cast.origin.extend(3.0))
                        .with_rotation(Quat::from_rotation_z(*angle)),
                ));
            }
            RitualForm::Orbit { .. } => {
                entity.with_children(|parent| {
                    for index in 0..5 {
                        parent.spawn((
                            TalismanDisc(index),
                            ShapeBuilder::with(&shapes::RegularPolygon {
                                sides: 6,
                                feature: shapes::RegularPolygonFeature::Radius(1.0),
                                ..default()
                            })
                            .fill(Color::srgb(0.32, 0.22, 0.08))
                            .stroke((color, 0.13))
                            .build(),
                            Transform::default(),
                            Visibility::Hidden,
                        ));
                    }
                });
            }
            RitualForm::Pulse { .. } => {
                entity.insert(
                    ShapeBuilder::with(&shapes::Circle {
                        radius: 1.0,
                        ..default()
                    })
                    .stroke((color, 0.025))
                    .build(),
                );
            }
        }
        entity.insert(RitualEffect {
            age: 0.0,
            remaining: duration,
            damage,
            radius,
            form,
        });
    }
}

/// A convex 120-degree fan supplies both the visible boundary and Rapier shape.
fn knife_points(radius: f32) -> Vec<Vec2> {
    std::iter::once(Vec2::ZERO)
        .chain(
            (0..=12).map(|index| Vec2::from_angle(-PI / 3.0 + index as f32 * PI / 18.0) * radius),
        )
        .collect()
}

#[expect(
    clippy::type_complexity,
    reason = "Disjoint ECS transforms and explicit Rapier target filters keep attack ownership and collisions visible."
)]
fn advance_rituals(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    context: ReadRapierContext,
    targets: DamageTargets,
    owners: Query<&Transform, (With<Player>, Without<RitualEffect>)>,
    mut effects: Query<
        (Entity, &mut RitualEffect, &mut Transform),
        (Without<Player>, Without<MarkedForDespawn>),
    >,
    mut damage: MessageWriter<DamageEvent>,
) {
    let Ok(context) = context.single() else {
        return;
    };
    let delta = time.delta_secs();
    for (entity, mut effect, mut transform) in &mut effects {
        let previous_age = effect.age;
        effect.age += delta;
        effect.remaining -= delta;
        let age = effect.age;
        let amount = effect.damage;
        let radius = effect.radius;
        let mut hit = Vec::new();
        match &mut effect.form {
            RitualForm::Knife {
                shape,
                struck,
                angle,
            } => {
                if !*struck {
                    hit = damage_shape(
                        &context,
                        shape,
                        transform.translation.truncate(),
                        *angle,
                        &targets,
                    );
                    *struck = true;
                }
            }
            RitualForm::Orbit {
                owner,
                previous_origin,
                count,
                contacts,
                ..
            } => {
                let Ok(owner) = owners.get(*owner) else {
                    commands.entity(entity).insert(MarkedForDespawn);
                    continue;
                };
                let origin = owner.translation.truncate();
                transform.translation = origin.extend(3.0);
                contacts.retain(|target, last| age - *last < 0.35 && targets.contains(*target));
                let shape = Collider::ball(radius);
                for index in 0..*count {
                    let phase = index as f32 * TAU / *count as f32;
                    let from =
                        *previous_origin + Vec2::from_angle(previous_age * 2.5 + phase) * 105.0;
                    let to = origin + Vec2::from_angle(age * 2.5 + phase) * 105.0;
                    // Rapier sweeps cover movement between frames, including player motion.
                    loop {
                        let include =
                            |target| targets.contains(target) && !contacts.contains_key(&target);
                        let Some((target, _)) = context.cast_shape(
                            from,
                            0.0,
                            to - from,
                            shape.raw.as_ref(),
                            ShapeCastOptions {
                                max_time_of_impact: 1.0,
                                stop_at_penetration: true,
                                ..default()
                            },
                            QueryFilter::default().predicate(&include),
                        ) else {
                            break;
                        };
                        contacts.insert(target, age);
                        hit.push(target);
                    }
                }
                *previous_origin = origin;
            }
            RitualForm::Pulse { struck } => {
                let current_radius = radius * (age / 0.65).clamp(0.01, 1.0);
                transform.scale = Vec3::splat(current_radius);
                hit = damage_shape(
                    &context,
                    &Collider::ball(current_radius),
                    transform.translation.truncate(),
                    0.0,
                    &targets,
                )
                .into_iter()
                .filter(|target| struck.insert(*target))
                .collect();
            }
        }
        for target in hit {
            damage.write(DamageEvent {
                target,
                amount,
                source: Some(entity),
            });
        }
        if effect.remaining <= 0.0 {
            // Keep the attack and its source until the normal post-combat cleanup.
            commands.entity(entity).insert(MarkedForDespawn);
        }
    }
}

fn position_discs(
    effects: Query<&RitualEffect>,
    mut discs: Query<(&ChildOf, &TalismanDisc, &mut Transform, &mut Visibility)>,
) {
    for (parent, disc, mut transform, mut visibility) in &mut discs {
        let Ok(effect) = effects.get(parent.parent()) else {
            continue;
        };
        let RitualForm::Orbit { count, .. } = effect.form else {
            continue;
        };
        *visibility = if disc.0 < count {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let angle = effect.age * 2.5 + disc.0 as f32 * TAU / count as f32;
        transform.translation = (Vec2::from_angle(angle) * 105.0).extend(0.0);
        transform.scale = Vec3::splat(effect.radius);
        transform.rotation = Quat::from_rotation_z(angle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Enemy, Health};
    use crate::tests::{game, player, step};
    use bevy::ecs::system::RunSystemOnce;

    fn enemy(app: &mut App, position: Vec2) -> Entity {
        app.world_mut()
            .spawn((
                Enemy {
                    speed: 0.0,
                    experience_value: 0,
                },
                Health {
                    current: 1000,
                    maximum: 1000,
                },
                Transform::from_translation(position.extend(0.0)),
            ))
            .id()
    }

    fn emit(app: &mut App, kind: WeaponType, owner: Entity, weapon: Entity, radius: f32) {
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
                        damage: 10,
                        radius,
                        level: 1,
                    },
                );
            })
            .unwrap();
    }

    #[test]
    fn knife_fan_hits_forward_colliders_once_and_excludes_rear() {
        let mut app = game();
        let owner = player(&mut app);
        let origin = app
            .world()
            .get::<Transform>(owner)
            .unwrap()
            .translation
            .truncate();
        let front = enemy(&mut app, origin + Vec2::X * 80.0);
        let rear = enemy(&mut app, origin - Vec2::X * 80.0);
        step(&mut app, 3);
        emit(&mut app, WeaponType::KnifeOfTheArt, owner, owner, 110.0);
        step(&mut app, 20);
        assert_eq!(app.world().get::<Health>(front).unwrap().current, 990);
        assert_eq!(app.world().get::<Health>(rear).unwrap().current, 1000);
        assert_eq!(
            app.world_mut()
                .query::<&RitualEffect>()
                .iter(app.world())
                .count(),
            0
        );
    }

    #[test]
    fn orbit_refreshes_one_controller_and_pause_freezes_lifetime() {
        let mut app = game();
        let owner = player(&mut app);
        let origin = app
            .world()
            .get::<Transform>(owner)
            .unwrap()
            .translation
            .truncate();
        let target = enemy(&mut app, origin + Vec2::X * 105.0);
        step(&mut app, 3);
        emit(&mut app, WeaponType::PlanetaryTalismans, owner, owner, 20.0);
        step(&mut app, 1);
        assert_eq!(app.world().get::<Health>(target).unwrap().current, 990);
        step(&mut app, 1);
        assert_eq!(app.world().get::<Health>(target).unwrap().current, 990);
        // A new cast recharges contact damage before the ordinary 0.35s repeat.
        emit(&mut app, WeaponType::PlanetaryTalismans, owner, owner, 20.0);
        step(&mut app, 1);
        assert_eq!(app.world().get::<Health>(target).unwrap().current, 980);
        for _ in 0..18 {
            emit(&mut app, WeaponType::PlanetaryTalismans, owner, owner, 20.0);
            step(&mut app, 1);
        }
        assert!(app.world().get::<Health>(target).unwrap().current < 1000);
        assert_eq!(
            app.world_mut()
                .query::<&RitualEffect>()
                .iter(app.world())
                .count(),
            1
        );
        assert_eq!(
            app.world_mut()
                .query::<&TalismanDisc>()
                .iter(app.world())
                .count(),
            5
        );
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Paused);
        step(&mut app, 1);
        let age = app
            .world_mut()
            .query::<&RitualEffect>()
            .single(app.world())
            .unwrap()
            .age;
        step(&mut app, 250);
        assert_eq!(
            app.world_mut()
                .query::<&RitualEffect>()
                .single(app.world())
                .unwrap()
                .age,
            age
        );
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Playing);
        step(&mut app, 210);
        assert_eq!(
            app.world_mut()
                .query::<&RitualEffect>()
                .iter(app.world())
                .count(),
            0
        );
        assert_eq!(
            app.world_mut()
                .query::<&TalismanDisc>()
                .iter(app.world())
                .count(),
            0
        );
    }

    #[test]
    fn recitation_hits_once_per_wave_and_fourth_beat_is_stronger() {
        let mut app = game();
        let owner = player(&mut app);
        let origin = app
            .world()
            .get::<Transform>(owner)
            .unwrap()
            .translation
            .truncate();
        let target = enemy(&mut app, origin + Vec2::X * 100.0);
        step(&mut app, 3);
        for beat in 0..4 {
            emit(
                &mut app,
                WeaponType::ResonantRecitation,
                owner,
                owner,
                150.0,
            );
            step(&mut app, 45);
            let expected = if beat < 3 {
                1000 - (beat + 1) * 10
            } else {
                950
            };
            assert_eq!(app.world().get::<Health>(target).unwrap().current, expected);
        }
        let telemetry = app.world().resource::<crate::telemetry::RunTelemetry>();
        assert_eq!(telemetry.hits_by_weapon[&WeaponType::ResonantRecitation], 4);
        assert_eq!(
            telemetry.damage_by_weapon[&WeaponType::ResonantRecitation],
            50
        );
    }
}
