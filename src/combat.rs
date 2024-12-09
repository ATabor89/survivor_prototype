use crate::components::{Combat, Health, Player};
use crate::death::MarkedForDeath;
use crate::weapon::PatternType;
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

#[derive(Component)]
pub struct ProjectileStats {
    pub damage: f32,
    pub pierce_remaining: u32,
    pub pierce_cooldown: f32,
    pub last_hit_time: f32,
}

impl ProjectileStats {
    pub fn new(damage: f32, pierce: u32) -> Self {
        Self {
            damage,
            pierce_remaining: pierce,
            pierce_cooldown: 0.1,
            last_hit_time: 0.0,
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
        info!("Processing damage event for {:?}, amount: {}", event.target, event.amount);

        let current_time = time.elapsed_secs();

        // Check for cooldown
        let should_damage = if let Ok(mut cooldown) = cooldown_query.get_mut(event.target) {
            let can_damage = current_time - cooldown.time >= cooldown.cooldown;
            if !can_damage {
                info!("Cooldown active. Current: {}, Last: {}, Diff: {}, Need: {}",
                    current_time, cooldown.time,
                    current_time - cooldown.time, cooldown.cooldown);
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
            info!("Health changed from {} to {} for {:?}",
                  old_health, health.current, event.target);

            if health.current <= 0.0 {
                info!("Marking {:?} for death at health {}", event.target, health.current);
                commands.entity(event.target).insert(MarkedForDeath);
            }
        } else {
            info!("No health component found for {:?}", event.target);
        }
    }
}
