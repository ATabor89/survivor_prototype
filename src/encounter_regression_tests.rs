use crate::components::{Enemy, Health};
use crate::content::{ActiveStage, GameContent};
use crate::resources::{GameState, GameStats, SpawnTimer};
use crate::tests::{count, game, key, player, state, step};
use crate::waves::{
    phase_index, Boss, EncounterTelegraph, RingMember, TriggerEncounter, WaveDirector,
};
use bevy::prelude::*;

#[test]
fn boss_health_bar_tracks_remaining_health_and_stays_left_anchored_while_paused() {
    let mut app = game();
    app.world_mut().write_message(TriggerEncounter(0));
    step(&mut app, 130);
    let boss = app
        .world_mut()
        .query_filtered::<Entity, With<Boss>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Paused);
    step(&mut app, 2);
    let maximum = app.world().get::<Health>(boss).unwrap().maximum;
    app.world_mut().get_mut::<Health>(boss).unwrap().current = maximum / 2;
    step(&mut app, 1);
    let (fill, sprite, transform) = app
        .world_mut()
        .query::<(&crate::waves::BossHealthFill, &Sprite, &Transform)>()
        .single(app.world())
        .unwrap();
    let expected = (maximum / 2) as f32 / maximum as f32;
    assert!((sprite.custom_size.unwrap().x / fill.full_width - expected).abs() < 0.001);
    assert!(
        (transform.translation.x - sprite.custom_size.unwrap().x / 2.0 + fill.full_width / 2.0)
            .abs()
            < 0.001
    );
    app.world_mut().get_mut::<Health>(boss).unwrap().current = -10;
    step(&mut app, 1);
    let sprite = app
        .world_mut()
        .query_filtered::<&Sprite, With<crate::waves::BossHealthFill>>()
        .single(app.world())
        .unwrap();
    assert_eq!(sprite.custom_size.unwrap().x, 0.0);
}

#[test]
fn authored_half_minute_phases_change_rates_without_minute_assumptions() {
    let mut app = game();
    let stage = app
        .world()
        .resource::<GameContent>()
        .stage(app.world().resource::<ActiveStage>());
    let phases = &stage.schedule;
    let before = phase_index(phases, 149.99);
    let after = phase_index(phases, 150.0);
    assert_ne!(before, after);
    // Authored transitions may increase pressure or deliberately offer recovery.
    assert_ne!(phases[after].interval, phases[before].interval);
    let interval = phases[after].interval;
    app.world_mut().resource_mut::<GameStats>().time_elapsed = 150.0;
    step(&mut app, 1);
    assert!(
        (app.world()
            .resource::<SpawnTimer>()
            .0
            .duration()
            .as_secs_f32()
            - interval)
            .abs()
            < 0.001
    );
}

#[test]
fn authored_boss_ring_is_telegraphed_once_pauses_and_expires_on_boss_death() {
    let mut app = game();
    let player = player(&mut app);
    app.world_mut().get_mut::<Health>(player).unwrap().current = 10000;
    let stage = app
        .world()
        .resource::<GameContent>()
        .stage(app.world().resource::<ActiveStage>());
    let index = stage
        .encounters
        .iter()
        .position(|event| event.ring.is_some())
        .unwrap();
    app.world_mut().write_message(TriggerEncounter(index));
    step(&mut app, 2);
    assert_eq!(count::<EncounterTelegraph>(&mut app), 1);
    assert_eq!(count::<Boss>(&mut app), 0);
    key(&mut app, KeyCode::Escape);
    step(&mut app, 2);
    assert_eq!(state(&app), GameState::Paused);
    step(&mut app, 180);
    assert_eq!(count::<EncounterTelegraph>(&mut app), 1);
    assert_eq!(count::<Boss>(&mut app), 0);
    key(&mut app, KeyCode::Escape);
    step(&mut app, 130);
    assert_eq!(count::<Boss>(&mut app), 1);
    assert!(count::<RingMember>(&mut app) >= 3);
    assert!(app
        .world()
        .resource::<WaveDirector>()
        .fired_encounters
        .contains(&index));
    let boss = app
        .world_mut()
        .query_filtered::<Entity, With<Boss>>()
        .single(app.world())
        .unwrap();
    app.world_mut().write_message(crate::combat::DamageEvent {
        target: boss,
        amount: 10000,
        source: None,
    });
    step(&mut app, 3);
    assert_eq!(count::<Boss>(&mut app), 0);
    assert_eq!(count::<RingMember>(&mut app), 0);
    assert_eq!(count::<EncounterTelegraph>(&mut app), 0);
}

#[test]
fn stage_duration_controls_victory_and_restart_clears_timeline_requests() {
    let mut app = game();
    app.world_mut().resource_mut::<GameContent>().stages[0].duration = 3.0;
    app.world_mut().resource_mut::<GameContent>().stages[0].win_condition =
        crate::content::StageWinCondition::TimedSurvival;
    app.world_mut().resource_mut::<GameStats>().time_elapsed = 2.99;
    step(&mut app, 3);
    assert_eq!(state(&app), GameState::Victory);
    assert_eq!(app.world().resource::<GameStats>().time_elapsed, 3.0);
    app.world_mut().write_message(TriggerEncounter(0));
    key(&mut app, KeyCode::Enter);
    step(&mut app, 5);
    assert_eq!(state(&app), GameState::Playing);
    assert_eq!(count::<Enemy>(&mut app), 0);
    assert!(app
        .world()
        .resource::<WaveDirector>()
        .fired_encounters
        .is_empty());
    assert_eq!(count::<EncounterTelegraph>(&mut app), 0);
}

fn finale_game() -> App {
    let mut app = game();
    let mut content = app.world_mut().resource_mut::<GameContent>();
    content.stages[0].duration = 3.0;
    content.stages[0].encounters.clear();
    content.stages[0].win_condition = crate::content::StageWinCondition::DefeatBoss {
        enemy: "warden".into(),
    };
    app.world_mut().resource_mut::<GameStats>().time_elapsed = 3.0;
    app
}

#[test]
fn final_boss_waits_through_pause_and_only_actual_death_wins_then_restart_resets() {
    use crate::waves::{FinalBoss, FinaleProgress};
    let mut app = finale_game();
    step(&mut app, 2);
    assert_eq!(state(&app), GameState::Playing);
    assert_eq!(
        app.world().resource::<WaveDirector>().finale,
        FinaleProgress::Telegraphing
    );
    key(&mut app, KeyCode::Escape);
    step(&mut app, 3);
    let paused = app.world().resource::<GameStats>().time_elapsed;
    step(&mut app, 150);
    assert_eq!(count::<FinalBoss>(&mut app), 0);
    assert_eq!(app.world().resource::<GameStats>().time_elapsed, paused);
    key(&mut app, KeyCode::Escape);
    step(&mut app, 135);
    assert!(app.world().resource::<GameStats>().time_elapsed > 3.0);
    let boss = app
        .world_mut()
        .query_filtered::<Entity, With<FinalBoss>>()
        .single(app.world())
        .unwrap();
    app.world_mut().entity_mut(boss).despawn();
    step(&mut app, 3);
    assert_eq!(state(&app), GameState::Playing);
    assert_eq!(
        app.world().resource::<WaveDirector>().finale,
        FinaleProgress::Telegraphing
    );
    step(&mut app, 135);
    let boss = app
        .world_mut()
        .query_filtered::<Entity, With<FinalBoss>>()
        .single(app.world())
        .unwrap();
    app.world_mut().write_message(crate::combat::DamageEvent {
        target: boss,
        amount: 10000,
        source: None,
    });
    step(&mut app, 4);
    assert_eq!(state(&app), GameState::Victory);
    assert_eq!(
        app.world().resource::<WaveDirector>().finale,
        FinaleProgress::Defeated
    );
    key(&mut app, KeyCode::Enter);
    step(&mut app, 4);
    assert_eq!(
        app.world().resource::<WaveDirector>().finale,
        FinaleProgress::Awaiting
    );
    assert_eq!(count::<FinalBoss>(&mut app), 0);
    assert_eq!(count::<EncounterTelegraph>(&mut app), 0);
}

#[test]
fn finale_retries_obstructed_spawn_and_never_turns_missing_boss_into_victory() {
    use bevy_rapier2d::prelude::*;
    let mut app = finale_game();
    let obstruction = app
        .world_mut()
        .spawn((
            RigidBody::Fixed,
            Collider::cuboid(1500.0, 1100.0),
            CollisionGroups::new(crate::arena::WORLD_GROUP, Group::ALL),
            Transform::default(),
        ))
        .id();
    step(&mut app, 170);
    assert_eq!(count::<crate::waves::FinalBoss>(&mut app), 0);
    assert_eq!(count::<EncounterTelegraph>(&mut app), 1);
    assert_eq!(state(&app), GameState::Playing);
    app.world_mut().entity_mut(obstruction).despawn();
    step(&mut app, 155);
    assert_eq!(count::<crate::waves::FinalBoss>(&mut app), 1);
    assert_eq!(count::<EncounterTelegraph>(&mut app), 0);
}

#[test]
fn simultaneous_final_boss_and_player_death_is_a_loss() {
    let mut app = finale_game();
    step(&mut app, 135);
    let boss = app
        .world_mut()
        .query_filtered::<Entity, With<crate::waves::FinalBoss>>()
        .single(app.world())
        .unwrap();
    let player = player(&mut app);
    app.world_mut()
        .get_mut::<crate::combat::DamageCooldown>(player)
        .unwrap()
        .time = -1.0;
    for target in [boss, player] {
        app.world_mut().write_message(crate::combat::DamageEvent {
            target,
            amount: 10000,
            source: None,
        });
    }
    step(&mut app, 4);
    assert_eq!(state(&app), GameState::GameOver);
}

#[test]
fn final_boss_death_is_retained_while_run_progress_is_paused() {
    let mut app = finale_game();
    step(&mut app, 135);
    let boss = app
        .world_mut()
        .query_filtered::<Entity, With<crate::waves::FinalBoss>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Paused);
    step(&mut app, 2);
    app.world_mut().entity_mut(boss).despawn();
    app.world_mut()
        .write_message(crate::events::EntityDeathEvent {
            entity: boss,
            position: Vec2::ZERO,
            exp_value: None,
        });
    step(&mut app, 6);
    assert_eq!(
        app.world().resource::<WaveDirector>().finale,
        crate::waves::FinaleProgress::Defeated
    );
    key(&mut app, KeyCode::Escape);
    step(&mut app, 4);
    assert_eq!(state(&app), GameState::Victory);
}

#[test]
fn authored_win_conditions_reject_unknown_bosses_and_default_to_timed_survival() {
    let mut content = GameContent::parse(crate::content::EMBEDDED_CONTENT).unwrap();
    assert!(content
        .stages
        .iter()
        .any(|stage| stage.win_condition == crate::content::StageWinCondition::TimedSurvival));
    content.stages[0].win_condition = crate::content::StageWinCondition::DefeatBoss {
        enemy: "misspelled_boss".into(),
    };
    assert!(content.validate().unwrap_err().contains("unknown boss"));
}

#[test]
fn enemies_spawn_inside_both_corridor_ends_and_avoid_the_player() {
    use crate::arena::{ArenaBounds, GameCamera};
    use bevy::camera::CameraProjection;
    use bevy::ecs::system::RunSystemOnce;
    use bevy_rapier2d::prelude::ReadRapierContext;
    for index in [1, 2] {
        for sign in [-1.0, 1.0] {
            let mut app = game();
            app.world_mut().resource_mut::<ActiveStage>().0 = index;
            app.world_mut()
                .resource_mut::<NextState<GameState>>()
                .set(GameState::Restarting);
            step(&mut app, 4);
            let player = player(&mut app);
            let bounds = app.world().resource::<ArenaBounds>().half_size;
            let axis = if index == 1 { 0 } else { 1 };
            app.world_mut()
                .get_mut::<Transform>(player)
                .unwrap()
                .translation[axis] = sign * (bounds[axis] - 35.0);
            for mut projection in app
                .world_mut()
                .query_filtered::<&mut Projection, With<GameCamera>>()
                .iter_mut(app.world_mut())
            {
                if let Projection::Orthographic(ortho) = &mut *projection {
                    ortho.update(1920.0, 1080.0);
                }
            }
            step(&mut app, 3);
            let position = app
                .world()
                .get::<Transform>(player)
                .unwrap()
                .translation
                .truncate();
            let blocked = app
                .world_mut()
                .run_system_once(move |context: ReadRapierContext| {
                    !crate::waves::position_is_free(
                        &context.single().unwrap(),
                        position,
                        14.0,
                        None,
                    )
                })
                .unwrap();
            assert!(
                blocked,
                "Rapier must reject spawning directly on the player"
            );
            app.world_mut()
                .resource_mut::<SpawnTimer>()
                .0
                .set_elapsed(std::time::Duration::from_secs(4));
            step(&mut app, 2);
            assert!(
                count::<Enemy>(&mut app) > 0,
                "no spawns for stage {index}, end {sign}"
            );
            for transform in app
                .world_mut()
                .query_filtered::<&Transform, With<Enemy>>()
                .iter(app.world())
            {
                assert!(transform.translation.truncate().abs().cmplt(bounds).all());
            }
        }
    }
}
