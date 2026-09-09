use crate::menu::WeaponUpgradeConfirmedEvent;
use crate::tests::{count, game, step};
use crate::weapons::weapon_upgrade::{
    apply_common_weapon_upgrades, WeaponUpgradeChange, WeaponUpgradeConfig, WeaponUpgradeSpec,
};
use crate::weapons::{Attack, WeaponArea, WeaponCooldown, WeaponDamage, WeaponMeta, WeaponType};
use bevy::prelude::*;

#[test]
fn repeated_cooldown_limit_breaks_keep_firing_at_a_positive_cadence() {
    let mut app = game();
    for _ in 0..75 {
        app.world_mut().write_message(WeaponUpgradeConfirmedEvent {
            weapon_type: WeaponType::MagickCircle,
            upgrade_spec: WeaponUpgradeSpec {
                changes: vec![WeaponUpgradeChange::Cooldown(-2)],
            },
        });
    }
    step(&mut app, 1);
    let cooldown = app
        .world_mut()
        .query::<&WeaponCooldown>()
        .single(app.world())
        .unwrap();
    assert_eq!(cooldown.cooldown_bonus, -150);
    assert_eq!(
        cooldown.timer.duration().as_secs_f32(),
        WeaponCooldown::MIN_SECONDS
    );
    step(&mut app, 30);
    // We continue firing, with a bounded rate instead of zero-duration timers.
    assert!((4..=6).contains(&count::<Attack>(&mut app)));
}

#[test]
fn accumulated_upgrade_bonuses_saturate_instead_of_overflowing() {
    let mut app = App::new();
    app.add_message::<WeaponUpgradeConfirmedEvent>()
        .add_systems(Update, apply_common_weapon_upgrades);
    let weapon = app
        .world_mut()
        .spawn((
            WeaponMeta {
                weapon_type: WeaponType::MagickCircle,
                level: 8,
            },
            WeaponDamage {
                base_amount: 10,
                damage_bonus: i32::MAX - 1,
            },
            WeaponArea {
                base_radius: 64.0,
                area_bonus: i32::MAX - 1,
            },
            WeaponCooldown {
                cooldown_bonus: i32::MIN + 1,
                ..default()
            },
        ))
        .id();
    app.world_mut().write_message(WeaponUpgradeConfirmedEvent {
        weapon_type: WeaponType::MagickCircle,
        upgrade_spec: WeaponUpgradeSpec {
            changes: vec![
                WeaponUpgradeChange::Damage(2),
                WeaponUpgradeChange::Area(2),
                WeaponUpgradeChange::Cooldown(-2),
            ],
        },
    });
    app.update();
    assert_eq!(
        app.world()
            .get::<WeaponDamage>(weapon)
            .unwrap()
            .damage_bonus,
        i32::MAX
    );
    assert_eq!(
        app.world().get::<WeaponArea>(weapon).unwrap().area_bonus,
        i32::MAX
    );
    assert_eq!(
        app.world()
            .get::<WeaponCooldown>(weapon)
            .unwrap()
            .cooldown_bonus,
        i32::MIN
    );
}

#[test]
fn every_limit_break_offer_changes_a_supported_stat() {
    let config = WeaponUpgradeConfig::default();
    let offers = config.get_next_upgrades(WeaponType::MagickCircle, 8);
    assert!(!offers.is_empty());
    for offer in offers {
        assert!(!offer.changes.is_empty());
        assert!(offer.changes.iter().all(|change| matches!(change,
            WeaponUpgradeChange::Damage(value)
            | WeaponUpgradeChange::Area(value)
            | WeaponUpgradeChange::Cooldown(value) if *value != 0
        )));
    }
    assert_eq!(
        WeaponUpgradeChange::Damage(2).to_string(),
        "Increase damage by 2%"
    );
    assert_eq!(
        WeaponUpgradeChange::Area(2).to_string(),
        "Increase radius by 2%"
    );
    assert_eq!(
        WeaponUpgradeChange::Cooldown(-2).to_string(),
        "Decrease cooldown by 2%"
    );
}
