//! What a finished run brings home, and where it goes.
//!
//! A run that reaches a result screen produces one [`ExpeditionReport`]. Runs
//! launched from the estate bank their haul into the [`Profile`]; sandbox runs
//! from the setup screen show the same report but leave saved progress alone.
use crate::profile::{Profile, Stockpile};
use crate::resources::{GameState, GameStats};
use bevy::prelude::*;

/// Conversion rates from run totals to resources. The estate's prices in
/// `assets/data/estate.ron` are set against these; see `design/ESTATE.md`.
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

/// Extra haul the estate grants its own expeditions, as a percentage of each
/// resource. Set when an estate expedition departs; sandbox runs ignore it.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HaulBonus {
    pub percent: Stockpile,
}

impl HaulBonus {
    /// The extra amount `haul` earns, rounded down per resource.
    pub fn extra(&self, haul: &Stockpile) -> Stockpile {
        let part = |amount: u32, percent: u32| {
            u32::try_from(u64::from(amount) * u64::from(percent) / 100).unwrap_or(u32::MAX)
        };
        Stockpile {
            gold: part(haul.gold, self.percent.gold),
            essence: part(haul.essence, self.percent.essence),
            seals: part(haul.seals, self.percent.seals),
        }
    }
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
    /// Everything brought home, including `bonus`.
    pub haul: Stockpile,
    /// The part of `haul` granted by the estate's haul bonuses.
    pub bonus: Stockpile,
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
            .init_resource::<HaulBonus>()
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

/// The result-screen system for `outcome`. It touches several resources once
/// per run, so it is a small exclusive system.
fn conclude(outcome: Outcome) -> impl FnMut(&mut World) {
    move |world| conclude_run(world, outcome)
}

fn conclude_run(world: &mut World, outcome: Outcome) {
    // A result screen can be re-entered (the debug overlay pauses over it);
    // a run is only ever reported and banked once.
    if world.resource::<LastExpedition>().0.is_some() {
        return;
    }
    let stats = world.resource::<GameStats>();
    let banked = *world.resource::<RunOrigin>() == RunOrigin::Estate;
    let mut haul = ExpeditionReport::haul_for(stats, outcome);
    let bonus = if banked {
        world.resource::<HaulBonus>().extra(&haul)
    } else {
        Stockpile::default()
    };
    haul.add(&bonus);
    let report = ExpeditionReport {
        outcome,
        seconds: stats.time_elapsed,
        kills: stats.enemies_killed,
        bosses_defeated: stats.bosses_defeated,
        haul,
        bonus,
        banked,
    };
    if banked {
        let mut profile = world.resource_mut::<Profile>();
        profile.stockpile.add(&haul);
        profile.expeditions = profile.expeditions.saturating_add(1);
        if outcome == Outcome::Victory {
            profile.victories = profile.victories.saturating_add(1);
        }
    }
    world.resource_mut::<LastExpedition>().0 = Some(report);
}

fn clear_report(mut last: ResMut<LastExpedition>) {
    last.0 = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn haul_bonuses_round_down_per_resource() {
        let bonus = HaulBonus {
            percent: Stockpile {
                gold: 0,
                essence: 25,
                seals: 50,
            },
        };
        let haul = Stockpile {
            gold: 40,
            essence: 81,
            seals: 3,
        };
        assert_eq!(
            bonus.extra(&haul),
            Stockpile {
                gold: 0,
                essence: 20,
                seals: 1
            }
        );
        assert_eq!(HaulBonus::default().extra(&haul), Stockpile::default());
    }
}
