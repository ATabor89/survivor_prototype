use crate::components::{Luck, Player};
use crate::death::MarkedForDespawn;
use crate::resources::GameState;
use crate::types::Rarity;
use crate::upgrade;
use crate::upgrade::{GenericUpgrade, UpgradePool, UpgradeType};
use crate::weapons::weapon_upgrade::{WeaponUpgradeConfig, WeaponUpgradeSpec};
use crate::weapons::{WeaponMeta, WeaponType};
use bevy::prelude::*;

// Base menu components
#[derive(Component)]
pub struct MenuRoot {
    pub menu_type: MenuType,
}

#[derive(Component)]
pub struct MenuItem {
    pub selected: bool,
}

#[derive(Component)]
pub struct MenuActionComponent {
    pub action: MenuAction,
}

#[derive(Component, Debug)]
pub enum MenuType {
    Main,
    Pause,
    LevelUp,
}

// Simplified menu actions
#[derive(Clone)]
pub enum MenuAction {
    StartGame,
    ResumeGame,
    QuitGame,
    SelectUpgrade(UpgradeChoice),
}

// Level-up specific components
#[derive(Debug, Component, Clone)]
pub struct UpgradeChoice {
    pub upgrade_type: UpgradeType,
    pub description: String,
    pub rarity: Rarity,
}

#[derive(Event)]
pub struct WeaponUpgradeConfirmedEvent {
    pub weapon_type: WeaponType,
    pub upgrade_spec: WeaponUpgradeSpec,
}

#[derive(Event)]
pub struct GenericUpgradeConfirmedEvent {
    pub generic_upgrade_type: GenericUpgrade,
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum MenuSystemSet {
    Navigation,
    Selection,
    Confirmation,
}

pub fn spawn_level_up_menu(
    mut commands: Commands,
    weapon_upgrade_config: Res<WeaponUpgradeConfig>,
    upgrade_pool: Res<UpgradePool>,
    existing_menu: Query<Entity, With<MenuRoot>>,
    weapon_query: Query<&WeaponMeta>,
    luck_query: Query<(&Player, &Luck)>,
) {
    if !existing_menu.is_empty() {
        return;
    }

    let Ok((_player, luck)) = luck_query.get_single() else {
        panic!("Unable to get player luck");
    };

    let weapons = weapon_query.iter().collect::<Vec<_>>();

    info!("Generating choices for level up menu");

    // Generate 3 random upgrade choices
    let choices = upgrade_pool.generate_choices(weapon_upgrade_config.as_ref(), luck, &weapons);

    info!("Choices: {:?}", choices);

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..default()
            },
            GlobalZIndex(100), // Ensure it's on top
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            MenuRoot {
                menu_type: MenuType::LevelUp,
            },
            MenuType::LevelUp,
        ))
        .with_children(|parent| {
            // Container for upgrade choices
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(20.0),
                        width: Val::Px(600.0), // Made wider
                        padding: UiRect::all(Val::Px(30.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BorderColor(Color::srgb(0.7, 0.7, 0.7)),
                    BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                ))
                .with_children(|parent| {
                    // Level Up Title
                    parent.spawn((
                        Text::new("Level Up!"),
                        TextFont {
                            font_size: 48.0, // Made larger
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.8, 0.0)), // Gold color
                    ));

                    // Spawn upgrade choices
                    for (index, choice) in choices.iter().enumerate() {
                        upgrade::spawn_upgrade_choice(parent, choice.clone(), index == 0);
                    }
                });
        });
}

pub(crate) fn get_rarity_color(rarity: &Rarity) -> Color {
    match rarity {
        Rarity::Common => Color::srgb(0.8, 0.8, 0.8),
        Rarity::Uncommon => Color::srgb(0.0, 0.8, 0.0),
        Rarity::Rare => Color::srgb(0.0, 0.5, 1.0),
        Rarity::Epic => Color::srgb(0.6, 0.0, 0.8),
        Rarity::Legendary => Color::srgb(1.0, 0.5, 0.0),
    }
}

// Navigation systems
pub fn menu_navigation(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut menu_query: Query<(Entity, &mut MenuItem, &MenuActionComponent, &Parent), With<Button>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let mut items: Vec<_> = menu_query.iter_mut().collect();

    if items.is_empty() {
        return;
    }

    // Find currently selected item
    let current_selected = items
        .iter()
        .position(|(_, item, _, _)| item.selected)
        .unwrap_or(0);

    // Calculate new selected index
    let items_len = items.len();
    let new_selected = if keyboard.just_pressed(KeyCode::ArrowUp)
        || keyboard.just_pressed(KeyCode::KeyW)
    {
        (current_selected + items_len - 1) % items_len
    } else if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
        (current_selected + 1) % items_len
    } else {
        current_selected
    };

    // Update selection states
    for (i, (_, ref mut item, _, _)) in items.iter_mut().enumerate() {
        item.selected = i == new_selected;
    }

    // Handle selection
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        if let Some((_, _, action_component, _)) = items.get(new_selected) {
            handle_menu_action(&action_component.action, &mut next_state);
        }
    }
}

// Basic pause menu spawning system
pub fn spawn_pause_menu(mut commands: Commands, existing_menu: Query<(Entity, &MenuRoot)>) {
    for (entity, root) in existing_menu.iter() {
        info!(
            "Found existing menu: {:?} of type {:?}",
            entity, root.menu_type
        );
    }

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            MenuRoot {
                menu_type: MenuType::Pause,
            },
        ))
        .with_children(|parent| {
            spawn_menu_container(parent, |parent| {
                spawn_menu_button(parent, "Resume", MenuAction::ResumeGame, true);
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
            Button { ..default() },
            Node {
                width: Val::Px(200.0),
                height: Val::Px(50.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
            MenuItem { selected },
            MenuActionComponent { action },
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(text),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

pub fn spawn_menu_container(
    parent: &mut ChildBuilder,
    spawn_content: impl FnOnce(&mut ChildBuilder),
) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(20.0),
                padding: UiRect::all(Val::Px(30.0)),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BorderColor(Color::srgb(0.7, 0.7, 0.7)),
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
        ))
        .with_children(|parent| {
            spawn_content(parent);
        });
}

pub fn cleanup_menu_state(
    mut commands: Commands,
    query: Query<Entity, (With<MenuRoot>, Without<MarkedForDespawn>)>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

pub fn update_menu_buttons(
    mut buttons: Query<(&MenuItem, &mut BackgroundColor, &Children, &Interaction)>,
    mut text_query: Query<(&mut Text, &mut TextColor)>,
) {
    for (menu_item, mut background_color, children, interaction) in buttons.iter_mut() {
        // Enhanced visual feedback
        let bg_color = match (*interaction, menu_item.selected) {
            (Interaction::Pressed, _) => Color::srgb(0.2, 0.2, 0.2),
            (Interaction::Hovered, _) => Color::srgb(0.4, 0.4, 0.4),
            (Interaction::None, true) => Color::srgb(0.35, 0.35, 0.4),
            (Interaction::None, false) => Color::srgb(0.3, 0.3, 0.3),
        };
        background_color.0 = bg_color;

        // Update text color
        if let Some(&child) = children.first() {
            if let Ok((_, mut text_color)) = text_query.get_mut(child) {
                text_color.0 = if menu_item.selected || matches!(interaction, Interaction::Hovered)
                {
                    Color::srgb(1.0, 0.84, 0.0)
                } else {
                    Color::WHITE
                };
            }
        }
    }
}

pub fn handle_menu_interactions(
    mut buttons: Query<(&Interaction, &mut MenuItem, &MenuActionComponent), With<Button>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, mut menu_item, action_component) in buttons.iter_mut() {
        // Only modify selection via mouse if the item isn't already selected via keyboard
        match *interaction {
            Interaction::Pressed => {
                handle_menu_action(&action_component.action, &mut next_state);
            }
            Interaction::Hovered => {
                // Only update selection via hover if not already selected (preserves keyboard selection)
                if !menu_item.selected {
                    menu_item.selected = true;
                }
            }
            Interaction::None => {
                // Only deselect if this was a mouse selection (hover)
                // This preserves keyboard selection
                if menu_item.selected && button_was_hovered(interaction) {
                    menu_item.selected = false;
                }
            }
        }
    }
}

fn button_was_hovered(interaction: &Interaction) -> bool {
    matches!(interaction, Interaction::Hovered)
}

fn handle_menu_action(action: &MenuAction, next_state: &mut NextState<GameState>) {
    match action {
        MenuAction::StartGame => next_state.set(GameState::Playing),
        MenuAction::ResumeGame => next_state.set(GameState::Playing),
        // MenuAction::OpenSettings => next_state.set(GameState::Playing), // Until settings is implemented
        MenuAction::QuitGame => next_state.set(GameState::Quit),
        MenuAction::SelectUpgrade(_) => {} // Handled by upgrade system
    }
}

pub fn handle_upgrade_selection_and_confirmation(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    menu_query: Query<(Entity, &MenuType)>,
    menu_items: Query<(&MenuItem, &MenuActionComponent, &Interaction)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut weapon_upgrade_events: EventWriter<WeaponUpgradeConfirmedEvent>,
    mut generic_upgrade_events: EventWriter<GenericUpgradeConfirmedEvent>,
) {
    // Only process for level up menu
    if !menu_query
        .iter()
        .any(|(_, menu_type)| matches!(menu_type, MenuType::LevelUp))
    {
        return;
    }

    // Handle confirmation via keyboard or mouse
    for (menu_item, action_component, interaction) in menu_items.iter() {
        let should_confirm = (menu_item.selected
            && (keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space)))
            || *interaction == Interaction::Pressed;

        if should_confirm {
            if let MenuAction::SelectUpgrade(upgrade) = &action_component.action {
                match &upgrade.upgrade_type {
                    UpgradeType::Weapon(weapon_type, weapon_upgrade_spec) => {
                        // Send the upgrade event
                        weapon_upgrade_events.send(WeaponUpgradeConfirmedEvent {
                            weapon_type: *weapon_type,
                            upgrade_spec: weapon_upgrade_spec.clone(),
                        });
                    }
                    UpgradeType::Generic(generic_upgrade) => {
                        generic_upgrade_events.send(GenericUpgradeConfirmedEvent {
                            generic_upgrade_type: generic_upgrade.clone(),
                        });
                    }
                }

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

// Plugin to organize it all
pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<WeaponUpgradeConfirmedEvent>()
            .configure_sets(
                Update,
                (
                    MenuSystemSet::Navigation,
                    MenuSystemSet::Selection,
                    MenuSystemSet::Confirmation,
                )
                    .chain(),
            )
            // Menu systems
            .add_systems(
                Update,
                (
                    menu_navigation,
                    handle_menu_interactions,
                    update_menu_buttons,
                    handle_upgrade_selection_and_confirmation,
                )
                    .chain()
                    .run_if(in_state(GameState::LevelUp).or(in_state(GameState::Paused))),
            )
            // State transitions
            .add_systems(OnEnter(GameState::Paused), spawn_pause_menu)
            .add_systems(OnExit(GameState::Paused), cleanup_menu_state)
            .add_systems(OnEnter(GameState::LevelUp), spawn_level_up_menu)
            .add_systems(OnExit(GameState::LevelUp), cleanup_menu_state);
    }
}
