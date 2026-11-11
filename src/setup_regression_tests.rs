use crate::{
    resources::GameState,
    setup::*,
    tests::{game, player, state, step},
    weapons::{WeaponMeta, WeaponType},
};
use bevy::prelude::*;
use rand::RngExt;

#[test]
fn recipe_roundtrip_validates_unknown_ids_duplicates_and_levels() {
    let app = game();
    let config = RunConfiguration::default();
    let text = ron::ser::to_string(&config).unwrap();
    let mut decoded: RunConfiguration = ron::from_str(&text).unwrap();
    validate_configuration(app.world(), &decoded).unwrap();
    decoded.stage = "missing".into();
    assert!(validate_configuration(app.world(), &decoded).is_err());
    decoded = config.clone();
    decoded.weapons.push(decoded.weapons[0].clone());
    assert!(validate_configuration(app.world(), &decoded).is_err());
    decoded = config.clone();
    decoded.weapons[0].level = 0;
    assert!(validate_configuration(app.world(), &decoded).is_err());
    decoded = config;
    decoded.version = 99;
    assert!(validate_configuration(app.world(), &decoded).is_err());
}

#[test]
fn seeded_streams_repeat_and_loot_does_not_change_wave_rolls() {
    let mut first = RunRng::new(42);
    let mut second = RunRng::new(42);
    let mut other = RunRng::new(43);
    for _ in 0..100 {
        let _: u64 = first.loot.random();
    }
    let a: Vec<u64> = (0..20).map(|_| first.waves.random()).collect();
    let b: Vec<u64> = (0..20).map(|_| second.waves.random()).collect();
    let c: Vec<u64> = (0..20).map(|_| other.waves.random()).collect();
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn configured_run_builds_multiple_weapons_and_passives_and_restart_repeats() {
    let mut app = game();
    let config = RunConfiguration {
        stage: "ash_walk".into(),
        seed: 987654,
        weapons: vec![
            WeaponLoadout {
                weapon: WeaponType::InscribedNeedles,
                level: 3,
                patterns: vec![],
            },
            WeaponLoadout {
                weapon: WeaponType::MagickCircle,
                level: 3,
                patterns: vec![
                    crate::weapons::magick_circle::PatternType::Banishment,
                    crate::weapons::magick_circle::PatternType::Protection,
                ],
            },
        ],
        passives: vec![PassiveLoadout {
            kind: crate::inventory::PassiveType::MaxHealth,
            level: 2,
        }],
        ..default()
    };
    *app.world_mut().resource_mut::<RunConfiguration>() = config;
    start_configured_run(app.world_mut()).unwrap();
    step(&mut app, 4);
    assert_eq!(state(&app), GameState::Playing);
    let owner = player(&mut app);
    assert_eq!(
        app.world()
            .get::<crate::inventory::PassiveInventory>(owner)
            .unwrap()
            .items[0]
            .level,
        2
    );
    assert!(
        app.world()
            .get::<crate::components::Health>(owner)
            .unwrap()
            .maximum
            > 100
    );
    let levels: Vec<_> = app
        .world_mut()
        .query::<&WeaponMeta>()
        .iter(app.world())
        .map(|m| m.level)
        .collect();
    assert_eq!(levels.len(), 2);
    assert!(levels.iter().all(|level| *level == 3));
    let first: u64 = app.world_mut().resource_mut::<RunRng>().upgrades.random();
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Restarting);
    step(&mut app, 4);
    let second: u64 = app.world_mut().resource_mut::<RunRng>().upgrades.random();
    assert_eq!(first, second);
}

#[test]
fn title_cleans_run_actors_and_freezes_clock_until_start() {
    let mut app = game();
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::MainMenu);
    step(&mut app, 5);
    assert_eq!(
        crate::tests::count::<crate::components::Player>(&mut app),
        0
    );
    assert_eq!(crate::tests::count::<WeaponMeta>(&mut app), 0);
    assert!(app.world().resource::<Time<Virtual>>().is_paused());
    step(&mut app, 60);
    assert_eq!(
        app.world()
            .resource::<crate::resources::GameStats>()
            .time_elapsed,
        0.0
    );
    start_configured_run(app.world_mut()).unwrap();
    step(&mut app, 4);
    assert_eq!(state(&app), GameState::Playing);
    assert_eq!(
        crate::tests::count::<crate::components::Player>(&mut app),
        1
    );
}

#[test]
fn telemetry_counts_actual_enemy_health_not_overkill_or_props() {
    let mut app = game();
    let source = app
        .world_mut()
        .spawn(crate::weapons::AttackSource(WeaponType::InscribedNeedles))
        .id();
    let enemy = app
        .world_mut()
        .spawn((
            crate::components::Enemy {
                speed: 0.0,
                experience_value: 0,
            },
            crate::components::Health {
                current: 7,
                maximum: 7,
            },
            Transform::from_xyz(500.0, 0.0, 0.0),
        ))
        .id();
    app.world_mut().write_message(crate::combat::DamageEvent {
        source: Some(source),
        target: enemy,
        amount: 100,
    });
    step(&mut app, 1);
    let telemetry = app.world().resource::<crate::telemetry::RunTelemetry>();
    assert_eq!(telemetry.damage_by_weapon[&WeaponType::InscribedNeedles], 7);
    assert_eq!(telemetry.hits_by_weapon[&WeaponType::InscribedNeedles], 1);
}

#[test]
fn invalid_import_preserves_staged_and_active_recipes_and_explicit_debug_patterns_roundtrip() {
    let mut app = game();
    let staged = app.world().resource::<RunConfiguration>().clone();
    let active = app.world().resource::<ActiveRunConfig>().0.clone();
    let selected_stage = app.world().resource::<crate::content::ActiveStage>().0;
    for input in [
        "not valid RON".to_owned(),
        " ".repeat(64 * 1024 + 1),
        ron::to_string(&RunConfiguration {
            stage: "missing".into(),
            ..staged.clone()
        })
        .unwrap(),
    ] {
        assert!(import_configuration_text(app.world_mut(), &input).is_err());
        assert_eq!(*app.world().resource::<RunConfiguration>(), staged);
        assert_eq!(app.world().resource::<ActiveRunConfig>().0, active);
        assert_eq!(
            app.world().resource::<crate::content::ActiveStage>().0,
            selected_stage
        );
    }
    let patterns = vec![crate::weapons::magick_circle::PatternType::Binding; 5];
    for mut circle in app
        .world_mut()
        .query::<&mut crate::weapons::magick_circle::MagickCircle>()
        .iter_mut(app.world_mut())
    {
        circle.patterns = patterns.clone();
    }
    let recipe = current_build_configuration(app.world_mut()).unwrap();
    assert_eq!(recipe.weapons[0].level, 1);
    assert_eq!(recipe.weapons[0].patterns, patterns);
    import_configuration_text(app.world_mut(), &ron::to_string(&recipe).unwrap()).unwrap();
    start_configured_run(app.world_mut()).unwrap();
    step(&mut app, 4);
    let actual = current_build_configuration(app.world_mut()).unwrap();
    assert_eq!(actual.weapons[0].patterns, patterns);
    assert_eq!(actual.weapons[0].level, 1);
}

#[test]
fn exporting_again_replaces_the_previous_recipe_atomically() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "survivors-recipe-{}-{unique}.ron",
        std::process::id()
    ));
    let first = RunConfiguration::default();
    write_configuration_to(&first, &path).unwrap();
    let second = RunConfiguration { seed: 42, ..first };
    write_configuration_to(&second, &path).unwrap();
    let restored: RunConfiguration =
        ron::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    std::fs::remove_file(path).unwrap();
    assert_eq!(restored, second);
}

fn click_setup_button(app: &mut App, label: &str) {
    let button = app
        .world_mut()
        .query_filtered::<(Entity, &Children), With<crate::ui_button::Button>>()
        .iter(app.world())
        .find(|(_, children)| {
            children.iter().any(|child| {
                app.world()
                    .get::<Text>(child)
                    .is_some_and(|text| text.0 == label)
            })
        })
        .map(|(entity, _)| entity)
        .expect("setup button exists");
    app.world_mut()
        .entity_mut(button)
        .insert(crate::ui_button::Pressed);
    step(app, 1);
    if let Ok(mut button) = app.world_mut().get_entity_mut(button) {
        button.remove::<crate::ui_button::Pressed>();
    }
}

#[test]
fn seed_edit_replaces_selection_and_clicking_begin_commits_before_starting() {
    let mut app = game();
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::MainMenu);
    step(&mut app, 3);
    click_setup_button(&mut app, "Edit seed");
    crate::tests::key(&mut app, KeyCode::Numpad4);
    crate::tests::key(&mut app, KeyCode::Numpad2);
    // Editing has not yet changed the selected recipe.
    assert_ne!(app.world().resource::<RunConfiguration>().seed, 42);
    click_setup_button(&mut app, "Begin expedition  [Enter]");
    step(&mut app, 4);
    assert_eq!(state(&app), GameState::Playing);
    assert_eq!(app.world().resource::<ActiveRunConfig>().0.seed, 42);
}

#[test]
fn title_keyboard_and_controller_navigation_select_stages_without_starting() {
    let mut app = game();
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::MainMenu);
    step(&mut app, 3);
    // Initial focus is Begin; wrap past the estate and Quit, then choose the
    // second stage.
    for _ in 0..4 {
        crate::tests::key(&mut app, KeyCode::ArrowDown);
    }
    crate::tests::key(&mut app, KeyCode::Enter);
    assert_eq!(state(&app), GameState::MainMenu);
    assert_eq!(app.world().resource::<RunConfiguration>().stage, "ash_walk");
    let pad = app.world_mut().spawn(Gamepad::default()).id();
    app.world_mut()
        .get_mut::<Gamepad>(pad)
        .unwrap()
        .digital_mut()
        .press(GamepadButton::DPadRight);
    step(&mut app, 1);
    app.world_mut()
        .get_mut::<Gamepad>(pad)
        .unwrap()
        .digital_mut()
        .clear();
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
        .clear();
    assert_eq!(state(&app), GameState::MainMenu);
    assert_eq!(
        app.world().resource::<RunConfiguration>().stage,
        "bell_descent"
    );
}
