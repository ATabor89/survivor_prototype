use crate::components::{Health, Luck, Player};
use crate::menu;
use crate::menu::{
    GenericUpgradeConfirmedEvent, MenuAction, MenuActionComponent, MenuItem, UpgradeChoice,
};
use crate::types::{EquipmentType, Rarity, StatType};
use crate::weapons::weapon_upgrade::{WeaponUpgradeConfig, WeaponUpgradeSpec};
use crate::weapons::{WeaponMeta, WeaponType};
use bevy::color::{Alpha, Color};
use bevy::hierarchy::{BuildChildren, ChildBuilder};
use bevy::log::info;
use bevy::prelude::*;
use rand::prelude::*;
use std::cmp::Ordering;

pub fn handle_generic_upgrade(
    mut upgrade_events: EventReader<GenericUpgradeConfirmedEvent>,
    mut player_query: Query<&mut Health, With<Player>>,
) {
    for generic_upgrade_event in upgrade_events.read() {
        match generic_upgrade_event.generic_upgrade_type {
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
    Weapon(WeaponType, WeaponUpgradeSpec),
    Generic(GenericUpgrade),
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

    pub fn generate_weapon_upgrades(
        weapon_upgrade_config: &WeaponUpgradeConfig,
        weapons: &[&WeaponMeta],
    ) -> Vec<UpgradeChoice> {
        let mut upgrades = Vec::new();

        for weapon_meta in weapons {
            info!("Processing weapon config from inventory: {:?}", weapon_meta);

            // Fetch the next upgrades from your config, using the weapon’s current level
            let specs =
                weapon_upgrade_config.get_next_upgrades(weapon_meta.weapon_type, weapon_meta.level);

            // For logging or reference
            let next_level = weapon_meta.level + 1;
            info!("Next level: {}", next_level);

            // Convert each `WeaponUpgradeSpec` to an `UpgradeChoice`
            for spec in specs {
                let description = format!(
                    "{} Level {}: {:?}",
                    weapon_meta.weapon_type, next_level, spec
                );

                upgrades.push(UpgradeChoice {
                    upgrade_type: UpgradeType::Weapon(weapon_meta.weapon_type, spec.clone()),
                    description,
                    rarity: Rarity::Common, // or more advanced logic
                });
            }
        }

        info!("Generated weapon upgrades: {:?}", upgrades);

        upgrades
    }

    pub fn generate_choices(
        &self,
        weapon_upgrade_config: &WeaponUpgradeConfig,
        luck: &Luck,
        weapons: &[&WeaponMeta],
    ) -> Vec<UpgradeChoice> {
        let mut rng = thread_rng();

        // Determine the number of upgrades to show
        let count = Self::calculate_count(luck, &mut rng);

        // Generate weapon-specific upgrades
        let mut choices = Self::generate_weapon_upgrades(weapon_upgrade_config, weapons);

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
    }
}
