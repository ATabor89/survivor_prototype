use crate::components::{Enemy, Health};
use crate::tests::{game, key, player, step};
use crate::weapons::magick_circle::MagickCircle;
use crate::weapons::projectiles::{spawn_projectile, Projectile, VesselBurst};
use crate::weapons::{
    grant_configured_weapon, AddWeaponEvent, AttackSource, WeaponDamage, WeaponMeta, WeaponType,
};
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

fn target(app: &mut App, position: Vec2) -> Entity {
    app.world_mut()
        .spawn((
            Enemy {
                speed: 0.0,
                experience_value: 0,
            },
            Health {
                current: 1000,
                maximum: 1000,
            },
            Transform::from_translation(position.extend(0.0)),
        ))
        .id()
}

fn projectile(app: &mut App, origin: Vec2, target: Vec2, weapon: WeaponType) -> Entity {
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            spawn_projectile(
                &mut commands,
                origin,
                target,
                weapon,
                if weapon == WeaponType::InscribedNeedles {
                    12
                } else {
                    24
                },
                if weapon == WeaponType::InscribedNeedles {
                    4.0
                } else {
                    72.0
                },
            )
        })
        .unwrap()
}

#[test]
fn fast_needles_use_rapier_sweeps_and_stop_after_three_unique_hits() {
    let mut app = game();
    for mut simulation in app
        .world_mut()
        .query::<&mut RapierContextSimulation>()
        .iter_mut(app.world_mut())
    {
        simulation
            .integration_parameters
            .normalized_max_linear_velocity = 20000.0;
    }
    let enemies: Vec<_> = [70.0, 130.0, 190.0, 250.0]
        .into_iter()
        .map(|x| target(&mut app, Vec2::new(x, 200.0)))
        .collect();
    step(&mut app, 3);
    let needle = projectile(
        &mut app,
        Vec2::new(0.0, 200.0),
        Vec2::new(600.0, 200.0),
        WeaponType::InscribedNeedles,
    );
    assert!(app.world().get::<Ccd>(needle).unwrap().enabled);
    app.world_mut().get_mut::<Velocity>(needle).unwrap().linear = Vec2::X * 18000.0;
    step(&mut app, 3);
    for enemy in &enemies[..3] {
        assert_eq!(app.world().get::<Health>(*enemy).unwrap().current, 988);
    }
    assert_eq!(app.world().get::<Health>(enemies[3]).unwrap().current, 1000);
    assert!(app.world().get_entity(needle).is_err());
    let telemetry = app.world().resource::<crate::telemetry::RunTelemetry>();
    assert_eq!(
        telemetry.damage_by_weapon[&WeaponType::InscribedNeedles],
        36
    );
    assert_eq!(telemetry.hits_by_weapon[&WeaponType::InscribedNeedles], 3);
}

#[test]
fn fast_projectile_hits_wall_before_enemy_and_persistent_overlap_hits_once() {
    let mut app = game();
    for mut simulation in app
        .world_mut()
        .query::<&mut RapierContextSimulation>()
        .iter_mut(app.world_mut())
    {
        simulation
            .integration_parameters
            .normalized_max_linear_velocity = 20000.0;
    }
    let behind = target(&mut app, Vec2::new(230.0, 220.0));
    app.world_mut().spawn((
        RigidBody::Fixed,
        Collider::cuboid(2.0, 35.0),
        CollisionGroups::new(crate::arena::WORLD_GROUP, Group::GROUP_3),
        Transform::from_xyz(120.0, 220.0, 0.0),
    ));
    step(&mut app, 3);
    let needle = projectile(
        &mut app,
        Vec2::new(0.0, 220.0),
        Vec2::new(600.0, 220.0),
        WeaponType::InscribedNeedles,
    );
    app.world_mut().get_mut::<Velocity>(needle).unwrap().linear = Vec2::X * 18000.0;
    step(&mut app, 3);
    assert_eq!(app.world().get::<Health>(behind).unwrap().current, 1000);
    assert!(app.world().get_entity(needle).is_err());

    let stationary_target = app
        .world_mut()
        .run_system_once(|mut commands: Commands| {
            crate::loot::spawn_breakable(
                &mut commands,
                Vec2::new(-200.0, 220.0),
                1000,
                0.0,
                "Clay urn",
                crate::loot::LootKind::Gold,
            )
        })
        .unwrap();
    step(&mut app, 3);
    let stationary = projectile(
        &mut app,
        Vec2::new(-200.0, 220.0),
        Vec2::new(-100.0, 220.0),
        WeaponType::InscribedNeedles,
    );
    app.world_mut()
        .get_mut::<Velocity>(stationary)
        .unwrap()
        .linear = Vec2::ZERO;
    step(&mut app, 30);
    assert_eq!(
        app.world()
            .get::<Health>(stationary_target)
            .unwrap()
            .current,
        988
    );
    assert!(app.world().get::<Projectile>(stationary).is_some());
}

#[test]
fn vessel_lands_and_bursts_once_using_rapier_area_membership() {
    let mut app = game();
    let near = target(&mut app, Vec2::new(175.0, -220.0));
    let far = target(&mut app, Vec2::new(280.0, -220.0));
    let prop = app
        .world_mut()
        .run_system_once(|mut commands: Commands| {
            crate::loot::spawn_breakable(
                &mut commands,
                Vec2::new(165.0, -240.0),
                30,
                0.0,
                "Clay urn",
                crate::loot::LootKind::Gold,
            )
        })
        .unwrap();
    step(&mut app, 3);
    let vessel = projectile(
        &mut app,
        Vec2::new(0.0, -220.0),
        Vec2::new(120.0, -220.0),
        WeaponType::AlchemicalVessel,
    );
    assert_eq!(
        app.world().get::<AttackSource>(vessel).unwrap().0,
        WeaponType::AlchemicalVessel
    );
    step(&mut app, 30);
    assert!(app.world().get_entity(vessel).is_err());
    assert_eq!(app.world().get::<Health>(near).unwrap().current, 976);
    assert_eq!(app.world().get::<Health>(far).unwrap().current, 1000);
    assert_eq!(app.world().get::<Health>(prop).unwrap().current, 6);
    assert_eq!(
        app.world()
            .resource::<crate::telemetry::RunTelemetry>()
            .damage_by_weapon[&WeaponType::AlchemicalVessel],
        24
    );
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<VesselBurst>>()
            .iter(app.world())
            .count(),
        1
    );
    step(&mut app, 30);
    assert_eq!(app.world().get::<Health>(near).unwrap().current, 976);
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<VesselBurst>>()
            .iter(app.world())
            .count(),
        0
    );
}

#[test]
fn acquired_projectile_weapons_automatically_aim_at_the_nearest_enemy() {
    let mut app = game();
    let player = player(&mut app);
    target(&mut app, Vec2::new(0.0, 120.0));
    target(&mut app, Vec2::new(250.0, 0.0));
    step(&mut app, 3);
    for kind in [WeaponType::InscribedNeedles, WeaponType::AlchemicalVessel] {
        grant_configured_weapon(app.world_mut(), player, kind, 1, &[]).unwrap();
    }
    for (meta, mut cooldown) in app
        .world_mut()
        .query::<(&WeaponMeta, &mut crate::weapons::WeaponCooldown)>()
        .iter_mut(app.world_mut())
    {
        if meta.weapon_type != WeaponType::MagickCircle {
            let duration = cooldown.timer.duration();
            cooldown.timer.set_elapsed(duration);
        }
    }
    step(&mut app, 1);
    let shots: Vec<_> = app
        .world_mut()
        .query_filtered::<(&AttackSource, &Velocity), With<Projectile>>()
        .iter(app.world())
        .map(|(source, velocity)| (source.0, velocity.linear))
        .collect();
    assert_eq!(shots.len(), 2);
    for (_, velocity) in shots {
        assert!(
            velocity.y > 0.0 && velocity.x.abs() < 0.001,
            "not aimed at nearest target: {velocity}"
        );
    }
}

#[test]
fn vessel_collides_with_wall_and_bursts_there_instead_of_at_original_target() {
    let mut app = game();
    let behind = target(&mut app, Vec2::new(280.0, 220.0));
    app.world_mut().spawn((
        RigidBody::Fixed,
        Collider::cuboid(2.0, 40.0),
        CollisionGroups::new(crate::arena::WORLD_GROUP, Group::GROUP_3),
        Transform::from_xyz(100.0, 220.0, 0.0),
    ));
    step(&mut app, 3);
    let vessel = projectile(
        &mut app,
        Vec2::new(0.0, 220.0),
        Vec2::new(280.0, 220.0),
        WeaponType::AlchemicalVessel,
    );
    step(&mut app, 24);
    assert!(app.world().get_entity(vessel).is_err());
    assert_eq!(app.world().get::<Health>(behind).unwrap().current, 1000);
    let burst = app
        .world_mut()
        .query_filtered::<&Transform, With<VesselBurst>>()
        .single(app.world())
        .unwrap();
    assert!(
        (85.0..=100.0).contains(&burst.translation.x),
        "vessel passed through wall: {}",
        burst.translation
    );
}

#[test]
fn stale_rapier_collider_of_a_despawned_enemy_does_not_stop_a_needle() {
    let mut app = game();
    for mut simulation in app
        .world_mut()
        .query::<&mut RapierContextSimulation>()
        .iter_mut(app.world_mut())
    {
        simulation
            .integration_parameters
            .normalized_max_linear_velocity = 20000.0;
    }
    let stale = target(&mut app, Vec2::new(60.0, 220.0));
    let live = target(&mut app, Vec2::new(150.0, 220.0));
    step(&mut app, 3);
    let needle = projectile(
        &mut app,
        Vec2::new(0.0, 220.0),
        Vec2::new(600.0, 220.0),
        WeaponType::InscribedNeedles,
    );
    app.world_mut().get_mut::<Velocity>(needle).unwrap().linear = Vec2::X * 18000.0;
    step(&mut app, 1);
    // Rapier still has this collider until its next synchronization pass, while
    // the next Weapons update will sweep the projectile's completed movement.
    app.world_mut().entity_mut(stale).despawn();
    step(&mut app, 1);
    assert_eq!(app.world().get::<Health>(live).unwrap().current, 988);
    assert!(app.world().get::<Projectile>(needle).is_some());
}

#[test]
fn projectile_motion_and_lifetime_pause_with_run() {
    let mut app = game();
    let needle = projectile(
        &mut app,
        Vec2::ZERO,
        Vec2::X * 500.0,
        WeaponType::InscribedNeedles,
    );
    step(&mut app, 4);
    assert!((app.world().get::<Velocity>(needle).unwrap().linear.x - 720.0).abs() < 0.1);
    let travelled = app.world().get::<Transform>(needle).unwrap().translation.x;
    assert!(
        (40.0..=50.0).contains(&travelled),
        "production Rapier settings slowed the needle: {travelled}"
    );
    key(&mut app, KeyCode::Escape);
    step(&mut app, 2);
    let paused = app.world().get::<Transform>(needle).unwrap().translation;
    step(&mut app, 90);
    assert_eq!(
        app.world().get::<Transform>(needle).unwrap().translation,
        paused
    );
    key(&mut app, KeyCode::Escape);
    step(&mut app, 150);
    assert!(app.world().get_entity(needle).is_err());
}

#[test]
fn expiring_circle_final_tick_keeps_source_and_records_only_remaining_health() {
    let mut app = game();
    let enemy = target(&mut app, Vec2::new(300.0, 200.0));
    app.world_mut().get_mut::<Health>(enemy).unwrap().current = 55;
    step(&mut app, 3);
    app.world_mut()
        .run_system_once(|mut commands: Commands| {
            crate::weapons::magick_circle::spawn_magick_circle_attack(
                &mut commands,
                Vec3::new(300.0, 200.0, 0.0),
                10,
                64.0,
                crate::weapons::magick_circle::PatternType::Banishment,
                None,
            );
        })
        .unwrap();
    step(&mut app, 182);
    assert!(app.world().get_entity(enemy).is_err());
    let telemetry = app.world().resource::<crate::telemetry::RunTelemetry>();
    assert_eq!(telemetry.damage_by_weapon[&WeaponType::MagickCircle], 55);
    assert_eq!(telemetry.hits_by_weapon[&WeaponType::MagickCircle], 6);
}

#[test]
fn configured_progression_and_duplicate_acquisitions_share_real_weapon_stats() {
    let mut app = game();
    let player = player(&mut app);
    grant_configured_weapon(
        app.world_mut(),
        player,
        WeaponType::InscribedNeedles,
        2,
        &[],
    )
    .unwrap();
    for _ in 0..3 {
        app.world_mut().write_message(AddWeaponEvent {
            player,
            weapon_type: WeaponType::InscribedNeedles,
        });
    }
    step(&mut app, 1);
    let needles: Vec<_> = app
        .world_mut()
        .query::<(&WeaponMeta, &WeaponDamage)>()
        .iter(app.world())
        .filter(|(meta, _)| meta.weapon_type == WeaponType::InscribedNeedles)
        .map(|(meta, damage)| (meta.level, damage.damage_bonus))
        .collect();
    assert_eq!(needles, [(2, 25)]);
    assert!(grant_configured_weapon(
        app.world_mut(),
        player,
        WeaponType::InscribedNeedles,
        2,
        &[]
    )
    .is_err());
    let circle = app
        .world_mut()
        .query_filtered::<Entity, With<MagickCircle>>()
        .single(app.world())
        .unwrap();
    app.world_mut().entity_mut(circle).despawn();
    grant_configured_weapon(app.world_mut(), player, WeaponType::MagickCircle, 2, &[]).unwrap();
    let damage = app
        .world_mut()
        .query::<(&WeaponMeta, &WeaponDamage)>()
        .iter(app.world())
        .find(|(meta, _)| meta.weapon_type == WeaponType::MagickCircle)
        .unwrap()
        .1;
    assert_eq!(damage.damage_bonus, 20);
    assert_eq!(
        (damage.base_amount as f32 * (1.0 + damage.damage_bonus as f32 / 100.0)).floor() as i32,
        12
    );
}

#[test]
fn upgrade_preview_matches_fired_attack_and_cooldown_with_passive_modifiers() {
    use crate::weapons::weapon_upgrade::{
        preview_weapon_upgrade, WeaponUpgradeChange, WeaponUpgradeSpec,
    };
    use crate::weapons::{Attack, Damage, WeaponArea, WeaponCooldown};
    let mut app = game();
    let player = player(&mut app);
    crate::inventory::grant_passive(
        app.world_mut(),
        player,
        crate::inventory::PassiveType::Cooldown,
    )
    .unwrap();
    let stats = *app
        .world()
        .get::<crate::components::PlayerStats>(player)
        .unwrap();
    let spec = WeaponUpgradeSpec {
        changes: vec![
            WeaponUpgradeChange::Damage(20),
            WeaponUpgradeChange::Area(10),
            WeaponUpgradeChange::Cooldown(-10),
        ],
    };
    let weapon = app
        .world_mut()
        .query_filtered::<Entity, With<WeaponMeta>>()
        .single(app.world())
        .unwrap();
    let preview = preview_weapon_upgrade(
        &spec,
        app.world().get::<WeaponDamage>(weapon).unwrap(),
        app.world().get::<WeaponArea>(weapon).unwrap(),
        app.world().get::<WeaponCooldown>(weapon).unwrap(),
        &stats,
    );
    assert_eq!(preview.damage, (10, 12));
    assert!(
        preview.cooldown.0 < 3.5,
        "preview ignored the passive cooldown modifier"
    );
    assert!(preview.description(&spec).contains("Damage 10 -> 12"));
    assert!(preview.description(&spec).contains("Radius 64.0 -> 70.4"));
    app.world_mut()
        .get_mut::<WeaponCooldown>(weapon)
        .unwrap()
        .timer
        .set_elapsed(std::time::Duration::from_secs(10));
    app.world_mut()
        .write_message(crate::menu::WeaponUpgradeConfirmedEvent {
            weapon_type: WeaponType::MagickCircle,
            upgrade_spec: spec,
        });
    step(&mut app, 1);
    let (damage, collider) = app
        .world_mut()
        .query_filtered::<(&Damage, &Collider), With<Attack>>()
        .single(app.world())
        .unwrap();
    assert_eq!(damage.amount, preview.damage.1);
    assert!((collider.raw.as_ball().unwrap().radius - preview.radius.1).abs() < 0.001);
    let actual_cooldown = app
        .world()
        .get::<WeaponCooldown>(weapon)
        .unwrap()
        .timer
        .duration()
        .as_secs_f32();
    assert!((actual_cooldown - preview.cooldown.1).abs() < 0.0001);
}
