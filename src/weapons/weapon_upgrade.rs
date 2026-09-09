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

/// Effective values include the player's current passive modifiers and the
/// same rounding and cooldown floor used when firing an attack.
pub(crate) struct WeaponUpgradePreview {
    pub damage: (i32, i32),
    pub radius: (f32, f32),
    pub cooldown: (f32, f32),
}

impl WeaponUpgradePreview {
    pub(crate) fn description(&self, spec: &WeaponUpgradeSpec) -> String {
        spec.changes
            .iter()
            .map(|change| match change {
                WeaponUpgradeChange::Damage(_) => {
                    format!("Damage {} -> {}", self.damage.0, self.damage.1)
                }
                WeaponUpgradeChange::Area(_) => {
                    format!("Radius {:.1} -> {:.1}", self.radius.0, self.radius.1)
                }
                WeaponUpgradeChange::Cooldown(_) => format!(
                    "Cooldown {:.2}s -> {:.2}s",
                    self.cooldown.0, self.cooldown.1
                ),
                WeaponUpgradeChange::AddCircle { .. } => change.to_string(),
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}

pub(crate) fn preview_weapon_upgrade(
    spec: &WeaponUpgradeSpec,
    damage: &WeaponDamage,
    area: &WeaponArea,
    cooldown: &WeaponCooldown,
    player: &crate::components::PlayerStats,
) -> WeaponUpgradePreview {
    let mut next_damage = *damage;
    let mut next_area = *area;
    let mut next_cooldown = cooldown.clone();
    for change in &spec.changes {
        apply_stat_change(change, &mut next_damage, &mut next_area, &mut next_cooldown);
    }
    WeaponUpgradePreview {
        damage: (
            damage.effective_amount(player.damage_multiplier),
            next_damage.effective_amount(player.damage_multiplier),
        ),
        radius: (
            area.effective_radius(player.area_multiplier),
            next_area.effective_radius(player.area_multiplier),
        ),
        cooldown: (
            cooldown.effective_seconds(player.cooldown_reduction),
            next_cooldown.effective_seconds(player.cooldown_reduction),
        ),
    }
}

#[derive(Debug, Clone)]
pub struct WeaponUpgradeData {
    pub progression: Vec<WeaponUpgradeSpec>,
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
                        WeaponUpgradeChange::AddCircle { .. } => {}
                        _ => apply_stat_change(change, &mut damage, &mut area, &mut cooldown),
                    }
                }
            }
        }
    }
}

pub(crate) fn apply_stat_change(
    change: &WeaponUpgradeChange,
    damage: &mut WeaponDamage,
    area: &mut WeaponArea,
    cooldown: &mut WeaponCooldown,
) {
    match change {
        WeaponUpgradeChange::Damage(value) => {
            damage.damage_bonus = damage.damage_bonus.saturating_add(*value)
        }
        WeaponUpgradeChange::Area(value) => {
            area.area_bonus = area.area_bonus.saturating_add(*value)
        }
        WeaponUpgradeChange::Cooldown(value) => {
            cooldown.cooldown_bonus = cooldown.cooldown_bonus.saturating_add(*value)
        }
        WeaponUpgradeChange::AddCircle { .. } => {}
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
                    changes: vec![
                        WeaponUpgradeChange::Damage(20),
                        WeaponUpgradeChange::Area(10),
                    ],
                },
                // Level 3: First additional circle
                WeaponUpgradeSpec {
                    changes: vec![WeaponUpgradeChange::AddCircle { pattern: None }],
                },
                // Level 4: Second circle + minor boost
                WeaponUpgradeSpec {
                    changes: vec![
                        WeaponUpgradeChange::AddCircle { pattern: None },
                        WeaponUpgradeChange::Damage(10),
                        WeaponUpgradeChange::Area(5),
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
                        WeaponUpgradeChange::Damage(20),
                        WeaponUpgradeChange::Area(5),
                    ],
                },
                // Level 7: Fifth circle + minor boost
                WeaponUpgradeSpec {
                    changes: vec![
                        WeaponUpgradeChange::AddCircle { pattern: None },
                        WeaponUpgradeChange::Damage(10),
                        WeaponUpgradeChange::Area(5),
                    ],
                },
                // Level 8: Final circle + major power spike
                WeaponUpgradeSpec {
                    changes: vec![
                        WeaponUpgradeChange::AddCircle { pattern: None },
                        WeaponUpgradeChange::Damage(20),
                        WeaponUpgradeChange::Area(10),
                    ],
                },
            ],
        };

        data.insert(WeaponType::MagickCircle, magick_circle_data);
        let spec = |changes| WeaponUpgradeSpec { changes };
        use WeaponUpgradeChange::{Area, Cooldown, Damage};
        data.insert(
            WeaponType::InscribedNeedles,
            WeaponUpgradeData {
                progression: vec![
                    spec(vec![Damage(25)]),
                    spec(vec![Cooldown(-10)]),
                    spec(vec![Damage(25), Area(25)]),
                    spec(vec![Cooldown(-10)]),
                    spec(vec![Damage(25)]),
                    spec(vec![Area(25), Cooldown(-10)]),
                    spec(vec![Damage(25), Cooldown(-10)]),
                ],
            },
        );
        data.insert(
            WeaponType::AlchemicalVessel,
            WeaponUpgradeData {
                progression: vec![
                    spec(vec![Damage(25)]),
                    spec(vec![Area(20)]),
                    spec(vec![Cooldown(-10), Damage(25)]),
                    spec(vec![Area(20)]),
                    spec(vec![Damage(25)]),
                    spec(vec![Cooldown(-10), Area(20)]),
                    spec(vec![Damage(25), Area(20)]),
                ],
            },
        );

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
            Vec::new()
        }
    }

    pub fn max_level(&self, weapon: WeaponType) -> u32 {
        self.data[&weapon].progression.len() as u32 + 1
    }
}
