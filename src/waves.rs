//! Authored spawn phases and one-shot encounters share enemy definitions and Rapier placement.
use crate::{
    arena::{camera_half_view, ArenaBounds, GameCamera, WORLD_GROUP},
    components::{Enemy, Health, Player},
    content::{
        ActiveStage, EnemyDefinition, GameContent, StageLayout, StageWinCondition, TimedEncounter,
        WavePhase,
    },
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
use rand::RngExt;
use std::collections::HashSet;

#[derive(Resource, Default)]
pub struct WaveDirector {
    pub spawned: usize,
    pub phase: Option<usize>,
    pub fired_encounters: HashSet<usize>,
    pub finale: FinaleProgress,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FinaleProgress {
    #[default]
    Awaiting,
    Telegraphing,
    Active(Entity),
    Defeated,
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct EnemyKind(pub String);
#[derive(Component)]
pub struct Boss;
#[derive(Component)]
pub struct FinalBoss;
#[derive(Component)]
pub(crate) struct BossHealthFill {
    pub full_width: f32,
}
#[derive(Component)]
struct NaturalSpawn;

/// A debug request uses the same telegraph and encounter execution as the timeline.
#[derive(Message)]
pub struct TriggerEncounter(pub usize);

/// Starts the authored finale through its normal warning and spawn path.
#[derive(Message)]
pub struct TriggerFinale;

#[derive(Component)]
pub struct EncounterTelegraph {
    encounter: TimedEncounter,
    center: Vec2,
    timer: Timer,
    finale: bool,
    placement_ready: bool,
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
            .add_message::<TriggerFinale>()
            .add_systems(Update, update_boss_health_bars.after(GameplaySets::Combat))
            .add_systems(
                Update,
                (
                    spawn_wave,
                    schedule_finale,
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
    rng: &mut impl RngExt,
) -> Option<Vec2> {
    let edge = view + Vec2::splat(70.0);
    for _ in 0..32 {
        let along = rng.random::<f32>() * 2.0 - 1.0;
        let side = match layout {
            StageLayout::Arena => rng.random_range(0..4),
            StageLayout::Horizontal => rng.random_range(0..2),
            StageLayout::Vertical => 2 + rng.random_range(0..2),
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

fn enemy_definition<'a>(
    content: &'a GameContent,
    phase: &WavePhase,
    rng: &mut impl RngExt,
) -> &'a EnemyDefinition {
    let total: u64 = phase
        .enemies
        .iter()
        .map(|entry| u64::from(entry.weight))
        .sum();
    let mut roll = rng.random_range(0..total);
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
            crate::enemy_behavior::EnemyBrain::new(definition.behavior),
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
    mut rng: ResMut<crate::setup::RunRng>,
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
            &mut rng.waves,
        ) else {
            continue;
        };
        let enemy = spawn_enemy(
            &mut commands,
            &textures,
            enemy_definition(&content, phase, &mut rng.waves),
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
            Visibility::Inherited,
            EncounterTelegraph {
                encounter,
                center,
                timer: Timer::from_seconds(2.0, TimerMode::Once),
                finale: false,
                placement_ready: true,
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
    reason = "Finale scheduling validates run progress, player lifetime, and pending scene entities."
)]
fn schedule_finale(
    mut commands: Commands,
    mut stats: ResMut<GameStats>,
    content: Res<GameContent>,
    active: Res<ActiveStage>,
    mut director: ResMut<WaveDirector>,
    mut requests: MessageReader<TriggerFinale>,
    player: Query<(&Transform, &Health), With<Player>>,
    bosses: Query<(), With<FinalBoss>>,
    pending: Query<&EncounterTelegraph>,
    context: ReadRapierContext,
) {
    let requested = requests.read().count() > 0;
    let stage = content.stage(&active);
    let StageWinCondition::DefeatBoss { enemy } = &stage.win_condition else {
        return;
    };
    let Ok((player, health)) = player.single() else {
        return;
    };
    if health.current <= 0 {
        return;
    }
    if requested {
        stats.time_elapsed = stats.time_elapsed.max(stage.duration);
    }
    if stats.time_elapsed < stage.duration {
        return;
    }
    // External cleanup is not a kill. Recreate missing required actors rather
    // than granting victory or leaving an unwinnable run.
    match director.finale {
        FinaleProgress::Active(entity) if bosses.contains(entity) => return,
        FinaleProgress::Telegraphing if pending.iter().any(|event| event.finale) => return,
        FinaleProgress::Defeated => return,
        _ => {}
    }
    director.finale = FinaleProgress::Telegraphing;
    let Ok(context) = context.single() else {
        return;
    };
    let position = finale_position(
        &context,
        player.translation.truncate(),
        Vec2::from_array(stage.half_size),
    );
    let center = position.unwrap_or(player.translation.truncate());
    commands.spawn((
        RunEntity,
        Name::new("Final boss warning"),
        EncounterTelegraph {
            encounter: TimedEncounter {
                at: stage.duration,
                enemy: enemy.clone(),
                count: 1,
                ring: None,
                reward_chest: false,
            },
            center,
            timer: Timer::from_seconds(2.0, TimerMode::Once),
            finale: true,
            placement_ready: position.is_some(),
        },
        if position.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        },
        ShapeBuilder::with(&shapes::Circle {
            radius: 70.0,
            center: Vec2::ZERO,
        })
        .stroke((Color::srgba(1.0, 0.3, 0.15, 0.95), 5.0))
        .build(),
        Transform::from_translation(center.extend(2.0)),
    ));
}

fn finale_position(context: &RapierContext, origin: Vec2, bounds: Vec2) -> Option<Vec2> {
    for radius in [
        450.0, 350.0, 550.0, 250.0, 650.0, 150.0, 750.0, 850.0, 950.0, 100.0,
    ] {
        for index in 0..32 {
            let candidate =
                origin + Vec2::from_angle(index as f32 / 32.0 * std::f32::consts::TAU) * radius;
            if candidate.abs().cmple(bounds - Vec2::splat(24.0)).all()
                && position_is_free(context, candidate, 20.0, None)
            {
                return Some(candidate);
            }
        }
    }
    None
}

#[expect(
    clippy::too_many_arguments,
    reason = "Encounter execution needs scene geometry and shared authored enemy definitions."
)]
#[expect(
    clippy::type_complexity,
    reason = "The player and telegraph transform queries explicitly exclude each other."
)]
fn finish_telegraphs(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    stats: Res<GameStats>,
    content: Res<GameContent>,
    textures: Res<GameTextures>,
    bounds: Res<ArenaBounds>,
    context: ReadRapierContext,
    mut pending: Query<(
        Entity,
        &mut EncounterTelegraph,
        &mut Transform,
        &mut Visibility,
    )>,
    active: Res<ActiveStage>,
    player: Query<(&Transform, &Health), (With<Player>, Without<EncounterTelegraph>)>,
    mut director: ResMut<WaveDirector>,
) {
    let Ok((player_transform, health)) = player.single() else {
        return;
    };
    if health.current <= 0 {
        return;
    }
    let Ok(context) = context.single() else {
        return;
    };
    for (entity, mut pending, mut transform, mut visibility) in &mut pending {
        if !pending.finale && stats.time_elapsed >= content.stage(&active).duration {
            commands.entity(entity).despawn();
            continue;
        }
        if !pending.timer.tick(time.delta()).just_finished() {
            continue;
        }
        if pending.finale
            && (!pending.placement_ready || !position_is_free(&context, pending.center, 20.0, None))
        {
            if let Some(position) = finale_position(
                &context,
                player_transform.translation.truncate(),
                bounds.half_size,
            ) {
                pending.center = position;
                pending.placement_ready = true;
                pending.timer = Timer::from_seconds(2.0, TimerMode::Once);
                transform.translation = position.extend(2.0);
                *visibility = Visibility::Inherited;
            } else {
                pending.placement_ready = false;
                pending.timer = Timer::from_seconds(0.25, TimerMode::Once);
                *visibility = Visibility::Hidden;
            }
            continue;
        }
        let event = &pending.encounter;
        let definition = content
            .enemy(&event.enemy)
            .expect("validated encounter enemy");
        let mut leader = None;
        for index in 0..event.count {
            let mut position = None;
            let attempts = if pending.finale { 1 } else { 24 };
            for attempt in 0..attempts {
                let angle = (index as f32 / event.count as f32 + (attempt % 24) as f32 / 24.0)
                    * std::f32::consts::TAU;
                let radius = if pending.finale { 0.0 } else { 150.0 };
                let candidate = pending.center + Vec2::from_angle(angle) * radius;
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
            if pending.finale {
                commands.entity(boss).insert(FinalBoss);
                director.finale = FinaleProgress::Active(boss);
            }
            commands
                .entity(boss)
                .insert(Boss)
                .with_children(|children| {
                    let width = definition.size.max(48.0);
                    let height = definition.size * 0.5 + 10.0;
                    children.spawn((
                        Sprite::from_color(
                            Color::srgb(0.12, 0.08, 0.07),
                            Vec2::new(width + 4.0, 10.0),
                        ),
                        Transform::from_xyz(0.0, height, 1.0),
                    ));
                    children.spawn((
                        BossHealthFill { full_width: width },
                        Sprite::from_color(Color::srgb(1.0, 0.77, 0.25), Vec2::new(width, 6.0)),
                        Transform::from_xyz(0.0, height, 2.0),
                    ));
                });
            if event.reward_chest {
                commands.entity(boss).insert(crate::loot::ChestDrop);
            }
            leader = Some(boss);
        }
        if pending.finale && leader.is_none() {
            // Retry through Rapier after enemies move or obstructing props clear.
            // A failed spawn never consumes the required encounter.
            pending.timer = Timer::from_seconds(0.25, TimerMode::Once);
            continue;
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

// Presentation stays responsive to debug edits while the simulation is paused.
fn update_boss_health_bars(
    bosses: Query<&Health, With<Boss>>,
    mut fills: Query<(&BossHealthFill, &ChildOf, &mut Sprite, &mut Transform)>,
) {
    for (fill, parent, mut sprite, mut transform) in &mut fills {
        let Ok(health) = bosses.get(parent.parent()) else {
            continue;
        };
        let fraction = if health.maximum > 0 {
            (health.current as f32 / health.maximum as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let width = fill.full_width * fraction;
        sprite.custom_size = Some(Vec2::new(width, 6.0));
        transform.translation.x = (width - fill.full_width) * 0.5;
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
