use crate::components::Health;
use crate::death::MarkedForDeath;
use bevy::prelude::*;

#[derive(Event)]
pub struct DamageEvent {
    pub target: Entity,
    pub amount: f32,
    pub source: Option<Entity>,
}

#[derive(Component)]
pub struct LastDamageTime {
    pub time: f32,
    pub cooldown: f32,
}

#[derive(Component)]
pub struct DamageCooldown {
    pub time: f32,
    pub cooldown: f32,
}

impl Default for DamageCooldown {
    fn default() -> Self {
        Self {
            time: 0.0,
            cooldown: 0.25, // Default to 0.25s between damage
        }
    }
}

impl Default for LastDamageTime {
    fn default() -> Self {
        Self {
            time: 0.0,
            cooldown: 0.25, // Default to 0.25s between damage
        }
    }
}

pub fn handle_damage(
    time: Res<Time<Virtual>>,
    mut commands: Commands,
    mut damage_events: EventReader<DamageEvent>,
    mut health_query: Query<&mut Health>,
    mut cooldown_query: Query<&mut DamageCooldown>,
) {
    for event in damage_events.read() {
        info!(
            "Processing damage event for {:?}, amount: {}",
            event.target, event.amount
        );

        let current_time = time.elapsed_secs();

        // Check for cooldown
        let should_damage = if let Ok(mut cooldown) = cooldown_query.get_mut(event.target) {
            let can_damage = current_time - cooldown.time >= cooldown.cooldown;
            if !can_damage {
                info!(
                    "Cooldown active. Current: {}, Last: {}, Diff: {}, Need: {}",
                    current_time,
                    cooldown.time,
                    current_time - cooldown.time,
                    cooldown.cooldown
                );
            } else {
                cooldown.time = current_time;
                info!("Updated cooldown time to: {}", current_time);
            }
            can_damage
        } else {
            info!("No cooldown component - damage allowed");
            true
        };

        if !should_damage {
            continue;
        }

        // Apply damage
        if let Ok(mut health) = health_query.get_mut(event.target) {
            let old_health = health.current;
            health.current -= event.amount;
            info!(
                "Health changed from {} to {} for {:?}",
                old_health, health.current, event.target
            );

            if health.current <= 0.0 {
                info!(
                    "Marking {:?} for death at health {}",
                    event.target, health.current
                );
                commands.entity(event.target).insert(MarkedForDeath);
            }
        } else {
            info!("No health component found for {:?}", event.target);
        }
    }
}
