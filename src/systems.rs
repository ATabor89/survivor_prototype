use crate::combat::DamageCooldown;
use crate::components::{
    AreaMultiplier, CooldownReduction, DamageMultiplier, Enemy, Health, Luck, Player,
};
use crate::experience::Experience;
use crate::resources::{GameState, GameTextures};
use crate::weapons::{BindingEffect, InvocationPull, StartingWeapon, WeaponType};
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

// Startup system to load textures and create atlas layouts
pub fn load_textures(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // Load the sprite sheet images
    let player_texture: Handle<Image> = asset_server.load("sprites/occult-characters-v1.png");
    let enemy_texture = player_texture.clone();

    // Create texture atlas layouts
    let player_layout = TextureAtlasLayout::from_grid(
        UVec2::new(887, 887), // Two square cells in the generated 1774x887 atlas.
        2,
        1,    // One row: player at cell 0, enemy at cell 1.
        None, // Padding
        None, // Offset
    );

    let enemy_layout = TextureAtlasLayout::from_grid(
        UVec2::new(887, 887),
        2,
        1,    // Shared character atlas; enemies use cell 1.
        None, // Padding
        None, // Offset
    );

    // Store the layouts
    let player_layout_handle = texture_atlas_layouts.add(player_layout);
    let enemy_layout_handle = texture_atlas_layouts.add(enemy_layout);

    // Store handles in our resource
    commands.insert_resource(GameTextures {
        player: player_texture,
        enemies: enemy_texture,
        player_layout: player_layout_handle,
        enemies_layout: enemy_layout_handle,
    });
}

pub fn quit_game(mut app_exit_events: MessageWriter<AppExit>) {
    info!("quit_game system called");
    app_exit_events.write(AppExit::Success);
}

// Each input handler is focused but can check game state
pub fn gameplay_movement_system(
    game_state: Res<State<GameState>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    gamepads: Query<&Gamepad>,
    mut query: Query<(&Player, &mut KinematicCharacterController)>,
    #[cfg(feature = "dev-tools")] debug: Res<crate::dev_tools::DebugSession>,
) {
    // Only process movement in Playing state
    if *game_state.get() != GameState::Playing {
        return;
    }

    for (player, mut controller) in query.iter_mut() {
        let mut direction = Vec2::ZERO;
        #[cfg(feature = "dev-tools")]
        let keyboard_enabled = !(debug.open && debug.wants_keyboard_input);
        #[cfg(not(feature = "dev-tools"))]
        let keyboard_enabled = true;

        if keyboard_enabled && keyboard.pressed(KeyCode::KeyW) {
            direction.y += 1.0;
        }
        if keyboard_enabled && keyboard.pressed(KeyCode::KeyS) {
            direction.y -= 1.0;
        }
        if keyboard_enabled && keyboard.pressed(KeyCode::KeyA) {
            direction.x -= 1.0;
        }
        if keyboard_enabled && keyboard.pressed(KeyCode::KeyD) {
            direction.x += 1.0;
        }

        for gamepad in &gamepads {
            let stick = gamepad.left_stick();
            if stick.length_squared() > 0.04 {
                direction += stick;
            }
        }
        controller.translation =
            Some(direction.clamp_length_max(1.0) * player.speed * time.delta_secs());
    }
}

pub fn universal_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    game_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    #[cfg(feature = "dev-tools")] debug: Res<crate::dev_tools::DebugSession>,
) {
    #[cfg(feature = "dev-tools")]
    if debug.open || debug.consumed_pause_input {
        return;
    }
    if keyboard.just_pressed(KeyCode::Escape)
        || gamepads
            .iter()
            .any(|pad| pad.just_pressed(GamepadButton::Start))
    {
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
    if let Ok(mut rapier_config) = config_query.single_mut() {
        match game_state.get() {
            #[cfg(feature = "dev-tools")]
            GameState::DebugTools => {
                rapier_config.physics_pipeline_active = false;
                time.pause();
            }
            GameState::Playing => {
                // Resume physics and time
                rapier_config.physics_pipeline_active = true;
                time.unpause();
            }
            GameState::Paused
            | GameState::LevelUp
            | GameState::Treasure
            | GameState::GameOver
            | GameState::Victory
            | GameState::Restarting => {
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
        crate::run::RunEntity,
        Name::new("Occultist"),
        Player {
            speed: 150.0,
            magnet_strength: 150.0, // Base vacuum range
            magnet_speed: 1.0,      // Base vacuum speed multiplier
        },
        CooldownReduction::default(), // Will be 0.0
        DamageMultiplier::default(),  // Will be 1.0
        AreaMultiplier::default(),    // Will be 1.0
        Luck::default(),
        Sprite {
            image: game_textures.player.clone(),
            custom_size: Some(Vec2::splat(48.0)),
            texture_atlas: Some(TextureAtlas {
                layout: game_textures.player_layout.clone(),
                index: 0,
            }),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
        Experience {
            current: 0,
            level: 1,
        },
        // Add Health component here
        Health {
            current: 100,
            maximum: 100,
        },
        DamageCooldown::default(),
        StartingWeapon(WeaponType::MagickCircle),
    ));
}

pub fn enemy_movement(
    player_query: Query<&Transform, With<Player>>,
    mut enemy_query: Query<
        (Entity, &Transform, &Enemy, &mut Velocity),
        Without<crate::waves::RingMember>,
    >,
    binding_query: Query<&BindingEffect>,
    pull_query: Query<&InvocationPull>,
) {
    if let Ok(player_transform) = player_query.single() {
        for (entity, transform, enemy, mut velocity) in enemy_query.iter_mut() {
            let direction =
                (player_transform.translation - transform.translation).normalize_or_zero();
            let base_velocity = direction.truncate() * enemy.speed * 0.8;

            // Check if enemy is under binding effect
            let binding_strength = if let Ok(binding) = binding_query.get(entity) {
                binding.strength
            } else {
                0.0
            };

            // Apply movement reduction based on binding strength
            let mut final_velocity = base_velocity * (1.0 - binding_strength);

            // Check if enemy is being pulled by an Invocation circle
            if let Ok(pull) = pull_query.get(entity) {
                // Forced movement overrides pursuit, including cancellation to zero.
                final_velocity = pull.velocity;
            }

            velocity.linear = final_velocity;
        }
    }
}
