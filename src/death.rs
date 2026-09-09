use crate::components::{Enemy, Health, Player};
use crate::events::EntityDeathEvent;
use crate::resources::{GameState, GameStats};
use bevy::prelude::*;

#[derive(Component)]
pub struct MarkedForDeath;

#[derive(Component)]
pub struct MarkedForDespawn;

#[expect(
    clippy::type_complexity,
    reason = "The ECS queries distinguish player death from generic deaths and exclude processed entities."
)]
pub fn death_system(
    mut commands: Commands,
    mut game_stats: ResMut<GameStats>,
    player_query: Query<
        (Entity, &Health, Option<&Transform>),
        (With<Player>, Without<MarkedForDespawn>),
    >,
    marked_entities: Query<
        (Entity, Option<&Transform>, Option<&Enemy>),
        (With<MarkedForDeath>, Without<MarkedForDespawn>),
    >,
    mut death_events: MessageWriter<EntityDeathEvent>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    // Resolve player death first, then finish other deaths in this frame so
    // simultaneous kills still award their XP and appear in the final score.
    let mut dead_player = None;
    if let Ok((entity, health, transform)) = player_query.single() {
        if health.current <= 0 {
            commands.entity(entity).insert(MarkedForDespawn);
            death_events.write(EntityDeathEvent {
                entity,
                position: transform.map_or(Vec2::ZERO, |t| t.translation.truncate()),
                exp_value: None,
            });
            next_state.set(GameState::GameOver);
            dead_player = Some(entity);
        }
    }

    // Handle marked entities
    for (entity, transform, enemy) in marked_entities.iter() {
        if Some(entity) == dead_player {
            continue;
        }
        if enemy.is_some() {
            game_stats.enemies_killed += 1;
        }

        // Send death event before despawning
        death_events.write(EntityDeathEvent {
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
        commands.entity(entity).despawn();
    }
}
