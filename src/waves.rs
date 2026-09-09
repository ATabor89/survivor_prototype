//! Authored spawn phases and one-shot encounters share enemy definitions and Rapier placement.
use crate::{
    arena::{camera_half_view, ArenaBounds, GameCamera, WORLD_GROUP},
    components::{Enemy, Health, Player},
    content::{ActiveStage, EnemyDefinition, GameContent, StageLayout, TimedEncounter, WavePhase},
    death::MarkedForDeath,
    resources::{GameState, GameStats, GameTextures, SpawnTimer, WaveConfig},
    run::RunEntity,
    weapons::{BindingEffect, InvocationPull},
    GameplaySets,
};
use bevy::prelude::*;
use bevy_prototype_lyon::{
    geometry::{ShapeBuilder, ShapeBuilderBase},
    shapes,
};
use bevy_rapier2d::prelude::*;
use std::collections::HashSet;

#[derive(Resource, Default)]
pub struct WaveDirector {
    pub spawned: usize,
    pub phase: Option<usize>,
    pub fired_encounters: HashSet<usize>,
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct EnemyKind(pub String);
#[derive(Component)]
pub struct Boss;
#[derive(Component)]
struct NaturalSpawn;

/// A debug request uses the same telegraph and encounter execution as the timeline.
#[derive(Message)]
pub struct TriggerEncounter(pub usize);

#[derive(Component)]
pub struct EncounterTelegraph {
    encounter: TimedEncounter,
    center: Vec2,
    timer: Timer,
}

#[derive(Component)]
pub struct RingMember {
    center: Vec2,
    radius: Vec2,
    angle: f32,
    pulse: f32,
    period: f32,
    started: f32,
    expires: f32,
    boss: Option<Entity>,
}

pub fn phase_index(phases: &[WavePhase], elapsed: f32) -> usize {
    phases
        .partition_point(|phase| phase.start <= elapsed)
        .saturating_sub(1)
}

fn formation_geometry(event: &TimedEncounter, origin: Vec2, bounds: Vec2) -> (Vec2, Vec2) {
    let Some(ring) = &event.ring else {
        return (origin, Vec2::ZERO);
    };
    let radius = Vec2::splat(ring.radius).min((bounds - Vec2::splat(32.0)) / (1.0 + ring.pulse));
    let extent = radius * (1.0 + ring.pulse);
    let center = origin.clamp(
        -bounds + extent + Vec2::splat(24.0),
        bounds - extent - Vec2::splat(24.0),
    );
    (center, radius)
}

// The HUD groups phases into run minutes, while the schedule permits any timestamp.
pub fn wave_index(elapsed: f32) -> usize {
    (elapsed.max(0.0) / 60.0) as usize
}

pub struct WavesPlugin;
impl Plugin for WavesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WaveDirector>()
            .add_message::<TriggerEncounter>()
            .add_systems(
                Update,
                (
                    spawn_wave,
                    schedule_encounters,
                    finish_telegraphs,
                    recycle_distant_enemies,
                )
                    .chain()
                    .in_set(GameplaySets::Spawning)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                move_ring_members
                    .in_set(GameplaySets::Movement)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

pub fn position_is_free(
    context: &RapierContext,
    position: Vec2,
    radius: f32,
    exclude: Option<Entity>,
) -> bool {
    let collider = Collider::ball(radius);
    let mut filter = QueryFilter::default()
        .exclude_sensors()
        .groups(CollisionGroups::new(
            Group::GROUP_2,
            WORLD_GROUP | Group::GROUP_1 | Group::GROUP_2 | Group::GROUP_6,
        ));
    filter.exclude_collider = exclude;
    let mut occupied = false;
    context.intersect_shape(position, 0.0, collider.raw.as_ref(), filter, |_| {
        occupied = true;
        false
    });
    !occupied
}

/// Layout controls which edges can supply enemies; Rapier validates each candidate.
pub fn offscreen_position(
    context: &RapierContext,
    center: Vec2,
    view: Vec2,
    bounds: Vec2,
    layout: StageLayout,
) -> Option<Vec2> {
    let edge = view + Vec2::splat(70.0);
    for _ in 0..32 {
        let along = rand::random::<f32>() * 2.0 - 1.0;
        let side = match layout {
            StageLayout::Arena => rand::random::<u32>() % 4,
            StageLayout::Horizontal => rand::random::<u32>() % 2,
            StageLayout::Vertical => 2 + rand::random::<u32>() % 2,
        };
        let offset = match side {
            0 => Vec2::new(-edge.x, along * view.y.min(bounds.y - 30.0)),
            1 => Vec2::new(edge.x, along * view.y.min(bounds.y - 30.0)),
            2 => Vec2::new(along * view.x.min(bounds.x - 30.0), -edge.y),
            _ => Vec2::new(along * view.x.min(bounds.x - 30.0), edge.y),
        };
        let candidate = center + offset;
        if candidate.abs().cmple(bounds - Vec2::splat(24.0)).all()
            && position_is_free(context, candidate, 14.0, None)
        {
            return Some(candidate);
        }
    }
    None
}

fn enemy_definition<'a>(content: &'a GameContent, phase: &WavePhase) -> &'a EnemyDefinition {
    let total: u64 = phase
        .enemies
        .iter()
        .map(|entry| u64::from(entry.weight))
        .sum();
    let mut roll = rand::random::<u64>() % total;
    for entry in &phase.enemies {
        if roll < u64::from(entry.weight) {
            return content.enemy(&entry.id).expect("validated enemy reference");
        }
        roll -= u64::from(entry.weight);
    }
    unreachable!("validated nonempty weighted enemy pool")
}

pub fn spawn_enemy(
    commands: &mut Commands,
    textures: &GameTextures,
    definition: &EnemyDefinition,
    position: Vec2,
    health_scale: f32,
) -> Entity {
    let health = ((definition.health as f32 * health_scale).round() as i32).max(1);
    commands
        .spawn((
            RunEntity,
            EnemyKind(definition.id.clone()),
            Name::new(definition.name.clone()),
            Enemy {
                speed: definition.speed,
                experience_value: definition.xp,
            },
            Health {
                current: health,
                maximum: health,
            },
            Sprite {
                image: textures.enemies.clone(),
                custom_size: Some(Vec2::splat(definition.size)),
                color: Color::srgba(
                    definition.color[0],
                    definition.color[1],
                    definition.color[2],
                    definition.color[3],
                ),
                texture_atlas: Some(TextureAtlas {
                    layout: textures.enemies_layout.clone(),
                    index: 1,
                }),
                ..default()
            },
            Transform::from_translation(position.extend(0.0)),
        ))
        .id()
}

#[expect(
    clippy::too_many_arguments,
    reason = "Bevy injects independent authored data, simulation resources, and scene queries."
)]
fn spawn_wave(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    stats: Res<GameStats>,
    bounds: Res<ArenaBounds>,
    content: Res<GameContent>,
    active: Res<ActiveStage>,
    textures: Res<GameTextures>,
    mut timer: ResMut<SpawnTimer>,
    mut config: ResMut<WaveConfig>,
    mut director: ResMut<WaveDirector>,
    enemies: Query<(), With<Enemy>>,
    player: Query<&Health, With<Player>>,
    camera: Query<(&Transform, &Projection), With<GameCamera>>,
    context: ReadRapierContext,
    #[cfg(feature = "dev-tools")] debug: Res<crate::dev_tools::DebugSession>,
) {
    let stage = content.stage(&active);
    if stats.time_elapsed >= stage.duration || !player.single().is_ok_and(|h| h.current > 0) {
        return;
    }
    let index = phase_index(&stage.schedule, stats.time_elapsed);
    let phase = &stage.schedule[index];
    config.current_wave = wave_index(stats.time_elapsed) as u32 + 1;
    config.max_enemies = phase.cap as u32;
    if director.phase != Some(index) {
        director.phase = Some(index);
        timer
            .0
            .set_duration(std::time::Duration::from_secs_f32(phase.interval));
        // Keep elapsed progress, allowing a phase transition to change cadence immediately.
    }
    #[cfg(feature = "dev-tools")]
    if !debug.waves {
        return;
    }
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }
    let current = enemies.iter().count();
    let desired = phase
        .batch
        .max(phase.minimum.saturating_sub(current))
        .min(32);
    let slots = phase.cap.saturating_sub(current).min(desired);
    let Ok((camera, projection)) = camera.single() else {
        return;
    };
    let Ok(context) = context.single() else {
        return;
    };
    for _ in 0..slots {
        let Some(position) = offscreen_position(
            &context,
            camera.translation.truncate(),
            camera_half_view(projection),
            bounds.half_size,
            stage.layout,
        ) else {
            continue;
        };
        let enemy = spawn_enemy(
            &mut commands,
            &textures,
            enemy_definition(&content, phase),
            position,
            phase.health_scale,
        );
        commands.entity(enemy).insert(NaturalSpawn);
        director.spawned += 1;
    }
}

#[cfg_attr(
    feature = "dev-tools",
    expect(
        clippy::too_many_arguments,
        reason = "Timeline scheduling reads independent content and scene resources."
    )
)]
fn schedule_encounters(
    mut commands: Commands,
    stats: Res<GameStats>,
    content: Res<GameContent>,
    active: Res<ActiveStage>,
    mut director: ResMut<WaveDirector>,
    mut requests: MessageReader<TriggerEncounter>,
    player: Query<(&Transform, &Health), With<Player>>,
    #[cfg(feature = "dev-tools")] debug: Res<crate::dev_tools::DebugSession>,
) {
    let stage = content.stage(&active);
    let Ok((player, health)) = player.single() else {
        return;
    };
    if health.current <= 0 || stats.time_elapsed >= stage.duration {
        return;
    }
    let mut due: Vec<usize> = requests
        .read()
        .map(|request| request.0)
        .filter(|index| *index < stage.encounters.len())
        .collect();
    #[cfg(feature = "dev-tools")]
    let natural_enabled = debug.waves;
    #[cfg(not(feature = "dev-tools"))]
    let natural_enabled = true;
    if natural_enabled {
        for (index, event) in stage.encounters.iter().enumerate() {
            if event.at <= stats.time_elapsed && !director.fired_encounters.contains(&index) {
                due.push(index);
            }
        }
    }
    due.sort_unstable();
    due.dedup();
    for index in due {
        director.fired_encounters.insert(index);
        let encounter = stage.encounters[index].clone();
        let radius = encounter.ring.as_ref().map_or(150.0, |ring| ring.radius);
        let (center, _) = formation_geometry(
            &encounter,
            player.translation.truncate(),
            Vec2::from_array(stage.half_size),
        );
        commands.spawn((
            RunEntity,
            Name::new("Encounter warning"),
            EncounterTelegraph {
                encounter,
                center,
                timer: Timer::from_seconds(2.0, TimerMode::Once),
            },
            ShapeBuilder::with(&shapes::Circle {
                radius,
                center: Vec2::ZERO,
            })
            .stroke((Color::srgba(1.0, 0.7, 0.25, 0.85), 4.0))
            .build(),
            Transform::from_translation(center.extend(2.0)),
        ));
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Encounter execution needs scene geometry and shared authored enemy definitions."
)]
fn finish_telegraphs(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    stats: Res<GameStats>,
    content: Res<GameContent>,
    textures: Res<GameTextures>,
    bounds: Res<ArenaBounds>,
    context: ReadRapierContext,
    mut pending: Query<(Entity, &mut EncounterTelegraph)>,
    active: Res<ActiveStage>,
    player: Query<&Health, With<Player>>,
) {
    if stats.time_elapsed >= content.stage(&active).duration
        || !player.single().is_ok_and(|health| health.current > 0)
    {
        return;
    }
    let Ok(context) = context.single() else {
        return;
    };
    for (entity, mut pending) in &mut pending {
        if !pending.timer.tick(time.delta()).just_finished() {
            continue;
        }
        let event = &pending.encounter;
        let definition = content
            .enemy(&event.enemy)
            .expect("validated encounter enemy");
        let mut leader = None;
        for index in 0..event.count {
            let mut position = None;
            for attempt in 0..24 {
                let angle = (index as f32 / event.count as f32 + attempt as f32 / 24.0)
                    * std::f32::consts::TAU;
                let candidate = pending.center + Vec2::from_angle(angle) * 150.0;
                if candidate
                    .abs()
                    .cmple(bounds.half_size - Vec2::splat(24.0))
                    .all()
                    && position_is_free(&context, candidate, 20.0, None)
                {
                    position = Some(candidate);
                    break;
                }
            }
            let Some(position) = position else {
                continue;
            };
            let boss = spawn_enemy(&mut commands, &textures, definition, position, 1.0);
            commands
                .entity(boss)
                .insert(Boss)
                .with_children(|children| {
                    children.spawn((
                        Sprite::from_color(Color::srgb(1.0, 0.77, 0.25), Vec2::new(28.0, 7.0)),
                        Transform::from_xyz(0.0, definition.size * 0.45, 1.0),
                    ));
                });
            if event.reward_chest {
                commands.entity(boss).insert(crate::loot::ChestDrop);
            }
            leader = Some(boss);
        }
        if let Some(ring) = &event.ring {
            let definition = content.enemy(&ring.enemy).expect("validated ring enemy");
            let (center, radius) = formation_geometry(event, pending.center, bounds.half_size);
            for index in 0..ring.count {
                let angle = index as f32 / ring.count as f32 * std::f32::consts::TAU;
                let position = center + Vec2::from_angle(angle) * radius;
                if !position_is_free(&context, position, 14.0, None) {
                    continue;
                }
                let member = spawn_enemy(&mut commands, &textures, definition, position, 1.0);
                commands.entity(member).insert(RingMember {
                    center,
                    radius,
                    angle,
                    pulse: ring.pulse,
                    period: ring.period,
                    started: stats.time_elapsed,
                    expires: stats.time_elapsed + ring.duration,
                    boss: leader,
                });
            }
        }
        commands.entity(entity).despawn();
    }
}

#[expect(
    clippy::type_complexity,
    reason = "The query declares formation movement and optional combat modifiers explicitly."
)]
fn move_ring_members(
    mut commands: Commands,
    stats: Res<GameStats>,
    bosses: Query<&Health, With<Boss>>,
    mut members: Query<
        (
            Entity,
            &RingMember,
            &Transform,
            &mut Velocity,
            Option<&BindingEffect>,
            Option<&InvocationPull>,
        ),
        Without<MarkedForDeath>,
    >,
) {
    for (entity, member, transform, mut velocity, binding, pull) in &mut members {
        if stats.time_elapsed >= member.expires
            || member
                .boss
                .is_some_and(|boss| !bosses.get(boss).is_ok_and(|h| h.current > 0))
        {
            commands.entity(entity).despawn();
            continue;
        }
        let phase = (stats.time_elapsed - member.started) / member.period * std::f32::consts::TAU;
        let radius = member.radius * (1.0 - member.pulse * phase.sin());
        let target = member.center + Vec2::from_angle(member.angle) * radius;
        let steering = ((target - transform.translation.truncate()) * 6.0).clamp_length_max(90.0);
        velocity.linear = steering * (1.0 - binding.map_or(0.0, |effect| effect.strength))
            + pull.map_or(Vec2::ZERO, |effect| effect.velocity);
    }
}

fn recycle_distant_enemies(
    mut commands: Commands,
    camera: Query<(&Transform, &Projection), With<GameCamera>>,
    enemies: Query<(Entity, &Transform), With<NaturalSpawn>>,
) {
    let Ok((camera, projection)) = camera.single() else {
        return;
    };
    let extent = camera_half_view(projection) * 3.0 + Vec2::splat(300.0);
    for (entity, transform) in &enemies {
        if (transform.translation.truncate() - camera.translation.truncate())
            .abs()
            .cmpgt(extent)
            .any()
        {
            commands.entity(entity).despawn();
        }
    }
}
