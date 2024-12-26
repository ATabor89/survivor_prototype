use crate::combat::DamageCooldown;
use crate::components::{
    AreaMultiplier, Combat, CooldownReduction, DamageMultiplier, Enemy, Experience, Health, Player,
};
use crate::resources::{GameState, GameTextures, SpawnTimer, WaveConfig};
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use crate::weapon::{StartingWeapon, WeaponType};

// Startup system to load textures and create atlas layouts
pub fn load_textures(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // Load the sprite sheet images
    let player_texture: Handle<Image> = asset_server.load("sprites/player.png");
    let enemy_texture: Handle<Image> = asset_server.load("sprites/enemies.png");
    let projectile_texture: Handle<Image> = asset_server.load("sprites/projectiles.png");

    // Create texture atlas layouts
    let player_layout = TextureAtlasLayout::from_grid(
        UVec2::new(32, 32), // Sprite size
        1,
        1,    // Grid size (1x1 for now)
        None, // Padding
        None, // Offset
    );

    let enemy_layout = TextureAtlasLayout::from_grid(
        UVec2::new(32, 32), // Sprite size
        2,
        1,    // Grid size (2 types of enemies)
        None, // Padding
        None, // Offset
    );

    let projectile_layout = TextureAtlasLayout::from_grid(
        UVec2::new(16, 16), // Sprite size
        2,
        1,    // Grid size (2 types of projectiles)
        None, // Padding
        None, // Offset
    );

    // Store the layouts
    let player_layout_handle = texture_atlas_layouts.add(player_layout);
    let enemy_layout_handle = texture_atlas_layouts.add(enemy_layout);
    let projectile_layout_handle = texture_atlas_layouts.add(projectile_layout);

    // Store handles in our resource
    commands.insert_resource(GameTextures {
        player: player_texture,
        enemies: enemy_texture,
        projectiles: projectile_texture,
        player_layout: player_layout_handle,
        enemies_layout: enemy_layout_handle,
        projectiles_layout: projectile_layout_handle,
    });
}

pub fn quit_game(mut app_exit_events: ResMut<Events<AppExit>>) {
    info!("quit_game system called");
    app_exit_events.send(AppExit::Success);
}

// Each input handler is focused but can check game state
pub fn gameplay_movement_system(
    game_state: Res<State<GameState>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&Player, &mut Transform)>,
) {
    // Only process movement in Playing state
    if *game_state.get() != GameState::Playing {
        return;
    }

    for (player, mut transform) in query.iter_mut() {
        let mut direction = Vec3::ZERO;

        if keyboard.pressed(KeyCode::KeyW) {
            direction.y += 1.0;
        }
        if keyboard.pressed(KeyCode::KeyS) {
            direction.y -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyA) {
            direction.x -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            direction.x += 1.0;
        }

        if direction != Vec3::ZERO {
            direction = direction.normalize();
            transform.translation += direction * player.speed * time.delta_secs();
        }
    }
}

pub fn universal_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    game_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        match *game_state.get() {
            GameState::Playing => next_state.set(GameState::Paused),
            GameState::Paused => next_state.set(GameState::Playing),
            GameState::Settings => next_state.set(GameState::Playing),
            GameState::MainMenu => next_state.set(GameState::Quit),
            _ => {}
        }
    }
}

pub fn handle_pause_state(
    mut config_query: Query<&mut RapierConfiguration>,
    mut time: ResMut<Time<Virtual>>,
    game_state: Res<State<GameState>>,
) {
    if let Ok(mut rapier_config) = config_query.get_single_mut() {
        match game_state.get() {
            GameState::Playing => {
                // Resume physics and time
                rapier_config.physics_pipeline_active = true;
                time.unpause();
            }
            GameState::Paused | GameState::LevelUp | GameState::GameOver => {
                // Pause physics and time for any state where the game should be frozen
                rapier_config.physics_pipeline_active = false;
                time.pause();
            }
            _ => {} // Other states don't affect physics/time
        }
    }
}

pub fn spawn_player(mut commands: Commands, game_textures: Res<GameTextures>) {
    commands.spawn((
        Player {
            speed: 150.0,
            magnet_strength: 150.0, // Base vacuum range
            magnet_speed: 1.0,      // Base vacuum speed multiplier
        },
        CooldownReduction::default(), // Will be 0.0
        DamageMultiplier::default(),  // Will be 1.0
        AreaMultiplier::default(),    // Will be 1.0
        Sprite {
            image: game_textures.player.clone(),
            custom_size: Some(Vec2::new(32.0, 32.0)),
            texture_atlas: Some(TextureAtlas {
                layout: game_textures.player_layout.clone(),
                index: 0,
            }),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
        Combat {
            attack_damage: 10.0,
            attack_speed: 0.60,
            last_attack: 0.0,
        },
        Experience {
            current: 0,
            level: 1,
        },
        // Add Health component here
        Health {
            current: 100.0,
            maximum: 100.0,
        },
        DamageCooldown::default(),
        StartingWeapon(WeaponType::MagickCircle),
    ));

    commands.spawn(Camera2d::default());
}

pub fn spawn_enemies(
    mut commands: Commands,
    game_textures: Res<GameTextures>,
    time: Res<Time<Virtual>>,
    mut timer: ResMut<SpawnTimer>,
    wave_config: Res<WaveConfig>,
    enemy_query: Query<&Enemy>,
    player_query: Query<&Transform, With<Player>>,
) {
    if timer.0.tick(time.delta()).just_finished()
        && enemy_query.iter().count() < wave_config.max_enemies as usize
    {
        // Use get_single() instead of single() to handle missing player gracefully
        let player_transform = match player_query.get_single() {
            Ok(transform) => transform,
            Err(_) => return, // If no player exists, just return
        };

        let spawn_distance = 400.0;
        let random_angle = rand::random::<f32>() * std::f32::consts::TAU;
        let spawn_position = player_transform.translation
            + Vec3::new(
                random_angle.cos() * spawn_distance,
                random_angle.sin() * spawn_distance,
                0.0,
            );

        let sprite_index = if rand::random::<f32>() > 0.5 { 0 } else { 1 };

        commands.spawn((
            Enemy {
                speed: 100.0,
                experience_value: 50,
            },
            Sprite {
                image: game_textures.enemies.clone(),
                custom_size: Some(Vec2::new(32.0, 32.0)),
                texture_atlas: Some(TextureAtlas {
                    layout: game_textures.enemies_layout.clone(),
                    index: sprite_index,
                }),
                ..default()
            },
            Transform::from_translation(spawn_position),
            Health {
                current: 20.0,
                maximum: 20.0,
            },
        ));
    }
}

// Update enemy movement using Rapier's velocity system
pub fn enemy_movement(
    player_query: Query<&Transform, With<Player>>,
    mut enemy_query: Query<(&Transform, &Enemy, &mut Velocity)>,
) {
    if let Ok(player_transform) = player_query.get_single() {
        for (transform, enemy, mut velocity) in enemy_query.iter_mut() {
            let direction = (player_transform.translation - transform.translation).normalize();
            // Reduce speed slightly to make collisions more stable
            velocity.linvel = direction.truncate() * enemy.speed * 0.8;
        }
    }
}
