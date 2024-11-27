mod combat;
mod components;
mod death;
mod events;
mod experience;
mod menu;
mod physics;
mod resources;
mod systems;
mod types;
mod ui;
mod upgrade;
mod weapon;

use crate::combat::{circle_combat_system, handle_damage, DamageEvent};
use crate::death::{cleanup_marked_entities, death_system};
use crate::events::EntityDeathEvent;
use crate::experience::ExperiencePlugin;
use crate::menu::MenuPlugin;
use crate::physics::PhysicsPlugin;
use crate::resources::{GameState, GameStats, SpawnTimer, WaveConfig};
use crate::systems::{
    enemy_movement, gameplay_movement_system, handle_pause_state, load_textures, quit_game,
    spawn_enemies, spawn_player, universal_input_system,
};
use crate::ui::{cleanup_ui, spawn_ui, update_game_timer, update_health_ui, update_kill_counter};
use crate::weapon::update_circle_magick;
use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
use bevy_prototype_lyon::prelude::ShapePlugin;
use upgrade::UpgradePool;

// First, let's organize our systems into sets for better control
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
enum GameplaySets {
    Input,
    UI,
    Movement,
    Combat,
    Spawning,
    Physics,
    Cleanup,
}

pub struct SurvivorsGamePlugin;

impl Plugin for SurvivorsGamePlugin {
    fn build(&self, app: &mut App) {
        app
            // Resources
            .init_resource::<Time<Virtual>>()
            .init_resource::<GameStats>()
            .init_resource::<SpawnTimer>()
            .init_resource::<WaveConfig>()
            .init_resource::<UpgradePool>()
            // Events
            .add_event::<DamageEvent>()
            .add_event::<EntityDeathEvent>()
            // States
            .insert_state(GameState::Playing)
            // Plugins
            .add_plugins(MenuPlugin)
            .add_plugins(PhysicsPlugin)
            .add_plugins(ExperiencePlugin)
            // Startup systems
            .add_systems(Startup, (load_textures, spawn_player.after(load_textures)))
            // Configure system sets
            .configure_sets(
                Update,
                (
                    GameplaySets::Input,
                    GameplaySets::UI,
                    GameplaySets::Physics,
                    GameplaySets::Movement,
                    GameplaySets::Combat,
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
                    update_circle_magick,
                    death_system,
                )
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
                    // Spawning
                    spawn_enemies
                        .in_set(GameplaySets::Spawning)
                        .run_if(in_state(GameState::Playing)),
                    // Combat
                    circle_combat_system
                        .in_set(GameplaySets::Combat)
                        .run_if(in_state(GameState::Playing)),
                ),
            )
            // UI-related systems
            .add_systems(
                OnEnter(GameState::Playing),
                spawn_ui.in_set(GameplaySets::UI),
            )
            .add_systems(OnExit(GameState::Playing), cleanup_ui)
            .add_systems(
                Update,
                (update_health_ui, update_game_timer, update_kill_counter)
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

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(LogPlugin {
                    level: Level::INFO,                                // Only show INFO and above
                    filter: "wgpu=error,bevy_render=info".to_string(), // Customize per-crate logging
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Survivors-Like Prototype".to_string(),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
        )
        // .add_plugins(DefaultPlugins)
        .add_plugins(ShapePlugin)
        .add_plugins(SurvivorsGamePlugin)
        .run();
}
