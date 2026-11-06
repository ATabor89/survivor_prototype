use crate::menu::{MenuAction, MenuActionComponent, MenuItem, MenuRoot, MenuType};
use crate::resources::GameState;
use crate::tests::{count, game, key, player, state, step};
use crate::weapons::magick_circle::{MagickCircle, PatternType};
use crate::weapons::WeaponMeta;
use bevy::prelude::*;

fn enter_menu(app: &mut App, state: GameState) {
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(state);
    step(app, 2);
}

fn buttons(app: &mut App) -> Vec<Entity> {
    let root = app
        .world_mut()
        .query_filtered::<Entity, With<MenuRoot>>()
        .single(app.world())
        .unwrap();
    let mut children = app.world_mut().query::<&Children>();
    children
        .query(app.world())
        .iter_descendants(root)
        .filter(|entity| app.world().get::<MenuItem>(*entity).is_some())
        .collect()
}

#[test]
fn menu_navigation_preserves_visual_order_and_ignores_unrelated_buttons() {
    let mut app = game();
    enter_menu(&mut app, GameState::Paused);
    let items = buttons(&mut app);
    assert_eq!(items.len(), 3);
    // A new component changes the first button's archetype and query position.
    app.world_mut()
        .entity_mut(items[0])
        .insert(Name::new("Resume"));
    let unrelated = app
        .world_mut()
        .spawn((
            crate::ui_button::button(),
            MenuItem { selected: false },
            MenuActionComponent {
                action: MenuAction::QuitGame,
            },
        ))
        .id();
    key(&mut app, KeyCode::ArrowDown);
    assert!(app.world().get::<MenuItem>(items[1]).unwrap().selected);
    assert!(!app.world().get::<MenuItem>(unrelated).unwrap().selected);
    key(&mut app, KeyCode::ArrowUp);
    assert!(app.world().get::<MenuItem>(items[0]).unwrap().selected);
    key(&mut app, KeyCode::Enter);
    step(&mut app, 2);
    assert_eq!(state(&app), GameState::Playing);
}

#[test]
fn mouse_click_wins_over_simultaneous_keyboard_upgrade_selection() {
    let mut app = game();
    let player = player(&mut app);
    app.world_mut()
        .get_mut::<crate::components::Health>(player)
        .unwrap()
        .current = 50;
    enter_menu(&mut app, GameState::LevelUp);
    let spec = app
        .world()
        .resource::<crate::weapons::weapon_upgrade::WeaponUpgradeConfig>()
        .get_next_upgrades(crate::weapons::WeaponType::MagickCircle, 1)
        .remove(0);
    crate::tests::set_selected_upgrade(
        &mut app,
        crate::upgrade::UpgradeType::Weapon(crate::weapons::WeaponType::MagickCircle, spec),
    );
    let heal = buttons(&mut app)
        .into_iter()
        .find(|entity| !app.world().get::<MenuItem>(*entity).unwrap().selected)
        .unwrap();
    app.world_mut()
        .entity_mut(heal)
        .insert(MenuActionComponent {
            action: MenuAction::SelectUpgrade(crate::menu::UpgradeChoice {
                upgrade_type: crate::upgrade::UpgradeType::Generic(
                    crate::upgrade::GenericUpgrade::HealthPickup(20),
                ),
                description: "Heal input regression".into(),
                rarity: crate::types::Rarity::Common,
            }),
        });
    app.world_mut()
        .entity_mut(heal)
        .insert(crate::ui_button::Pressed);
    key(&mut app, KeyCode::Enter);
    step(&mut app, 4);
    assert_eq!(state(&app), GameState::Playing);
    assert_eq!(
        app.world()
            .get::<crate::components::Health>(player)
            .unwrap()
            .current,
        70
    );
    assert_eq!(
        app.world_mut()
            .query::<&WeaponMeta>()
            .single(app.world())
            .unwrap()
            .level,
        1
    );
}

#[test]
fn repeated_pattern_menus_each_require_fresh_confirmation() {
    let mut app = game();
    let weapon = app
        .world_mut()
        .query_filtered::<Entity, With<WeaponMeta>>()
        .single(app.world())
        .unwrap();
    app.world_mut().get_mut::<WeaponMeta>(weapon).unwrap().level = 2;
    for expected_level in [3, 4] {
        enter_menu(&mut app, GameState::LevelUp);
        let spec = app
            .world()
            .resource::<crate::weapons::weapon_upgrade::WeaponUpgradeConfig>()
            .get_next_upgrades(crate::weapons::WeaponType::MagickCircle, expected_level - 1)
            .remove(0);
        crate::tests::set_selected_upgrade(
            &mut app,
            crate::upgrade::UpgradeType::Weapon(crate::weapons::WeaponType::MagickCircle, spec),
        );
        key(&mut app, KeyCode::Enter);
        step(&mut app, 2);
        assert_eq!(state(&app), GameState::LevelUp);
        assert_eq!(
            app.world().get::<WeaponMeta>(weapon).unwrap().level,
            expected_level - 1
        );
        assert_eq!(
            app.world_mut()
                .query::<&MenuRoot>()
                .single(app.world())
                .unwrap()
                .menu_type,
            MenuType::PatternSelect
        );
        key(&mut app, KeyCode::ArrowDown);
        key(&mut app, KeyCode::Enter);
        step(&mut app, 4);
        assert_eq!(state(&app), GameState::Playing);
        assert_eq!(
            app.world().get::<WeaponMeta>(weapon).unwrap().level,
            expected_level
        );
        assert_eq!(
            app.world()
                .get::<MagickCircle>(weapon)
                .unwrap()
                .patterns
                .last(),
            Some(&PatternType::Binding)
        );
        assert_eq!(count::<MenuRoot>(&mut app), 0);
    }
}

#[test]
fn gamepad_can_navigate_and_confirm_pause_menu() {
    let mut app = game();
    let pad = app.world_mut().spawn(Gamepad::default()).id();
    enter_menu(&mut app, GameState::Paused);
    let items = buttons(&mut app);
    app.world_mut()
        .get_mut::<Gamepad>(pad)
        .unwrap()
        .digital_mut()
        .press(GamepadButton::DPadDown);
    step(&mut app, 1);
    assert!(app.world().get::<MenuItem>(items[1]).unwrap().selected);
    app.world_mut()
        .get_mut::<Gamepad>(pad)
        .unwrap()
        .digital_mut()
        .reset_all();
    app.world_mut()
        .get_mut::<Gamepad>(pad)
        .unwrap()
        .digital_mut()
        .press(GamepadButton::DPadUp);
    step(&mut app, 1);
    app.world_mut()
        .get_mut::<Gamepad>(pad)
        .unwrap()
        .digital_mut()
        .reset_all();
    app.world_mut()
        .get_mut::<Gamepad>(pad)
        .unwrap()
        .digital_mut()
        .press(GamepadButton::South);
    step(&mut app, 1);
    app.world_mut()
        .get_mut::<Gamepad>(pad)
        .unwrap()
        .digital_mut()
        .reset_all();
    step(&mut app, 2);
    assert_eq!(state(&app), GameState::Playing);
}

#[test]
fn hud_shows_twelve_slots_and_pause_reports_the_actual_build() {
    let mut app = game();
    let player = player(&mut app);
    crate::inventory::grant_passive(
        app.world_mut(),
        player,
        crate::inventory::PassiveType::MoveSpeed,
    )
    .unwrap();
    step(&mut app, 3);
    let slots: Vec<_> = app
        .world_mut()
        .query::<&crate::ui::BuildSlot>()
        .iter(app.world())
        .map(|slot| (slot.kind, slot.index))
        .collect();
    assert_eq!(
        slots
            .iter()
            .filter(|(kind, _)| *kind == crate::ui::BuildSlotKind::Weapon)
            .count(),
        6
    );
    assert_eq!(
        slots
            .iter()
            .filter(|(kind, _)| *kind == crate::ui::BuildSlotKind::Passive)
            .count(),
        6
    );
    let labels: Vec<_> = app
        .world_mut()
        .query_filtered::<&Text, With<crate::ui::BuildSlotLabel>>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect();
    assert_eq!(
        labels
            .iter()
            .filter(|text| text.as_str() == "Empty")
            .count(),
        10
    );
    assert!(labels
        .iter()
        .any(|text| text.contains("Magick Circle") && text.contains("Lv. 1")));
    let passive_name = app
        .world()
        .resource::<crate::inventory::PassiveCatalog>()
        .get(crate::inventory::PassiveType::MoveSpeed)
        .name
        .clone();
    assert!(labels
        .iter()
        .any(|text| text.contains(&passive_name) && text.contains("Lv. 1")));
    enter_menu(&mut app, GameState::Paused);
    let readouts: Vec<_> = app
        .world_mut()
        .query_filtered::<&Text, With<crate::menu::PauseReadout>>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect();
    assert!(readouts.iter().any(|text| text.contains(&passive_name)));
    assert!(readouts.iter().any(|text| text.contains("Movement speed")));
    assert!(readouts
        .iter()
        .any(|text| text.contains("Ceremonial") && text.contains("Damage")));
    assert!(app
        .world_mut()
        .query::<&MenuActionComponent>()
        .iter(app.world())
        .any(|action| matches!(action.action, MenuAction::ReturnToTitle)));
}

#[test]
fn final_build_survives_player_cleanup_and_resets_on_restart() {
    let mut app = game();
    let player = player(&mut app);
    crate::inventory::grant_passive(
        app.world_mut(),
        player,
        crate::inventory::PassiveType::MoveSpeed,
    )
    .unwrap();
    step(&mut app, 2);
    app.world_mut()
        .resource_mut::<crate::telemetry::RunTelemetry>()
        .damage_by_weapon
        .insert(crate::weapons::WeaponType::MagickCircle, 1234);
    app.world_mut()
        .get_mut::<crate::combat::DamageCooldown>(player)
        .unwrap()
        .time = -1.0;
    app.world_mut().write_message(crate::combat::DamageEvent {
        target: player,
        amount: 1000,
        source: None,
    });
    step(&mut app, 3);
    assert_eq!(state(&app), GameState::GameOver);
    assert_eq!(count::<crate::components::Player>(&mut app), 0);
    let snapshot = app
        .world()
        .resource::<crate::ui::FinalBuild>()
        .0
        .as_ref()
        .unwrap();
    assert_eq!(snapshot.weapons.len(), 1);
    assert_eq!(snapshot.passives.len(), 1);
    let passive_name = snapshot.passives[0].name.clone();
    let labels: Vec<_> = app
        .world_mut()
        .query_filtered::<&Text, With<crate::menu::FinalBuildReadout>>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect();
    assert!(labels
        .iter()
        .any(|text| text.contains("Magick Circle") && text.contains("1234")));
    assert!(labels.iter().any(|text| text.contains(&passive_name)));
    key(&mut app, KeyCode::Enter);
    step(&mut app, 4);
    assert_eq!(state(&app), GameState::Playing);
    assert!(app.world().resource::<crate::ui::FinalBuild>().0.is_none());
}
