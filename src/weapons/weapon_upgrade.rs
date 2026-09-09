use crate::menu::WeaponUpgradeConfirmedEvent;
use crate::weapons::magick_circle::PatternType;
use crate::weapons::{WeaponArea, WeaponCooldown, WeaponDamage, WeaponMeta, WeaponType};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub enum WeaponUpgradeChange {
    Damage(i32),
    Area(i32),
    Cooldown(i32),
    AddCircle { pattern: Option<PatternType> },
}

impl std::fmt::Display for WeaponUpgradeChange {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            WeaponUpgradeChange::Damage(damage) => write!(f, "Increase damage by {}%", damage),
            WeaponUpgradeChange::Area(area) => write!(f, "Increase radius by {}%", area),
            WeaponUpgradeChange::Cooldown(cooldown) => {
                write!(f, "Decrease cooldown by {}%", cooldown.unsigned_abs())
            }
            WeaponUpgradeChange::AddCircle { .. } => {
                write!(f, "Add a new Magick Circle")
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
    mut events: MessageReader<WeaponUpgradeConfirmedEvent>,
    mut weapon_query: Query<&mut WeaponMeta>,
) {
    for event in events.read() {
        for mut meta in weapon_query.iter_mut() {
            if meta.weapon_type == event.weapon_type {
                meta.level = meta.level.saturating_add(1);
            }
        }
    }
}

pub fn apply_common_weapon_upgrades(
    mut upgrade_events: MessageReader<WeaponUpgradeConfirmedEvent>,
    mut weapon_query: Query<(
        &mut WeaponDamage,
        &mut WeaponArea,
        &mut WeaponCooldown,
        &WeaponMeta,
    )>,
) {
    for upgrade_event in upgrade_events.read() {
        for (mut damage, mut area, mut cooldown, meta) in &mut weapon_query {
            if meta.weapon_type == upgrade_event.weapon_type {
                for change in &upgrade_event.upgrade_spec.changes {
                    match change {
                        WeaponUpgradeChange::Damage(value) => {
                            damage.damage_bonus = damage.damage_bonus.saturating_add(*value);
                        }
                        WeaponUpgradeChange::Area(value) => {
                            area.area_bonus = area.area_bonus.saturating_add(*value);
                        }
                        WeaponUpgradeChange::Cooldown(value) => {
                            cooldown.cooldown_bonus =
                                cooldown.cooldown_bonus.saturating_add(*value);
                        }
                        WeaponUpgradeChange::AddCircle { .. } => {}
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

        let magick_circle_data = WeaponUpgradeData {
            progression: vec![
                // Level 2: Initial power boost
                WeaponUpgradeSpec {
                    changes: vec![WeaponUpgradeChange::Damage(2), WeaponUpgradeChange::Area(1)],
                },
                // Level 3: First additional circle
                WeaponUpgradeSpec {
                    changes: vec![WeaponUpgradeChange::AddCircle { pattern: None }],
                },
                // Level 4: Second circle + minor boost
                WeaponUpgradeSpec {
                    changes: vec![
                        WeaponUpgradeChange::AddCircle { pattern: None },
                        WeaponUpgradeChange::Damage(1),
                        WeaponUpgradeChange::Area(1),
                    ],
                },
                // Level 5: Third circle
                WeaponUpgradeSpec {
                    changes: vec![WeaponUpgradeChange::AddCircle { pattern: None }],
                },
                // Level 6: Significant power boost + fourth circle
                WeaponUpgradeSpec {
                    changes: vec![
                        WeaponUpgradeChange::AddCircle { pattern: None },
                        WeaponUpgradeChange::Damage(2),
                        WeaponUpgradeChange::Area(1),
                    ],
                },
                // Level 7: Fifth circle + minor boost
                WeaponUpgradeSpec {
                    changes: vec![
                        WeaponUpgradeChange::AddCircle { pattern: None },
                        WeaponUpgradeChange::Damage(1),
                        WeaponUpgradeChange::Area(1),
                    ],
                },
                // Level 8: Final circle + major power spike
                WeaponUpgradeSpec {
                    changes: vec![
                        WeaponUpgradeChange::AddCircle { pattern: None },
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
                    changes: vec![WeaponUpgradeChange::Cooldown(-2)],
                },
            ],
        };

        data.insert(WeaponType::MagickCircle, magick_circle_data);

        Self { data }
    }

    pub fn get_next_upgrades(&self, weapon: WeaponType, level: u32) -> Vec<WeaponUpgradeSpec> {
        let weapon_upgrade_data = self
            .data
            .get(&weapon)
            .unwrap_or_else(|| panic!("Unable to find weapon {} in weapon upgrade config", weapon));

        // Index zero describes the upgrade from weapon level 1 to level 2.
        let index = level.checked_sub(1).expect("Weapon levels start at 1") as usize;
        if let Some(upgrade) = weapon_upgrade_data.progression.get(index) {
            vec![upgrade.clone()]
        } else {
            weapon_upgrade_data.limit_breaks.to_vec()
        }
    }
}
