use crate::components::{Health, Luck, Player};
use crate::inventory::{PassiveCatalog, PassiveInventory, PassiveType, MAX_WEAPONS};
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
use strum::IntoEnumIterator;

pub fn handle_generic_upgrade(
    mut upgrade_events: MessageReader<GenericUpgradeConfirmedEvent>,
    mut player_query: Query<&mut Health, With<Player>>,
    mut stats: ResMut<crate::resources::GameStats>,
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
            GenericUpgrade::GoldPickup(amount) => stats.gold = stats.gold.saturating_add(amount),
        }
    }
}

#[derive(Resource, Default)]
pub struct UpgradePool;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GenericUpgrade {
    HealthPickup(i32), // Amount to heal
    GoldPickup(u32),
}

impl std::fmt::Display for GenericUpgrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenericUpgrade::HealthPickup(_) => write!(f, "Philosopher's Elixir"),
            GenericUpgrade::GoldPickup(_) => write!(f, "Gold purse"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum UpgradeType {
    Weapon(WeaponType, WeaponUpgradeSpec),
    AcquireWeapon(WeaponType),
    Passive(PassiveType, u8),
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
                let description = format!(
                    "Level {next_level}: {changes}{}",
                    weapon_meta.weapon_type.level_bonus(next_level)
                );

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
        passives: &PassiveInventory,
        catalog: &PassiveCatalog,
        rng: &mut impl Rng,
    ) -> Vec<UpgradeChoice> {
        let count = Self::calculate_count(luck, rng);
        let mut choices = Self::generate_weapon_upgrades(weapon_upgrade_config, weapons);
        if weapons.len() < MAX_WEAPONS {
            for weapon in WeaponType::iter() {
                if weapons.iter().any(|owned| owned.weapon_type == weapon) {
                    continue;
                }
                choices.push(UpgradeChoice {
                    upgrade_type: UpgradeType::AcquireWeapon(weapon),
                    description: format!("New weapon: {}", weapon.summary()),
                    rarity: Rarity::Common,
                });
            }
        }
        for definition in &catalog.items {
            if passives.can_upgrade(definition.kind, catalog) {
                let level = passives.level(definition.kind) + 1;
                choices.push(UpgradeChoice {
                    upgrade_type: UpgradeType::Passive(definition.kind, level),
                    description: format!(
                        "{} - {}",
                        definition.name,
                        definition.level_description(level)
                    ),
                    rarity: Rarity::Common,
                });
            }
        }
        // Useful permanent upgrades take precedence; healing remains available
        // when the inventory cannot fill the normal offer count.
        if choices.len() < count {
            choices.extend(Self::generate_generic_choices());
        }
        Self::select_random_owned(choices, count, rng)
    }

    fn calculate_count(luck: &Luck, rng: &mut impl Rng) -> usize {
        const LUCK_FACTOR: f32 = 0.02;
        if rng.random::<f32>() < (luck.0 as f32) * LUCK_FACTOR {
            4
        } else {
            3
        }
    }

    fn select_random_owned<T, I>(iter: I, count: usize, rng: &mut impl Rng) -> Vec<T>
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
            parent.spawn((
                ImageNode::default(),
                crate::art::ItemIcon(icon),
                Node {
                    width: Val::Px(52.0),
                    height: Val::Px(52.0),
                    ..default()
                },
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

fn get_upgrade_display_info(choice: &UpgradeChoice) -> (usize, String, String) {
    match &choice.upgrade_type {
        UpgradeType::Weapon(weapon_type, ..) | UpgradeType::AcquireWeapon(weapon_type) => {
            let icon = weapon_type.icon();
            (
                icon,
                format!("{} Weapon", weapon_type),
                choice.description.clone(),
            )
        }
        UpgradeType::Passive(kind, _) => {
            let icon = match kind {
                PassiveType::MoveSpeed => 3,
                PassiveType::PickupRange => 4,
                PassiveType::MaxHealth => 5,
                PassiveType::Cooldown => 6,
            };
            // The offer embeds the authored item name, so this stays independent
            // of resource access while the menu creates its hierarchy.
            let name = choice
                .description
                .split(" - ")
                .next()
                .unwrap_or("Passive item")
                .to_owned();
            (icon, name, choice.description.clone())
        }
        UpgradeType::Generic(generic_type) => {
            let icon = match generic_type {
                GenericUpgrade::HealthPickup(_) => 7,
                GenericUpgrade::GoldPickup(_) => 9,
            };
            (
                icon,
                format!("{}", generic_type),
                choice.description.clone(),
            )
        }
    }
}
