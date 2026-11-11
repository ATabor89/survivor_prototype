use super::*;
use crate::expedition::{
    ExpeditionReport, LastExpedition, Outcome, RunOrigin, KILLS_PER_ESSENCE, SEALS_PER_BOSS,
    VICTORY_SEALS,
};
use crate::profile::{Profile, Stockpile};
use crate::tests::{game, key, player, state, step};

fn set_run_totals(app: &mut App, kills: u32, gold: u32, bosses: u32) {
    let mut stats = app.world_mut().resource_mut::<GameStats>();
    stats.enemies_killed = kills;
    stats.gold = gold;
    stats.bosses_defeated = bosses;
}

fn kill_player(app: &mut App) {
    let player = player(app);
    app.world_mut()
        .get_mut::<crate::combat::DamageCooldown>(player)
        .unwrap()
        .time = -1.0;
    app.world_mut().write_message(crate::combat::DamageEvent {
        target: player,
        amount: 10_000,
        source: None,
    });
    step(app, 3);
    assert_eq!(state(app), GameState::GameOver);
}

fn win(app: &mut App) {
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Victory);
    step(app, 2);
    assert_eq!(state(app), GameState::Victory);
}

fn report(app: &App) -> ExpeditionReport {
    app.world()
        .resource::<LastExpedition>()
        .0
        .clone()
        .expect("a finished run has a report")
}

fn summary_text(app: &mut App) -> String {
    app.world_mut()
        .query_filtered::<&Text, With<crate::menu::RunSummaryReadout>>()
        .single(app.world())
        .unwrap()
        .0
        .clone()
}

#[test]
fn haul_converts_run_totals_and_rewards_victory() {
    let stats = GameStats {
        enemies_killed: 10 * KILLS_PER_ESSENCE + KILLS_PER_ESSENCE - 1,
        gold: 7,
        bosses_defeated: 2,
        ..default()
    };
    let defeat = ExpeditionReport::haul_for(&stats, Outcome::Defeat);
    assert_eq!(
        defeat,
        Stockpile {
            gold: 7,
            essence: 10,
            seals: 2 * SEALS_PER_BOSS,
        }
    );
    let victory = ExpeditionReport::haul_for(&stats, Outcome::Victory);
    assert_eq!(victory.seals, defeat.seals + VICTORY_SEALS);
    assert_eq!(
        (victory.gold, victory.essence),
        (defeat.gold, defeat.essence)
    );
}

#[test]
fn sandbox_defeat_reports_a_haul_without_touching_the_profile() {
    let mut app = game();
    set_run_totals(&mut app, 45, 6, 0);
    kill_player(&mut app);

    let report = report(&app);
    assert_eq!(report.outcome, Outcome::Defeat);
    assert!(!report.banked);
    assert_eq!(
        report.haul,
        Stockpile {
            gold: 6,
            essence: 4,
            seals: 0
        }
    );
    assert_eq!(*app.world().resource::<Profile>(), Profile::default());
    let summary = summary_text(&mut app);
    assert!(summary.contains("Haul: Gold 6, Essence 4"), "{summary}");
    assert!(summary.contains("not banked"), "{summary}");
}

#[test]
fn estate_victory_banks_the_haul_once_and_reports_it() {
    let mut app = game();
    *app.world_mut().resource_mut::<RunOrigin>() = RunOrigin::Estate;
    set_run_totals(&mut app, 120, 15, 1);
    win(&mut app);

    let expected = Stockpile {
        gold: 15,
        essence: 12,
        seals: SEALS_PER_BOSS + VICTORY_SEALS,
    };
    let first = report(&app);
    assert_eq!(first.outcome, Outcome::Victory);
    assert!(first.banked);
    assert_eq!(first.haul, expected);
    assert!(summary_text(&mut app).contains("banked to the estate"));

    // Re-entering the result screen must not report or bank the run again.
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Paused);
    step(&mut app, 2);
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Victory);
    step(&mut app, 2);

    let profile = app.world().resource::<Profile>();
    assert_eq!(profile.stockpile, expected);
    assert_eq!((profile.expeditions, profile.victories), (1, 1));
}

#[test]
fn each_estate_run_banks_separately_and_restart_clears_the_report() {
    let mut app = game();
    *app.world_mut().resource_mut::<RunOrigin>() = RunOrigin::Estate;
    set_run_totals(&mut app, 30, 2, 0);
    kill_player(&mut app);
    assert_eq!(app.world().resource::<Profile>().stockpile.essence, 3);

    key(&mut app, KeyCode::ArrowDown); // Return to estate -> Depart again
    key(&mut app, KeyCode::Enter);
    step(&mut app, 4);
    assert_eq!(state(&app), GameState::Playing);
    assert!(app.world().resource::<LastExpedition>().0.is_none());
    assert_eq!(app.world().resource::<GameStats>().bosses_defeated, 0);

    set_run_totals(&mut app, 20, 1, 0);
    kill_player(&mut app);
    let profile = app.world().resource::<Profile>();
    assert_eq!(
        profile.stockpile,
        Stockpile {
            gold: 3,
            essence: 5,
            seals: 0
        }
    );
    assert_eq!((profile.expeditions, profile.victories), (2, 0));
}

#[test]
fn only_boss_deaths_count_as_bosses_defeated() {
    use crate::components::{Enemy, Health};
    use crate::death::MarkedForDeath;
    let mut app = game();
    let enemy = |app: &mut App| {
        app.world_mut()
            .spawn((
                Enemy {
                    speed: 0.0,
                    experience_value: 0,
                },
                Health {
                    current: 0,
                    maximum: 1,
                },
                Transform::from_xyz(900.0, 900.0, 0.0),
                MarkedForDeath,
            ))
            .id()
    };
    enemy(&mut app);
    let boss = enemy(&mut app);
    app.world_mut().entity_mut(boss).insert(crate::waves::Boss);
    step(&mut app, 2);

    let stats = app.world().resource::<GameStats>();
    assert_eq!((stats.enemies_killed, stats.bosses_defeated), (2, 1));
}
