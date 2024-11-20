use crate::components::*;
use crate::death::MarkedForDespawn;
use crate::events::EntityDeathEvent;
use crate::menu::{handle_upgrade_selection, spawn_level_up_menu, standard_menu_navigation, update_menu_buttons};
use crate::resources::GameState;
use bevy::prelude::*;
use bevy::utils::HashSet;
use bevy_rapier2d::prelude::*;

pub struct ExperiencePlugin;

impl Plugin for ExperiencePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                spawn_experience_orbs,
                vacuum_system,
                collect_experience_orbs,
                check_level_up,
            )
                .chain()
                .run_if(in_state(GameState::Playing)),
        )
            .add_systems(OnEnter(GameState::LevelUp), spawn_level_up_menu);
    }
}

// Could move this to a config resource if we want to make it data-driven
fn calculate_experience_needed(level: u32) -> u32 {
    // Simple exponential scaling: each level needs 25% more XP than the last
    // Level 1->2: 100 XP
    // Level 2->3: 125 XP
    // Level 3->4: 156 XP
    // etc.
    let base_xp = 100;
    let scaling = 1.25f32;
    (base_xp as f32 * scaling.powi((level - 1) as i32)) as u32
}

pub fn check_level_up(
    mut player_query: Query<&mut Experience, With<Player>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if let Ok(mut experience) = player_query.get_single_mut() {
        let xp_needed = calculate_experience_needed(experience.level);

        if experience.current >= xp_needed {
            // Bank the leftover XP
            experience.current -= xp_needed;
            experience.level += 1;

            // Trigger level up menu
            next_state.set(GameState::LevelUp);
        }
    }
}

fn spawn_experience_orbs(mut commands: Commands, mut death_events: EventReader<EntityDeathEvent>) {
    for event in death_events.read() {
        if let Some(exp_value) = event.exp_value {
            commands.spawn((
                ExperienceOrb { value: exp_value },
                Vacuumable::default(),
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::srgb(0.5, 0.8, 1.0),
                        custom_size: Some(Vec2::new(8.0, 8.0)),
                        ..default()
                    },
                    transform: Transform::from_translation(event.position.extend(0.0)),
                    ..default()
                },
                // Add Rapier components
                RigidBody::Dynamic,
                Collider::ball(4.0), // Smaller collision radius than visual
                Sensor,              // Make it a sensor so it doesn't affect physics
                ActiveEvents::COLLISION_EVENTS,
                CollisionGroups::new(
                    Group::GROUP_4, // Experience orb group
                    Group::GROUP_1, // Player group
                ),
            ));
        }
    }
}

fn vacuum_system(
    mut params: ParamSet<(
        Query<(&Transform, &Player)>,
        Query<(Entity, &mut Transform, &Vacuumable)>,
    )>,
    time: Res<Time>,
) {
    // Collect player data first
    let player_data = {
        let player_query = params.p0();
        if let Ok((transform, player)) = player_query.get_single() {
            Some((
                transform.translation,
                player.magnet_strength,
                player.magnet_speed,
            ))
        } else {
            None
        }
    };

    // Early return if no player
    let (player_pos, magnet_strength, magnet_speed) = match player_data {
        Some(data) => data,
        None => return,
    };

    // Then update vacuumable items
    for (_, mut item_transform, vacuumable) in params.p1().iter_mut() {
        let to_player = player_pos - item_transform.translation;
        let distance = to_player.length();

        if distance < magnet_strength {
            let vacuum_influence = 1.0 - (distance / magnet_strength).powi(2);
            let vacuum_direction = to_player.normalize();

            let speed = vacuumable.base_speed
                * (vacuum_influence * 2.0 + vacuum_influence.powi(3))
                * magnet_speed;

            item_transform.translation += vacuum_direction * speed * time.delta_seconds();
        }
    }
}

fn collect_experience_orbs(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    mut player_query: Query<&mut Experience, With<Player>>,
    orb_query: Query<(Entity, &ExperienceOrb), Without<MarkedForDespawn>>, // Don't process already marked orbs
) {
    let mut processed_orbs = HashSet::new(); // Track orbs we've processed this frame

    if let Ok(mut player_exp) = player_query.get_single_mut() {
        for event in collision_events.read() {
            if let CollisionEvent::Started(e1, e2, _) = event {
                // Check if one entity is an experience orb
                let (orb_entity, exp_orb) = if let Ok(query_result) = orb_query.get(*e1) {
                    query_result
                } else if let Ok(query_result) = orb_query.get(*e2) {
                    query_result
                } else {
                    continue;
                };

                // Skip if we've already processed this orb
                if processed_orbs.contains(&orb_entity) {
                    continue;
                }

                info!("Collected {} experience", exp_orb.value);
                player_exp.current += exp_orb.value;
                processed_orbs.insert(orb_entity);
                commands.entity(orb_entity).insert(MarkedForDespawn);
            }
        }
    }
}
