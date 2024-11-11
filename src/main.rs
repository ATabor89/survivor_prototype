mod components;
mod resources;
mod systems;
mod menu;
mod types;
mod physics;

use crate::menu::{cleanup_menu, spawn_pause_menu, MenuPlugin};
use crate::physics::{handle_collision_events, setup_physics_bodies, PhysicsPlugin};
use crate::resources::{GameState, GameStats, SpawnTimer, UpgradePool, WaveConfig};
use crate::systems::{combat_system, death_system, enemy_movement, gameplay_movement_system, load_textures, quit_game, spawn_enemies, spawn_player, universal_input_system};
use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;

// First, let's organize our systems into sets for better control
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
enum GameplaySets {
    Input,
    Movement,
    Combat,
    Spawning,
    Physics,
}

pub struct SurvivorsGamePlugin;

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

            // Configure system sets
            .configure_sets(Update, (
                GameplaySets::Input,
                GameplaySets::Physics,
                GameplaySets::Movement,
                GameplaySets::Combat,
                GameplaySets::Spawning,
            ).chain())

            // Add systems to sets and run them only in Playing state
            .add_systems(Update, (
                // Input
                (gameplay_movement_system, enemy_movement)
                    .in_set(GameplaySets::Movement)
                    .run_if(in_state(GameState::Playing)),

                // Physics and combat response
                (
                    setup_physics_bodies,
                    (handle_collision_events, death_system).chain(),
                )
                    .in_set(GameplaySets::Physics)
                    .run_if(in_state(GameState::Playing)),

                // Spawning
                spawn_enemies
                    .in_set(GameplaySets::Spawning)
                    .run_if(in_state(GameState::Playing)),

                // Combat
                combat_system
                    .in_set(GameplaySets::Combat)
                    .run_if(in_state(GameState::Playing)),
            ))

            // Menu-related systems
            .add_systems(OnEnter(GameState::Paused), spawn_pause_menu)
            .add_systems(OnExit(GameState::Paused), cleanup_menu)
            .add_systems(OnEnter(GameState::Quit), quit_game)
            .add_systems(OnEnter(GameState::Settings), |mut next_state: ResMut<NextState<GameState>>| {
                println!("Settings would be shown here");
                next_state.set(GameState::Playing);
            })

            // Universal input handling
            .add_systems(Update, universal_input_system.in_set(GameplaySets::Input));
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(LogPlugin {
            level: Level::INFO,  // Only show INFO and above
            filter: "wgpu=error,bevy_render=info".to_string(),  // Customize per-crate logging
            ..default()
        }))
        // .add_plugins(DefaultPlugins)
        .add_plugins(PhysicsPlugin)
        .add_plugins(SurvivorsGamePlugin)
        .add_plugins(MenuPlugin)
        .run();
}