use crate::components::{Enemy, Health};
use crate::dev_tools::{DebugAction, DebugSession, DebugTab};
use crate::resources::{GameState, GameStats};
use crate::tests::{count, game, key, player, state, step};
use bevy::prelude::*;

fn live_debug_menu(app: &mut App) {
    key(app, KeyCode::F1);
    step(app, 2);
    assert_eq!(state(app), GameState::DebugTools);
    {
        let mut debug = app.world_mut().resource_mut::<DebugSession>();
        debug.pause_simulation = false;
        debug.waves = false;
    }
    step(app, 3);
    assert_eq!(state(app), GameState::Playing);
    assert!(app.world().resource::<DebugSession>().open);
}

#[test]
fn debug_visibility_and_simulation_pause_are_independent() {
    let mut app = game();
    live_debug_menu(&mut app);
    assert_eq!(app.world().resource::<DebugSession>().tab, DebugTab::Run);
    let before = app.world().resource::<GameStats>().time_elapsed;
    step(&mut app, 4);
    assert!(app.world().resource::<GameStats>().time_elapsed > before);

    key(&mut app, KeyCode::F2);
    step(&mut app, 2);
    assert_eq!(
        app.world().resource::<DebugSession>().tab,
        DebugTab::Weapons
    );
    assert_eq!(state(&app), GameState::Playing);
    app.world_mut()
        .resource_mut::<DebugSession>()
        .pause_simulation = true;
    step(&mut app, 3);
    assert_eq!(state(&app), GameState::DebugTools);
    let paused = app.world().resource::<GameStats>().time_elapsed;
    step(&mut app, 4);
    assert_eq!(app.world().resource::<GameStats>().time_elapsed, paused);
    key(&mut app, KeyCode::F1);
    step(&mut app, 3);
    assert_eq!(state(&app), GameState::Playing);
    assert!(!app.world().resource::<DebugSession>().open);
}

#[test]
fn debug_shortcuts_close_without_opening_the_normal_pause_menu() {
    let mut app = game();
    key(&mut app, KeyCode::F2);
    step(&mut app, 2);
    assert_eq!(state(&app), GameState::DebugTools);
    assert_eq!(
        app.world().resource::<DebugSession>().tab,
        DebugTab::Weapons
    );
    key(&mut app, KeyCode::Escape);
    step(&mut app, 3);
    assert_eq!(state(&app), GameState::Playing);

    live_debug_menu(&mut app);
    key(&mut app, KeyCode::Escape);
    step(&mut app, 3);
    assert_eq!(state(&app), GameState::Playing);
    assert!(!app.world().resource::<DebugSession>().open);
    key(&mut app, KeyCode::Escape);
    step(&mut app, 2);
    assert_eq!(state(&app), GameState::Paused);
}

#[test]
fn live_debug_actions_work_and_keyboard_capture_blocks_wasd() {
    let mut app = game();
    let player = player(&mut app);
    live_debug_menu(&mut app);
    app.world_mut().get_mut::<Health>(player).unwrap().current = 40;
    {
        let mut debug = app.world_mut().resource_mut::<DebugSession>();
        debug.wants_keyboard_input = true;
        debug.actions = vec![
            DebugAction::Heal,
            DebugAction::SpawnTargets { moving: false },
        ];
    }
    // Headless tests inject egui's capture result into the same input resource.
    let before = app.world().get::<Transform>(player).unwrap().translation;
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyW);
    step(&mut app, 5);
    assert_eq!(
        app.world().get::<Transform>(player).unwrap().translation,
        before
    );
    assert_eq!(app.world().get::<Health>(player).unwrap().current, 100);
    assert_eq!(count::<Enemy>(&mut app), 8);
    app.world_mut()
        .resource_mut::<DebugSession>()
        .wants_keyboard_input = false;
    step(&mut app, 5);
    assert!(app.world().get::<Transform>(player).unwrap().translation.y > before.y);
}

#[test]
fn modal_game_screens_take_priority_over_live_debug_controls() {
    for modal in [
        GameState::LevelUp,
        GameState::Treasure,
        GameState::GameOver,
        GameState::Victory,
    ] {
        let mut app = game();
        live_debug_menu(&mut app);
        app.world_mut()
            .resource_mut::<DebugSession>()
            .wants_keyboard_input = true;
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(modal.clone());
        step(&mut app, 2);
        assert_eq!(state(&app), modal);
        let debug = app.world().resource::<DebugSession>();
        assert!(!debug.open);
        assert!(!debug.wants_keyboard_input);
        assert!(app.world().resource::<Time<Virtual>>().is_paused());
        key(&mut app, KeyCode::F1);
        step(&mut app, 2);
        assert_eq!(state(&app), modal);
        assert!(!app.world().resource::<DebugSession>().open);
    }
}

#[test]
fn phase_shortcut_uses_authored_times_and_last_wave_never_rewinds() {
    let mut app = game();
    key(&mut app, KeyCode::F1);
    step(&mut app, 2);
    let starts: Vec<_> = app
        .world()
        .resource::<crate::content::GameContent>()
        .stage(app.world().resource::<crate::content::ActiveStage>())
        .schedule
        .iter()
        .skip(1)
        .map(|phase| phase.start)
        .collect();
    for start in starts {
        app.world_mut()
            .resource_mut::<DebugSession>()
            .actions
            .push(DebugAction::AdvancePhase);
        step(&mut app, 2);
        assert_eq!(app.world().resource::<GameStats>().time_elapsed, start);
    }
    let last = app.world().resource::<GameStats>().time_elapsed;
    app.world_mut().resource_mut::<DebugSession>().actions =
        vec![DebugAction::AdvancePhase, DebugAction::AdvanceWave];
    step(&mut app, 2);
    assert_eq!(app.world().resource::<GameStats>().time_elapsed, last);
    assert_eq!(state(&app), GameState::DebugTools);
}

#[test]
fn stage_picker_restarts_and_victory_shortcut_uses_that_stages_duration() {
    let mut app = game();
    let old_player = player(&mut app);
    key(&mut app, KeyCode::F1);
    step(&mut app, 2);
    app.world_mut()
        .resource_mut::<DebugSession>()
        .actions
        .push(DebugAction::SelectStage(1));
    step(&mut app, 5);
    assert_eq!(state(&app), GameState::Playing);
    assert_eq!(app.world().resource::<crate::content::ActiveStage>().0, 1);
    assert_ne!(player(&mut app), old_player);
    assert!(app.world().resource::<GameStats>().time_elapsed < 1.0);
    assert!(!app.world().resource::<DebugSession>().open);
    let duration = app
        .world()
        .resource::<crate::content::GameContent>()
        .stage(app.world().resource::<crate::content::ActiveStage>())
        .duration;
    key(&mut app, KeyCode::F1);
    step(&mut app, 2);
    app.world_mut()
        .resource_mut::<DebugSession>()
        .actions
        .push(DebugAction::TestVictory);
    step(&mut app, 4);
    assert_eq!(state(&app), GameState::Victory);
    assert_eq!(app.world().resource::<GameStats>().time_elapsed, duration);
}

#[test]
fn encounter_requests_survive_pausing_until_simulation_resumes() {
    let mut app = game();
    key(&mut app, KeyCode::F1);
    step(&mut app, 2);
    {
        let mut debug = app.world_mut().resource_mut::<DebugSession>();
        debug.waves = false;
        debug.actions.push(DebugAction::TriggerEncounter(0));
    }
    // Several paused frames would expire a plain Bevy buffered message.
    step(&mut app, 10);
    assert_eq!(
        app.world().resource::<DebugSession>().queued_encounters,
        vec![0]
    );
    assert_eq!(count::<crate::waves::EncounterTelegraph>(&mut app), 0);
    key(&mut app, KeyCode::F1);
    step(&mut app, 3);
    assert_eq!(state(&app), GameState::Playing);
    assert!(app
        .world()
        .resource::<DebugSession>()
        .queued_encounters
        .is_empty());
    assert_eq!(count::<crate::waves::EncounterTelegraph>(&mut app), 1);
}
