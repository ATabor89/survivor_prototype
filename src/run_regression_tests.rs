use super::*;
use crate::arena::{ArenaBounds, ArenaRoot, GameCamera};
use crate::components::{Enemy, Health, Player};
use crate::experience::{Experience, ExperienceOrb};
use crate::tests::{count, game, key, player, state, step};
use crate::weapons::{Attack, WeaponMeta};
use bevy_rapier2d::prelude::*;

#[test]
fn camera_world_view_stays_consistent_across_window_resolutions() {
    use bevy::camera::CameraProjection;
    let mut app = game();
    let mut projection = app
        .world_mut()
        .query_filtered::<&Projection, With<GameCamera>>()
        .single(app.world())
        .unwrap()
        .clone();
    let Projection::Orthographic(ref mut ortho) = projection else {
        panic!("expected 2D projection");
    };
    for (width, height) in [(1280.0, 720.0), (5120.0, 2880.0), (640.0, 360.0)] {
        ortho.update(width, height);
        assert!(ortho
            .area
            .size()
            .abs_diff_eq(Vec2::new(1280.0, 720.0), 0.01));
    }
}

#[test]
fn controller_movement_is_analog_capped_with_keyboard_and_pauses() {
    let mut app = game();
    let player = player(&mut app);
    let mut pad = Gamepad::default();
    pad.analog_mut().set(GamepadAxis::LeftStickX, 0.5);
    let pad = app.world_mut().spawn(pad).id();
    let start = app.world().get::<Transform>(player).unwrap().translation;
    step(&mut app, 12);
    let moved = app.world().get::<Transform>(player).unwrap().translation;
    assert!((moved.x - start.x - 15.0).abs() < 0.1);
    app.world_mut()
        .get_mut::<Gamepad>(pad)
        .unwrap()
        .analog_mut()
        .set(GamepadAxis::LeftStickX, 1.0);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyD);
    step(&mut app, 12);
    let capped = app.world().get::<Transform>(player).unwrap().translation;
    assert!((capped.x - moved.x - 30.0).abs() < 0.1);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release_all();
    app.world_mut()
        .get_mut::<Gamepad>(pad)
        .unwrap()
        .digital_mut()
        .press(GamepadButton::Start);
    step(&mut app, 1);
    app.world_mut()
        .get_mut::<Gamepad>(pad)
        .unwrap()
        .digital_mut()
        .clear();
    step(&mut app, 2);
    assert_eq!(state(&app), GameState::Paused);
    let paused = app.world().get::<Transform>(player).unwrap().translation;
    step(&mut app, 12);
    assert_eq!(
        app.world().get::<Transform>(player).unwrap().translation,
        paused
    );
}

#[cfg(feature = "dev-tools")]
#[test]
fn lab_wave_and_victory_shortcuts_use_normal_run_transitions() {
    use crate::dev_tools::{DebugAction, DebugSession};
    let mut app = game();
    key(&mut app, KeyCode::F1);
    step(&mut app, 2);
    app.world_mut()
        .resource_mut::<DebugSession>()
        .actions
        .push(DebugAction::AdvanceWave);
    step(&mut app, 1);
    assert_eq!(state(&app), GameState::DebugTools);
    assert_eq!(app.world().resource::<GameStats>().time_elapsed, 60.0);
    app.world_mut().resource_mut::<GameStats>().time_elapsed = 570.0;
    app.world_mut()
        .resource_mut::<DebugSession>()
        .actions
        .push(DebugAction::AdvanceWave);
    step(&mut app, 1);
    assert_eq!(app.world().resource::<GameStats>().time_elapsed, 570.0);
    app.world_mut()
        .resource_mut::<DebugSession>()
        .actions
        .push(DebugAction::TestVictory);
    step(&mut app, 4);
    assert_eq!(state(&app), GameState::Victory);
    key(&mut app, KeyCode::Enter);
    step(&mut app, 4);
    // The persistent debug workspace retains its pause preference on restart.
    assert_eq!(state(&app), GameState::DebugTools);
    assert!(app.world().resource::<DebugSession>().open);
    assert!(app.world().resource::<DebugSession>().waves);
    assert!(!app.world().resource::<DebugSession>().invulnerable);
}

#[test]
fn rapier_walls_block_player_and_allow_sliding_in_every_direction() {
    let mut app = game();
    let player = player(&mut app);
    let bounds = *app.world().resource::<ArenaBounds>();
    // Start clear of contact, move diagonally into each wall, then slide each way.
    for (position, into, tangent, axis) in [
        (
            Vec3::new(bounds.half_size.x - 20.0, 0.0, 0.0),
            KeyCode::KeyD,
            KeyCode::KeyW,
            1,
        ),
        (
            Vec3::new(bounds.half_size.x - 20.0, 0.0, 0.0),
            KeyCode::KeyD,
            KeyCode::KeyS,
            1,
        ),
        (
            Vec3::new(-bounds.half_size.x + 20.0, 0.0, 0.0),
            KeyCode::KeyA,
            KeyCode::KeyW,
            1,
        ),
        (
            Vec3::new(-bounds.half_size.x + 20.0, 0.0, 0.0),
            KeyCode::KeyA,
            KeyCode::KeyS,
            1,
        ),
        (
            Vec3::new(0.0, bounds.half_size.y - 20.0, 0.0),
            KeyCode::KeyW,
            KeyCode::KeyA,
            0,
        ),
        (
            Vec3::new(0.0, bounds.half_size.y - 20.0, 0.0),
            KeyCode::KeyW,
            KeyCode::KeyD,
            0,
        ),
        (
            Vec3::new(0.0, -bounds.half_size.y + 20.0, 0.0),
            KeyCode::KeyS,
            KeyCode::KeyA,
            0,
        ),
        (
            Vec3::new(0.0, -bounds.half_size.y + 20.0, 0.0),
            KeyCode::KeyS,
            KeyCode::KeyD,
            0,
        ),
    ] {
        app.world_mut()
            .get_mut::<Transform>(player)
            .unwrap()
            .translation = position;
        step(&mut app, 2);
        for _ in 0..24 {
            let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            input.press(into);
            input.press(tangent);
            step(&mut app, 1);
        }
        // clear() preserves held buttons, so release deliberately between cases.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release_all();
        let actual = app.world().get::<Transform>(player).unwrap().translation;
        assert!(
            actual.x.abs() <= bounds.half_size.x - 11.0,
            "crossed vertical wall: {actual}"
        );
        assert!(
            actual.y.abs() <= bounds.half_size.y - 11.0,
            "crossed horizontal wall: {actual}"
        );
        assert!(
            (actual[axis] - position[axis]).abs() > 20.0,
            "failed to slide: {into:?} {tangent:?} {actual}"
        );
    }
    assert!(app
        .world()
        .get::<KinematicCharacterControllerOutput>(player)
        .is_some());
}

#[test]
fn ten_minute_victory_pause_and_restart_reset_run_but_preserve_arena() {
    let mut app = game();
    app.world_mut()
        .resource_mut::<crate::content::GameContent>()
        .stages[0]
        .win_condition = crate::content::StageWinCondition::TimedSurvival;
    let original_player = player(&mut app);
    app.world_mut().resource_mut::<GameStats>().time_elapsed = 599.8;
    key(&mut app, KeyCode::Escape);
    step(&mut app, 2);
    let paused = app.world().resource::<GameStats>().time_elapsed;
    step(&mut app, 60);
    assert_eq!(app.world().resource::<GameStats>().time_elapsed, paused);
    key(&mut app, KeyCode::Escape);
    step(&mut app, 20);
    assert_eq!(state(&app), GameState::Victory);
    assert_eq!(app.world().resource::<GameStats>().time_elapsed, 600.0);
    key(&mut app, KeyCode::Enter);
    step(&mut app, 4);
    assert_eq!(state(&app), GameState::Playing);
    let fresh_player = player(&mut app);
    assert_ne!(fresh_player, original_player);
    assert_eq!(
        app.world().get::<Experience>(fresh_player).unwrap().level,
        1
    );
    assert_eq!(
        app.world().get::<Health>(fresh_player).unwrap().current,
        100
    );
    assert!(app.world().resource::<GameStats>().time_elapsed < 0.2);
    assert_eq!(app.world().resource::<GameStats>().enemies_killed, 0);
    assert_eq!(count::<ArenaRoot>(&mut app), 1);
    assert_eq!(count::<GameCamera>(&mut app), 1);
    assert_eq!(count::<WeaponMeta>(&mut app), 1);
    assert_eq!(count::<Enemy>(&mut app), 0);
    assert_eq!(count::<ExperienceOrb>(&mut app), 0);
    assert_eq!(count::<Attack>(&mut app), 0);
}

#[test]
fn lethal_damage_wins_over_victory_and_game_over_can_restart() {
    let mut app = game();
    let player = player(&mut app);
    app.world_mut().resource_mut::<GameStats>().time_elapsed = 599.999;
    app.world_mut()
        .get_mut::<combat::DamageCooldown>(player)
        .unwrap()
        .time = -1.0;
    app.world_mut().write_message(DamageEvent {
        target: player,
        amount: 1000,
        source: None,
    });
    step(&mut app, 2);
    assert_eq!(state(&app), GameState::GameOver);
    assert_eq!(count::<Player>(&mut app), 0);
    key(&mut app, KeyCode::Enter);
    step(&mut app, 4);
    assert_eq!(state(&app), GameState::Playing);
    assert_eq!(count::<Player>(&mut app), 1);
}

#[test]
fn waves_escalate_spawn_offscreen_inside_bounds_and_respect_caps() {
    let mut app = game();
    app.world_mut().resource_mut::<GameStats>().time_elapsed = 180.0;
    let content = app.world().resource::<crate::content::GameContent>();
    let stage = content.stage(app.world().resource::<crate::content::ActiveStage>());
    let phase = stage.schedule[waves::phase_index(&stage.schedule, 180.0)].clone();
    app.world_mut()
        .resource_mut::<SpawnTimer>()
        .0
        .set_elapsed(std::time::Duration::from_secs(3));
    step(&mut app, 2);
    assert_eq!(app.world().resource::<WaveConfig>().current_wave, 4);
    assert_eq!(
        count::<Enemy>(&mut app),
        phase.batch.max(phase.minimum).min(32)
    );
    let bounds = app.world().resource::<ArenaBounds>().half_size;
    let kinds: Vec<_> = app
        .world_mut()
        .query::<&waves::EnemyKind>()
        .iter(app.world())
        .cloned()
        .collect();
    assert!(kinds
        .iter()
        .all(|kind| phase.enemies.iter().any(|entry| entry.id == kind.0)));
    for position in app
        .world_mut()
        .query_filtered::<&Transform, With<Enemy>>()
        .iter(app.world())
    {
        let pos = position.translation.truncate();
        assert!(pos.abs().cmplt(bounds).all());
        assert!(
            pos.x.abs() > 640.0 || pos.y.abs() > 360.0,
            "spawned in view: {pos}"
        );
    }
    for _ in count::<Enemy>(&mut app)..phase.cap {
        app.world_mut().spawn((
            Enemy {
                speed: 0.0,
                experience_value: 0,
            },
            Health {
                current: 20,
                maximum: 20,
            },
            Transform::from_xyz(1400.0, 1000.0, 0.0),
        ));
    }
    app.world_mut()
        .resource_mut::<SpawnTimer>()
        .0
        .set_elapsed(std::time::Duration::from_secs(3));
    step(&mut app, 1);
    assert_eq!(count::<Enemy>(&mut app), phase.cap);
}
