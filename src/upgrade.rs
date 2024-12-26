use crate::components::{Player, PlayerStats};
use crate::menu;
use crate::menu::{
    MenuAction, MenuActionComponent, MenuItem, UpgradeChoice, UpgradeConfirmedEvent, UpgradeType,
};
use crate::types::{EquipmentType, Rarity, StatType};
use crate::weapon::{WeaponConfig, WeaponInventory, WeaponType, MAX_WEAPON_LEVEL};
use bevy::color::{Alpha, Color};
use bevy::hierarchy::{BuildChildren, ChildBuilder};
use bevy::log::info;
use bevy::prelude::*;
use bevy::utils::HashMap;
use strum::IntoEnumIterator;

#[derive(Resource)]
pub struct UpgradePool {
    weapons: Vec<(WeaponType, Rarity)>,
    equipment: Vec<(EquipmentType, Rarity)>,
    stats: Vec<(StatType, Rarity)>,
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
                    upgrades.push(UpgradeChoice {
                        upgrade_type: UpgradeType::Weapon(weapon_config.weapon_type, next_upgrade.clone()),
                        description: format!(
                            "{} Level {}, Add a second circle of banishment",
                            weapon_config.weapon_type,
                            weapon_config.level + 1,
                        ),
                        rarity: Rarity::Common, // We can make this more sophisticated later
                    });
                }
            }
        }
        
        info!("Generated weapon upgrades: {:?}", upgrades);

        upgrades
    }

    pub fn generate_choices(&self, count: usize) -> Vec<UpgradeChoice> {
        // First get actual weapon upgrades
        let mut choices = Vec::new();

        // For now, hardcode player's weapon inventory since we haven't hooked that up yet
        let inventory = WeaponInventory {
            weapons: vec![WeaponConfig {
                weapon_type: WeaponType::MagickCircle,
                level: 1,
            }],
        };

        let weapon_upgrades = Self::generate_weapon_upgrades(&inventory);
        choices.extend(weapon_upgrades);

        // If we need more choices, we can fill with other types of upgrades
        // (equipment, stats, etc.)
        // while choices.len() < count {
        //     // Add placeholder upgrades or leave empty depending on preference
        // }

        choices
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

// System to apply confirmed upgrades
pub fn apply_confirmed_upgrade(
    mut upgrade_events: EventReader<UpgradeConfirmedEvent>,
    mut player_stats: Query<&mut PlayerStats>,
) {
    for event in upgrade_events.read() {
        if let Ok(mut stats) = player_stats.get_single_mut() {
            match &event.upgrade.upgrade_type {
                UpgradeType::Stat(stat_type) => match stat_type {
                    StatType::Health => stats.health *= 1.1,
                    StatType::Speed => stats.speed *= 1.1,
                    StatType::Attack => stats.attack *= 1.1,
                    StatType::Defense => stats.defense *= 1.1,
                    StatType::Luck => stats.luck *= 1.1,
                },
                UpgradeType::Weapon(weapon_type, weapon_upgrade) => {
                    info!("Adding weapon: {:?}", weapon_type);
                    // TODO: Implement weapon system
                }
                UpgradeType::Equipment(equipment_type) => {
                    info!("Adding equipment: {:?}", equipment_type);
                    // TODO: Implement equipment system
                }
            }
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
