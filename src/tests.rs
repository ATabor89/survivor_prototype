use super::*;
use bevy::asset::AssetPlugin;
use bevy::ecs::system::RunSystemOnce;
use bevy::gizmos::GizmoPlugin;
use bevy::mesh::MeshPlugin;
use bevy::scene::ScenePlugin;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use bevy::transform::TransformPlugin;
use bevy_rapier2d::prelude::*;
use components::{Enemy, Health, Player};
use experience::{Experience, ExperienceOrb};
use menu::{MenuAction, MenuActionComponent, MenuItem, MenuRoot, MenuType};
use std::time::Duration;
use weapons::magick_circle::{spawn_magick_circle_attack, MagickCircle, PatternType};
use weapons::{WeaponDamage, WeaponMeta};

// Run the actual game plugins and Rapier without a window/GPU. Input is supplied
// directly so each test controls the exact frame in which a key is pressed.
fn game() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: format!("{}/assets", env!("CARGO_MANIFEST_DIR")),
            ..default()
        },
        TransformPlugin,
        StatesPlugin,
        MeshPlugin,
        ScenePlugin,
        GizmoPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<TextureAtlasLayout>()
    .init_resource::<ButtonInput<KeyCode>>()
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )))
    .add_plugins(SurvivorsGamePlugin);
    app.finish();
    app.cleanup();
    step(&mut app, 3);
    app
}

fn step(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
    }
}

fn key(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
    step(app, 1);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(key);
}

fn player(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(app.world())
        .unwrap()
}

fn count<T: Component>(app: &mut App) -> usize {
    app.world_mut()
        .query_filtered::<Entity, With<T>>()
        .iter(app.world())
        .count()
}

fn state(app: &App) -> GameState {
    app.world().resource::<State<GameState>>().get().clone()
}

fn circle(app: &mut App, pattern: PatternType, position: Vec3) -> Entity {
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            spawn_magick_circle_attack(&mut commands, position, 10, 64.0, pattern, None)
        })
        .unwrap()
}

#[test]
fn overlaps_persist_between_staggered_damage_ticks_and_clear_on_exit() {
    let mut app = game();
    let enemy = app
        .world_mut()
        .spawn((
            Enemy {
                speed: 0.0,
                experience_value: 1,
            },
            Health {
                current: 1000,
                maximum: 1000,
            },
            Transform::from_xyz(250.0, 0.0, 0.0),
        ))
        .id();
    let first = circle(
        &mut app,
        PatternType::Banishment,
        Vec3::new(250.0, 0.0, 0.0),
    );
    let second = circle(&mut app, PatternType::Binding, Vec3::new(260.0, 0.0, 0.0));
    app.world_mut()
        .get_mut::<weapons::AreaEffect>(second)
        .unwrap()
        .last_tick = 0.25;
    step(&mut app, 5);
    for _ in 0..45 {
        step(&mut app, 1);
        let overlaps = app.world().resource::<weapons::overlap::ActiveOverlaps>();
        assert_eq!(overlaps.overlaps.len(), 1);
        assert!(overlaps
            .overlaps
            .contains_key(&weapons::overlap::CirclePair::new(first, second)));
        assert_eq!(
            app.world()
                .get::<weapons::overlap::OverlappingPatterns>(enemy)
                .unwrap()
                .circle_entities
                .len(),
            2
        );
    }
    // Continuous tracking must not turn the 0.5-second damage cadence into per-frame damage.
    assert_eq!(app.world().get::<Health>(enemy).unwrap().current, 990);
    app.world_mut()
        .get_mut::<Transform>(enemy)
        .unwrap()
        .translation
        .x = 600.0;
    step(&mut app, 4);
    assert!(app
        .world()
        .resource::<weapons::overlap::ActiveOverlaps>()
        .overlaps
        .is_empty());
    assert!(app
        .world()
        .get::<weapons::overlap::OverlappingPatterns>(enemy)
        .is_none());
}

#[test]
fn each_pattern_keeps_its_shape_effect_and_recursive_cleanup() {
    for pattern in [
        PatternType::Banishment,
        PatternType::Binding,
        PatternType::Invocation,
        PatternType::Manifestation,
        PatternType::Protection,
    ] {
        let mut app = game();
        let position = if pattern == PatternType::Protection {
            Vec3::ZERO
        } else {
            Vec3::new(250.0, 0.0, 0.0)
        };
        let enemy = app
            .world_mut()
            .spawn((
                Enemy {
                    speed: 0.0,
                    experience_value: 1,
                },
                Health {
                    current: 1000,
                    maximum: 1000,
                },
                Transform::from_translation(position + Vec3::X * 20.0),
            ))
            .id();
        let attack = circle(&mut app, pattern, position);
        assert_eq!(
            count::<weapons::Sigil>(&mut app),
            pattern.visuals().num_sigils as usize
        );
        assert!(app
            .world()
            .get::<bevy_prototype_lyon::entity::Shape>(attack)
            .is_some());
        step(&mut app, 40);
        match pattern {
            PatternType::Banishment => {
                assert!(app.world().get::<Health>(enemy).unwrap().current <= 990)
            }
            PatternType::Manifestation => {
                assert!(app.world().get::<Health>(enemy).unwrap().current <= 985)
            }
            PatternType::Binding => {
                assert!(app.world().get::<weapons::BindingEffect>(enemy).is_some())
            }
            PatternType::Invocation => assert!(
                app.world().get::<Transform>(enemy).unwrap().translation.x < position.x + 20.0
            ),
            PatternType::Protection => {
                let player = player(&mut app);
                assert!(app.world().get::<weapons::ProtectionBuff>(player).is_some());
            }
        }
        // Exercise the lifetime system and Bevy's new linked-child despawning.
        app.world_mut()
            .get_mut::<weapons::Lifetime>(attack)
            .unwrap()
            .timer
            .set_elapsed(Duration::from_secs(4));
        step(&mut app, 2);
        assert!(app.world().get_entity(attack).is_err());
        assert_eq!(count::<weapons::Sigil>(&mut app), 0);
    }
}

#[test]
fn startup_movement_pause_and_resume() {
    let mut app = game();
    let player = player(&mut app);
    assert_eq!(app.world().get::<Health>(player).unwrap().current, 100);
    assert_eq!(count::<MagickCircle>(&mut app), 1);
    assert_eq!(count::<ui::GameUI>(&mut app), 1);
    let config = app
        .world_mut()
        .query::<&RapierConfiguration>()
        .single(app.world())
        .unwrap();
    assert_eq!(config.gravity, Vec2::ZERO);

    let start = app.world().get::<Transform>(player).unwrap().translation;
    key(&mut app, KeyCode::KeyD);
    assert!(app.world().get::<Transform>(player).unwrap().translation.x > start.x);

    key(&mut app, KeyCode::Escape);
    step(&mut app, 1);
    assert_eq!(state(&app), GameState::Paused);
    let paused_time = app.world().resource::<Time<Virtual>>().elapsed();
    let paused_position = app.world().get::<Transform>(player).unwrap().translation;
    key(&mut app, KeyCode::KeyD);
    step(&mut app, 4);
    assert_eq!(
        app.world().resource::<Time<Virtual>>().elapsed(),
        paused_time
    );
    assert_eq!(
        app.world().get::<Transform>(player).unwrap().translation,
        paused_position
    );
    assert!(
        !app.world_mut()
            .query::<&RapierConfiguration>()
            .single(app.world())
            .unwrap()
            .physics_pipeline_active
    );

    key(&mut app, KeyCode::Escape);
    step(&mut app, 3);
    assert_eq!(state(&app), GameState::Playing);
    assert!(app.world().resource::<Time<Virtual>>().elapsed() > paused_time);
    assert_eq!(count::<MenuRoot>(&mut app), 0);
    assert_eq!(count::<ui::GameUI>(&mut app), 1);
}

#[test]
fn death_emits_one_orb_and_one_kill() {
    let mut app = game();
    let enemy = app
        .world_mut()
        .spawn((
            Enemy {
                speed: 0.0,
                experience_value: 1,
            },
            Health {
                current: 10,
                maximum: 10,
            },
            Transform::from_xyz(250.0, 0.0, 0.0),
        ))
        .id();
    app.world_mut().write_message(DamageEvent {
        target: enemy,
        amount: 10,
        source: None,
    });
    step(&mut app, 5);
    assert!(app.world().get_entity(enemy).is_err());
    assert_eq!(app.world().resource::<GameStats>().enemies_killed, 1);
    assert_eq!(count::<ExperienceOrb>(&mut app), 1);
    let orb = app
        .world_mut()
        .query::<&ExperienceOrb>()
        .single(app.world())
        .unwrap();
    assert_eq!(orb.value, 66);

    let orb_entity = app
        .world_mut()
        .query_filtered::<Entity, With<ExperienceOrb>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .get_mut::<Transform>(orb_entity)
        .unwrap()
        .translation = Vec3::new(5.0, 0.0, 0.0);
    step(&mut app, 8);
    assert_eq!(count::<ExperienceOrb>(&mut app), 0);
    let player = player(&mut app);
    assert_eq!(app.world().get::<Experience>(player).unwrap().current, 66);
}

#[test]
fn level_up_pattern_selection_requires_a_second_confirmation() {
    let mut app = game();
    let weapon = app
        .world_mut()
        .query_filtered::<Entity, With<WeaponMeta>>()
        .single(app.world())
        .unwrap();
    app.world_mut().get_mut::<WeaponMeta>(weapon).unwrap().level = 2;
    let player = player(&mut app);
    app.world_mut()
        .get_mut::<Experience>(player)
        .unwrap()
        .current = 100;
    step(&mut app, 2);
    assert_eq!(state(&app), GameState::LevelUp);

    // The first offer is the level-3 AddCircle upgrade.
    key(&mut app, KeyCode::Enter);
    step(&mut app, 2);
    assert_eq!(state(&app), GameState::LevelUp);
    assert_eq!(app.world().get::<WeaponMeta>(weapon).unwrap().level, 2);
    assert_eq!(
        app.world()
            .get::<MagickCircle>(weapon)
            .unwrap()
            .patterns
            .len(),
        1
    );
    assert!(app
        .world_mut()
        .query::<&MenuType>()
        .iter(app.world())
        .any(|m| *m == MenuType::PatternSelect));

    // Select Binding by keyboard, then confirm and ensure delivery happens once.
    key(&mut app, KeyCode::ArrowDown);
    key(&mut app, KeyCode::Enter);
    step(&mut app, 4);
    assert_eq!(state(&app), GameState::Playing);
    assert_eq!(app.world().get::<WeaponMeta>(weapon).unwrap().level, 3);
    assert_eq!(
        app.world().get::<MagickCircle>(weapon).unwrap().patterns,
        vec![PatternType::Banishment, PatternType::Binding]
    );
    assert!(!app
        .world()
        .contains_resource::<menu::PendingAddCircleUpgrade>());
    assert_eq!(count::<MenuRoot>(&mut app), 0);
    assert_eq!(count::<ui::GameUI>(&mut app), 1);
}

#[test]
fn mouse_upgrade_and_healing_survive_the_return_to_playing() {
    let mut app = game();
    let player = player(&mut app);
    app.world_mut()
        .get_mut::<Experience>(player)
        .unwrap()
        .current = 100;
    step(&mut app, 2);
    let weapon_button = app.world_mut().query::<(Entity, &MenuActionComponent)>().iter(app.world())
        .find(|(_, action)| matches!(action.action, MenuAction::SelectUpgrade(ref choice) if matches!(choice.upgrade_type, upgrade::UpgradeType::Weapon(..))))
        .map(|(entity, _)| entity).unwrap();
    app.world_mut()
        .entity_mut(weapon_button)
        .insert(Interaction::Pressed);
    step(&mut app, 4);
    assert_eq!(state(&app), GameState::Playing);
    assert_eq!(
        app.world_mut()
            .query::<&WeaponMeta>()
            .single(app.world())
            .unwrap()
            .level,
        2
    );
    assert_eq!(
        app.world_mut()
            .query::<&WeaponDamage>()
            .single(app.world())
            .unwrap()
            .damage_bonus,
        2
    );

    app.world_mut().get_mut::<Health>(player).unwrap().current = 85;
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::LevelUp);
    step(&mut app, 1);
    let heal_button = app.world_mut().query::<(Entity, &MenuActionComponent)>().iter(app.world())
        .find(|(_, action)| matches!(action.action, MenuAction::SelectUpgrade(ref choice) if matches!(choice.upgrade_type, upgrade::UpgradeType::Generic(upgrade::GenericUpgrade::HealthPickup(_)))))
        .map(|(entity, _)| entity).unwrap();
    // Clear keyboard selection to make this exercise the mouse path exclusively.
    for mut item in app
        .world_mut()
        .query::<&mut MenuItem>()
        .iter_mut(app.world_mut())
    {
        item.selected = false;
    }
    app.world_mut()
        .entity_mut(heal_button)
        .insert(Interaction::Pressed);
    step(&mut app, 4);
    assert_eq!(state(&app), GameState::Playing);
    assert_eq!(app.world().get::<Health>(player).unwrap().current, 100);
}
