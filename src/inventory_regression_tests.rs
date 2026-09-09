use crate::{
    components::{CooldownReduction, Health, Luck, Player, PlayerStats},
    inventory::{
        grant_passive, PassiveCatalog, PassiveInventory, PassiveItem, PassiveType,
        PassiveUpgradeEvent, MAX_PASSIVES, MAX_WEAPONS,
    },
    loot::{spawn_loot, LootKind, PendingTreasure},
    resources::{GameState, GameStats},
    tests::{game, key, player, state, step},
    upgrade::{GenericUpgrade, UpgradePool, UpgradeType},
    weapons::{weapon_upgrade::WeaponUpgradeConfig, AddWeaponEvent, WeaponMeta, WeaponType},
};
use bevy::{ecs::system::RunSystemOnce, prelude::*};
use rand::{rngs::StdRng, SeedableRng};
use strum::IntoEnumIterator;

#[test]
fn passive_stats_are_calculated_from_base_and_duplicate_offers_do_not_double_upgrade() {
    let mut app = game();
    let player = player(&mut app);
    for _ in 0..2 {
        app.world_mut().write_message(PassiveUpgradeEvent {
            player,
            kind: PassiveType::MoveSpeed,
            level: 1,
        });
    }
    step(&mut app, 2);
    assert_eq!(
        app.world()
            .get::<PassiveInventory>(player)
            .unwrap()
            .level(PassiveType::MoveSpeed),
        1
    );
    assert!((app.world().get::<Player>(player).unwrap().speed - 165.0).abs() < 0.01);
    grant_passive(app.world_mut(), player, PassiveType::MoveSpeed).unwrap();
    assert!((app.world().get::<Player>(player).unwrap().speed - 180.0).abs() < 0.01);
    app.world_mut().get_mut::<Health>(player).unwrap().current = 50;
    grant_passive(app.world_mut(), player, PassiveType::MaxHealth).unwrap();
    grant_passive(app.world_mut(), player, PassiveType::PickupRange).unwrap();
    grant_passive(app.world_mut(), player, PassiveType::Cooldown).unwrap();
    let stats = app.world().get::<PlayerStats>(player).unwrap();
    assert_eq!(stats.max_health, 120);
    assert_eq!(app.world().get::<Health>(player).unwrap().current, 70);
    assert!((stats.pickup_range - 180.0).abs() < 0.01);
    assert!((stats.cooldown_reduction - 0.04).abs() < 0.001);
    assert_eq!(
        app.world()
            .get::<CooldownReduction>(player)
            .unwrap()
            .percent,
        stats.cooldown_reduction
    );
    assert_eq!(stats.armor, 0.0);
    assert_eq!(stats.regeneration, 0.0);
    step(&mut app, 4);
    assert!((app.world().get::<PlayerStats>(player).unwrap().move_speed - 180.0).abs() < 0.01);
    assert_eq!(app.world().get::<Health>(player).unwrap().current, 70);
}

#[test]
fn weapon_acquisition_is_distinct_and_capped_items_disappear_from_offers() {
    let mut app = game();
    let player = player(&mut app);
    for _ in 0..3 {
        app.world_mut().write_message(AddWeaponEvent {
            player,
            weapon_type: WeaponType::InscribedNeedles,
        });
    }
    step(&mut app, 2);
    assert_eq!(
        app.world_mut()
            .query::<&WeaponMeta>()
            .iter(app.world())
            .filter(|meta| meta.weapon_type == WeaponType::InscribedNeedles)
            .count(),
        1
    );
    assert_eq!(MAX_WEAPONS, 6);
    assert_eq!(MAX_PASSIVES, 6);
    let config = WeaponUpgradeConfig::default();
    let catalog = PassiveCatalog::default();
    let owned: Vec<_> = WeaponType::iter()
        .map(|weapon_type| WeaponMeta {
            weapon_type,
            level: config.max_level(weapon_type),
        })
        .collect();
    let refs: Vec<_> = owned.iter().collect();
    let passives = PassiveInventory {
        items: catalog
            .items
            .iter()
            .map(|definition| PassiveItem {
                kind: definition.kind,
                level: definition.max_level(),
            })
            .collect(),
    };
    let mut rng = StdRng::seed_from_u64(5);
    for _ in 0..10 {
        let offers =
            UpgradePool.generate_choices(&config, &Luck(20), &refs, &passives, &catalog, &mut rng);
        assert!(!offers.is_empty());
        assert!(offers
            .iter()
            .all(|offer| matches!(offer.upgrade_type, UpgradeType::Generic(_))));
    }
    for _ in 0..catalog.get(PassiveType::MoveSpeed).max_level() {
        grant_passive(app.world_mut(), player, PassiveType::MoveSpeed).unwrap();
    }
    assert!(grant_passive(app.world_mut(), player, PassiveType::MoveSpeed).is_err());
}

#[test]
fn offer_rng_is_reproducible_and_never_reacquires_owned_items() {
    let config = WeaponUpgradeConfig::default();
    let catalog = PassiveCatalog::default();
    let weapon = WeaponMeta {
        weapon_type: WeaponType::MagickCircle,
        level: 1,
    };
    let passives = PassiveInventory {
        items: vec![PassiveItem {
            kind: PassiveType::Cooldown,
            level: 1,
        }],
    };
    let mut first = StdRng::seed_from_u64(111);
    let mut second = StdRng::seed_from_u64(111);
    for _ in 0..10 {
        let first_offers = UpgradePool.generate_choices(
            &config,
            &Luck(20),
            &[&weapon],
            &passives,
            &catalog,
            &mut first,
        );
        let second_offers = UpgradePool.generate_choices(
            &config,
            &Luck(20),
            &[&weapon],
            &passives,
            &catalog,
            &mut second,
        );
        assert_eq!(
            first_offers
                .iter()
                .map(|offer| &offer.upgrade_type)
                .collect::<Vec<_>>(),
            second_offers
                .iter()
                .map(|offer| &offer.upgrade_type)
                .collect::<Vec<_>>()
        );
        for offer in first_offers {
            assert!(!matches!(
                offer.upgrade_type,
                UpgradeType::AcquireWeapon(WeaponType::MagickCircle)
            ));
            assert!(!matches!(
                offer.upgrade_type,
                UpgradeType::Passive(PassiveType::Cooldown, 1)
            ));
        }
    }
}

#[test]
fn chest_upgrades_owned_passives_sequentially_and_capped_builds_receive_gold() {
    let mut app = game();
    let player = player(&mut app);
    let cap = app
        .world()
        .resource::<WeaponUpgradeConfig>()
        .max_level(WeaponType::MagickCircle);
    for mut meta in app
        .world_mut()
        .query::<&mut WeaponMeta>()
        .iter_mut(app.world_mut())
    {
        meta.level = cap;
    }
    grant_passive(app.world_mut(), player, PassiveType::Cooldown).unwrap();
    app.world_mut()
        .run_system_once(|mut commands: Commands| {
            spawn_loot(&mut commands, Vec2::ZERO, LootKind::Chest);
        })
        .unwrap();
    step(&mut app, 4);
    assert_eq!(state(&app), GameState::Treasure);
    let rewards = app.world().resource::<PendingTreasure>().rewards.clone();
    let levels: Vec<_> = rewards
        .iter()
        .filter_map(|reward| match reward.upgrade {
            UpgradeType::Passive(PassiveType::Cooldown, level) => Some(level),
            _ => None,
        })
        .collect();
    assert!(!levels.is_empty());
    assert_eq!(levels, (2..2 + levels.len() as u8).collect::<Vec<_>>());
    assert!(rewards.iter().all(|reward| matches!(
        reward.upgrade,
        UpgradeType::Passive(PassiveType::Cooldown, _)
            | UpgradeType::Generic(GenericUpgrade::GoldPickup(_))
    )));
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Enter);
    step(&mut app, 4);
    assert_eq!(
        app.world()
            .get::<PassiveInventory>(player)
            .unwrap()
            .level(PassiveType::Cooldown),
        1 + levels.len() as u8
    );
    while grant_passive(app.world_mut(), player, PassiveType::Cooldown).is_ok() {}
    let gold = app.world().resource::<GameStats>().gold;
    app.world_mut()
        .run_system_once(|mut commands: Commands| {
            spawn_loot(&mut commands, Vec2::ZERO, LootKind::Chest);
        })
        .unwrap();
    step(&mut app, 4);
    let rewards = app.world().resource::<PendingTreasure>().rewards.clone();
    assert!(!rewards.is_empty());
    assert!(rewards.iter().all(|reward| matches!(
        reward.upgrade,
        UpgradeType::Generic(GenericUpgrade::GoldPickup(25))
    )));
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Enter);
    step(&mut app, 4);
    assert_eq!(
        app.world().resource::<GameStats>().gold,
        gold + 25 * rewards.len() as u32
    );
}

#[test]
fn restart_clears_passives_and_restores_character_base_stats() {
    let mut app = game();
    let original = player(&mut app);
    for kind in PassiveType::iter() {
        grant_passive(app.world_mut(), original, kind).unwrap();
    }
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Restarting);
    step(&mut app, 4);
    let fresh = player(&mut app);
    assert_ne!(original, fresh);
    assert!(app
        .world()
        .get::<PassiveInventory>(fresh)
        .unwrap()
        .items
        .is_empty());
    let stats = app.world().get::<PlayerStats>(fresh).unwrap();
    assert_eq!(stats.max_health, 100);
    assert_eq!(stats.move_speed, 150.0);
    assert_eq!(stats.pickup_range, 150.0);
    assert_eq!(stats.cooldown_reduction, 0.0);
}
