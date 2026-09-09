use crate::components::{Enemy, Health};
use crate::content::{ActiveStage, GameContent};
use crate::experience::ExperienceOrb;
use crate::loot::{spawn_breakable, spawn_loot, Breakable, ChestDrop, LootKind, PendingTreasure};
use crate::resources::{GameState, GameStats};
use crate::tests::{count, game, key, player, state, step};
use crate::weapons::magick_circle::{spawn_magick_circle_attack, MagickCircle, PatternType};
use crate::weapons::weapon_upgrade::WeaponUpgradeChange;
use crate::weapons::{BindingEffect, InvocationPull, WeaponMeta};
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

fn circle(app: &mut App, pattern: PatternType, position: Vec2) {
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            spawn_magick_circle_attack(
                &mut commands,
                position.extend(0.0),
                10,
                64.0,
                pattern,
                None,
            );
        })
        .unwrap();
}

fn pickup_count(app: &mut App, kind: LootKind) -> usize {
    app.world_mut()
        .query::<&LootKind>()
        .iter(app.world())
        .filter(|pickup| **pickup == kind)
        .count()
}

#[test]
fn props_take_circle_damage_and_drop_loot_without_enemy_effects_xp_or_kills() {
    let mut app = game();
    let position = Vec2::new(300.0, 0.0);
    let prop = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            spawn_breakable(&mut commands, position, 10, 1.0, "Clay urn", LootKind::Gold)
        })
        .unwrap();
    circle(
        &mut app,
        PatternType::Invocation,
        position + Vec2::new(25.0, 0.0),
    );
    circle(&mut app, PatternType::Binding, position);
    step(&mut app, 40);
    assert!(app.world().get::<BindingEffect>(prop).is_none());
    assert!(app.world().get::<InvocationPull>(prop).is_none());
    assert_eq!(app.world().get::<Health>(prop).unwrap().current, 10);
    let gold = pickup_count(&mut app, LootKind::Gold);
    circle(&mut app, PatternType::Banishment, position);
    step(&mut app, 40);
    assert!(app.world().get_entity(prop).is_err());
    assert_eq!(pickup_count(&mut app, LootKind::Gold), gold + 1);
    assert_eq!(app.world().resource::<GameStats>().enemies_killed, 0);
    assert_eq!(count::<ExperienceOrb>(&mut app), 0);
    step(&mut app, 10);
    assert_eq!(pickup_count(&mut app, LootKind::Gold), gold + 1);
}

#[test]
fn rapier_pickup_intersections_heal_and_credit_gold_exactly_once() {
    let mut app = game();
    let player = player(&mut app);
    app.world_mut().get_mut::<Health>(player).unwrap().current = 50;
    let initial_gold = app.world().resource::<GameStats>().gold;
    app.world_mut()
        .run_system_once(|mut commands: Commands| {
            spawn_loot(&mut commands, Vec2::ZERO, LootKind::Healing);
            spawn_loot(&mut commands, Vec2::ZERO, LootKind::Gold);
        })
        .unwrap();
    step(&mut app, 4);
    assert_eq!(app.world().get::<Health>(player).unwrap().current, 70);
    assert_eq!(app.world().resource::<GameStats>().gold, initial_gold + 1);
    step(&mut app, 10);
    assert_eq!(app.world().get::<Health>(player).unwrap().current, 70);
    assert_eq!(app.world().resource::<GameStats>().gold, initial_gold + 1);
}

#[test]
fn boss_death_drops_one_chest_before_entity_cleanup() {
    let mut app = game();
    let before = pickup_count(&mut app, LootKind::Chest);
    let boss = app
        .world_mut()
        .spawn((
            ChestDrop,
            Enemy {
                speed: 0.0,
                experience_value: 20,
            },
            Health {
                current: 10,
                maximum: 10,
            },
            Transform::from_xyz(300.0, 0.0, 0.0),
        ))
        .id();
    app.world_mut().write_message(crate::combat::DamageEvent {
        target: boss,
        amount: 10,
        source: None,
    });
    step(&mut app, 3);
    assert!(app.world().get_entity(boss).is_err());
    assert_eq!(pickup_count(&mut app, LootKind::Chest), before + 1);
    step(&mut app, 10);
    assert_eq!(pickup_count(&mut app, LootKind::Chest), before + 1);
}

#[test]
fn treasure_pauses_and_claims_real_progression_once_including_added_circles() {
    let mut app = game();
    let weapon = app
        .world_mut()
        .query_filtered::<Entity, With<WeaponMeta>>()
        .single(app.world())
        .unwrap();
    app.world_mut().get_mut::<WeaponMeta>(weapon).unwrap().level = 2;
    app.world_mut()
        .run_system_once(|mut commands: Commands| {
            spawn_loot(&mut commands, Vec2::ZERO, LootKind::Chest);
        })
        .unwrap();
    step(&mut app, 4);
    assert_eq!(state(&app), GameState::Treasure);
    let rewards = app.world().resource::<PendingTreasure>().rewards.clone();
    assert!([1, 3, 5].contains(&rewards.len()));
    let crate::upgrade::UpgradeType::Weapon(_, first_upgrade) = &rewards[0].upgrade else {
        panic!("expected owned weapon reward");
    };
    assert!(first_upgrade
        .changes
        .iter()
        .any(|change| matches!(change, WeaponUpgradeChange::AddCircle { pattern: Some(_) })));
    let added_circles = rewards
        .iter()
        .filter_map(|reward| match &reward.upgrade {
            crate::upgrade::UpgradeType::Weapon(_, spec) => Some(&spec.changes),
            _ => None,
        })
        .flatten()
        .filter(|change| matches!(change, WeaponUpgradeChange::AddCircle { .. }))
        .count();
    let clock = app.world().resource::<GameStats>().time_elapsed;
    step(&mut app, 10);
    assert_eq!(app.world().resource::<GameStats>().time_elapsed, clock);
    // Reveal first, then explicitly claim; skipping animation grants nothing.
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.world().get::<WeaponMeta>(weapon).unwrap().level, 2);
    key(&mut app, KeyCode::Enter);
    step(&mut app, 3);
    assert_eq!(state(&app), GameState::Playing);
    assert_eq!(
        app.world().get::<WeaponMeta>(weapon).unwrap().level,
        2 + rewards.len() as u32
    );
    assert_eq!(
        app.world()
            .get::<MagickCircle>(weapon)
            .unwrap()
            .patterns
            .len(),
        1 + added_circles
    );
    assert!(app.world().resource::<PendingTreasure>().rewards.is_empty());
    step(&mut app, 4);
    assert_eq!(
        app.world().get::<WeaponMeta>(weapon).unwrap().level,
        2 + rewards.len() as u32
    );
}

#[test]
fn replenished_props_remain_offscreen_and_respect_authored_cap() {
    let mut app = game();
    let active = app.world().resource::<ActiveStage>().0;
    {
        let mut content = app.world_mut().resource_mut::<GameContent>();
        content.stages[active].breakables.count = 5;
        content.stages[active].breakables.respawn_seconds = 0.05;
    }
    step(&mut app, 20);
    assert_eq!(count::<Breakable>(&mut app), 5);
    for transform in app
        .world_mut()
        .query_filtered::<&Transform, With<Breakable>>()
        .iter(app.world())
    {
        let position = transform.translation.truncate();
        assert!(
            position.x.abs() > 640.0 || position.y.abs() > 360.0,
            "prop appeared onscreen: {position}"
        );
    }
    step(&mut app, 20);
    assert_eq!(count::<Breakable>(&mut app), 5);
}
