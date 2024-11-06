use bevy::math::FloatOrd;
use bevy::prelude::*;
use crate::components::{Combat, Enemy, Experience, Health, Player, Projectile, Velocity};
use crate::resources::{GameState, GameTextures, SpawnTimer, WaveConfig};

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
        1, 1,                  // Grid size (1x1 for now)
        None,                  // Padding
        None,                  // Offset
    );

    let enemy_layout = TextureAtlasLayout::from_grid(
        UVec2::new(32, 32), // Sprite size
        2, 1,                  // Grid size (2 types of enemies)
        None,                  // Padding
        None,                  // Offset
    );

    let projectile_layout = TextureAtlasLayout::from_grid(
        UVec2::new(16, 16), // Sprite size
        2, 1,                  // Grid size (2 types of projectiles)
        None,                  // Padding
        None,                  // Offset
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

        if keyboard.pressed(KeyCode::KeyW) { direction.y += 1.0; }
        if keyboard.pressed(KeyCode::KeyS) { direction.y -= 1.0; }
        if keyboard.pressed(KeyCode::KeyA) { direction.x -= 1.0; }
        if keyboard.pressed(KeyCode::KeyD) { direction.x += 1.0; }

        if direction != Vec3::ZERO {
            direction = direction.normalize();
            transform.translation += direction * player.speed * time.delta_seconds();
        }
    }
}

pub fn menu_input_system(
    game_state: Res<State<GameState>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    // Only process menu inputs in MainMenu state
    if *game_state.get() != GameState::MainMenu {
        return;
    }

    if keyboard.just_pressed(KeyCode::Enter) {
        next_state.set(GameState::Playing);
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
            GameState::Settings => next_state.set(GameState::Playing), // Return to game from settings
            GameState::MainMenu => next_state.set(GameState::Quit),
            _ => {}
        }
    }
}

// Optional: Define input system sets for organization
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
enum InputSet {
    Universal,
    Menu,
    Gameplay,
}

// Schedule systems based on their requirements
pub fn build_input_schedule(app: &mut App) {
    app
        .add_systems(Update, (
            universal_input_system,
            menu_input_system,
            gameplay_movement_system,
        ).chain())
        // Optional: Use set configuration for more control
        .configure_sets(Update, (
            InputSet::Universal,
            InputSet::Menu,
            InputSet::Gameplay,
        ).chain());
}

pub fn spawn_player(
    mut commands: Commands,
    game_textures: Res<GameTextures>,
) {
    commands.spawn((
        Player {
            health: 100.0,
            max_health: 100.0,
            speed: 150.0,
        },
        SpriteBundle {
            texture: game_textures.player.clone(),
            sprite: Sprite {
                custom_size: Some(Vec2::new(32.0, 32.0)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        TextureAtlas {
            layout: game_textures.player_layout.clone(),
            index: 0,
        },
        Combat {
            attack_damage: 10.0,
            attack_speed: 1.0,
            last_attack: 0.0,
        },
        Experience {
            current: 0,
            level: 1,
        },
    ));

    commands.spawn(Camera2dBundle::default());
}

pub fn spawn_enemies(
    mut commands: Commands,
    game_textures: Res<GameTextures>,
    time: Res<Time>,
    mut timer: ResMut<SpawnTimer>,
    wave_config: Res<WaveConfig>,
    enemy_query: Query<&Enemy>,
    player_query: Query<&Transform, With<Player>>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        if enemy_query.iter().count() < wave_config.max_enemies as usize {
            let player_transform = player_query.single();

            let spawn_distance = 400.0;
            let random_angle = rand::random::<f32>() * std::f32::consts::TAU;
            let spawn_position = player_transform.translation +
                Vec3::new(
                    random_angle.cos() * spawn_distance,
                    random_angle.sin() * spawn_distance,
                    0.0
                );

            // Randomly select enemy variant (0 or 1)
            let sprite_index = if rand::random::<f32>() > 0.5 { 0 } else { 1 };

            commands.spawn((
                Enemy {
                    health: 50.0,
                    speed: 100.0,
                    experience_value: 10,
                },
                SpriteBundle {
                    texture: game_textures.enemies.clone(),
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(32.0, 32.0)),
                        ..default()
                    },
                    transform: Transform::from_translation(spawn_position),
                    ..default()
                },
                TextureAtlas {
                    layout: game_textures.enemies_layout.clone(),
                    index: sprite_index,
                },
                Health {
                    current: 50.0,
                    maximum: 50.0,
                },
            ));
        }
    }
}

pub fn enemy_movement(
    time: Res<Time>,
    player_query: Query<&Transform, With<Player>>,
    mut enemy_query: Query<(&Enemy, &mut Transform), Without<Player>>,
) {
    let player_transform = player_query.single();

    for (enemy, mut enemy_transform) in enemy_query.iter_mut() {
        let direction = (player_transform.translation - enemy_transform.translation).normalize();
        enemy_transform.translation += direction * enemy.speed * time.delta_seconds();
    }
}

// Combat system updated similarly with new sprite handling
pub fn combat_system(
    mut commands: Commands,
    game_textures: Res<GameTextures>,
    time: Res<Time>,
    mut player_query: Query<(&Transform, &mut Combat), With<Player>>,
    enemy_query: Query<(Entity, &Transform), With<Enemy>>,
) {
    for (player_transform, mut combat) in player_query.iter_mut() {
        if time.elapsed_seconds() - combat.last_attack >= 1.0 / combat.attack_speed {
            if let Some((_, enemy_transform)) = enemy_query
                .iter()
                .min_by_key(|(_, transform)| {
                    FloatOrd((transform.translation - player_transform.translation).length())
                }) {
                let direction = (enemy_transform.translation - player_transform.translation).normalize();
                let velocity = Vec2::new(direction.x, direction.y);

                commands.spawn((
                    Projectile {
                        damage: combat.attack_damage,
                        speed: 300.0,
                    },
                    SpriteBundle {
                        texture: game_textures.projectiles.clone(),
                        sprite: Sprite {
                            custom_size: Some(Vec2::new(16.0, 16.0)),
                            ..default()
                        },
                        transform: Transform::from_translation(player_transform.translation),
                        ..default()
                    },
                    TextureAtlas {
                        layout: game_textures.projectiles_layout.clone(),
                        index: 0,
                    },
                    Velocity(velocity),
                ));

                combat.last_attack = time.elapsed_seconds();
            }
        }
    }
}

pub fn projectile_movement(
    mut commands: Commands,
    time: Res<Time>,
    mut projectile_query: Query<(Entity, &Projectile, &mut Transform, &Velocity)>,
) {
    for (entity, projectile, mut transform, velocity) in projectile_query.iter_mut() {
        transform.translation += Vec3::new(velocity.0.x, velocity.0.y, 0.0) * projectile.speed * time.delta_seconds();

        // Despawn projectiles that have traveled too far
        if transform.translation.length() > 1000.0 {
            commands.entity(entity).despawn();
        }
    }
}
