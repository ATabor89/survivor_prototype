use crate::combat::{DamageCooldown, DamageEvent};
use crate::components::{Enemy, Health};
use crate::events::EntityDeathEvent;
use crate::experience::{Experience, ExperienceOrb};
use crate::resources::{GameState, GameStats};
use crate::tests::{count, game, player, state, step};
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

#[test]
fn an_orb_spawned_on_the_player_stays_finite_and_collects_once() {
    let mut app = game();
    let player = player(&mut app);
    let position = app
        .world()
        .get::<Transform>(player)
        .unwrap()
        .translation
        .truncate();
    app.world_mut().write_message(EntityDeathEvent {
        entity: player,
        position,
        exp_value: Some(7),
    });
    // The newly spawned orb goes through vacuum and Rapier before the following
    // update can read its intersection. Zero distance must not poison physics.
    step(&mut app, 1);
    let mut orbs = app
        .world_mut()
        .query_filtered::<(&Transform, &Velocity), With<ExperienceOrb>>();
    let (transform, velocity) = orbs.single(app.world()).unwrap();
    assert!(transform.translation.is_finite());
    assert!(velocity.linear.is_finite());
    step(&mut app, 5);
    assert_eq!(count::<ExperienceOrb>(&mut app), 0);
    assert_eq!(app.world().get::<Experience>(player).unwrap().current, 7);
}

#[test]
fn an_orb_touching_before_pause_collects_after_its_started_message_expires() {
    let mut app = game();
    let player = player(&mut app);
    app.world_mut()
        .get_mut::<crate::components::Player>(player)
        .unwrap()
        .magnet_speed = 0.0;
    let position = app
        .world()
        .get::<Transform>(player)
        .unwrap()
        .translation
        .truncate();
    app.world_mut().write_message(EntityDeathEvent {
        entity: player,
        position: position + Vec2::X * 8.0,
        exp_value: Some(11),
    });
    // This update spawns the orb; PostUpdate physics establishes contact. Pause
    // before collection's next Update and wait out Bevy's message retention.
    step(&mut app, 1);
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Paused);
    step(&mut app, 6);
    assert_eq!(state(&app), GameState::Paused);
    assert_eq!(count::<ExperienceOrb>(&mut app), 1);
    assert_eq!(app.world().get::<Experience>(player).unwrap().current, 0);

    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Playing);
    step(&mut app, 5);
    assert_eq!(count::<ExperienceOrb>(&mut app), 0);
    assert_eq!(app.world().get::<Experience>(player).unwrap().current, 11);
}

#[test]
fn simultaneous_deaths_finish_scoring_and_game_over_beats_banked_level_up() {
    let mut app = game();
    let player = player(&mut app);
    app.world_mut().get_mut::<Health>(player).unwrap().current = 1;
    app.world_mut()
        .get_mut::<DamageCooldown>(player)
        .unwrap()
        .time = -1.0;
    app.world_mut()
        .get_mut::<Experience>(player)
        .unwrap()
        .current = 100;
    let enemy = app
        .world_mut()
        .spawn((
            Enemy {
                speed: 0.0,
                experience_value: 7,
            },
            Health {
                current: 10,
                maximum: 10,
            },
            Transform::from_xyz(250.0, 0.0, 0.0),
        ))
        .id();
    for (target, amount) in [(player, 1), (enemy, 10)] {
        app.world_mut().write_message(DamageEvent {
            target,
            amount,
            source: None,
        });
    }
    step(&mut app, 2);
    assert_eq!(state(&app), GameState::GameOver);
    assert_eq!(app.world().resource::<GameStats>().enemies_killed, 1);
    assert!(app.world().get_entity(enemy).is_err());
    assert!(app.world().get_entity(player).is_err());
    assert_eq!(count::<ExperienceOrb>(&mut app), 1);
}

#[test]
fn damage_cooldown_is_shared_across_sources_and_nonpositive_hits_do_not_consume_it() {
    let mut app = game();
    let player = player(&mut app);
    app.world_mut()
        .get_mut::<DamageCooldown>(player)
        .unwrap()
        .time = -1.0;
    for amount in [0, -10, 20, 30] {
        app.world_mut().write_message(DamageEvent {
            target: player,
            amount,
            source: None,
        });
    }
    step(&mut app, 1);
    assert_eq!(app.world().get::<Health>(player).unwrap().current, 80);
    step(&mut app, 5);
    app.world_mut().write_message(DamageEvent {
        target: player,
        amount: 20,
        source: None,
    });
    step(&mut app, 1);
    assert_eq!(app.world().get::<Health>(player).unwrap().current, 80);
    step(&mut app, 16);
    app.world_mut().write_message(DamageEvent {
        target: player,
        amount: 20,
        source: None,
    });
    step(&mut app, 1);
    assert_eq!(app.world().get::<Health>(player).unwrap().current, 60);
}
