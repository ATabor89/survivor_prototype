use crate::components::{Enemy, Health};
use crate::content::{ActiveStage, GameContent};
use crate::resources::{GameState, GameStats, SpawnTimer};
use crate::tests::{count, game, key, player, state, step};
use crate::waves::{
    phase_index, Boss, EncounterTelegraph, RingMember, TriggerEncounter, WaveDirector,
};
use bevy::prelude::*;

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
    assert!(phases[after].interval < phases[before].interval);
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
