use crate::components::{Health, Luck, Player};
use crate::menu;
use crate::menu::{
    MenuAction, MenuActionComponent, MenuItem, UpgradeChoice, UpgradeConfirmedEvent,
};
use crate::types::{EquipmentType, Rarity, StatType};
use crate::weapon::{WeaponInventory, WeaponType, WeaponUpgrade, MAX_WEAPON_LEVEL};
use bevy::color::{Alpha, Color};
use bevy::hierarchy::{BuildChildren, ChildBuilder};
use bevy::log::info;
use bevy::prelude::*;
use bevy::utils::HashMap;
use rand::prelude::*;
use std::cmp::Ordering;
use strum::IntoEnumIterator;

pub fn handle_generic_upgrade(
    mut upgrade_events: EventReader<UpgradeConfirmedEvent>,
    mut player_query: Query<&mut Health, With<Player>>,
) {
    for event in upgrade_events.read() {
        if let UpgradeType::Generic(generic_upgrade) = &event.upgrade.upgrade_type {
            match generic_upgrade {
                GenericUpgrade::HealthPickup(amount) => {
                    if let Ok(mut health) = player_query.get_single_mut() {
                        let new_health = (health.current + amount).min(health.maximum);
                        info!(
                            "Healing player for {amount} (from {current} to {new})",
                            amount = amount,
                            current = health.current,
                            new = new_health
                        );
                        health.current = new_health;
                    }
                }
                GenericUpgrade::ResourcePickup(_) => {
                    // We'll implement this later
                    info!("Resource pickup not yet implemented");
                }
            }
        }
    }
}

#[derive(Resource)]
pub struct UpgradePool {
    weapons: Vec<(WeaponType, Rarity)>,
    equipment: Vec<(EquipmentType, Rarity)>,
    stats: Vec<(StatType, Rarity)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GenericUpgrade {
    HealthPickup(i32),   // Amount to heal
    ResourcePickup(u32), // Amount of resource to gain
}

impl std::fmt::Display for GenericUpgrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenericUpgrade::HealthPickup(_) => write!(f, "Philosopher's Elixir"),
            GenericUpgrade::ResourcePickup(_) => write!(f, "Void Shards"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum UpgradeType {
    Weapon(WeaponType, WeaponUpgrade),
    Generic(GenericUpgrade),
    Equipment(EquipmentType),
    Stat(StatType),
}

impl Default for UpgradePool {
    fn default() -> Self {
        Self::new()
    }
}

impl UpgradePool {
    pub fn new() -> Self {
        Self {
            weapons: vec![(WeaponType::MagickCircle, Rarity::Common)],
            equipment: vec![
                (EquipmentType::Armor, Rarity::Common),
                (EquipmentType::Ring, Rarity::Uncommon),
                (EquipmentType::Amulet, Rarity::Rare),
                (EquipmentType::Boots, Rarity::Common),
                (EquipmentType::Gloves, Rarity::Uncommon),
            ],
            stats: vec![
                (StatType::Health, Rarity::Common),
                (StatType::Speed, Rarity::Common),
                (StatType::Attack, Rarity::Uncommon),
                (StatType::Defense, Rarity::Common),
                (StatType::Luck, Rarity::Rare),
            ],
        }
    }

    pub fn generate_generic_choices() -> Vec<UpgradeChoice> {
        vec![
            UpgradeChoice {
                upgrade_type: UpgradeType::Generic(GenericUpgrade::HealthPickup(20)),
                description: "Restore health with a Philosopher's Elixir".to_string(),
                rarity: Rarity::Common,
            },
            UpgradeChoice {
                upgrade_type: UpgradeType::Generic(GenericUpgrade::ResourcePickup(100)),
                description: "Gather Void Shards".to_string(),
                rarity: Rarity::Common,
            },
        ]
    }

    pub fn generate_weapon_upgrades(inventory: &WeaponInventory) -> Vec<UpgradeChoice> {
        let mut upgrades = Vec::new();

        for weapon_config in &inventory.weapons {
            info!(
                "Processing weapon config from inventory: {:?}",
                weapon_config
            );

            // If weapon isn't max level, get next upgrade
            if weapon_config.level < MAX_WEAPON_LEVEL as u32 {
                info!("Weapon is below max level");
                let progression = weapon_config.weapon_type.get_progression();
                if let Some(next_upgrade) = progression.get(weapon_config.level as usize - 1) {
                    info!("Next upgrade found: {:?}", next_upgrade);
                    let level = weapon_config.level + 1;
                    info!("Next level: {:?}", level);
                    upgrades.push(UpgradeChoice {
                        upgrade_type: UpgradeType::Weapon(
                            weapon_config.weapon_type,
                            next_upgrade.clone(),
                        ),
                        description: format!(
                            "{} Level {}, {}",
                            weapon_config.weapon_type, level, next_upgrade,
                        ),
                        rarity: Rarity::Common, // We can make this more sophisticated later
                    });
                }
            } else {
                info!("Weapon is at or above max level");
                let limit_breaks = weapon_config.weapon_type.get_limit_break_options();
                info!("Limit break options {:?}", limit_breaks);
                let level = weapon_config.level + 1;
                info!("Next level: {:?}", level);
                for limit_break in limit_breaks {
                    let description = format!(
                        "{} Level {}, {}",
                        weapon_config.weapon_type, level, limit_break
                    );

                    upgrades.push(UpgradeChoice {
                        upgrade_type: UpgradeType::Weapon(weapon_config.weapon_type, limit_break),
                        description,
                        rarity: Rarity::Common,
                    });
                }
            }
        }

        info!("Generated weapon upgrades: {:?}", upgrades);

        upgrades
    }

    pub fn generate_choices(&self, luck: &Luck, inventory: &WeaponInventory) -> Vec<UpgradeChoice> {
        let mut rng = thread_rng();

        // Determine the number of upgrades to show
        let count = Self::calculate_count(luck, &mut rng);

        // Generate weapon-specific upgrades
        let mut choices = Self::generate_weapon_upgrades(inventory);

        // Adjust the list to ensure the correct count
        match choices.len().cmp(&count) {
            Ordering::Greater => {
                // Randomly select the required number of upgrades
                choices = Self::select_random_owned(choices, count, &mut rng);
            }
            Ordering::Less => {
                // Randomly select the needed number of generics to fill the gap
                let needed_generics = Self::select_random_owned(
                    Self::generate_generic_choices(),
                    count - choices.len(),
                    &mut rng,
                );
                choices.extend(needed_generics);
            }
            Ordering::Equal => (), // No adjustments needed
        }

        choices
    }

    fn calculate_count(luck: &Luck, rng: &mut impl Rng) -> usize {
        const LUCK_FACTOR: f32 = 0.02;
        if rng.gen::<f32>() < (luck.0 as f32) * LUCK_FACTOR {
            4
        } else {
            3
        }
    }

    fn select_random<T>(pool: &[T], count: usize, rng: &mut impl Rng) -> Vec<T>
    where
        T: Clone,
    {
        pool.iter()
            .choose_multiple(rng, count)
            .into_iter()
            .cloned()
            .collect()
    }

    fn select_random_owned<T, I>(iter: I, count: usize, rng: &mut impl Rng) -> Vec<T>
    where
        T: Clone,
        I: IntoIterator<Item = T>,
    {
        iter.into_iter().choose_multiple(rng, count)
    }
}

// Track both the type and level of each upgrade
#[derive(Component, Default)]
pub struct UpgradeTracker {
    // Track number of times each stat has been upgraded
    pub stats: HashMap<StatType, u32>,
    // Track weapon levels (Some(level) if owned, None if not owned)
    pub weapons: HashMap<WeaponType, Option<u32>>,
    // Track equipment levels
    pub equipment: HashMap<EquipmentType, Option<u32>>,
}

impl UpgradeTracker {
    pub fn new() -> Self {
        let mut stats = HashMap::new();
        let mut weapons = HashMap::new();
        let mut equipment = HashMap::new();

        // Initialize all possible stats at 0 upgrades
        for stat_type in [
            StatType::Health,
            StatType::Speed,
            StatType::Attack,
            StatType::Defense,
            StatType::Luck,
        ] {
            stats.insert(stat_type, 0);
        }

        // Initialize all weapons as not owned
        for weapon_type in WeaponType::iter() {
            weapons.insert(weapon_type, None);
        }

        weapons
            .entry(WeaponType::MagickCircle)
            .and_modify(|e| *e = Some(1));

        // Initialize all equipment as not owned
        for equipment_type in [
            EquipmentType::Armor,
            EquipmentType::Ring,
            EquipmentType::Amulet,
            EquipmentType::Boots,
            EquipmentType::Gloves,
        ] {
            equipment.insert(equipment_type, None);
        }

        Self {
            stats,
            weapons,
            equipment,
        }
    }

    pub fn upgrade_stat(&mut self, stat_type: &StatType) -> u32 {
        let count = self.stats.entry(stat_type.clone()).or_insert(0);
        *count += 1;
        *count
    }

    pub fn upgrade_weapon(&mut self, weapon_type: &WeaponType) -> u32 {
        let level = self.weapons.entry(weapon_type.clone()).or_insert(None);
        *level = Some(level.unwrap_or(0) + 1);
        level.unwrap()
    }

    pub fn upgrade_equipment(&mut self, equipment_type: &EquipmentType) -> u32 {
        let level = self.equipment.entry(equipment_type.clone()).or_insert(None);
        *level = Some(level.unwrap_or(0) + 1);
        level.unwrap()
    }

    pub fn get_stat_level(&self, stat_type: &StatType) -> u32 {
        *self.stats.get(stat_type).unwrap_or(&0)
    }

    pub fn has_weapon(&self, weapon_type: &WeaponType) -> bool {
        self.weapons.get(weapon_type).unwrap_or(&None).is_some()
    }

    pub fn get_weapon_level(&self, weapon_type: &WeaponType) -> Option<u32> {
        *self.weapons.get(weapon_type).unwrap_or(&None)
    }

    pub fn has_equipment(&self, equipment_type: &EquipmentType) -> bool {
        self.equipment
            .get(equipment_type)
            .unwrap_or(&None)
            .is_some()
    }

    pub fn get_equipment_level(&self, equipment_type: &EquipmentType) -> Option<u32> {
        *self.equipment.get(equipment_type).unwrap_or(&None)
    }
}

pub fn spawn_upgrade_choice(parent: &mut ChildBuilder, choice: UpgradeChoice, is_first: bool) {
    let (icon, name, description) = get_upgrade_display_info(&choice);

    parent
        .spawn((
            Button { ..default() },
            BorderColor(menu::get_rarity_color(&choice.rarity).with_alpha(0.5)),
            BackgroundColor(if is_first {
                Color::srgb(0.3, 0.3, 0.4)
            } else {
                Color::srgb(0.2, 0.2, 0.2)
            }),
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(100.0), // Made taller
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(16.0),
                border: UiRect::all(Val::Px(2.0)),
                margin: UiRect::vertical(Val::Px(4.0)),
                ..default()
            },
            MenuItem { selected: is_first },
            MenuActionComponent {
                action: MenuAction::SelectUpgrade(choice.clone()),
            },
        ))
        .with_children(|parent| {
            // Icon placeholder
            parent.spawn((
                Text::new(icon),
                TextFont {
                    font_size: 32.0, // Made larger
                    ..default()
                },
                TextColor(menu::get_rarity_color(&choice.rarity)),
            ));

            // Text container
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|parent| {
                    // Upgrade name
                    parent.spawn((
                        Text::new(name),
                        TextFont {
                            font_size: 24.0, // Made larger
                            ..default()
                        },
                        TextColor(menu::get_rarity_color(&choice.rarity)),
                    ));

                    // Description
                    parent.spawn((
                        Text::new(description),
                        TextFont {
                            font_size: 18.0, // Made larger
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                    ));
                });
        });
}

fn get_upgrade_display_info(choice: &UpgradeChoice) -> (&'static str, String, String) {
    match &choice.upgrade_type {
        UpgradeType::Weapon(weapon_type, ..) => {
            let icon = match weapon_type {
                WeaponType::MagickCircle => "🔮",
                // We can add more weapon types here as we implement them
            };
            (
                icon,
                format!("{} Weapon", weapon_type),
                choice.description.clone(),
            )
        }
        UpgradeType::Generic(generic_type) => {
            let icon = match generic_type {
                GenericUpgrade::HealthPickup(_) => "⚗️",
                GenericUpgrade::ResourcePickup(_) => "💎",
            };
            (
                icon,
                format!("{}", generic_type),
                choice.description.clone(),
            )
        }
        UpgradeType::Equipment(equipment_type) => {
            let icon = match equipment_type {
                EquipmentType::Armor => "🛡️",
                EquipmentType::Ring => "💍",
                EquipmentType::Amulet => "📿",
                EquipmentType::Boots => "👢",
                EquipmentType::Gloves => "🧤",
            };
            (
                icon,
                format!("{}", equipment_type),
                choice.description.clone(),
            )
        }
        UpgradeType::Stat(stat_type) => {
            let icon = match stat_type {
                StatType::Health => "❤️",
                StatType::Speed => "👟",
                StatType::Attack => "⚡",
                StatType::Defense => "🛡️",
                StatType::Luck => "🍀",
            };
            (
                icon,
                format!("{} Up", stat_type),
                choice.description.clone(),
            )
        }
    }
}

pub fn add_upgrade_tracking(
    mut commands: Commands,
    query: Query<Entity, (With<Player>, Without<UpgradeTracker>)>,
) {
    for entity in query.iter() {
        commands.entity(entity).insert(UpgradeTracker::new());
    }
}
