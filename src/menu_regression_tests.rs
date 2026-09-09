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
    assert_eq!(items.len(), 2);
    // A new component changes the first button's archetype and query position.
    app.world_mut()
        .entity_mut(items[0])
        .insert(Name::new("Resume"));
    let unrelated = app
        .world_mut()
        .spawn((
            Button,
            MenuItem { selected: false },
            MenuActionComponent {
                action: MenuAction::QuitGame,
            },
        ))
        .id();
    key(&mut app, KeyCode::ArrowDown);
    assert!(app.world().get::<MenuItem>(items[1]).unwrap().selected);
    assert!(!app.world().get::<MenuItem>(unrelated).unwrap().selected);
    key(&mut app, KeyCode::ArrowDown);
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
    let heal = app.world_mut().query::<(Entity, &MenuActionComponent)>()
        .iter(app.world()).find_map(|(entity, action)| {
            matches!(&action.action, MenuAction::SelectUpgrade(choice) if matches!(choice.upgrade_type, crate::upgrade::UpgradeType::Generic(_))).then_some(entity)
        }).unwrap();
    app.world_mut()
        .entity_mut(heal)
        .insert(Interaction::Pressed);
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
