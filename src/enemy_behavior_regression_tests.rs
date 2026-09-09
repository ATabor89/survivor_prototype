use crate::{
    components::{Enemy, Health},
    content::{ChargeConfig, EnemyBehavior, GameContent, RangedConfig, EMBEDDED_CONTENT},
    enemy_behavior::{spawn_hostile_projectile, EnemyBrain, EnemyTelegraph, HostileProjectile},
    resources::GameState,
    tests::{count, game, key, player, step},
    weapons::{BindingEffect, InvocationPull},
};
use bevy::{ecs::system::RunSystemOnce, prelude::*};
use bevy_rapier2d::prelude::*;

fn charge() -> ChargeConfig {
    ChargeConfig {
        trigger_range: 300.0,
        windup: 0.3,
        dash_speed: 300.0,
        dash_seconds: 0.2,
        recovery: 0.4,
    }
}

fn ranged() -> RangedConfig {
    RangedConfig {
        preferred_range: 250.0,
        windup: 0.25,
        cooldown: 0.3,
        projectile_speed: 400.0,
        projectile_lifetime: 1.0,
        damage: 10,
        projectile_count: 1,
        spread_radians: 0.0,
    }
}

fn charger_behavior(c: ChargeConfig) -> EnemyBehavior {
    EnemyBehavior::Charger {
        trigger_range: c.trigger_range,
        windup: c.windup,
        dash_speed: c.dash_speed,
        dash_seconds: c.dash_seconds,
        recovery: c.recovery,
    }
}

fn ranged_behavior(r: RangedConfig) -> EnemyBehavior {
    EnemyBehavior::Ranged {
        preferred_range: r.preferred_range,
        windup: r.windup,
        cooldown: r.cooldown,
        projectile_speed: r.projectile_speed,
        projectile_lifetime: r.projectile_lifetime,
        damage: r.damage,
        projectile_count: r.projectile_count,
        spread_radians: r.spread_radians,
    }
}

fn enemy(app: &mut App, position: Vec2, behavior: EnemyBehavior) -> Entity {
    app.world_mut()
        .spawn((
            Enemy {
                speed: 50.0,
                experience_value: 0,
            },
            EnemyBrain::new(behavior),
            Health {
                current: 1000,
                maximum: 1000,
            },
            Transform::from_translation(position.extend(0.0)),
        ))
        .id()
}

fn bolt(app: &mut App, origin: Vec2, direction: Vec2, config: RangedConfig) -> Entity {
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            spawn_hostile_projectile(&mut commands, origin, direction, &config)
        })
        .unwrap()
}

#[test]
fn charger_warns_locks_aim_then_recovers_and_obeys_crowd_control() {
    let mut app = game();
    let enemy = enemy(&mut app, Vec2::new(-180.0, 0.0), charger_behavior(charge()));
    step(&mut app, 4);
    assert_eq!(count::<EnemyTelegraph>(&mut app), 1);
    assert_eq!(
        app.world().get::<Velocity>(enemy).unwrap().linear,
        Vec2::ZERO
    );
    assert!(app.world().get::<Ccd>(enemy).unwrap().enabled);
    // Move out of the warned line. The charge must not retarget during its windup.
    let player = player(&mut app);
    app.world_mut()
        .get_mut::<Transform>(player)
        .unwrap()
        .translation
        .y = 120.0;
    step(&mut app, 17);
    let velocity = app.world().get::<Velocity>(enemy).unwrap().linear;
    assert!(
        velocity.x > 250.0 && velocity.y.abs() < 0.01,
        "{velocity:?}"
    );
    assert_eq!(count::<EnemyTelegraph>(&mut app), 0);
    app.world_mut().entity_mut(enemy).insert(BindingEffect {
        strength: 1.0,
        source: player,
        timer: Timer::from_seconds(2.0, TimerMode::Once),
    });
    step(&mut app, 2);
    assert_eq!(
        app.world().get::<Velocity>(enemy).unwrap().linear,
        Vec2::ZERO
    );
    app.world_mut().entity_mut(enemy).insert(InvocationPull {
        velocity: Vec2::new(-30.0, 50.0),
    });
    app.world_mut()
        .run_system_once(crate::systems::enemy_movement)
        .unwrap();
    assert_eq!(
        app.world().get::<Velocity>(enemy).unwrap().linear,
        Vec2::new(-30.0, 50.0)
    );
    app.world_mut()
        .entity_mut(enemy)
        .remove::<(BindingEffect, InvocationPull)>();
    step(&mut app, 14);
    assert_eq!(
        app.world().get::<Velocity>(enemy).unwrap().linear,
        Vec2::ZERO
    );
}

#[test]
fn charging_body_stops_at_rapier_world_geometry() {
    let mut app = game();
    let player = player(&mut app);
    app.world_mut()
        .get_mut::<Transform>(player)
        .unwrap()
        .translation
        .y = 200.0;
    app.world_mut().spawn((
        RigidBody::Fixed,
        Collider::cuboid(3.0, 80.0),
        CollisionGroups::new(crate::arena::WORLD_GROUP, Group::GROUP_1 | Group::GROUP_2),
        Transform::from_xyz(-80.0, 200.0, 0.0),
    ));
    let enemy = enemy(
        &mut app,
        Vec2::new(-180.0, 200.0),
        charger_behavior(ChargeConfig {
            dash_speed: 2000.0,
            ..charge()
        }),
    );
    step(&mut app, 38);
    assert!(app.world().get::<Transform>(enemy).unwrap().translation.x < -90.0);
    assert_eq!(app.world().get::<Health>(player).unwrap().current, 100);
}

#[test]
fn ranged_enemy_approaches_warns_and_fires_the_warned_direction() {
    let mut app = game();
    let enemy = enemy(&mut app, Vec2::new(-270.0, 0.0), ranged_behavior(ranged()));
    step(&mut app, 4);
    assert!(app.world().get::<Velocity>(enemy).unwrap().linear.x > 30.0);
    assert_eq!(count::<EnemyTelegraph>(&mut app), 0);
    app.world_mut()
        .get_mut::<Transform>(enemy)
        .unwrap()
        .translation
        .x = -200.0;
    step(&mut app, 3);
    assert_eq!(count::<EnemyTelegraph>(&mut app), 1);
    assert_eq!(count::<HostileProjectile>(&mut app), 0);
    let player = player(&mut app);
    app.world_mut()
        .get_mut::<Transform>(player)
        .unwrap()
        .translation
        .y = 100.0;
    step(&mut app, 15);
    let velocity = app
        .world_mut()
        .query_filtered::<&Velocity, With<HostileProjectile>>()
        .single(app.world())
        .unwrap();
    assert!(velocity.linear.x > 390.0 && velocity.linear.y.abs() < 0.01);
    assert_eq!(
        app.world().get::<Velocity>(enemy).unwrap().linear,
        Vec2::ZERO
    );
}

#[test]
fn hostile_sweep_hits_player_once_ignores_enemies_and_stops_at_walls() {
    let mut app = game();
    step(&mut app, 20); // Initial shared player damage cooldown has elapsed.
    let player = player(&mut app);
    enemy(&mut app, Vec2::new(-70.0, 0.0), EnemyBehavior::Pursuer);
    let first = bolt(&mut app, Vec2::new(-120.0, 0.0), Vec2::X, ranged());
    step(&mut app, 20);
    assert!(app.world().get_entity(first).is_err());
    assert_eq!(app.world().get::<Health>(player).unwrap().current, 90);
    app.world_mut().spawn((
        RigidBody::Fixed,
        Collider::cuboid(2.0, 30.0),
        CollisionGroups::new(crate::arena::WORLD_GROUP, Group::GROUP_3),
        Transform::from_xyz(60.0, 0.0, 0.0),
    ));
    step(&mut app, 3);
    let blocked = bolt(&mut app, Vec2::new(120.0, 0.0), Vec2::NEG_X, ranged());
    step(&mut app, 20);
    assert!(app.world().get_entity(blocked).is_err());
    assert_eq!(app.world().get::<Health>(player).unwrap().current, 90);
}

#[test]
fn hostile_bolts_use_shared_protection_and_invulnerability_window() {
    let mut app = game();
    step(&mut app, 20);
    let player = player(&mut app);
    app.world_mut()
        .run_system_once(|mut commands: Commands| {
            crate::weapons::magick_circle::spawn_magick_circle_attack(
                &mut commands,
                Vec3::ZERO,
                10,
                64.0,
                crate::weapons::magick_circle::PatternType::Protection,
                None,
            );
        })
        .unwrap();
    step(&mut app, 3);
    bolt(&mut app, Vec2::new(-40.0, 0.0), Vec2::X, ranged());
    bolt(&mut app, Vec2::new(40.0, 0.0), Vec2::NEG_X, ranged());
    step(&mut app, 10);
    assert_eq!(app.world().get::<Health>(player).unwrap().current, 95);
    assert_eq!(
        app.world()
            .resource::<crate::telemetry::RunTelemetry>()
            .damage_taken,
        5
    );
}

#[test]
fn pause_freezes_windup_and_projectiles_and_restart_cleans_both() {
    let mut app = game();
    let enemy = enemy(&mut app, Vec2::new(-200.0, 0.0), charger_behavior(charge()));
    let bolt = bolt(&mut app, Vec2::new(200.0, 200.0), Vec2::X, ranged());
    step(&mut app, 3);
    key(&mut app, KeyCode::Escape);
    step(&mut app, 2);
    let position = app.world().get::<Transform>(bolt).unwrap().translation;
    step(&mut app, 90);
    assert_eq!(
        app.world().get::<Transform>(bolt).unwrap().translation,
        position
    );
    assert_eq!(count::<EnemyTelegraph>(&mut app), 1);
    assert_eq!(
        app.world().get::<Velocity>(enemy).unwrap().linear,
        Vec2::ZERO
    );
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Restarting);
    step(&mut app, 3);
    assert!(app.world().get_entity(bolt).is_err());
    assert!(app.world().get_entity(enemy).is_err());
    assert_eq!(count::<EnemyTelegraph>(&mut app), 0);
    assert_eq!(count::<HostileProjectile>(&mut app), 0);
}

#[test]
fn warden_alternates_a_warned_volley_and_charge_and_bolts_expire() {
    let mut app = game();
    let enemy = enemy(
        &mut app,
        Vec2::new(-200.0, 0.0),
        EnemyBehavior::Warden {
            charge: charge(),
            ranged: RangedConfig {
                projectile_count: 3,
                spread_radians: 0.6,
                ..ranged()
            },
        },
    );
    step(&mut app, 19);
    assert_eq!(count::<HostileProjectile>(&mut app), 3);
    let directions: Vec<_> = app
        .world_mut()
        .query_filtered::<&Velocity, With<HostileProjectile>>()
        .iter(app.world())
        .map(|v| v.linear.y)
        .collect();
    assert!(directions.iter().any(|y| *y > 100.0) && directions.iter().any(|y| *y < -100.0));
    step(&mut app, 38);
    assert!(app.world().get::<Velocity>(enemy).unwrap().linear.x > 250.0);
    // Killing the shooter removes its children, while existing projectiles retain finite lives.
    app.world_mut().despawn(enemy);
    step(&mut app, 70);
    assert_eq!(count::<EnemyTelegraph>(&mut app), 0);
    assert_eq!(count::<HostileProjectile>(&mut app), 0);
}

#[test]
fn content_defaults_to_pursuit_and_rejects_invalid_attack_timing() {
    let default: crate::content::EnemyDefinition = ron::from_str(
        "(id:\"test\",name:\"Test\",health:10,speed:20.0,size:24.0,color:(1.0,1.0,1.0,1.0),xp:1)",
    )
    .unwrap();
    assert!(matches!(default.behavior, EnemyBehavior::Pursuer));
    let mut content = GameContent::parse(EMBEDDED_CONTENT).unwrap();
    for config in [
        ChargeConfig {
            windup: 0.0,
            ..charge()
        },
        ChargeConfig {
            dash_speed: f32::NAN,
            ..charge()
        },
    ] {
        content.enemies[0].behavior = charger_behavior(config);
        assert!(content.validate().is_err());
    }
    for config in [
        RangedConfig {
            projectile_lifetime: 0.0,
            ..ranged()
        },
        RangedConfig {
            projectile_count: 0,
            ..ranged()
        },
        RangedConfig {
            spread_radians: f32::INFINITY,
            ..ranged()
        },
    ] {
        content.enemies[0].behavior = ranged_behavior(config);
        assert!(content.validate().is_err());
    }
}
