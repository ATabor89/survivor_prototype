//! Delayed ground writing and bounded branching emanations. Rapier owns hit geometry.
use super::arsenal::{damage_shape, Cast, DamageTargets};
use super::{Attack, AttackSource, WeaponType};
use crate::{
    combat::DamageEvent, death::MarkedForDespawn, resources::GameState, run::RunEntity,
    GameplaySets,
};
use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use bevy_rapier2d::prelude::*;

pub struct InscriptionWeaponsPlugin;
impl Plugin for InscriptionWeaponsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (prepare_writing, advance_writing, animate_strokes)
                .chain()
                .after(super::weapon_firing_system)
                .in_set(GameplaySets::Weapons)
                .run_if(in_state(GameState::Playing)),
        );
    }
}

const MAX_ACTIVE: usize = 4;
const WRITING_LIFETIME: f32 = 2.2;

#[derive(Component)]
struct PendingWriting(Cast);

#[derive(Component)]
struct Writing {
    weapon: Entity,
    elapsed: f32,
    damage: i32,
    pattern: Pattern,
}

enum Pattern {
    Inscription {
        center: Vec2,
        radius: f32,
        next_stroke: usize,
        rows_hit: HashSet<Entity>,
        columns_hit: HashSet<Entity>,
    },
    Lattice {
        nodes: Vec<Node>,
    },
}

struct Node {
    target: Entity,
    parent: Option<usize>,
    start: Vec2,
    end: Vec2,
    at: f32,
    damage: i32,
    discharged: bool,
}

#[derive(Component)]
struct Stroke {
    start: Vec2,
    end: Vec2,
    at: f32,
    width: f32,
    node: Option<usize>,
}

#[derive(Component)]
struct NodeMarker(usize);

pub fn cast(commands: &mut Commands, cast: Cast) {
    // No target means no ground attack. Cooldowns still advance normally.
    if cast.target.is_none() {
        return;
    }
    commands.spawn((
        Attack,
        AttackSource(cast.kind),
        RunEntity,
        PendingWriting(cast),
        Name::new(format!("{} writing", cast.kind)),
        Transform::default(),
        Visibility::default(),
    ));
}

fn prepare_writing(
    mut commands: Commands,
    pending: Query<(Entity, &PendingWriting)>,
    active: Query<&Writing, Without<MarkedForDespawn>>,
    context: ReadRapierContext,
    targets: DamageTargets,
    positions: Query<&Transform>,
) {
    let Ok(context) = context.single() else {
        return;
    };
    let mut counts = HashMap::<Entity, usize>::default();
    for writing in &active {
        *counts.entry(writing.weapon).or_default() += 1;
    }
    for (entity, pending) in &pending {
        let cast = pending.0;
        let count = counts.entry(cast.weapon).or_default();
        if *count >= MAX_ACTIVE {
            commands
                .entity(entity)
                .insert(MarkedForDespawn)
                .remove::<PendingWriting>();
            continue;
        }
        *count += 1;
        let (_, center) = cast.target.expect("casts without targets are rejected");
        let pattern = if cast.kind == WeaponType::AngelicInscription {
            let radius = cast.radius.clamp(24.0, 600.0);
            for index in 0..6 {
                let (start, end) = inscription_line(center, radius, index);
                spawn_stroke(
                    &mut commands,
                    entity,
                    start,
                    end,
                    stroke_time(index),
                    radius * 0.36,
                    None,
                );
            }
            Pattern::Inscription {
                center,
                radius,
                next_stroke: 0,
                rows_hit: HashSet::default(),
                columns_hit: HashSet::default(),
            }
        } else {
            let mut candidates = damage_shape(
                &context,
                &Collider::ball(cast.radius.clamp(32.0, 1400.0)),
                cast.origin,
                0.0,
                &targets,
            )
            .into_iter()
            .filter_map(|target| {
                positions
                    .get(target)
                    .ok()
                    .map(|t| (target, t.translation.truncate()))
            })
            .collect::<Vec<_>>();
            candidates.sort_by(|a, b| {
                a.1.distance_squared(center)
                    .total_cmp(&b.1.distance_squared(center))
                    .then(a.0.cmp(&b.0))
            });
            candidates.truncate(64);
            let limit = (3 + cast.level / 2).min(6) as usize;
            let nodes = branch_nodes(cast.origin, candidates, limit, cast.damage.max(0));
            for (index, node) in nodes.iter().enumerate() {
                spawn_stroke(
                    &mut commands,
                    entity,
                    node.start,
                    node.end,
                    node.at,
                    4.0,
                    Some(index),
                );
            }
            Pattern::Lattice { nodes }
        };
        commands
            .entity(entity)
            .remove::<PendingWriting>()
            .insert(Writing {
                weapon: cast.weapon,
                elapsed: 0.0,
                damage: cast.damage.max(0),
                pattern,
            });
    }
}

// Only acquisition uses distance: contact resolution below always uses Rapier.
fn branch_nodes(
    origin: Vec2,
    mut candidates: Vec<(Entity, Vec2)>,
    limit: usize,
    damage: i32,
) -> Vec<Node> {
    let mut nodes: Vec<Node> = Vec::new();
    let Some((target, first)) = candidates.first().copied() else {
        return nodes;
    };
    candidates.remove(0);
    nodes.push(Node {
        target,
        parent: None,
        start: origin,
        end: first,
        at: 0.45,
        damage: 0,
        discharged: false,
    });
    while nodes.len() < limit && !candidates.is_empty() {
        let (candidate, parent) = (0..candidates.len())
            .flat_map(|candidate| (0..nodes.len()).map(move |parent| (candidate, parent)))
            .min_by(|(a, pa), (b, pb)| {
                candidates[*a]
                    .1
                    .distance_squared(nodes[*pa].end)
                    .total_cmp(&candidates[*b].1.distance_squared(nodes[*pb].end))
            })
            .expect("nonempty candidates and nodes");
        let (target, end) = candidates.remove(candidate);
        nodes.push(Node {
            target,
            parent: Some(parent),
            start: nodes[parent].end,
            end,
            at: nodes[parent].at + 0.2,
            damage: 0,
            discharged: false,
        });
    }
    let count = nodes.len() as i32;
    for (index, node) in nodes.iter_mut().enumerate() {
        node.damage = damage / count + i32::from((index as i32) < damage % count);
    }
    nodes
}

fn inscription_line(center: Vec2, radius: f32, index: usize) -> (Vec2, Vec2) {
    let offset = (index % 3) as f32 - 1.0;
    if index < 3 {
        (
            center + Vec2::new(-radius, offset * radius * 0.45),
            center + Vec2::new(radius, offset * radius * 0.45),
        )
    } else {
        (
            center + Vec2::new(offset * radius * 0.65, -radius * 0.65),
            center + Vec2::new(offset * radius * 0.65, radius * 0.65),
        )
    }
}

fn stroke_time(index: usize) -> f32 {
    if index < 3 {
        0.6 + index as f32 * 0.16
    } else {
        1.25 + (index - 3) as f32 * 0.16
    }
}

fn spawn_stroke(
    commands: &mut Commands,
    parent: Entity,
    start: Vec2,
    end: Vec2,
    at: f32,
    width: f32,
    node: Option<usize>,
) {
    let delta = end - start;
    commands.entity(parent).with_children(|children| {
        children.spawn((
            Stroke {
                start,
                end,
                at,
                width,
                node,
            },
            Sprite::from_color(
                Color::srgba(0.55, 0.85, 1.0, 0.22),
                Vec2::new(delta.length().max(2.0), width),
            ),
            Transform::from_translation(((start + end) * 0.5).extend(2.5))
                .with_rotation(Quat::from_rotation_z(delta.to_angle())),
        ));
        // Small diamonds distinguish written nodes from enemy warning lines.
        let mut marker = children.spawn((
            Sprite::from_color(Color::srgba(0.8, 0.9, 1.0, 0.6), Vec2::splat(8.0)),
            Transform::from_translation(end.extend(2.6))
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
        ));
        if let Some(index) = node {
            marker.insert(NodeMarker(index));
        }
    });
}

fn advance_writing(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    context: ReadRapierContext,
    targets: DamageTargets,
    positions: Query<&Transform>,
    mut writing: Query<(Entity, &mut Writing), Without<MarkedForDespawn>>,
    mut damage: MessageWriter<DamageEvent>,
) {
    let Ok(context) = context.single() else {
        return;
    };
    for (entity, mut writing) in &mut writing {
        writing.elapsed += time.delta_secs();
        let elapsed = writing.elapsed;
        let amount = writing.damage;
        match &mut writing.pattern {
            Pattern::Inscription {
                center,
                radius,
                next_stroke,
                rows_hit,
                columns_hit,
            } => {
                while *next_stroke < 6 && elapsed >= stroke_time(*next_stroke) {
                    let index = *next_stroke;
                    let (start, end) = inscription_line(*center, *radius, index);
                    let shape = Collider::cuboid(start.distance(end) * 0.5, *radius * 0.18);
                    let hits = damage_shape(
                        &context,
                        &shape,
                        (start + end) * 0.5,
                        (end - start).to_angle(),
                        &targets,
                    );
                    let (already_hit, damage_amount) = if index < 3 {
                        (&mut *rows_hit, amount / 2)
                    } else {
                        (&mut *columns_hit, amount - amount / 2)
                    };
                    for target in hits {
                        if already_hit.insert(target) {
                            damage.write(DamageEvent {
                                target,
                                amount: damage_amount,
                                source: Some(entity),
                            });
                        }
                    }
                    *next_stroke += 1;
                }
            }
            Pattern::Lattice { nodes } => {
                // Selection and topology stay fixed; living endpoints follow their
                // selected foes until impact. Dead targets retain their last point.
                for node in nodes.iter_mut().filter(|node| !node.discharged) {
                    if targets.contains(node.target) {
                        if let Ok(transform) = positions.get(node.target) {
                            node.end = transform.translation.truncate();
                        }
                    }
                }
                for index in 0..nodes.len() {
                    if !nodes[index].discharged {
                        if let Some(parent) = nodes[index].parent {
                            nodes[index].start = nodes[parent].end;
                        }
                    }
                }
                for node in nodes
                    .iter_mut()
                    .filter(|node| !node.discharged && elapsed >= node.at)
                {
                    let hits =
                        damage_shape(&context, &Collider::ball(24.0), node.end, 0.0, &targets);
                    let count = hits.len() as i32;
                    for (index, target) in hits.into_iter().enumerate() {
                        let amount =
                            node.damage / count + i32::from((index as i32) < node.damage % count);
                        if amount > 0 {
                            damage.write(DamageEvent {
                                target,
                                amount,
                                source: Some(entity),
                            });
                        }
                    }
                    node.discharged = true;
                }
            }
        }
        if elapsed >= WRITING_LIFETIME {
            commands.entity(entity).insert(MarkedForDespawn);
        }
    }
}

fn animate_strokes(
    writing: Query<&Writing>,
    mut strokes: Query<(&ChildOf, &Stroke, &mut Sprite, &mut Transform)>,
    mut markers: Query<(&ChildOf, &NodeMarker, &mut Transform), Without<Stroke>>,
) {
    for (parent, stroke, mut sprite, mut transform) in &mut strokes {
        let Ok(writing) = writing.get(parent.parent()) else {
            continue;
        };
        let since = writing.elapsed - stroke.at;
        let active = (0.0..0.22).contains(&since);
        let alpha = if since < 0.0 {
            0.14
        } else if active {
            0.75
        } else {
            0.06
        };
        let pulse = stroke.node.is_some();
        sprite.color = if pulse {
            Color::srgba(0.95, 0.74, 1.0, alpha)
        } else {
            Color::srgba(0.65, 0.88, 1.0, alpha)
        };
        let (start, end) = match (&writing.pattern, stroke.node) {
            (Pattern::Lattice { nodes }, Some(index)) => (nodes[index].start, nodes[index].end),
            _ => (stroke.start, stroke.end),
        };
        let delta = end - start;
        transform.rotation = Quat::from_rotation_z(delta.to_angle());
        if pulse && (-0.18..0.0).contains(&since) {
            let progress = (since + 0.18) / 0.18;
            sprite.custom_size = Some(Vec2::new(
                (delta.length() * 0.16).max(6.0),
                stroke.width * 2.0,
            ));
            transform.translation = start.lerp(end, progress).extend(2.7);
        } else {
            sprite.custom_size = Some(Vec2::new(
                delta.length().max(2.0),
                stroke.width * if active && pulse { 2.5 } else { 1.0 },
            ));
            transform.translation = ((start + end) * 0.5).extend(2.5);
        }
    }
    for (parent, marker, mut transform) in &mut markers {
        if let Ok(Writing {
            pattern: Pattern::Lattice { nodes },
            ..
        }) = writing.get(parent.parent())
        {
            transform.translation = nodes[marker.0].end.extend(2.6);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        components::{Enemy, Health},
        tests::{count, game, key, player, step},
    };
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
                RigidBody::Fixed,
                Collider::ball(12.0),
                CollisionGroups::new(Group::GROUP_2, Group::ALL),
                Transform::from_translation(position.extend(0.0)),
            ))
            .id()
    }

    fn launch(app: &mut App, kind: WeaponType, target: Entity, center: Vec2, weapon: Entity) {
        let owner = player(app);
        app.world_mut()
            .run_system_once(move |mut commands: Commands| {
                cast(
                    &mut commands,
                    Cast {
                        owner,
                        weapon,
                        kind,
                        origin: center - Vec2::X * 100.0,
                        facing: Vec2::X,
                        target: Some((target, center)),
                        damage: 45,
                        radius: 140.0,
                        level: 8,
                    },
                )
            })
            .unwrap();
    }

    #[test]
    fn inscription_locks_ground_and_hits_once_per_orientation_through_rapier() {
        let mut app = game();
        let center = Vec2::new(700.0, 700.0);
        let selected = enemy(&mut app, center);
        let stationary = enemy(&mut app, center + Vec2::splat(10.0));
        step(&mut app, 3);
        launch(
            &mut app,
            WeaponType::AngelicInscription,
            selected,
            center,
            selected,
        );
        step(&mut app, 3);
        app.world_mut()
            .get_mut::<Transform>(selected)
            .unwrap()
            .translation += Vec3::new(400.0, 400.0, 0.0);
        step(&mut app, 20);
        assert_eq!(
            app.world().get::<Health>(stationary).unwrap().current,
            1000,
            "warning must precede damage"
        );
        step(&mut app, 90);
        assert_eq!(app.world().get::<Health>(stationary).unwrap().current, 955);
        assert_eq!(
            app.world().get::<Health>(selected).unwrap().current,
            1000,
            "writing cannot follow its original target"
        );
        step(&mut app, 30);
        assert_eq!(count::<Writing>(&mut app), 0);
        assert_eq!(count::<Stroke>(&mut app), 0);
    }

    #[test]
    fn lattice_branches_without_multiplying_its_damage_budget() {
        let origin = Vec2::ZERO;
        let mut world = World::new();
        let nodes = branch_nodes(
            origin,
            vec![
                (world.spawn_empty().id(), Vec2::X * 20.0),
                (world.spawn_empty().id(), Vec2::new(30.0, 20.0)),
                (world.spawn_empty().id(), Vec2::new(30.0, -20.0)),
            ],
            6,
            47,
        );
        assert_eq!(nodes.iter().map(|node| node.damage).sum::<i32>(), 47);
        assert_eq!(nodes[1].start, nodes[0].end);
        assert_eq!(nodes[2].start, nodes[0].end);

        let mut app = game();
        let center = Vec2::new(700.0, 700.0);
        // Overlapping colliders stress budget splitting at every pulse node.
        let targets = (0..3).map(|_| enemy(&mut app, center)).collect::<Vec<_>>();
        step(&mut app, 3);
        launch(
            &mut app,
            WeaponType::EmanationLattice,
            targets[0],
            center,
            targets[0],
        );
        step(&mut app, 100);
        let removed = targets
            .iter()
            .map(|target| 1000 - app.world().get::<Health>(*target).unwrap().current)
            .sum::<i32>();
        assert_eq!(removed, 45);
    }

    #[test]
    fn lattice_tracks_a_moving_selected_enemy_until_impact_and_updates_its_marker() {
        let mut app = game();
        let center = Vec2::new(700.0, 700.0);
        let target = enemy(&mut app, center);
        step(&mut app, 3);
        launch(
            &mut app,
            WeaponType::EmanationLattice,
            target,
            center,
            target,
        );
        step(&mut app, 2);
        for _ in 0..10 {
            app.world_mut()
                .get_mut::<Transform>(target)
                .unwrap()
                .translation
                .y += 3.0;
            step(&mut app, 1);
        }
        let marker = app
            .world_mut()
            .query_filtered::<&Transform, With<NodeMarker>>()
            .single(app.world())
            .unwrap();
        assert!((marker.translation.y - (center.y + 30.0)).abs() < 0.01);
        for _ in 0..25 {
            app.world_mut()
                .get_mut::<Transform>(target)
                .unwrap()
                .translation
                .y += 3.0;
            step(&mut app, 1);
        }
        // At impact this target is far outside the original 24px node sphere.
        assert_eq!(app.world().get::<Health>(target).unwrap().current, 955);
        let writing = app
            .world_mut()
            .query::<&Writing>()
            .single(app.world())
            .unwrap();
        let Pattern::Lattice { nodes } = &writing.pattern else {
            panic!("expected lattice");
        };
        assert!(nodes[0].discharged);
        assert!(nodes[0].end.y > center.y + 40.0);
        assert!(
            nodes[0].end.y < center.y + 105.0,
            "discharged nodes stop following"
        );
    }

    #[test]
    fn writing_freezes_when_paused_and_reset_removes_the_entire_tree() {
        let mut app = game();
        let center = Vec2::new(700.0, 700.0);
        let target = enemy(&mut app, center);
        step(&mut app, 3);
        launch(
            &mut app,
            WeaponType::AngelicInscription,
            target,
            center,
            target,
        );
        step(&mut app, 3);
        key(&mut app, KeyCode::Escape);
        step(&mut app, 2);
        let before = app
            .world_mut()
            .query::<&Writing>()
            .single(app.world())
            .unwrap()
            .elapsed;
        step(&mut app, 120);
        assert_eq!(
            app.world_mut()
                .query::<&Writing>()
                .single(app.world())
                .unwrap()
                .elapsed,
            before
        );
        assert_eq!(app.world().get::<Health>(target).unwrap().current, 1000);
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Restarting);
        step(&mut app, 3);
        assert_eq!(count::<Writing>(&mut app), 0);
        assert_eq!(count::<Stroke>(&mut app), 0);
        assert_eq!(count::<PendingWriting>(&mut app), 0);
    }

    #[test]
    fn simultaneous_casts_are_capped_per_weapon_before_creating_visual_children() {
        let mut app = game();
        let center = Vec2::new(700.0, 700.0);
        let target = enemy(&mut app, center);
        step(&mut app, 3);
        for _ in 0..40 {
            launch(
                &mut app,
                WeaponType::AngelicInscription,
                target,
                center,
                target,
            );
        }
        step(&mut app, 2);
        assert_eq!(count::<Writing>(&mut app), MAX_ACTIVE);
        assert_eq!(count::<Stroke>(&mut app), MAX_ACTIVE * 6);
        assert_eq!(count::<PendingWriting>(&mut app), 0);
    }
}
