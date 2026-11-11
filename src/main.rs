mod arena;
#[cfg(test)]
mod arsenal_regression_tests;
mod art;
mod audio_controls;
mod autoplay;
mod combat;
mod components;
mod content;
mod death;
#[cfg(feature = "dev-tools")]
mod dev_tools;
mod enemy_behavior;
mod estate;
mod events;
mod expedition;
mod experience;
mod feedback;
mod game_audio;
mod headless;
mod inventory;
mod loot;
mod menu;
mod physics;
mod profile;
mod resources;
mod run;
mod setup;
mod storage;
mod systems;
mod telemetry;
mod types;
mod ui;
mod ui_button;
mod upgrade;
mod waves;
mod weapons;

#[cfg(all(test, feature = "dev-tools"))]
mod debug_regression_tests;
#[cfg(test)]
mod encounter_regression_tests;
#[cfg(test)]
mod enemy_behavior_regression_tests;
#[cfg(test)]
mod expedition_regression_tests;
#[cfg(test)]
mod inventory_regression_tests;
#[cfg(test)]
mod loot_regression_tests;
#[cfg(test)]
mod map_regression_tests;
#[cfg(test)]
mod menu_regression_tests;
#[cfg(test)]
mod physics_regression_tests;
#[cfg(test)]
mod projectile_regression_tests;
#[cfg(test)]
mod run_regression_tests;
#[cfg(test)]
mod setup_regression_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod weapon_regression_tests;

use crate::combat::{handle_damage, DamageEvent};
use crate::death::{cleanup_marked_entities, death_system};
use crate::events::EntityDeathEvent;
use crate::experience::ExperiencePlugin;
use crate::menu::{GenericUpgradeConfirmedEvent, MenuPlugin};
use crate::physics::PhysicsPlugin;
use crate::resources::{GameState, GameStats, SpawnTimer, WaveConfig};
use crate::systems::{
    enemy_movement, gameplay_movement_system, handle_pause_state, load_textures, quit_game,
    spawn_player, universal_input_system,
};
use crate::ui::{cleanup_ui, spawn_ui, update_game_timer, update_health_ui, update_kill_counter};
use crate::upgrade::handle_generic_upgrade;
use crate::weapons::WeaponPlugin;
use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
use bevy_prototype_lyon::prelude::ShapePlugin;
use upgrade::UpgradePool;

// First, let's organize our systems into sets for better control
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
enum GameplaySets {
    Input,
    UI,
    Weapons,
    Movement,
    Combat,
    Experience,
    Progress,
    Spawning,
    Physics,
    Cleanup,
}

pub struct SurvivorsGamePlugin;

impl Plugin for SurvivorsGamePlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "dev-tools")]
        app.add_plugins(dev_tools::DebugActionsPlugin);
        app
            // Resources
            .init_resource::<Time<Virtual>>()
            .init_resource::<GameStats>()
            .init_resource::<SpawnTimer>()
            .init_resource::<WaveConfig>()
            .init_resource::<UpgradePool>()
            .init_resource::<telemetry::RunTelemetry>()
            .init_resource::<game_audio::AudioSettings>()
            // Events
            .add_message::<DamageEvent>()
            .add_message::<combat::DamageApplied>()
            .add_message::<EntityDeathEvent>()
            .add_message::<GenericUpgradeConfirmedEvent>()
            // States
            .insert_state(GameState::Playing)
            // Plugins
            .add_plugins(ui_button::UiButtonPlugin)
            .add_plugins(MenuPlugin)
            .add_plugins(PhysicsPlugin)
            .add_plugins(ExperiencePlugin)
            .add_plugins(WeaponPlugin)
            .add_plugins((arena::ArenaPlugin, run::RunPlugin, waves::WavesPlugin))
            .add_plugins(loot::LootPlugin)
            .add_plugins(feedback::FeedbackPlugin)
            .add_plugins(audio_controls::AudioControlsPlugin)
            .add_plugins(enemy_behavior::EnemyBehaviorPlugin)
            .add_plugins((
                inventory::InventoryPlugin,
                setup::SetupPlugin,
                art::ArtPlugin,
            ))
            .add_plugins((
                profile::ProfilePlugin,
                expedition::ExpeditionPlugin,
                estate::EstatePlugin,
            ))
            // Startup systems
            .add_systems(
                Startup,
                (
                    load_textures,
                    spawn_player
                        .after(load_textures)
                        .run_if(in_state(GameState::Playing)),
                ),
            )
            // Configure system sets
            .configure_sets(
                Update,
                (
                    GameplaySets::Input,
                    GameplaySets::UI,
                    GameplaySets::Physics,
                    GameplaySets::Weapons,
                    GameplaySets::Movement,
                    GameplaySets::Combat,
                    GameplaySets::Experience,
                    GameplaySets::Progress,
                    GameplaySets::Spawning,
                    GameplaySets::Cleanup,
                )
                    .chain(),
            )
            // Add systems by set
            .add_systems(
                Update,
                (
                    // Combat
                    handle_damage,
                    death_system,
                )
                    .chain()
                    .in_set(GameplaySets::Combat)
                    .after(GameplaySets::Physics)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                cleanup_marked_entities
                    .in_set(GameplaySets::Cleanup)
                    .run_if(in_state(GameState::Playing)),
            )
            // Add systems to sets and run them only in Playing state
            .add_systems(
                Update,
                (
                    // Input
                    (gameplay_movement_system, enemy_movement)
                        .in_set(GameplaySets::Movement)
                        .run_if(in_state(GameState::Playing)),
                ),
            )
            .add_systems(Update, handle_generic_upgrade)
            // UI-related systems
            .add_systems(
                OnEnter(GameState::Playing),
                spawn_ui.in_set(GameplaySets::UI),
            )
            .add_systems(OnExit(GameState::Playing), cleanup_ui)
            .add_systems(
                Update,
                (
                    update_health_ui,
                    update_game_timer,
                    update_kill_counter,
                    ui::update_run_status,
                    ui::update_boss_status,
                    ui::update_build_slots,
                )
                    .in_set(GameplaySets::UI)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnEnter(GameState::Quit), quit_game)
            .add_systems(
                OnEnter(GameState::Settings),
                |mut next_state: ResMut<NextState<GameState>>| {
                    println!("Settings would be shown here");
                    next_state.set(GameState::Playing);
                },
            )
            // Universal input handling
            .add_systems(Update, universal_input_system.in_set(GameplaySets::Input))
            .add_systems(
                Update,
                handle_pause_state
                    .in_set(GameplaySets::Input)
                    .before(GameplaySets::Physics),
            );
    }
}

fn main() -> std::process::ExitCode {
    // `survivors_prototype simulate ...` plays headless runs and exits; any
    // other invocation opens the game.
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("simulate") {
        return autoplay::cli::run(args);
    }
    match play() {
        AppExit::Success => std::process::ExitCode::SUCCESS,
        AppExit::Error(code) => std::process::ExitCode::from(code.get()),
    }
}

fn play() -> AppExit {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(LogPlugin {
                level: Level::INFO,                                // Only show INFO and above
                filter: "wgpu=error,bevy_render=info".to_string(), // Customize per-crate logging
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Survivors-Like Prototype".to_string(),
                    resolution: (1920, 1080).into(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
    )
    .add_plugins(ShapePlugin)
    .add_plugins(SurvivorsGamePlugin)
    .add_plugins(game_audio::GameAudioPlugin);
    // Headless gameplay harnesses start in Playing; the executable opens setup.
    app.insert_state(GameState::MainMenu);
    // Likewise, only the executable reads and writes saved progress on disk.
    let save_path = std::path::Path::new(profile::SAVE_PATH);
    app.insert_resource(profile::Profile::load_or_default(save_path))
        .insert_resource(profile::ProfileStore::at(save_path));
    #[cfg(feature = "dev-tools")]
    app.add_plugins((
        bevy_egui::EguiPlugin {
            // Both UI systems share the stage camera. Order debug tools after
            // Bevy UI so modal backdrops cannot cover the persistent panel.
            ui_render_order: bevy_egui::UiRenderOrder::EguiAboveBevyUi,
            ..default()
        },
        dev_tools::DebugUiPlugin,
    ));
    app.run()
}
