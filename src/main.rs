mod components;
mod resources;
mod systems;
mod menu;
mod types;

use bevy::prelude::*;
use crate::menu::{cleanup_menu, spawn_pause_menu, MenuPlugin};
use crate::resources::{GameState, GameStats, SpawnTimer, UpgradePool, WaveConfig};
use crate::systems::{build_input_schedule, combat_system, enemy_movement, gameplay_movement_system, load_textures, projectile_movement, quit_game, spawn_enemies, spawn_player, universal_input_system};

// First, let's organize our systems into sets for better control
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
enum GameplaySets {
    Input,
    Movement,
    Combat,
    Spawning,
}

pub struct SurvivorsGamePlugin;

// Update the plugin to use these sets and handle state transitions
impl Plugin for SurvivorsGamePlugin {
    fn build(&self, app: &mut App) {
        app
            // Resources
            .init_resource::<GameStats>()
            .init_resource::<SpawnTimer>()
            .init_resource::<WaveConfig>()
            .init_resource::<UpgradePool>()

            // States
            .insert_state(GameState::Playing)

            // Startup systems
            .add_systems(Startup, (
                load_textures,
                spawn_player.after(load_textures),
            ))

            // Gameplay systems in sets
            .configure_sets(Update, (
                GameplaySets::Input,
                GameplaySets::Movement,
                GameplaySets::Combat,
                GameplaySets::Spawning,
            ).chain())

            // Add systems to sets and run them only in Playing state
            .add_systems(Update, (
                gameplay_movement_system
                    .in_set(GameplaySets::Movement)
                    .run_if(in_state(GameState::Playing)),

                (spawn_enemies, enemy_movement)
                    .in_set(GameplaySets::Spawning)
                    .run_if(in_state(GameState::Playing)),

                (combat_system, projectile_movement)
                    .in_set(GameplaySets::Combat)
                    .run_if(in_state(GameState::Playing)),
            ))

            // Menu-related systems
            .add_systems(OnEnter(GameState::Paused), spawn_pause_menu)
            .add_systems(OnExit(GameState::Paused), cleanup_menu)
            .add_systems(OnEnter(GameState::Quit), quit_game)
            .add_systems(OnEnter(GameState::Settings), |mut next_state: ResMut<NextState<GameState>>| {
                // Temporary handling until Settings is implemented
                println!("Settings would be shown here");
                next_state.set(GameState::Playing);
            })

            // Universal input handling
            .add_systems(Update, universal_input_system.in_set(GameplaySets::Input));
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SurvivorsGamePlugin)
        .add_plugins(MenuPlugin)
        .run();
}