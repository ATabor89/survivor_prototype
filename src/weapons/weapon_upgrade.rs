use crate::menu::WeaponUpgradeConfirmedEvent;
use crate::weapons::magick_circle::PatternType;
use crate::weapons::{WeaponArea, WeaponCooldown, WeaponDamage, WeaponMeta, WeaponType};
use bevy::prelude::*;
use bevy::utils::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum WeaponUpgradeChange {
    Damage(i32),
    Area(i32),
    Cooldown(i32),
    Duration(i32),
    AddCircle {
        pattern: PatternType,
        // offset_angle: f32,
    },
}

impl std::fmt::Display for WeaponUpgradeChange {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            WeaponUpgradeChange::Damage(damage) => write!(f, "Increase damage by {}", damage),
            WeaponUpgradeChange::Area(area) => write!(f, "Increase area by {}", area),
            WeaponUpgradeChange::Cooldown(cooldown) => {
                write!(f, "Decrease cooldown by {}", cooldown)
            }
            WeaponUpgradeChange::Duration(duration) => {
                write!(f, "Increase duration by {}", duration)
            }
            WeaponUpgradeChange::AddCircle { pattern, .. } => {
                write!(f, "Add a {} Magick Circle", pattern)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WeaponUpgradeSpec {
    pub changes: Vec<WeaponUpgradeChange>,
}

#[derive(Debug, Clone)]
pub struct WeaponUpgradeData {
    pub progression: Vec<WeaponUpgradeSpec>,
    pub limit_breaks: Vec<WeaponUpgradeSpec>,
}

// A system dedicated solely to increasing the level of the weapon in order to prevent multiple level updates from different upgrade systems
pub fn update_weapon_level(
    mut events: EventReader<WeaponUpgradeConfirmedEvent>,
    mut weapon_query: Query<&mut WeaponMeta>,
) {
    for event in events.read() {
        for mut meta in weapon_query.iter_mut() {
            if meta.weapon_type == event.weapon_type {
                meta.level += 1;
            }
        }
    }
}

pub fn apply_common_weapon_upgrades(
    mut upgrade_events: EventReader<WeaponUpgradeConfirmedEvent>,
    mut weapon_query: Query<(
        &mut WeaponDamage,
        &mut WeaponArea,
        &mut WeaponCooldown,
        // Option<&mut WeaponDuration>,  // if you have such a component
        &WeaponMeta,
    )>,
) {
    for upgrade_event in upgrade_events.read() {
        // We already have the final `upgrade_spec` in `upgrade_event`
        for (mut damage, mut area, mut cooldown, /*duration, */ meta) in weapon_query.iter_mut() {
            if meta.weapon_type == upgrade_event.weapon_type {
                for change in &upgrade_event.upgrade_spec.changes {
                    match &change {
                        WeaponUpgradeChange::Damage(value) => {
                            damage.damage_bonus += *value;
                        }
                        WeaponUpgradeChange::Area(value) => {
                            area.area_bonus += *value;
                        }
                        WeaponUpgradeChange::Cooldown(value) => {
                            cooldown.cooldown_bonus += *value;
                        }
                        WeaponUpgradeChange::Duration(_) => {
                            // If you have a separate "duration" component:
                            // if let Some(mut dur) = duration {
                            //     dur.duration_bonus += *value;
                            // }
                        }
                        // We'll ignore specialized changes (e.g. AddCircle) here.
                        // A separate "magick circle upgrades" system can handle that.
                        _ => {
                            // no-op here
                        }
                    }
                }
            }
        }
    }
}

#[derive(Resource)]
pub struct WeaponUpgradeConfig {
    pub data: HashMap<WeaponType, WeaponUpgradeData>,
}

impl Default for WeaponUpgradeConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl WeaponUpgradeConfig {
    pub fn new() -> Self {
        let mut data = HashMap::new();

        // For MagickCircle, fill in all the level-based UpgradeSpecs
        let magick_circle_data = WeaponUpgradeData {
            progression: vec![
                // Level 2: Initial power boost
                WeaponUpgradeSpec {
                    changes: vec![WeaponUpgradeChange::Damage(2), WeaponUpgradeChange::Area(1)],
                },
                // Level 3: First additional circle
                WeaponUpgradeSpec {
                    changes: vec![WeaponUpgradeChange::AddCircle {
                        pattern: PatternType::Banishment,
                        // offset_angle: std::f32::consts::PI,
                    }],
                },
                // Level 4: Second circle + minor boost
                WeaponUpgradeSpec {
                    changes: vec![
                        WeaponUpgradeChange::AddCircle {
                            pattern: PatternType::Banishment,
                            // offset_angle: std::f32::consts::PI * 0.5,
                        },
                        WeaponUpgradeChange::Damage(1),
                        WeaponUpgradeChange::Area(1),
                    ],
                },
                // Level 5: Third circle
                WeaponUpgradeSpec {
                    changes: vec![WeaponUpgradeChange::AddCircle {
                        pattern: PatternType::Banishment,
                        // offset_angle: std::f32::consts::PI * 1.5,
                    }],
                },
                // Level 6: Significant power boost + fourth circle
                WeaponUpgradeSpec {
                    changes: vec![
                        WeaponUpgradeChange::AddCircle {
                            pattern: PatternType::Banishment,
                            // offset_angle: std::f32::consts::PI * 2.0,
                        },
                        WeaponUpgradeChange::Damage(2),
                        WeaponUpgradeChange::Area(1),
                    ],
                },
                // Level 7: Fifth circle + minor boost
                WeaponUpgradeSpec {
                    changes: vec![
                        WeaponUpgradeChange::AddCircle {
                            pattern: PatternType::Banishment,
                            // offset_angle: std::f32::consts::PI * 2.5,
                        },
                        WeaponUpgradeChange::Damage(1),
                        WeaponUpgradeChange::Area(1),
                    ],
                },
                // Level 8: Final circle + major power spike
                WeaponUpgradeSpec {
                    changes: vec![
                        WeaponUpgradeChange::AddCircle {
                            pattern: PatternType::Banishment,
                            // offset_angle: std::f32::consts::PI * 3.0,
                        },
                        WeaponUpgradeChange::Damage(3),
                        WeaponUpgradeChange::Area(2),
                    ],
                },
            ],
            limit_breaks: vec![
                WeaponUpgradeSpec {
                    changes: vec![WeaponUpgradeChange::Damage(2)],
                },
                WeaponUpgradeSpec {
                    changes: vec![WeaponUpgradeChange::Area(2)],
                },
                WeaponUpgradeSpec {
                    changes: vec![WeaponUpgradeChange::Duration(2)],
                },
                WeaponUpgradeSpec {
                    changes: vec![WeaponUpgradeChange::Cooldown(-2)],
                },
            ],
        };

        // Insert into the map
        data.insert(WeaponType::MagickCircle, magick_circle_data);

        // Potentially do the same for other weapon types...
        Self { data }
    }

    pub fn get_next_upgrades(&self, weapon: WeaponType, level: u32) -> Vec<WeaponUpgradeSpec> {
        let weapon_upgrade_data = self
            .data
            .get(&weapon)
            .unwrap_or_else(|| panic!("Unable to find weapon {} in weapon upgrade config", weapon));

        if level < weapon_upgrade_data.progression.len() as u32 {
            vec![weapon_upgrade_data.progression[level as usize].clone()]
        } else {
            weapon_upgrade_data.limit_breaks.to_vec()
        }
    }
}
