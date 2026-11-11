//! Play one run from start to finish without a window.
use super::pilot::{AutoplayPlugin, FlightLog};
use super::policy::{MovementStyle, UpgradePolicy};
use crate::expedition::{ExpeditionReport, Outcome};
use crate::headless;
use crate::profile::Stockpile;
use crate::resources::{GameState, GameStats};
use crate::setup::{self, RunConfiguration};
use crate::telemetry::RunTelemetry;
use crate::weapons::WeaponType;
use bevy::ecs::schedule::{Schedules, SingleThreadedExecutor};
use bevy::prelude::*;
use std::time::{Duration, Instant};

/// Updates a run may spend outside `Playing` before it counts as stuck. Menus
/// close within a few frames of the pilot's input, so this is generous.
const STALL_LIMIT: u32 = 600;

/// One run to play: what to bring, how to spend level-ups, and when to stop.
#[derive(Clone, Debug)]
pub struct Scenario {
    pub recipe: RunConfiguration,
    pub movement: MovementStyle,
    pub policy: UpgradePolicy,
    /// Simulated seconds after which a run still in progress is stopped.
    pub max_seconds: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ending {
    Victory,
    Defeat,
    /// Still alive when the scenario's time limit ran out.
    TimedOut,
}

impl Ending {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Victory => "victory",
            Self::Defeat => "defeat",
            Self::TimedOut => "timed_out",
        }
    }
}

/// What happened in one simulated run.
#[derive(Clone, Debug, PartialEq)]
pub struct RunResult {
    pub ending: Ending,
    /// Simulated seconds survived.
    pub seconds: f32,
    pub level: u32,
    /// Level at the end of each completed minute.
    pub level_by_minute: Vec<u32>,
    pub kills: u32,
    pub bosses: u32,
    pub damage_taken: u64,
    /// `None` if the player was never observed alive.
    pub lowest_health: Option<i32>,
    /// Health removed from enemies by each weapon, largest first.
    pub damage_by_weapon: Vec<(WeaponType, u64)>,
    /// Weapons owned at the end and their levels.
    pub build: Vec<(WeaponType, u32)>,
    /// What the run would bring back to the estate.
    pub haul: Stockpile,
}

/// Play `scenario` to its end. Returns an error for an invalid recipe or a run
/// that stops making progress.
///
/// The same scenario gives the same result on the same build of the game: the
/// run's random streams and the pilot's are both seeded from the recipe, and
/// systems run one at a time so their order never depends on thread timing.
pub fn simulate(scenario: &Scenario) -> Result<RunResult, String> {
    let mut app = headless::build();
    app.add_plugins(AutoplayPlugin {
        movement: scenario.movement,
        upgrades: scenario.policy,
        focus: scenario.recipe.weapons.iter().map(|w| w.weapon).collect(),
        seed: scenario.recipe.seed,
    });
    headless::finish(&mut app);
    for (_, schedule) in app.world_mut().resource_mut::<Schedules>().iter_mut() {
        schedule.set_executor(SingleThreadedExecutor::new());
    }
    // The headless game boots straight into a default run; replace it.
    app.update();
    setup::start_run(app.world_mut(), scenario.recipe.clone())?;

    let mut stalled = 0;
    let ending = loop {
        app.update();
        match app.world().resource::<State<GameState>>().get() {
            GameState::Victory => break Ending::Victory,
            GameState::GameOver => break Ending::Defeat,
            GameState::Playing => {
                stalled = 0;
                if app.world().resource::<GameStats>().time_elapsed >= scenario.max_seconds {
                    break Ending::TimedOut;
                }
            }
            other => {
                stalled += 1;
                if stalled > STALL_LIMIT {
                    return Err(format!(
                        "the run made no progress for {STALL_LIMIT} updates in {other:?}"
                    ));
                }
            }
        }
    };
    Ok(collect(app.world(), ending))
}

fn collect(world: &World, ending: Ending) -> RunResult {
    let stats = world.resource::<GameStats>();
    let telemetry = world.resource::<RunTelemetry>();
    let log = world.resource::<FlightLog>();
    let mut damage_by_weapon: Vec<_> = telemetry
        .damage_by_weapon
        .iter()
        .map(|(weapon, damage)| (*weapon, *damage))
        .collect();
    // Break ties by weapon so the order never depends on hash iteration.
    damage_by_weapon.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let outcome = match ending {
        Ending::Victory => Outcome::Victory,
        Ending::Defeat | Ending::TimedOut => Outcome::Defeat,
    };
    RunResult {
        ending,
        seconds: stats.time_elapsed,
        level: log.level,
        level_by_minute: log.level_by_minute.clone(),
        kills: stats.enemies_killed,
        bosses: stats.bosses_defeated,
        damage_taken: telemetry.damage_taken,
        lowest_health: log.lowest_health,
        damage_by_weapon,
        build: log.weapons.clone(),
        haul: ExpeditionReport::haul_for(stats, outcome),
    }
}

/// A finished scenario, with how long it took to simulate.
#[derive(Debug)]
pub struct Finished {
    pub scenario: Scenario,
    pub result: Result<RunResult, String>,
    pub wall: Duration,
}

/// Simulate every scenario on up to `jobs` threads, calling `progress` as each
/// one finishes. Results come back in the order the scenarios were given.
pub fn simulate_all(
    scenarios: Vec<Scenario>,
    jobs: usize,
    mut progress: impl FnMut(usize, &Finished),
) -> Vec<Finished> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;

    let next = AtomicUsize::new(0);
    let (sender, receiver) = mpsc::channel();
    let mut finished: Vec<Option<Finished>> = scenarios.iter().map(|_| None).collect();
    std::thread::scope(|scope| {
        for _ in 0..jobs.clamp(1, scenarios.len().max(1)) {
            let (sender, next, scenarios) = (sender.clone(), &next, &scenarios);
            scope.spawn(move || loop {
                let index = next.fetch_add(1, Ordering::Relaxed);
                let Some(scenario) = scenarios.get(index) else {
                    break;
                };
                let started = Instant::now();
                // A panic in one run should cost that run, not the batch.
                let result = std::panic::catch_unwind(|| simulate(scenario))
                    .unwrap_or_else(|_| Err("the run panicked; see the message above".into()));
                let outcome = Finished {
                    scenario: scenario.clone(),
                    result,
                    wall: started.elapsed(),
                };
                if sender.send((index, outcome)).is_err() {
                    break;
                }
            });
        }
        drop(sender);
        for (done, (index, outcome)) in receiver.iter().enumerate() {
            progress(done + 1, &outcome);
            finished[index] = Some(outcome);
        }
    });
    finished.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::WeaponLoadout;

    fn scenario(seconds: f32) -> Scenario {
        Scenario {
            recipe: RunConfiguration {
                seed: 7,
                ..default()
            },
            movement: MovementStyle::Evasive,
            policy: UpgradePolicy::Greedy,
            max_seconds: seconds,
        }
    }

    #[test]
    fn a_capped_run_stops_at_its_limit_and_reports_what_it_saw() {
        let result = simulate(&scenario(20.0)).unwrap();
        assert_eq!(result.ending, Ending::TimedOut);
        assert!((20.0..20.1).contains(&result.seconds), "{result:?}");
        assert!(result.level >= 1);
        assert_eq!(result.lowest_health.map(|health| health > 0), Some(true));
        assert_eq!(
            result.build.first().map(|w| w.0),
            Some(WeaponType::MagickCircle)
        );
        // Nothing is kept from the default run the headless game boots into.
        assert!(result.level_by_minute.is_empty());
    }

    #[test]
    fn the_same_scenario_plays_out_the_same_way_twice() {
        let first = simulate(&scenario(45.0)).unwrap();
        let second = simulate(&scenario(45.0)).unwrap();
        assert_eq!(first, second);
        assert!(first.kills > 0, "the pilot's weapon should land hits");
    }

    #[test]
    fn the_pilot_spends_level_ups_and_the_run_continues() {
        // An hour's limit would hang here if a level-up menu were never closed.
        let mut scenario = scenario(150.0);
        scenario.recipe.weapons.push(WeaponLoadout {
            weapon: WeaponType::InscribedNeedles,
            level: 3,
            patterns: vec![],
        });
        let result = simulate(&scenario).unwrap();
        assert!(result.level > 1, "{result:?}");
        assert_eq!(result.level_by_minute.len(), result.seconds as usize / 60);
        assert!(result.build.len() >= 2, "{result:?}");
    }

    #[test]
    fn a_pilot_that_stands_still_is_overrun() {
        let mut scenario = scenario(600.0);
        scenario.movement = MovementStyle::Still;
        scenario.policy = UpgradePolicy::Random;
        let result = simulate(&scenario).unwrap();
        assert_eq!(result.ending, Ending::Defeat, "{result:?}");
        assert!(result.damage_taken >= 100);
        assert_eq!(result.haul.essence, result.kills / 10);
    }

    #[test]
    fn an_invalid_recipe_is_an_error_and_batches_keep_their_order() {
        let mut bad = scenario(5.0);
        bad.recipe.stage = "missing".into();
        let mut seen = 0;
        let finished = simulate_all(vec![scenario(5.0), bad, scenario(6.0)], 2, |done, _| {
            seen = done;
        });
        assert_eq!(seen, 3);
        assert!(finished[0].result.is_ok());
        assert!(finished[1].result.is_err());
        assert_eq!(finished[2].scenario.max_seconds, 6.0);
    }
}
