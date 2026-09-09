use crate::components::Health;
use crate::death::{MarkedForDeath, MarkedForDespawn};
use crate::weapons::ProtectionBuff;
use bevy::prelude::*;

#[derive(Message)]
pub struct DamageEvent {
    pub target: Entity,
    pub amount: i32,
    pub source: Option<Entity>,
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

#[expect(
    clippy::type_complexity,
    reason = "The ECS query keeps optional damage modifiers and dying-target exclusions explicit."
)]
pub fn handle_damage(
    time: Res<Time<Virtual>>,
    mut commands: Commands,
    mut damage_events: MessageReader<DamageEvent>,
    mut targets: Query<
        (
            &mut Health,
            Option<&mut DamageCooldown>,
            Option<&ProtectionBuff>,
        ),
        (Without<MarkedForDeath>, Without<MarkedForDespawn>),
    >,
    #[cfg(feature = "dev-tools")] debug: Res<crate::dev_tools::DebugSession>,
    #[cfg(feature = "dev-tools")] player_query: Query<(), With<crate::components::Player>>,
) {
    let current_time = time.elapsed_secs();
    for event in damage_events.read() {
        #[cfg(feature = "dev-tools")]
        if debug.invulnerable && player_query.contains(event.target) {
            continue;
        }
        let Ok((mut health, cooldown, protection)) = targets.get_mut(event.target) else {
            continue;
        };
        if health.current <= 0 || event.amount <= 0 {
            continue;
        }

        // A target's cooldown applies across damage sources. Enemies have no
        // cooldown, so independent circles continue to contribute their damage.
        if let Some(mut cooldown) = cooldown {
            if current_time - cooldown.time < cooldown.cooldown {
                continue;
            }
            cooldown.time = current_time;
        }

        let effective_damage = if let Some(protection) = protection {
            let reduced = (event.amount as f32 * (1.0 - protection.damage_reduction)) as i32;
            // Protection reduces positive damage but never grants immunity.
            reduced.max(1)
        } else {
            event.amount
        };

        health.current -= effective_damage;
        debug!(
            "Applied {} damage to {:?} from {:?}; health is {}",
            effective_damage, event.target, event.source, health.current
        );

        if health.current <= 0 {
            commands.entity(event.target).insert(MarkedForDeath);
        }
    }
}
