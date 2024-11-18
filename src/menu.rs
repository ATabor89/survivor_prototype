use bevy::prelude::*;
use crate::components::PlayerStats;
use crate::resources::{GameState, UpgradePool};
use crate::types::{EquipmentType, Rarity, StatType, WeaponType};

// Base menu components
#[derive(Component)]
pub struct MenuRoot;

#[derive(Component)]
pub struct MenuItem {
    pub selected: bool,
    pub action: MenuAction,
}

#[derive(Component)]
pub struct MenuText;

// Specific menu types
#[derive(Component)]
pub enum MenuType {
    Main,
    Pause,
    Settings,
    LevelUp,
}

// Actions that can be triggered by menu items
#[derive(Clone)]
pub enum MenuAction {
    // Standard menu actions
    StartGame,
    ResumeGame,
    OpenSettings,
    QuitGame,

    // Level-up specific actions
    SelectUpgrade(UpgradeChoice),
}

// Level-up specific components
#[derive(Component, Clone)]
pub struct UpgradeChoice {
    pub upgrade_type: UpgradeType,
    pub description: String,
    pub rarity: Rarity,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UpgradeType {
    Weapon(WeaponType),
    Equipment(EquipmentType),
    Stat(StatType),
}

// Systems for all menus
pub fn menu_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut menu_query: Query<(&MenuType, &mut MenuItem)>,
) {
    // Handle common input behaviors
}

// Specific systems for level-up menu
pub fn spawn_level_up_menu(
    mut commands: Commands,
    player_stats: Query<&PlayerStats>,
    upgrade_pool: Res<UpgradePool>,
) {
    commands.spawn((
        MenuRoot,
        MenuType::LevelUp,
        NodeBundle {
            // UI styling
            ..default()
        },
    ))
        .with_children(|parent| {
            // Generate random upgrade choices
            let choices = upgrade_pool.generate_choices(3);

            for choice in choices {
                spawn_upgrade_choice(parent, choice);
            }
        });
}

pub fn spawn_upgrade_choice(parent: &mut ChildBuilder, choice: UpgradeChoice) {
    parent.spawn((
        MenuItem {
            selected: false,
            action: MenuAction::SelectUpgrade(choice.clone()),
        },
        // UI styling and layout
    ))
        .with_children(|choice_parent| {
            // Add icon, description, etc.
        });
}

// Navigation systems
pub fn standard_menu_navigation(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut menu_query: Query<(Entity, &mut MenuItem)>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let mut items: Vec<_> = menu_query.iter_mut().collect();

    if items.is_empty() {
        return;
    }

    // Find currently selected item
    let current_selected = items
        .iter()
        .position(|(_, item)| item.selected)
        .unwrap_or(0);

    // Calculate new selected index
    let items_len = items.len();
    let new_selected = if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
        (current_selected + items_len - 1) % items_len
    } else if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
        (current_selected + 1) % items_len
    } else {
        current_selected
    };

    // Update selection states
    for (i, (_, ref mut item)) in items.iter_mut().enumerate() {
        item.selected = i == new_selected;
    }

    // Handle selection
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        if let Some((_, item)) = items.get(new_selected) {
            handle_menu_action(&item.action, &mut next_state);
        }
    }
}

pub fn level_up_menu_navigation(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut menu_query: Query<(&MenuType, &mut MenuItem)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut player_query: Query<&mut PlayerStats>,
) {
    let mut items: Vec<_> = menu_query
        .iter_mut()
        .filter(|(menu_type, _)| matches!(menu_type, MenuType::LevelUp))
        .collect();

    if items.is_empty() {
        return;
    }

    // Similar navigation logic to standard_menu_navigation
    let current_selected = items
        .iter()
        .position(|(_, item)| item.selected)
        .unwrap_or(0);

    // Calculate new selected index
    let items_len = items.len();
    let new_selected = if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
        (current_selected + items_len - 1) % items_len
    } else if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
        (current_selected + 1) % items_len
    } else {
        current_selected
    };

    // Update selection states
    for (i, (_, ref mut item)) in items.iter_mut().enumerate() {
        item.selected = i == new_selected;
    }

    // Handle selection (Enter or Space)
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        if let Some((_, item)) = items.get(current_selected) {
            handle_menu_action(&item.action, &mut next_state);
        }
    }
}

pub fn cleanup_menu(
    mut commands: Commands,
    menu_query: Query<Entity, With<MenuRoot>>,
) {
    for entity in menu_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

// Helper functions
fn handle_menu_action(action: &MenuAction, next_state: &mut NextState<GameState>) {
    match action {
        MenuAction::StartGame => next_state.set(GameState::Playing),
        MenuAction::ResumeGame => next_state.set(GameState::Playing),
        MenuAction::OpenSettings => {
            // For now, just return to Playing state since Settings isn't implemented
            println!("Settings menu not yet implemented!");
            next_state.set(GameState::Playing);
            // When implementing settings, use:
            // next_state.set(GameState::Settings);
        },
        MenuAction::QuitGame => next_state.set(GameState::Quit),
        MenuAction::SelectUpgrade(_) => {}, // Handled elsewhere
    }
}

pub fn apply_upgrade(upgrade: &UpgradeChoice, player_query: &mut Query<&mut PlayerStats>) {
    if let Ok(mut stats) = player_query.get_single_mut() {
        match &upgrade.upgrade_type {
            UpgradeType::Weapon(_) => {
                // Add weapon to inventory
            }
            UpgradeType::Equipment(_) => {
                // Add equipment to inventory
            }
            UpgradeType::Stat(stat_type) => {
                match stat_type {
                    StatType::Health => stats.health *= 1.1,
                    StatType::Speed => stats.speed *= 1.1,
                    StatType::Attack => stats.attack *= 1.1,
                    StatType::Defense => stats.defense *= 1.1,
                    StatType::Luck => stats.luck *= 1.1,
                }
            }
        }
    }
}

// Basic pause menu spawning system
pub fn spawn_pause_menu(mut commands: Commands) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
                ..default()
            },
            MenuRoot,
            MenuType::Pause,
        ))
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(10.0),
                        padding: UiRect::all(Val::Px(20.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                    ..default()
                })
                .with_children(|parent| {
                    // Resume button
                    spawn_menu_button(parent, "Resume", MenuAction::ResumeGame, true);
                    // Settings button
                    spawn_menu_button(parent, "Settings", MenuAction::OpenSettings, false);
                    // Quit button
                    spawn_menu_button(parent, "Quit", MenuAction::QuitGame, false);
                });
        });
}

// Helper function to spawn menu buttons
pub fn spawn_menu_button(
    parent: &mut ChildBuilder,
    text: &str,
    action: MenuAction,
    selected: bool,
) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(200.0),
                    height: Val::Px(50.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                ..default()
            },
            MenuItem {
                selected,
                action,
            },
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                text,
                TextStyle {
                    font_size: 20.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));
        });
}

fn update_menu_buttons(
    mut buttons: Query<(
        &MenuItem,
        &mut BackgroundColor,
        &Children,
        &Interaction,
    )>,
    mut text_query: Query<&mut Text>,
) {
    for (menu_item, mut background_color, children, interaction) in buttons.iter_mut() {
        // Determine background color based on state
        let bg_color = match *interaction {
            Interaction::Pressed => Color::srgb(0.2, 0.2, 0.2),
            Interaction::Hovered => Color::srgb(0.4, 0.4, 0.4),
            Interaction::None => {
                if menu_item.selected {
                    Color::srgb(0.35, 0.35, 0.4)
                } else {
                    Color::srgb(0.3, 0.3, 0.3)
                }
            }
        };
        background_color.0 = bg_color;

        // Update text color if this button has child text
        if let Some(&child) = children.first() {
            if let Ok(mut text) = text_query.get_mut(child) {
                text.sections[0].style.color = if menu_item.selected {
                    Color::srgb(1.0, 1.0, 0.0)
                } else {
                    Color::WHITE
                };
            }
        }
    }
}

// Plugin to organize it all
pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                menu_hover_system,
                standard_menu_navigation,
                update_menu_buttons,
                handle_button_interactions,
            )
                .run_if(in_state(GameState::Paused))
        );
    }
}

// Add this system to handle button hover states
fn menu_hover_system(
    mut buttons: Query<(&Interaction, &mut MenuItem)>,
) {
    for (interaction, mut menu_item) in buttons.iter_mut() {
        // Only update selection on hover if not already selected by keyboard
        if !menu_item.selected {
            menu_item.selected = matches!(interaction, Interaction::Hovered);
        }
    }
}

// Add this system to handle button clicks
fn handle_button_interactions(
    mut next_state: ResMut<NextState<GameState>>,
    buttons: Query<(&Interaction, &MenuItem), Changed<Interaction>>,
) {
    for (interaction, menu_item) in buttons.iter() {
        if *interaction == Interaction::Pressed {
            handle_menu_action(&menu_item.action, &mut next_state);
        }
    }
}

// Helper function to determine if we're in level-up menu
pub fn in_level_up_menu(menu_query: Query<&MenuType>) -> bool {
    menu_query
        .iter()
        .any(|menu_type| matches!(menu_type, MenuType::LevelUp))
}