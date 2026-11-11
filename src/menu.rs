use crate::components::{Luck, Player};
use crate::inventory::{PassiveCatalog, PassiveInventory, PassiveUpgradeEvent};
use crate::resources::{GameState, GameStats};
use crate::types::Rarity;
use crate::ui_button::{button, Button, ButtonInteraction, ButtonStateQuery, Pressed};
use crate::upgrade;
use crate::upgrade::{GenericUpgrade, UpgradePool, UpgradeType};
use crate::weapons::magick_circle::PatternType;
use crate::weapons::weapon_upgrade::{WeaponUpgradeChange, WeaponUpgradeConfig, WeaponUpgradeSpec};
use crate::weapons::{AddWeaponEvent, WeaponMeta, WeaponType};
use bevy::platform::collections::HashSet;
use bevy::prelude::*;

use strum::IntoEnumIterator;

// Plugin to organize it all
pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<WeaponUpgradeConfirmedEvent>()
            .add_message::<MenuActivation>()
            .init_resource::<crate::ui::FinalBuild>()
            .add_systems(
                Update,
                crate::ui::capture_final_build
                    .after(crate::GameplaySets::Progress)
                    .before(crate::GameplaySets::Cleanup),
            )
            .add_systems(OnEnter(GameState::MainMenu), crate::ui::reset_final_build)
            .add_systems(OnEnter(GameState::Restarting), crate::ui::reset_final_build)
            .add_systems(
                Update,
                (
                    collect_menu_activation,
                    dispatch_menu_action,
                    update_menu_buttons,
                    update_pause_readouts,
                )
                    .chain(),
            )
            .add_systems(OnEnter(GameState::Paused), spawn_pause_menu)
            .add_systems(OnExit(GameState::Paused), cleanup_menu_state)
            .add_systems(OnEnter(GameState::LevelUp), spawn_level_up_menu)
            .add_systems(OnExit(GameState::LevelUp), cleanup_menu_state)
            .add_systems(
                OnEnter(GameState::GameOver),
                spawn_game_over_menu.after(crate::expedition::ConcludeExpedition),
            )
            .add_systems(OnExit(GameState::GameOver), cleanup_menu_state)
            .add_systems(
                OnEnter(GameState::Victory),
                spawn_victory_menu.after(crate::expedition::ConcludeExpedition),
            )
            .add_systems(OnExit(GameState::Victory), cleanup_menu_state);
    }
}

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

#[derive(Component, Debug, Clone, Copy, Eq, PartialEq)]
pub enum MenuType {
    Pause,
    LevelUp,
    PatternSelect,
    GameOver,
    Victory,
}

// Simplified menu actions
#[derive(Clone)]
pub enum MenuAction {
    RestartRun,
    ResumeGame,
    ReturnToTitle,
    QuitGame,
    SelectUpgrade(UpgradeChoice),
    SelectPattern(PatternType),
}

// Level-up specific components
#[derive(Debug, Component, Clone)]
pub struct UpgradeChoice {
    pub upgrade_type: UpgradeType,
    pub description: String,
    pub rarity: Rarity,
}

#[derive(Message)]
pub struct WeaponUpgradeConfirmedEvent {
    pub weapon_type: WeaponType,
    pub upgrade_spec: WeaponUpgradeSpec,
}

#[derive(Message)]
pub struct GenericUpgradeConfirmedEvent {
    pub generic_upgrade_type: GenericUpgrade,
}

#[expect(
    clippy::too_many_arguments,
    reason = "Level-up choices use the actual inventory, passive catalog, and seeded offer stream."
)]
pub fn spawn_level_up_menu(
    mut commands: Commands,
    weapon_upgrade_config: Res<WeaponUpgradeConfig>,
    upgrade_pool: Res<UpgradePool>,
    existing_menu: Query<Entity, With<MenuRoot>>,
    weapon_query: Query<(
        &WeaponMeta,
        &crate::weapons::WeaponDamage,
        &crate::weapons::WeaponArea,
        &crate::weapons::WeaponCooldown,
    )>,
    luck_query: Query<(&Luck, &PassiveInventory, &crate::components::PlayerStats), With<Player>>,
    catalog: Res<PassiveCatalog>,
    mut rng: ResMut<crate::setup::RunRng>,
) {
    if !existing_menu.is_empty() {
        return;
    }

    let Ok((luck, passives, player_stats)) = luck_query.single() else {
        return;
    };

    let weapons = weapon_query
        .iter()
        .map(|(meta, ..)| meta)
        .collect::<Vec<_>>();

    // Offer only implemented upgrades, with luck allowing an extra choice.
    let mut choices = upgrade_pool.generate_choices(
        weapon_upgrade_config.as_ref(),
        luck,
        &weapons,
        passives,
        &catalog,
        &mut rng.upgrades,
    );
    for choice in &mut choices {
        if let UpgradeType::Weapon(kind, spec) = &choice.upgrade_type {
            if let Some((meta, damage, area, cooldown)) = weapon_query
                .iter()
                .find(|(meta, ..)| meta.weapon_type == *kind)
            {
                let preview = crate::weapons::weapon_upgrade::preview_weapon_upgrade(
                    spec,
                    damage,
                    area,
                    cooldown,
                    player_stats,
                );
                choice.description = format!(
                    "Level {}: {}{}",
                    meta.level.saturating_add(1),
                    preview.description(spec),
                    kind.level_bonus(meta.level.saturating_add(1))
                );
            }
        }
    }

    debug!("Level-up choices: {:?}", choices);

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
                        width: Val::Px(600.0),
                        padding: UiRect::all(Val::Px(30.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgb(0.7, 0.7, 0.7)),
                    BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                ))
                .with_children(|parent| {
                    // Level Up Title
                    parent.spawn((
                        Text::new("Level Up!"),
                        TextFont {
                            font_size: FontSize::Px(48.0),
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.8, 0.0)), // Gold color
                    ));

                    // Spawn upgrade choices
                    for (index, choice) in choices.into_iter().enumerate() {
                        upgrade::spawn_upgrade_choice(parent, choice, index == 0);
                    }
                });
        });
}

// Add pattern selection menu spawning
pub fn spawn_pattern_select_menu(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(100),
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            MenuRoot {
                menu_type: MenuType::PatternSelect,
            },
            MenuType::PatternSelect,
        ))
        .with_children(|parent| {
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(20.0),
                    width: Val::Px(600.0),
                    padding: UiRect::all(Val::Px(30.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Select Circle Pattern"),
                        TextFont {
                            font_size: FontSize::Px(48.0),
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.8, 0.0)),
                    ));

                    // Spawn buttons for each pattern type
                    for (i, pattern) in PatternType::iter().enumerate() {
                        spawn_pattern_button(
                            parent,
                            &pattern.to_string(),
                            &pattern.get_description(),
                            MenuAction::SelectPattern(pattern),
                            i == 0, // First one selected by default
                        );
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

/// One activation per frame, scoped to the active menu root.
#[derive(Message)]
struct MenuActivation {
    root: Entity,
    action: MenuAction,
}

#[derive(Default)]
struct MenuInputState {
    root: Option<Entity>,
    pressed_buttons: HashSet<Entity>,
}

type MenuButtons<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut MenuItem,
        &'static MenuActionComponent,
        Has<Pressed>,
    ),
    With<Button>,
>;

#[cfg_attr(
    feature = "dev-tools",
    expect(
        clippy::too_many_arguments,
        reason = "Debug input capture is separate from the menu's gamepad, keyboard, hierarchy, and activation state."
    )
)]
fn collect_menu_activation(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    roots: Query<Entity, With<MenuRoot>>,
    children: Query<&Children>,
    mut buttons: MenuButtons,
    mut input: Local<MenuInputState>,
    mut activations: MessageWriter<MenuActivation>,
    #[cfg(feature = "dev-tools")] debug: Res<crate::dev_tools::DebugSession>,
) {
    let Ok(root) = roots.single() else {
        input.root = None;
        input.pressed_buttons.clear();
        return;
    };
    // Children order remains stable when buttons change ECS archetypes, and
    // unrelated UI buttons cannot join the current menu's navigation.
    let items: Vec<_> = children
        .iter_descendants(root)
        .filter(|entity| buttons.contains(*entity))
        .collect();
    let pressed_buttons: HashSet<_> = items
        .iter()
        .copied()
        .filter(|entity| buttons.get(*entity).is_ok_and(|(_, _, pressed)| pressed))
        .collect();
    // A new root never consumes the input that opened it. Track held mouse
    // buttons too, so a held click cannot select an item on a replacement menu.
    if input.root != Some(root) {
        input.root = Some(root);
        input.pressed_buttons = pressed_buttons;
        return;
    }
    if items.is_empty() {
        return;
    }
    #[cfg(feature = "dev-tools")]
    let (keyboard_allowed, pointer_allowed) = (!debug.blocks_keyboard(), !debug.blocks_pointer());
    #[cfg(not(feature = "dev-tools"))]
    let (keyboard_allowed, pointer_allowed) = (true, true);
    let selected = items
        .iter()
        .position(|entity| buttons.get(*entity).is_ok_and(|(item, _, _)| item.selected))
        .unwrap_or(0);
    let up = keyboard_allowed
        && (keyboard.just_pressed(KeyCode::ArrowUp)
            || keyboard.just_pressed(KeyCode::KeyW)
            || gamepads
                .iter()
                .any(|pad| pad.just_pressed(GamepadButton::DPadUp)));
    let down = keyboard_allowed
        && (keyboard.just_pressed(KeyCode::ArrowDown)
            || keyboard.just_pressed(KeyCode::KeyS)
            || gamepads
                .iter()
                .any(|pad| pad.just_pressed(GamepadButton::DPadDown)));
    let selected = if up {
        (selected + items.len() - 1) % items.len()
    } else if down {
        (selected + 1) % items.len()
    } else {
        selected
    };
    for (index, entity) in items.iter().enumerate() {
        if let Ok((mut item, _, _)) = buttons.get_mut(*entity) {
            item.selected = index == selected;
        }
    }
    let clicked = items.iter().copied().find(|entity| {
        pointer_allowed
            && pressed_buttons.contains(entity)
            && !input.pressed_buttons.contains(entity)
    });
    let confirmed = keyboard_allowed
        && (keyboard.just_pressed(KeyCode::Enter)
            || keyboard.just_pressed(KeyCode::Space)
            || gamepads
                .iter()
                .any(|pad| pad.just_pressed(GamepadButton::South)));
    // An explicit click wins over keyboard focus if both occur together.
    if let Some(entity) = clicked.or_else(|| confirmed.then_some(items[selected])) {
        if let Ok((_, action, _)) = buttons.get(entity) {
            activations.write(MenuActivation {
                root,
                action: action.action.clone(),
            });
        }
    }
    input.pressed_buttons = pressed_buttons;
}

#[derive(Component, Clone, Copy)]
pub(crate) enum PauseReadout {
    Run,
    Stats,
    Weapons,
    Passives,
}

pub fn spawn_pause_menu(mut commands: Commands) {
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
            GlobalZIndex(100),
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)),
            MenuRoot {
                menu_type: MenuType::Pause,
            },
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: Val::Px(920.0),
                        max_width: Val::Percent(95.0),
                        padding: UiRect::all(Val::Px(24.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(14.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.075, 0.075, 0.10)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("Paused"),
                        TextFont {
                            font_size: FontSize::Px(30.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.94, 0.82, 0.52)),
                    ));
                    spawn_pause_readout(panel, PauseReadout::Run, 16.0);
                    panel
                        .spawn(Node {
                            column_gap: Val::Px(24.0),
                            ..default()
                        })
                        .with_children(|columns| {
                            for (kind, title, width) in [
                                (PauseReadout::Stats, "PLAYER STATS", 270.0),
                                (PauseReadout::Weapons, "WEAPONS", 300.0),
                                (PauseReadout::Passives, "PASSIVES", 235.0),
                            ] {
                                columns
                                    .spawn(Node {
                                        width: Val::Px(width),
                                        flex_direction: FlexDirection::Column,
                                        row_gap: Val::Px(10.0),
                                        ..default()
                                    })
                                    .with_children(|column| {
                                        column.spawn((
                                            Text::new(title),
                                            TextFont {
                                                font_size: FontSize::Px(15.0),
                                                ..default()
                                            },
                                            TextColor(Color::srgb(0.75, 0.68, 0.48)),
                                        ));
                                        spawn_pause_readout(column, kind, 16.0);
                                    });
                            }
                        });
                    panel
                        .spawn(Node {
                            justify_content: JustifyContent::Center,
                            column_gap: Val::Px(12.0),
                            padding: UiRect::top(Val::Px(12.0)),
                            ..default()
                        })
                        .with_children(|buttons| {
                            for (index, (label, action)) in [
                                ("Resume", MenuAction::ResumeGame),
                                ("Main menu", MenuAction::ReturnToTitle),
                                ("Quit", MenuAction::QuitGame),
                            ]
                            .into_iter()
                            .enumerate()
                            {
                                spawn_compact_button(buttons, label, action, index == 0);
                            }
                        });
                });
        });
}

fn spawn_compact_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    action: MenuAction,
    selected: bool,
) {
    parent
        .spawn((
            button(),
            Node {
                width: Val::Px(140.0),
                height: Val::Px(38.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            MenuItem { selected },
            MenuActionComponent { action },
            BackgroundColor(Color::srgb(0.22, 0.23, 0.27)),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn spawn_pause_readout(parent: &mut ChildSpawnerCommands, kind: PauseReadout, size: f32) {
    parent.spawn((
        kind,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(Color::srgb(0.85, 0.85, 0.88)),
        Node {
            width: Val::Percent(100.0),
            // Wrapped inventory rows determine their own height; do not squeeze
            // their final lines when the neighboring columns are shorter.
            flex_shrink: 0.0,
            ..default()
        },
    ));
}

#[expect(
    clippy::too_many_arguments,
    reason = "Pause readouts combine build, seed, stage, progression, and combat telemetry."
)]
fn update_pause_readouts(
    build: crate::ui::BuildView,
    stats: Res<GameStats>,
    content: Res<crate::content::GameContent>,
    active: Res<crate::content::ActiveStage>,
    config: Res<crate::setup::ActiveRunConfig>,
    xp: Query<&crate::experience::Experience, With<Player>>,
    telemetry: Option<Res<crate::telemetry::RunTelemetry>>,
    mut labels: Query<(&PauseReadout, &mut Text)>,
) {
    if labels.is_empty() {
        return;
    }
    let snapshot = build.snapshot();
    let seconds = stats.time_elapsed as u32;
    for (kind, mut text) in &mut labels {
        let value = match kind {
            PauseReadout::Run => format!("{}  |  {:02}:{:02}  |  Level {}  |  Kills {}  |  Gold {}\nSeed {}  |  Damage taken {}", content.stage(&active).name, seconds / 60, seconds % 60,
                xp.single().map_or(1, |xp| xp.level), stats.enemies_killed, stats.gold, config.0.seed,
                telemetry.as_ref().map_or(0, |data| data.damage_taken)),
            PauseReadout::Stats => snapshot.as_ref().map_or_else(|| "No active player".into(), |build| build.stats.clone()),
            PauseReadout::Weapons | PauseReadout::Passives => {
                let count = if matches!(kind, PauseReadout::Weapons) { crate::inventory::MAX_WEAPONS } else { crate::inventory::MAX_PASSIVES };
                let entries = snapshot.as_ref().map(|build| if matches!(kind, PauseReadout::Weapons) { &build.weapons } else { &build.passives });
                (0..count).map(|index| {
                    entries.and_then(|items| items.get(index)).map_or_else(|| format!("{}. Empty", index + 1), |entry| {
                        if let Some(weapon) = entry.weapon {
                            let damage = telemetry.as_ref().and_then(|data| data.damage_by_weapon.get(&weapon)).copied().unwrap_or(0);
                            format!("{}. {}\n   Lv. {} | {}\n   Damage {damage}", index + 1, entry.name, entry.level, weapon.family())
                        } else { format!("{}. {}\n   Lv. {}", index + 1, entry.name, entry.level) }
                    })
                }).collect::<Vec<_>>().join("\n\n")
            }
        };
        if text.0 != value {
            text.0 = value;
        }
    }
}

// Specialized button for pattern selection - wider with title + description layout
pub fn spawn_pattern_button(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    description: &str,
    action: MenuAction,
    selected: bool,
) {
    parent
        .spawn((
            button(),
            Node {
                width: Val::Px(500.0),
                padding: UiRect::axes(Val::Px(20.0), Val::Px(12.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
            MenuItem { selected },
            MenuActionComponent { action },
        ))
        .with_children(|parent| {
            // Pattern name (bold/larger)
            parent.spawn((
                Text::new(title),
                TextFont {
                    font_size: FontSize::Px(22.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            // Description (smaller, dimmer)
            parent.spawn((
                Text::new(description),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ));
        });
}

pub fn spawn_menu_container(
    parent: &mut ChildSpawnerCommands,
    spawn_content: impl FnOnce(&mut ChildSpawnerCommands),
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
            BorderColor::all(Color::srgb(0.7, 0.7, 0.7)),
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
        ))
        .with_children(|parent| {
            spawn_content(parent);
        });
}

pub fn cleanup_menu_state(mut commands: Commands, query: Query<Entity, With<MenuRoot>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<PendingAddCircleUpgrade>();
}

pub fn update_menu_buttons(
    mut buttons: Query<(&MenuItem, &mut BackgroundColor, &Children, ButtonStateQuery)>,
    mut text_query: Query<&mut TextColor>,
) {
    for (menu_item, mut background_color, children, (pressed, hovered)) in buttons.iter_mut() {
        let interaction = ButtonInteraction::new(pressed, hovered);
        // Enhanced visual feedback
        let bg_color = match (interaction, menu_item.selected) {
            (ButtonInteraction::Pressed, _) => Color::srgb(0.2, 0.2, 0.2),
            (ButtonInteraction::Hovered, _) => Color::srgb(0.4, 0.4, 0.4),
            (ButtonInteraction::None, true) => Color::srgb(0.35, 0.35, 0.4),
            (ButtonInteraction::None, false) => Color::srgb(0.3, 0.3, 0.3),
        };
        background_color.0 = bg_color;

        // Update text color for all text children
        let is_highlighted = menu_item.selected || interaction == ButtonInteraction::Hovered;
        for (i, child) in children.iter().enumerate() {
            if let Ok(mut text_color) = text_query.get_mut(child) {
                text_color.0 = if is_highlighted {
                    if i == 0 {
                        Color::srgb(1.0, 0.84, 0.0) // Title: gold
                    } else {
                        Color::srgb(0.9, 0.75, 0.3) // Description: muted gold
                    }
                } else if i == 0 {
                    Color::WHITE // Title: white
                } else {
                    Color::srgb(0.7, 0.7, 0.7) // Description: gray
                };
            }
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Menu choices route through the existing weapon, acquisition, passive, and generic message APIs."
)]
fn dispatch_menu_action(
    mut commands: Commands,
    mut activations: MessageReader<MenuActivation>,
    roots: Query<&MenuRoot>,
    pending: Option<Res<PendingAddCircleUpgrade>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut weapon_upgrades: MessageWriter<WeaponUpgradeConfirmedEvent>,
    mut generic_upgrades: MessageWriter<GenericUpgradeConfirmedEvent>,
    mut acquire_weapons: MessageWriter<AddWeaponEvent>,
    mut passive_upgrades: MessageWriter<PassiveUpgradeEvent>,
    players: Query<Entity, With<Player>>,
) {
    for activation in activations.read() {
        let Ok(root) = roots.get(activation.root) else {
            continue;
        };
        match &activation.action {
            MenuAction::RestartRun => next_state.set(GameState::Restarting),
            MenuAction::ResumeGame => next_state.set(GameState::Playing),
            MenuAction::ReturnToTitle => next_state.set(GameState::MainMenu),
            MenuAction::QuitGame => next_state.set(GameState::Quit),
            MenuAction::SelectUpgrade(choice) if root.menu_type == MenuType::LevelUp => {
                commands.entity(activation.root).despawn();
                match &choice.upgrade_type {
                    UpgradeType::Weapon(weapon_type, spec) => {
                        if spec.changes.iter().any(|change| {
                            matches!(change, WeaponUpgradeChange::AddCircle { pattern: None })
                        }) {
                            commands.insert_resource(PendingAddCircleUpgrade {
                                weapon_type: *weapon_type,
                                upgrade_spec: spec.clone(),
                            });
                            spawn_pattern_select_menu(&mut commands);
                        } else {
                            weapon_upgrades.write(WeaponUpgradeConfirmedEvent {
                                weapon_type: *weapon_type,
                                upgrade_spec: spec.clone(),
                            });
                            next_state.set(GameState::Playing);
                        }
                    }
                    UpgradeType::Generic(generic) => {
                        generic_upgrades.write(GenericUpgradeConfirmedEvent {
                            generic_upgrade_type: *generic,
                        });
                        next_state.set(GameState::Playing);
                    }
                    UpgradeType::AcquireWeapon(weapon_type) => {
                        if let Ok(player) = players.single() {
                            acquire_weapons.write(AddWeaponEvent {
                                player,
                                weapon_type: *weapon_type,
                            });
                        }
                        next_state.set(GameState::Playing);
                    }
                    UpgradeType::Passive(kind, level) => {
                        if let Ok(player) = players.single() {
                            passive_upgrades.write(PassiveUpgradeEvent {
                                player,
                                kind: *kind,
                                level: *level,
                            });
                        }
                        next_state.set(GameState::Playing);
                    }
                }
            }
            MenuAction::SelectPattern(pattern) if root.menu_type == MenuType::PatternSelect => {
                let Some(pending) = pending.as_ref() else {
                    continue;
                };
                let mut spec = pending.upgrade_spec.clone();
                if let Some(WeaponUpgradeChange::AddCircle { pattern: slot }) =
                    spec.changes.iter_mut().find(|change| {
                        matches!(change, WeaponUpgradeChange::AddCircle { pattern: None })
                    })
                {
                    *slot = Some(*pattern);
                }
                weapon_upgrades.write(WeaponUpgradeConfirmedEvent {
                    weapon_type: pending.weapon_type,
                    upgrade_spec: spec,
                });
                commands.remove_resource::<PendingAddCircleUpgrade>();
                commands.entity(activation.root).despawn();
                next_state.set(GameState::Playing);
            }
            _ => {}
        }
    }
}

#[derive(Resource, Debug)]
pub struct PendingAddCircleUpgrade {
    pub weapon_type: WeaponType,
    pub upgrade_spec: WeaponUpgradeSpec,
}

fn spawn_game_over_menu(
    mut commands: Commands,
    stats: Res<GameStats>,
    build: Res<crate::ui::FinalBuild>,
    telemetry: Res<crate::telemetry::RunTelemetry>,
    report: Res<crate::expedition::LastExpedition>,
) {
    spawn_run_end_menu(
        &mut commands,
        MenuType::GameOver,
        "Run ended",
        "Try another build and survive the onslaught.",
        &stats,
        &build,
        &telemetry,
        report.0.as_ref(),
    );
}

fn spawn_victory_menu(
    mut commands: Commands,
    stats: Res<GameStats>,
    build: Res<crate::ui::FinalBuild>,
    telemetry: Res<crate::telemetry::RunTelemetry>,
    report: Res<crate::expedition::LastExpedition>,
) {
    spawn_run_end_menu(
        &mut commands,
        MenuType::Victory,
        "Victory!",
        "You survived the expedition.",
        &stats,
        &build,
        &telemetry,
        report.0.as_ref(),
    );
}

/// The haul line on a result screen, e.g. `Haul: Gold 12, Essence 8 (banked)`.
pub(crate) fn haul_summary(report: &crate::expedition::ExpeditionReport) -> String {
    let destination = if report.banked {
        "banked to the estate"
    } else {
        "sandbox run, not banked"
    };
    format!("Haul: {} ({destination})", report.haul)
}

#[expect(
    clippy::too_many_arguments,
    reason = "Result screens combine run totals, the final build, telemetry, and the expedition haul."
)]
fn spawn_run_end_menu(
    commands: &mut Commands,
    menu_type: MenuType,
    title: &str,
    description: &str,
    stats: &GameStats,
    build: &crate::ui::FinalBuild,
    telemetry: &crate::telemetry::RunTelemetry,
    report: Option<&crate::expedition::ExpeditionReport>,
) {
    let seconds = stats.time_elapsed as u32;
    let mut summary = format!(
        "Time: {:02}:{:02}   |   Enemies defeated: {}   |   Gold: {}\nDamage taken: {}",
        seconds / 60,
        seconds % 60,
        stats.enemies_killed,
        stats.gold,
        telemetry.damage_taken,
    );
    if let Some(report) = report {
        summary.push('\n');
        summary.push_str(&haul_summary(report));
    }
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(100),
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
            MenuRoot { menu_type },
            menu_type,
        ))
        .with_children(|parent| {
            spawn_menu_container(parent, |parent| {
                parent.spawn((
                    Text::new(title),
                    TextFont {
                        font_size: FontSize::Px(48.0),
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 0.8, 0.0)),
                ));
                parent.spawn((
                    Text::new(description),
                    TextFont {
                        font_size: FontSize::Px(18.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
                parent.spawn((
                    RunSummaryReadout,
                    Text::new(summary),
                    TextFont {
                        font_size: FontSize::Px(20.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.8, 0.8)),
                ));
                spawn_final_build(parent, build.0.as_ref(), telemetry);
                parent
                    .spawn(Node {
                        column_gap: Val::Px(12.0),
                        ..default()
                    })
                    .with_children(|parent| {
                        spawn_compact_button(parent, "Play again", MenuAction::RestartRun, true);
                        spawn_compact_button(parent, "Main menu", MenuAction::ReturnToTitle, false);
                        spawn_compact_button(parent, "Quit", MenuAction::QuitGame, false);
                    });
            });
        });
}

#[derive(Component)]
pub(crate) struct FinalBuildReadout;

#[derive(Component)]
pub(crate) struct RunSummaryReadout;

fn spawn_final_build(
    parent: &mut ChildSpawnerCommands,
    snapshot: Option<&crate::ui::BuildSnapshot>,
    telemetry: &crate::telemetry::RunTelemetry,
) {
    parent
        .spawn(Node {
            column_gap: Val::Px(30.0),
            ..default()
        })
        .with_children(|parent| {
            for (heading, entries) in [
                ("FINAL WEAPONS", snapshot.map(|build| &build.weapons)),
                ("FINAL PASSIVES", snapshot.map(|build| &build.passives)),
            ] {
                let details = entries.map_or_else(
                    || "No recorded build".into(),
                    |entries| {
                        if entries.is_empty() {
                            return "None".into();
                        }
                        entries
                            .iter()
                            .map(|entry| {
                                let damage = entry
                                    .weapon
                                    .map(|weapon| {
                                        format!(
                                            "\nDamage {}",
                                            telemetry
                                                .damage_by_weapon
                                                .get(&weapon)
                                                .copied()
                                                .unwrap_or(0)
                                        )
                                    })
                                    .unwrap_or_default();
                                format!("{}  Lv. {}{damage}", entry.name, entry.level)
                            })
                            .collect::<Vec<_>>()
                            .join("\n\n")
                    },
                );
                parent.spawn((
                    Text::new(format!("{heading}\n\n{details}")),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.85, 0.85, 0.88)),
                    Node {
                        width: Val::Px(310.0),
                        ..default()
                    },
                    FinalBuildReadout,
                ));
            }
        });
}
