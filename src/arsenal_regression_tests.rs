use crate::{
    components::Luck,
    inventory::{PassiveCatalog, PassiveInventory, MAX_WEAPONS},
    setup::{start_configured_run, validate_configuration, RunConfiguration, WeaponLoadout},
    tests::{game, key, player, step},
    upgrade::{UpgradePool, UpgradeType},
    weapons::{
        arsenal::Facing, weapon_upgrade::WeaponUpgradeConfig, AddWeaponEvent, WeaponArea,
        WeaponCooldown, WeaponDamage, WeaponMeta, WeaponType,
    },
};
use bevy::prelude::*;
use rand::{rngs::StdRng, SeedableRng};
use strum::IntoEnumIterator;

#[test]
fn every_weapon_roundtrips_and_starts_at_its_maximum_upgrade_level() {
    let mut app = game();
    for kind in WeaponType::iter() {
        let level = app
            .world()
            .resource::<WeaponUpgradeConfig>()
            .max_level(kind);
        let config = RunConfiguration {
            weapons: vec![WeaponLoadout {
                weapon: kind,
                level,
                patterns: vec![],
            }],
            ..default()
        };
        let encoded = ron::ser::to_string(&config).unwrap();
        let decoded: RunConfiguration = ron::from_str(&encoded).unwrap();
        assert_eq!(decoded, config);
        validate_configuration(app.world(), &decoded).unwrap();
        *app.world_mut().resource_mut::<RunConfiguration>() = decoded;
        start_configured_run(app.world_mut()).unwrap();
        step(&mut app, 4);
        let mut weapons = app
            .world_mut()
            .query::<(&WeaponMeta, &WeaponDamage, &WeaponArea, &WeaponCooldown)>();
        let (meta, damage, area, cooldown) = weapons.single(app.world()).unwrap();
        assert_eq!((meta.weapon_type, meta.level), (kind, level));
        assert!(damage.effective_amount(1.0) > 0);
        assert!(area.effective_radius(1.0) > 0.0);
        assert!(cooldown.effective_seconds(0.0) >= 0.1);
    }
}

#[test]
fn expanded_roster_fills_six_slots_and_suppresses_unowned_weapon_offers() {
    let mut app = game();
    let owner = player(&mut app);
    for kind in WeaponType::iter() {
        app.world_mut().write_message(AddWeaponEvent {
            player: owner,
            weapon_type: kind,
        });
    }
    step(&mut app, 2);
    let mut query = app.world_mut().query::<&WeaponMeta>();
    let owned: Vec<_> = query.iter(app.world()).collect();
    assert_eq!(owned.len(), MAX_WEAPONS);
    assert!(WeaponType::iter().count() > owned.len());
    let mut rng = StdRng::seed_from_u64(42);
    for _ in 0..32 {
        let choices = UpgradePool.generate_choices(
            app.world().resource::<WeaponUpgradeConfig>(),
            &Luck(0),
            &owned,
            &PassiveInventory::default(),
            app.world().resource::<PassiveCatalog>(),
            &mut rng,
        );
        assert!(choices
            .iter()
            .all(|choice| !matches!(choice.upgrade_type, UpgradeType::AcquireWeapon(_))));
    }
}

#[test]
fn facing_retains_last_movement_for_automatic_directional_weapons() {
    let mut app = game();
    let owner = player(&mut app);
    assert_eq!(app.world().get::<Facing>(owner).unwrap().0, Vec2::X);
    key(&mut app, KeyCode::KeyA);
    step(&mut app, 5);
    assert_eq!(app.world().get::<Facing>(owner).unwrap().0, Vec2::NEG_X);
    key(&mut app, KeyCode::KeyW);
    step(&mut app, 5);
    assert_eq!(app.world().get::<Facing>(owner).unwrap().0, Vec2::Y);
}
