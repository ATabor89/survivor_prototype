//! Run lifetime and terminal-state arbitration, independent of app lifetime.
use crate::{
    components::{Enemy, Health, Player},
    resources::{GameState, GameStats, SpawnTimer, WaveConfig},
    GameplaySets,
};
use bevy::prelude::*;

pub const SURVIVAL_SECONDS: f32 = 600.0;
#[derive(Component)]
pub struct RunEntity;

pub struct RunPlugin;
impl Plugin for RunPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            advance_run
                .in_set(GameplaySets::Progress)
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            OnEnter(GameState::Restarting),
            (reset_run, crate::systems::spawn_player, resume_run).chain(),
        );
    }
}

fn advance_run(
    time: Res<Time<Virtual>>,
    mut stats: ResMut<GameStats>,
    player: Query<&Health, With<Player>>,
    mut next: ResMut<NextState<GameState>>,
) {
    // Combat has already run. A lethal frame always wins over victory or XP.
    if player.single().is_ok_and(|health| health.current > 0) {
        stats.time_elapsed = (stats.time_elapsed + time.delta_secs()).min(SURVIVAL_SECONDS);
        if stats.time_elapsed >= SURVIVAL_SECONDS {
            next.set(GameState::Victory);
        }
    }
}

fn reset_run(world: &mut World) {
    use crate::experience::ExperienceOrb;
    use crate::weapons::{AddWeaponEvent, Attack, BindingEvent};
    let entities: Vec<_> = world
        .query_filtered::<Entity, Or<(
            With<RunEntity>,
            With<Player>,
            With<Enemy>,
            With<Attack>,
            With<ExperienceOrb>,
        )>>()
        .iter(world)
        .collect();
    for entity in entities {
        if let Ok(entity) = world.get_entity_mut(entity) {
            entity.despawn();
        }
    }
    *world.resource_mut::<GameStats>() = GameStats::default();
    *world.resource_mut::<SpawnTimer>() = SpawnTimer::default();
    *world.resource_mut::<WaveConfig>() = WaveConfig::default();
    *world.resource_mut::<crate::waves::WaveDirector>() = crate::waves::WaveDirector::default();
    world
        .resource_mut::<crate::weapons::overlap::EnemyCircleHits>()
        .hits
        .clear();
    world
        .resource_mut::<crate::weapons::overlap::ActiveOverlaps>()
        .overlaps
        .clear();
    world
        .resource_mut::<Messages<crate::combat::DamageEvent>>()
        .clear();
    world
        .resource_mut::<Messages<crate::events::EntityDeathEvent>>()
        .clear();
    world.resource_mut::<Messages<BindingEvent>>().clear();
    world.resource_mut::<Messages<AddWeaponEvent>>().clear();
    world
        .resource_mut::<Messages<crate::menu::WeaponUpgradeConfirmedEvent>>()
        .clear();
    world
        .resource_mut::<Messages<crate::menu::GenericUpgradeConfirmedEvent>>()
        .clear();
    #[cfg(feature = "dev-tools")]
    {
        *world.resource_mut::<crate::dev_tools::DebugSession>() =
            crate::dev_tools::DebugSession::default();
    }
}

fn resume_run(mut next: ResMut<NextState<GameState>>) {
    next.set(GameState::Playing);
}
