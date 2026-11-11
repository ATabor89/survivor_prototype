//! Run lifetime and terminal-state arbitration, independent of app lifetime.
use crate::{
    components::{Enemy, Health, Player},
    resources::{GameState, GameStats, SpawnTimer, WaveConfig},
    GameplaySets,
};
use bevy::prelude::*;

#[derive(Component)]
pub struct RunEntity;

pub struct RunPlugin;
impl Plugin for RunPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, record_final_boss_death.after(GameplaySets::Cleanup))
            .add_systems(
                Update,
                advance_run
                    .in_set(GameplaySets::Progress)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                OnEnter(GameState::Restarting),
                (
                    reset_run,
                    crate::systems::spawn_player,
                    crate::setup::apply_starting_loadout,
                    resume_run,
                )
                    .chain(),
            );
        app.add_systems(OnEnter(GameState::MainMenu), reset_run)
            .add_systems(OnEnter(GameState::Estate), reset_run);
    }
}

fn advance_run(
    time: Res<Time<Virtual>>,
    mut stats: ResMut<GameStats>,
    player: Query<&Health, With<Player>>,
    mut next: ResMut<NextState<GameState>>,
    content: Res<crate::content::GameContent>,
    active: Res<crate::content::ActiveStage>,
    director: Res<crate::waves::WaveDirector>,
) {
    use crate::{content::StageWinCondition, waves::FinaleProgress};
    // Combat has already run. A lethal frame always wins over victory or XP.
    if player.single().is_ok_and(|health| health.current > 0) {
        let stage = content.stage(&active);
        stats.time_elapsed += time.delta_secs();
        let won = match stage.win_condition {
            StageWinCondition::TimedSurvival => {
                stats.time_elapsed = stats.time_elapsed.min(stage.duration);
                stats.time_elapsed >= stage.duration
            }
            StageWinCondition::DefeatBoss { .. } => director.finale == FinaleProgress::Defeated,
        };
        if won {
            next.set(GameState::Victory);
        }
    }
}

// Read death messages even if a kill opens a level-up/treasure screen. Messages
// expire after two updates, whereas the run may remain paused indefinitely.
fn record_final_boss_death(
    mut director: ResMut<crate::waves::WaveDirector>,
    mut deaths: MessageReader<crate::events::EntityDeathEvent>,
) {
    for death in deaths.read() {
        if director.finale == crate::waves::FinaleProgress::Active(death.entity) {
            director.finale = crate::waves::FinaleProgress::Defeated;
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
    *world.resource_mut::<crate::telemetry::RunTelemetry>() =
        crate::telemetry::RunTelemetry::default();
    let stage_id = world
        .resource::<crate::content::GameContent>()
        .stage(world.resource::<crate::content::ActiveStage>())
        .id
        .clone();
    world
        .resource_mut::<crate::setup::ActiveRunConfig>()
        .0
        .stage = stage_id;
    let seed = world.resource::<crate::setup::ActiveRunConfig>().0.seed;
    *world.resource_mut::<crate::setup::RunRng>() = crate::setup::RunRng::new(seed);
    *world.resource_mut::<SpawnTimer>() = SpawnTimer::default();
    *world.resource_mut::<WaveConfig>() = WaveConfig::default();
    *world.resource_mut::<crate::waves::WaveDirector>() = crate::waves::WaveDirector::default();
    world
        .resource_mut::<Messages<crate::waves::TriggerEncounter>>()
        .clear();
    world
        .resource_mut::<Messages<crate::waves::TriggerFinale>>()
        .clear();
    crate::loot::reset_loot(world);
    world.remove_resource::<crate::menu::PendingAddCircleUpgrade>();
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
        .resource_mut::<Messages<crate::combat::DamageApplied>>()
        .clear();
    world
        .resource_mut::<Messages<crate::events::EntityDeathEvent>>()
        .clear();
    world.resource_mut::<Messages<BindingEvent>>().clear();
    world.resource_mut::<Messages<AddWeaponEvent>>().clear();
    world
        .resource_mut::<Messages<crate::inventory::PassiveUpgradeEvent>>()
        .clear();
    world
        .resource_mut::<Messages<crate::menu::WeaponUpgradeConfirmedEvent>>()
        .clear();
    world
        .resource_mut::<Messages<crate::menu::GenericUpgradeConfirmedEvent>>()
        .clear();
    #[cfg(feature = "dev-tools")]
    {
        crate::dev_tools::reset_debug_run(world);
    }
}

fn resume_run(
    mut next: ResMut<NextState<GameState>>,
    #[cfg(feature = "dev-tools")] debug: Res<crate::dev_tools::DebugSession>,
) {
    #[cfg(feature = "dev-tools")]
    if debug.open && debug.pause_simulation {
        next.set(GameState::DebugTools);
        return;
    }
    next.set(GameState::Playing);
}
