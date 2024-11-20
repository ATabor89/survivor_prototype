use crate::components::PlayerStats;
use crate::resources::{GameState, UpgradePool};
use crate::types::{EquipmentType, Rarity, StatType, WeaponType};
use bevy::prelude::*;

// Base menu components
#[derive(Component)]
pub struct MenuRoot;

#[derive(Component)]
pub struct MenuItem {
    pub selected: bool,
}

#[derive(Component)]
pub struct MenuActionComponent {
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

#[derive(Event)]
pub struct UpgradeConfirmedEvent {
    pub upgrade: UpgradeChoice,
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum MenuSystemSet {
    Navigation,
    Selection,
    Confirmation,
}

// Systems for all menus
pub fn menu_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut menu_query: Query<(&MenuType, &mut MenuItem)>,
) {
    // Handle common input behaviors
}

pub fn spawn_level_up_menu(mut commands: Commands, upgrade_pool: Res<UpgradePool>) {
    // Generate 3 random upgrade choices
    let choices = upgrade_pool.generate_choices(3);
    info!("Generated {} upgrade choices", choices.len());
    for choice in choices.iter() {
        info!("Choice: {:?}", choice.upgrade_type);
    }

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    ..default()
                },
                z_index: ZIndex::Global(100), // Ensure it's on top
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
                ..default()
            },
            MenuRoot,
            MenuType::LevelUp,
        ))
        .with_children(|parent| {
            // Container for upgrade choices
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(20.0),
                        width: Val::Px(600.0), // Made wider
                        padding: UiRect::all(Val::Px(30.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    border_color: BorderColor(Color::srgb(0.7, 0.7, 0.7)),
                    background_color: BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                    ..default()
                })
                .with_children(|parent| {
                    // Level Up Title
                    parent.spawn(TextBundle::from_section(
                        "Level Up!",
                        TextStyle {
                            font_size: 48.0,                   // Made larger
                            color: Color::srgb(1.0, 0.8, 0.0), // Gold color
                            ..default()
                        },
                    ));

                    // Spawn upgrade choices
                    for (index, choice) in choices.iter().enumerate() {
                        spawn_upgrade_choice(parent, choice.clone(), index == 0);
                    }
                });
        });
}

fn spawn_upgrade_choice(parent: &mut ChildBuilder, choice: UpgradeChoice, is_first: bool) {
    let (icon, name, description) = get_upgrade_display_info(&choice);

    parent
        .spawn((
            ButtonBundle {
                style: Style {
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
                border_color: BorderColor(get_rarity_color(&choice.rarity).with_alpha(0.5)),
                background_color: BackgroundColor(if is_first {
                    Color::srgb(0.3, 0.3, 0.4)
                } else {
                    Color::srgb(0.2, 0.2, 0.2)
                }),
                ..default()
            },
            MenuItem {
                selected: is_first,
            },
            MenuActionComponent { action: MenuAction::SelectUpgrade(choice.clone()), }
        ))
        .with_children(|parent| {
            // Icon placeholder
            parent.spawn(TextBundle::from_section(
                icon,
                TextStyle {
                    font_size: 32.0, // Made larger
                    color: get_rarity_color(&choice.rarity),
                    ..default()
                },
            ));

            // Text container
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    // Upgrade name
                    parent.spawn(TextBundle::from_section(
                        name,
                        TextStyle {
                            font_size: 24.0, // Made larger
                            color: get_rarity_color(&choice.rarity),
                            ..default()
                        },
                    ));

                    // Description
                    parent.spawn(TextBundle::from_section(
                        description,
                        TextStyle {
                            font_size: 18.0, // Made larger
                            color: Color::srgb(0.8, 0.8, 0.8),
                            ..default()
                        },
                    ));
                });
        });
}

fn get_rarity_color(rarity: &Rarity) -> Color {
    match rarity {
        Rarity::Common => Color::srgb(0.8, 0.8, 0.8),
        Rarity::Uncommon => Color::srgb(0.0, 0.8, 0.0),
        Rarity::Rare => Color::srgb(0.0, 0.5, 1.0),
        Rarity::Epic => Color::srgb(0.6, 0.0, 0.8),
        Rarity::Legendary => Color::srgb(1.0, 0.5, 0.0),
    }
}

fn get_upgrade_display_info(choice: &UpgradeChoice) -> (&'static str, String, String) {
    match &choice.upgrade_type {
        UpgradeType::Weapon(weapon_type) => {
            let icon = match weapon_type {
                WeaponType::Sword => "⚔️",
                WeaponType::Axe => "🪓",
                WeaponType::Spear => "🔱",
                WeaponType::Bow => "🏹",
                WeaponType::Magic => "🔮",
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

// Navigation systems
pub fn standard_menu_navigation(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut menu_query: Query<(Entity, &mut MenuItem, &MenuActionComponent)>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let mut items: Vec<_> = menu_query.iter_mut().collect();

    if items.is_empty() {
        return;
    }

    // Find currently selected item
    let current_selected = items
        .iter()
        .position(|(_, item, _)| item.selected)
        .unwrap_or(0);

    // Calculate new selected index
    let items_len = items.len();
    let new_selected = if keyboard.just_pressed(KeyCode::ArrowUp)
        || keyboard.just_pressed(KeyCode::KeyW)
    {
        (current_selected + items_len - 1) % items_len
    } else if keyboard.just_pressed(KeyCode::ArrowDown)
        || keyboard.just_pressed(KeyCode::KeyS)
    {
        (current_selected + 1) % items_len
    } else {
        current_selected
    };

    // Update selection states
    for (i, (_, ref mut item, _)) in items.iter_mut().enumerate() {
        item.selected = i == new_selected;
    }

    // Handle selection
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        if let Some((_, _, action_component)) = items.get(new_selected) {
            handle_menu_action(&action_component.action, &mut next_state);
        }
    }
}

pub fn level_up_menu_navigation(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut menu_query: Query<(&MenuType, &mut MenuItem, &MenuActionComponent)>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let mut items: Vec<_> = menu_query
        .iter_mut()
        .filter(|(menu_type, _, _)| matches!(menu_type, MenuType::LevelUp))
        .collect();

    if items.is_empty() {
        return;
    }

    // Similar navigation logic to standard_menu_navigation
    let current_selected = items
        .iter()
        .position(|(_, item, _)| item.selected)
        .unwrap_or(0);

    // Calculate new selected index
    let items_len = items.len();
    let new_selected = if keyboard.just_pressed(KeyCode::ArrowUp)
        || keyboard.just_pressed(KeyCode::KeyW)
    {
        (current_selected + items_len - 1) % items_len
    } else if keyboard.just_pressed(KeyCode::ArrowDown)
        || keyboard.just_pressed(KeyCode::KeyS)
    {
        (current_selected + 1) % items_len
    } else {
        current_selected
    };

    // Update selection states
    for (i, (_, ref mut item, _)) in items.iter_mut().enumerate() {
        item.selected = i == new_selected;
    }

    // Handle selection (Enter or Space)
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        if let Some((_, _, action_component)) = items.get(current_selected) {
            handle_menu_action(&action_component.action, &mut next_state);
        }
    }
}

pub fn cleanup_menu(mut commands: Commands, menu_query: Query<Entity, With<MenuRoot>>) {
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
        }
        MenuAction::QuitGame => next_state.set(GameState::Quit),
        MenuAction::SelectUpgrade(_) => {} // Handled elsewhere
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
                UpgradeType::Weapon(weapon_type) => {
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
            MenuItem { selected },
            MenuActionComponent { action },
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

pub fn update_menu_buttons(
    mut buttons: Query<(&MenuItem, &mut BackgroundColor, &Children, &Interaction)>,
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

pub fn handle_upgrade_selection(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    menu_query: Query<(Entity, &MenuType)>,
    menu_items: Query<(Entity, &MenuItem, &MenuActionComponent, &Interaction)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut upgrade_events: EventWriter<UpgradeConfirmedEvent>,
) {
    // Only process for level up menu
    if !menu_query
        .iter()
        .any(|(_, menu_type)| matches!(menu_type, MenuType::LevelUp))
    {
        return;
    }

    // Handle confirmation via keyboard or mouse
    for (entity, menu_item, action_component, interaction) in menu_items.iter() {
        let should_confirm = (menu_item.selected
            && (keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space)))
            || *interaction == Interaction::Pressed;

        if should_confirm {
            if let MenuAction::SelectUpgrade(upgrade) = &action_component.action {
                // Send the upgrade event
                upgrade_events.send(UpgradeConfirmedEvent {
                    upgrade: upgrade.clone(),
                });

                // Clean up menu
                for (menu_entity, _) in menu_query.iter() {
                    commands.entity(menu_entity).despawn_recursive();
                }

                // Return to playing state
                next_state.set(GameState::Playing);
                break;
            }
        }
    }
}

fn handle_upgrade_confirmation(
    commands: &mut Commands,
    next_state: &mut NextState<GameState>,
    upgrade_events: &mut EventWriter<UpgradeConfirmedEvent>,
    upgrade: UpgradeChoice,
    menu_query: Query<(Entity, &MenuType)>,
) {
    // Send the upgrade event
    upgrade_events.send(UpgradeConfirmedEvent { upgrade });

    // Clean up menu
    for (entity, _) in menu_query.iter() {
        commands.entity(entity).despawn_recursive();
    }

    // Return to playing state
    next_state.set(GameState::Playing);
}

fn get_selected_upgrade(
    menu_items: &Query<(&Interaction, &MenuActionComponent), Changed<Interaction>>,
) -> Option<UpgradeChoice> {
    for (interaction, action) in menu_items.iter() {
        if *interaction == Interaction::Hovered {
            if let MenuAction::SelectUpgrade(ref upgrade) = action.action {
                return Some(upgrade.clone());
            }
        }
    }
    None
}

pub fn get_selected_menu_action(
    menu_items: &Query<(&MenuItem, &MenuActionComponent)>,
) -> Option<MenuAction> {
    for (item, action_component) in menu_items.iter() {
        if item.selected {
            return Some(action_component.action.clone());
        }
    }
    None
}

// Plugin to organize it all
pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        // Add our event
        app.add_event::<UpgradeConfirmedEvent>();

        // Configure our system sets
        app.configure_sets(
            Update,
            (
                MenuSystemSet::Navigation,
                MenuSystemSet::Selection,
                MenuSystemSet::Confirmation,
            ).chain()
        );

        // Add systems for the level up menu state
        app.add_systems(
            Update,
            (
                menu_hover_system,
                standard_menu_navigation,
                handle_upgrade_selection,
                update_menu_buttons,
                apply_confirmed_upgrade,
            )
                .chain() // Use chain() to ensure sequential execution
                .run_if(in_state(GameState::LevelUp))
        );

        // Add systems for the pause menu state
        app.add_systems(
            Update,
            (
                standard_menu_navigation,
                handle_button_interactions,
                update_menu_buttons,
            )
                .chain()
                .run_if(in_state(GameState::Paused))
        );
    }
}

fn menu_hover_system(mut buttons: Query<(&Interaction, &mut MenuItem)>) {
    for (interaction, mut menu_item) in buttons.iter_mut() {
        // Only update selection on hover if not already selected by keyboard
        if !menu_item.selected {
            menu_item.selected = matches!(interaction, Interaction::Hovered);
        }
    }
}

fn handle_button_interactions(
    mut next_state: ResMut<NextState<GameState>>,
    buttons: Query<(&Interaction, &MenuItem, &MenuActionComponent), Changed<Interaction>>,
) {
    for (interaction, _, action_component) in buttons.iter() {
        if *interaction == Interaction::Pressed {
            handle_menu_action(&action_component.action, &mut next_state);
        }
    }
}

// Helper function to determine if we're in level-up menu
pub fn in_level_up_menu(menu_query: Query<&MenuType>) -> bool {
    menu_query
        .iter()
        .any(|menu_type| matches!(menu_type, MenuType::LevelUp))
}
