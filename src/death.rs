use crate::components::{Enemy, Health, Player};
use crate::events::EntityDeathEvent;
use crate::resources::{GameState, GameStats};
use bevy::prelude::*;

#[derive(Component)]
pub struct MarkedForDeath;

#[derive(Component)]
pub struct MarkedForDespawn;

pub fn death_system(
    mut commands: Commands,
    mut game_stats: ResMut<GameStats>,
    player_query: Query<(Entity, &Health), With<Player>>,
    marked_entities: Query<(Entity, Option<&Transform>, Option<&Enemy>), With<MarkedForDeath>>,
    mut death_events: EventWriter<EntityDeathEvent>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    // Check player death first
    if let Ok((entity, health)) = player_query.get_single() {
        if health.current <= 0.0 {
            commands.entity(entity).insert(MarkedForDespawn);
            death_events.send(EntityDeathEvent {
                entity,
                position: Vec2::ZERO, // Player position if needed
                exp_value: None,
            });
            next_state.set(GameState::GameOver);
            return;
        }
    }

    // Handle marked entities
    for (entity, transform, enemy) in marked_entities.iter() {
        if let Some(_enemy) = enemy {
            game_stats.enemies_killed += 1;
        }

        // Send death event before despawning
        death_events.send(EntityDeathEvent {
            entity,
            position: transform.map_or(Vec2::ZERO, |t| t.translation.truncate()),
            exp_value: enemy.map(|e| e.experience_value),
        });

        // Mark for despawn after death processing
        commands.entity(entity).insert(MarkedForDespawn);
    }
}

pub fn cleanup_marked_entities(
    mut commands: Commands,
    query: Query<Entity, With<MarkedForDespawn>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
