use crate::components::{Combat, Enemy, Experience, Health, Player, Projectile};
use crate::resources::{GameState, GameTextures, SpawnTimer, WaveConfig};
use bevy::math::FloatOrd;
use bevy::prelude::*;
use bevy::utils::HashSet;
use bevy_rapier2d::prelude::*;

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
    mut rapier_config: ResMut<RapierConfiguration>,
    mut time: ResMut<Time<Virtual>>,
    game_state: Res<State<GameState>>,
) {
    match game_state.get() {
        GameState::Paused => {
            // Pause physics
            rapier_config.physics_pipeline_active = false;
            // Pause virtual time
            time.pause();
        }
        GameState::Playing => {
            // Resume physics
            rapier_config.physics_pipeline_active = true;
            // Resume virtual time
            time.unpause();
        }
        _ => {}
    }
}

// Optional: Define input system sets for organization
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
enum InputSet {
    Universal,
    Menu,
    Gameplay,
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
    ));

    commands.spawn(Camera2dBundle::default());
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
    if timer.0.tick(time.delta()).just_finished() && enemy_query.iter().count() < wave_config.max_enemies as usize {
        // Use get_single() instead of single() to handle missing player gracefully
        let player_transform = match player_query.get_single() {
            Ok(transform) => transform,
            Err(_) => return, // If no player exists, just return
        };

        let spawn_distance = 400.0;
        let random_angle = rand::random::<f32>() * std::f32::consts::TAU;
        let spawn_position = player_transform.translation +
            Vec3::new(
                random_angle.cos() * spawn_distance,
                random_angle.sin() * spawn_distance,
                0.0,
            );

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

pub fn combat_system(
    mut commands: Commands,
    game_textures: Res<GameTextures>,
    time: Res<Time<Virtual>>,
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
                let velocity = Vect::new(direction.x, direction.y);

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
                    RigidBody::Dynamic,
                    Collider::ball(8.0),
                    LockedAxes::ROTATION_LOCKED,
                    ActiveEvents::COLLISION_EVENTS,
                    CollisionGroups::new(Group::GROUP_3, Group::GROUP_2),
                    Velocity::linear(velocity * 300.0),
                    Dominance::group(5),
                ));

                combat.last_attack = time.elapsed_seconds();
            }
        }
    }
}

// Update enemy movement using Rapier's velocity system
pub fn enemy_movement(
    time: Res<Time>,
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

pub fn death_system(
    mut commands: Commands,
    mut entity_query: ParamSet<(
        Query<(Entity, &Health), With<Player>>,
        Query<(Entity, &Health, &Enemy)>,
    )>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exp_query: Query<&mut Experience>,
) {
    // Check player death
    if let Ok((player_entity, player_health)) = entity_query.p0().get_single() {
        if player_health.current <= 0.0 {
            info!("Player died!");
            commands.entity(player_entity).despawn();
            next_state.set(GameState::GameOver);
            return;
        }
    }

    // Track entities marked for death to ensure we only process them once
    let mut dead_enemies: HashSet<Entity> = HashSet::new();
    let mut total_exp = 0;

    // Collect dead enemies
    for (enemy_entity, health, enemy) in entity_query.p1().iter() {
        if health.current <= 0.0 && !dead_enemies.contains(&enemy_entity) {
            dead_enemies.insert(enemy_entity);
            total_exp += enemy.experience_value;
            info!("Enemy died! Experience gained: {}", enemy.experience_value);
        }
    }

    // Handle experience and despawning
    if !dead_enemies.is_empty() {
        // Grant experience
        if let Ok(mut player_exp) = exp_query.get_single_mut() {
            player_exp.current += total_exp;

            let exp_needed = player_exp.level * 100;
            if player_exp.current >= exp_needed {
                info!("Level up! Current level: {}", player_exp.level + 1);
                player_exp.current -= exp_needed;
                player_exp.level += 1;
                next_state.set(GameState::LevelUp);
            }
        }

        // Despawn dead enemies
        for entity in dead_enemies {
            commands.entity(entity).despawn();
        }
    }
}