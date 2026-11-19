//! Difficulty adjustments carried by a run recipe.
//!
//! Stages already scale enemies by phase. These modifiers sit on top of that
//! for one whole run, so something outside the run (an estate ritual, a
//! hand-written recipe) can make it harder or easier without the run knowing
//! why. Values are percentages: 100 leaves the run unchanged.
use crate::components::{Enemy, Health};
use crate::setup::ActiveRunConfig;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Accepted range for every modifier, so a typo cannot make a run unplayable.
pub const MIN_PERCENT: u32 = 25;
pub const MAX_PERCENT: u32 = 400;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct RunModifiers {
    /// Every enemy's starting health, bosses included.
    pub enemy_health_percent: u32,
    /// Damage the player takes from any source, before protection.
    pub damage_taken_percent: u32,
}

impl Default for RunModifiers {
    fn default() -> Self {
        Self {
            enemy_health_percent: 100,
            damage_taken_percent: 100,
        }
    }
}

impl RunModifiers {
    pub fn is_neutral(&self) -> bool {
        *self == Self::default()
    }

    pub fn validate(&self) -> Result<(), String> {
        for (name, value) in [
            ("enemy_health_percent", self.enemy_health_percent),
            ("damage_taken_percent", self.damage_taken_percent),
        ] {
            if !(MIN_PERCENT..=MAX_PERCENT).contains(&value) {
                return Err(format!(
                    "{name} must be between {MIN_PERCENT} and {MAX_PERCENT}."
                ));
            }
        }
        Ok(())
    }

    pub fn enemy_health(&self, health: i32) -> i32 {
        scale(health, self.enemy_health_percent)
    }

    pub fn damage_taken(&self, amount: i32) -> i32 {
        scale(amount, self.damage_taken_percent)
    }
}

/// Scale a positive amount, rounding to the nearest whole point but never to zero.
fn scale(amount: i32, percent: u32) -> i32 {
    if amount <= 0 || percent == 100 {
        return amount;
    }
    let scaled = (i64::from(amount) * i64::from(percent) + 50) / 100;
    scaled.clamp(1, i64::from(i32::MAX)) as i32
}

pub struct RunModifiersPlugin;

impl Plugin for RunModifiersPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(scale_new_enemy_health);
    }
}

/// Every enemy is spawned with its health in the same bundle, so scaling it
/// when `Enemy` is added covers waves, bosses, and encounters alike.
fn scale_new_enemy_health(
    add: On<Add<Enemy>>,
    config: Option<Res<ActiveRunConfig>>,
    mut health: Query<&mut Health>,
) {
    let Some(config) = config else {
        return;
    };
    let modifiers = config.0.modifiers;
    if modifiers.is_neutral() {
        return;
    }
    if let Ok(mut health) = health.get_mut(add.entity) {
        health.maximum = modifiers.enemy_health(health.maximum);
        health.current = modifiers.enemy_health(health.current);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modifiers(enemy_health_percent: u32, damage_taken_percent: u32) -> RunModifiers {
        RunModifiers {
            enemy_health_percent,
            damage_taken_percent,
        }
    }

    #[test]
    fn scaling_rounds_but_never_reaches_zero() {
        let harder = modifiers(150, 50);
        assert_eq!(harder.enemy_health(10), 15);
        assert_eq!(harder.enemy_health(3), 5);
        assert_eq!(harder.damage_taken(1), 1);
        assert_eq!(harder.damage_taken(0), 0);
        assert_eq!(RunModifiers::default().damage_taken(7), 7);
    }

    #[test]
    fn damage_to_the_player_is_scaled_but_damage_to_enemies_is_not() {
        use crate::components::Health;
        use crate::tests::{game, player, step};
        let mut app = game();
        app.world_mut()
            .resource_mut::<ActiveRunConfig>()
            .0
            .modifiers = modifiers(100, 200);
        let player = player(&mut app);
        let enemy = app
            .world_mut()
            .spawn((
                Enemy {
                    speed: 0.0,
                    experience_value: 0,
                },
                Health {
                    current: 50,
                    maximum: 50,
                },
            ))
            .id();
        let before = app.world().get::<Health>(player).unwrap().current;
        app.world_mut()
            .get_mut::<crate::combat::DamageCooldown>(player)
            .unwrap()
            .time = -1.0;
        for target in [player, enemy] {
            app.world_mut().write_message(crate::combat::DamageEvent {
                target,
                amount: 7,
                source: None,
            });
        }
        step(&mut app, 1);
        assert_eq!(
            app.world().get::<Health>(player).unwrap().current,
            before - 14
        );
        assert_eq!(app.world().get::<Health>(enemy).unwrap().current, 43);
    }

    #[test]
    fn out_of_range_values_are_rejected() {
        assert!(RunModifiers::default().validate().is_ok());
        assert!(modifiers(10, 100).validate().is_err());
        assert!(modifiers(100, 900).validate().is_err());
    }

    #[test]
    fn recipes_without_modifiers_still_parse_and_are_written_without_them() {
        let config: crate::setup::RunConfiguration = ron::from_str(
            "(version: 1, stage: \"courtyard\", seed: 1, weapons: [(weapon: magick_circle, level: 1)])",
        )
        .unwrap();
        assert!(config.modifiers.is_neutral());
        let text = ron::to_string(&config).unwrap();
        assert!(!text.contains("modifiers"), "{text}");
    }
}
