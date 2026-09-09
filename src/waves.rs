//! A ten-minute data-driven spawn schedule, using Rapier queries for safe spawn locations.
use crate::{
    arena::{camera_half_view, ArenaBounds, GameCamera, WORLD_GROUP},
    components::{Enemy, Health, Player},
    resources::{GameState, GameStats, GameTextures, SpawnTimer, WaveConfig},
    run::{RunEntity, SURVIVAL_SECONDS},
    GameplaySets,
};
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

#[derive(Clone, Copy, Debug)]
pub struct WaveDefinition {
    pub interval: f32,
    pub batch: usize,
    pub cap: usize,
    pub health_scale: f32,
}

pub const WAVES: [WaveDefinition; 10] = [
    WaveDefinition {
        interval: 2.0,
        batch: 1,
        cap: 24,
        health_scale: 1.0,
    },
    WaveDefinition {
        interval: 1.7,
        batch: 2,
        cap: 36,
        health_scale: 1.0,
    },
    WaveDefinition {
        interval: 1.5,
        batch: 2,
        cap: 48,
        health_scale: 1.1,
    },
    WaveDefinition {
        interval: 1.3,
        batch: 3,
        cap: 60,
        health_scale: 1.2,
    },
    WaveDefinition {
        interval: 1.1,
        batch: 3,
        cap: 72,
        health_scale: 1.3,
    },
    WaveDefinition {
        interval: 1.0,
        batch: 4,
        cap: 84,
        health_scale: 1.4,
    },
    WaveDefinition {
        interval: 0.9,
        batch: 4,
        cap: 96,
        health_scale: 1.5,
    },
    WaveDefinition {
        interval: 0.8,
        batch: 5,
        cap: 108,
        health_scale: 1.6,
    },
    WaveDefinition {
        interval: 0.7,
        batch: 5,
        cap: 120,
        health_scale: 1.7,
    },
    WaveDefinition {
        interval: 0.6,
        batch: 6,
        cap: 132,
        health_scale: 1.8,
    },
];

#[derive(Resource, Default)]
pub struct WaveDirector {
    pub spawned: usize,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyKind {
    Acolyte,
    Runner,
    Brute,
}

pub fn wave_index(elapsed: f32) -> usize {
    ((elapsed.max(0.0) / 60.0) as usize).min(WAVES.len() - 1)
}

pub struct WavesPlugin;
impl Plugin for WavesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WaveDirector>().add_systems(
            Update,
            spawn_wave
                .in_set(GameplaySets::Spawning)
                .run_if(in_state(GameState::Playing)),
        );
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Bevy injects independent resources and queries for the wave director."
)]
fn spawn_wave(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    stats: Res<GameStats>,
    bounds: Res<ArenaBounds>,
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
    if stats.time_elapsed >= SURVIVAL_SECONDS || !player.single().is_ok_and(|h| h.current > 0) {
        return;
    }
    let index = wave_index(stats.time_elapsed);
    let wave = WAVES[index];
    config.current_wave = index as u32 + 1;
    config.max_enemies = wave.cap as u32;
    #[cfg(feature = "dev-tools")]
    if !debug.waves {
        return;
    }
    timer
        .0
        .set_duration(std::time::Duration::from_secs_f32(wave.interval));
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }
    let slots = wave
        .cap
        .saturating_sub(enemies.iter().count())
        .min(wave.batch);
    let Ok((camera, projection)) = camera.single() else {
        return;
    };
    let Ok(context) = context.single() else {
        return;
    };
    let center = camera.translation.truncate();
    let view = camera_half_view(projection) + Vec2::splat(70.0);
    for _ in 0..slots {
        let kind = if index >= 3 && director.spawned.is_multiple_of(7) {
            EnemyKind::Brute
        } else if index >= 1 && director.spawned.is_multiple_of(4) {
            EnemyKind::Runner
        } else {
            EnemyKind::Acolyte
        };
        let (health, speed, size, tint) = match kind {
            EnemyKind::Acolyte => (20.0, 100.0, 48.0, Color::WHITE),
            EnemyKind::Runner => (14.0, 135.0, 42.0, Color::srgb(1.0, 0.75, 0.5)),
            EnemyKind::Brute => (55.0, 65.0, 60.0, Color::srgb(0.72, 0.65, 1.0)),
        };
        let collider = Collider::ball(14.0);
        let mut position = None;
        for _ in 0..24 {
            let along = rand::random::<f32>() * 2.0 - 1.0;
            let offset = match rand::random::<u32>() % 4 {
                0 => Vec2::new(-view.x, along * view.y),
                1 => Vec2::new(view.x, along * view.y),
                2 => Vec2::new(along * view.x, -view.y),
                _ => Vec2::new(along * view.x, view.y),
            };
            let candidate = center + offset;
            if candidate
                .abs()
                .cmpgt(bounds.half_size - Vec2::splat(20.0))
                .any()
            {
                continue;
            }
            let filter = QueryFilter::default()
                .exclude_sensors()
                .groups(CollisionGroups::new(
                    Group::GROUP_2,
                    WORLD_GROUP | Group::GROUP_2,
                ));
            let mut occupied = false;
            context.intersect_shape(candidate, 0.0, collider.raw.as_ref(), filter, |_| {
                occupied = true;
                false
            });
            if !occupied {
                position = Some(candidate);
                break;
            }
        }
        let Some(position) = position else {
            continue;
        };
        let health = (health * wave.health_scale).round() as i32;
        commands.spawn((
            RunEntity,
            kind,
            Name::new(format!("{kind:?}")),
            Enemy {
                speed,
                experience_value: if kind == EnemyKind::Brute { 20 } else { 10 },
            },
            Health {
                current: health,
                maximum: health,
            },
            Sprite {
                image: textures.enemies.clone(),
                custom_size: Some(Vec2::splat(size)),
                color: tint,
                texture_atlas: Some(TextureAtlas {
                    layout: textures.enemies_layout.clone(),
                    index: 1,
                }),
                ..default()
            },
            Transform::from_translation(position.extend(0.0)),
        ));
        director.spawned += 1;
    }
}
