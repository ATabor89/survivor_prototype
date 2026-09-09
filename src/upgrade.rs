use crate::components::{Health, Luck, Player};
use crate::menu;
use crate::menu::{
    GenericUpgradeConfirmedEvent, MenuAction, MenuActionComponent, MenuItem, UpgradeChoice,
};
use crate::types::Rarity;
use crate::weapons::weapon_upgrade::{WeaponUpgradeConfig, WeaponUpgradeSpec};
use crate::weapons::{WeaponMeta, WeaponType};
use bevy::color::{Alpha, Color};

use bevy::log::info;
use bevy::prelude::*;
use rand::prelude::*;
use std::cmp::Ordering;

pub fn handle_generic_upgrade(
    mut upgrade_events: MessageReader<GenericUpgradeConfirmedEvent>,
    mut player_query: Query<&mut Health, With<Player>>,
) {
    for generic_upgrade_event in upgrade_events.read() {
        match generic_upgrade_event.generic_upgrade_type {
            GenericUpgrade::HealthPickup(amount) => {
                if let Ok(mut health) = player_query.single_mut() {
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
        }
    }
}

#[derive(Resource, Default)]
pub struct UpgradePool;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GenericUpgrade {
    HealthPickup(i32), // Amount to heal
}

impl std::fmt::Display for GenericUpgrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenericUpgrade::HealthPickup(_) => write!(f, "Philosopher's Elixir"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum UpgradeType {
    Weapon(WeaponType, WeaponUpgradeSpec),
    Generic(GenericUpgrade),
}

impl UpgradePool {
    pub fn generate_generic_choices() -> Vec<UpgradeChoice> {
        vec![UpgradeChoice {
            upgrade_type: UpgradeType::Generic(GenericUpgrade::HealthPickup(20)),
            description: "Restore up to 20 health with a Philosopher's Elixir".to_string(),
            rarity: Rarity::Common,
        }]
    }

    pub fn generate_weapon_upgrades(
        weapon_upgrade_config: &WeaponUpgradeConfig,
        weapons: &[&WeaponMeta],
    ) -> Vec<UpgradeChoice> {
        let mut upgrades = Vec::new();

        for weapon_meta in weapons {
            // Fetch the next upgrades from your config, using the weapon’s current level
            let specs =
                weapon_upgrade_config.get_next_upgrades(weapon_meta.weapon_type, weapon_meta.level);

            // For logging or reference
            let next_level = weapon_meta.level + 1;

            // Convert each `WeaponUpgradeSpec` to an `UpgradeChoice`
            for spec in specs {
                let changes = spec
                    .changes
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ");
                let description = format!("Level {next_level}: {changes}");

                upgrades.push(UpgradeChoice {
                    upgrade_type: UpgradeType::Weapon(weapon_meta.weapon_type, spec),
                    description,
                    rarity: Rarity::Common, // or more advanced logic
                });
            }
        }

        upgrades
    }

    pub fn generate_choices(
        &self,
        weapon_upgrade_config: &WeaponUpgradeConfig,
        luck: &Luck,
        weapons: &[&WeaponMeta],
    ) -> Vec<UpgradeChoice> {
        let mut rng = rand::rng();

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

    fn calculate_count(luck: &Luck, rng: &mut impl RngExt) -> usize {
        const LUCK_FACTOR: f32 = 0.02;
        if rng.random::<f32>() < (luck.0 as f32) * LUCK_FACTOR {
            4
        } else {
            3
        }
    }

    fn select_random_owned<T, I>(iter: I, count: usize, rng: &mut impl RngExt) -> Vec<T>
    where
        I: IntoIterator<Item = T>,
    {
        iter.into_iter().sample(rng, count)
    }
}

pub fn spawn_upgrade_choice(
    parent: &mut ChildSpawnerCommands,
    choice: UpgradeChoice,
    is_first: bool,
) {
    let (icon, name, description) = get_upgrade_display_info(&choice);

    parent
        .spawn((
            Button,
            BorderColor::all(menu::get_rarity_color(&choice.rarity).with_alpha(0.5)),
            BackgroundColor(if is_first {
                Color::srgb(0.3, 0.3, 0.4)
            } else {
                Color::srgb(0.2, 0.2, 0.2)
            }),
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(100.0),
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
                    font_size: FontSize::Px(32.0),
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
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(menu::get_rarity_color(&choice.rarity)),
                    ));

                    // Description
                    parent.spawn((
                        Text::new(description),
                        TextFont {
                            font_size: FontSize::Px(18.0),
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
                WeaponType::MagickCircle => "O",
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
                GenericUpgrade::HealthPickup(_) => "+",
            };
            (
                icon,
                format!("{}", generic_type),
                choice.description.clone(),
            )
        }
    }
}
