//! What a finished run brings home, and where it goes.
//!
//! A run that reaches a result screen produces one [`ExpeditionReport`]. Runs
//! launched from the estate bank their haul into the [`Profile`]; sandbox runs
//! from the setup screen show the same report but leave saved progress alone.
use crate::profile::{Profile, Stockpile};
use crate::resources::{GameState, GameStats};
use bevy::prelude::*;

/// Provisional conversion rates. Tune these once the estate has real prices.
pub const KILLS_PER_ESSENCE: u32 = 10;
pub const SEALS_PER_BOSS: u32 = 1;
pub const VICTORY_SEALS: u32 = 2;

/// Who launched the current run, which decides whether its haul is banked.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RunOrigin {
    /// Started from the setup screen with a free choice of build.
    #[default]
    Sandbox,
    /// Started from the estate; the haul returns there.
    Estate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Victory,
    Defeat,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExpeditionReport {
    pub outcome: Outcome,
    pub seconds: f32,
    pub kills: u32,
    pub bosses_defeated: u32,
    pub haul: Stockpile,
    /// Whether the haul was added to the saved profile.
    pub banked: bool,
}

impl ExpeditionReport {
    /// Everything collected is kept on defeat; victory adds a seal bonus.
    pub fn haul_for(stats: &GameStats, outcome: Outcome) -> Stockpile {
        let victory_seals = match outcome {
            Outcome::Victory => VICTORY_SEALS,
            Outcome::Defeat => 0,
        };
        Stockpile {
            gold: stats.gold,
            essence: stats.enemies_killed / KILLS_PER_ESSENCE,
            seals: stats
                .bosses_defeated
                .saturating_mul(SEALS_PER_BOSS)
                .saturating_add(victory_seals),
        }
    }
}

/// The report for the run that just ended; cleared when the next one starts.
#[derive(Resource, Default)]
pub struct LastExpedition(pub Option<ExpeditionReport>);

/// Result-screen systems that display the report run after this set.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConcludeExpedition;

pub struct ExpeditionPlugin;

impl Plugin for ExpeditionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RunOrigin>()
            .init_resource::<LastExpedition>()
            .add_systems(
                OnEnter(GameState::Victory),
                conclude(Outcome::Victory).in_set(ConcludeExpedition),
            )
            .add_systems(
                OnEnter(GameState::GameOver),
                conclude(Outcome::Defeat).in_set(ConcludeExpedition),
            )
            .add_systems(OnEnter(GameState::Restarting), clear_report)
            .add_systems(OnEnter(GameState::MainMenu), clear_report);
    }
}

fn conclude(
    outcome: Outcome,
) -> impl FnMut(Res<GameStats>, Res<RunOrigin>, ResMut<LastExpedition>, ResMut<Profile>) {
    move |stats, origin, mut last, mut profile| {
        // A result screen can be re-entered (the debug overlay pauses over it);
        // a run is only ever reported and banked once.
        if last.0.is_some() {
            return;
        }
        let haul = ExpeditionReport::haul_for(&stats, outcome);
        let banked = *origin == RunOrigin::Estate;
        if banked {
            profile.stockpile.add(&haul);
            profile.expeditions = profile.expeditions.saturating_add(1);
            if outcome == Outcome::Victory {
                profile.victories = profile.victories.saturating_add(1);
            }
        }
        last.0 = Some(ExpeditionReport {
            outcome,
            seconds: stats.time_elapsed,
            kills: stats.enemies_killed,
            bosses_defeated: stats.bosses_defeated,
            haul,
            banked,
        });
    }
}

fn clear_report(mut last: ResMut<LastExpedition>) {
    last.0 = None;
}
