//! Versioned run recipes and separate seeded streams; physics is not a replay format.
use crate::ui_button::{button, ButtonInteraction, ButtonStateQuery, Pressed};
use crate::{
    components::Player,
    content::{ActiveStage, GameContent},
    inventory::{PassiveType, MAX_PASSIVES, MAX_WEAPONS},
    resources::GameState,
    weapons::{magick_circle::PatternType, weapon_upgrade::WeaponUpgradeConfig, WeaponType},
};
#[cfg(any(test, feature = "dev-tools"))]
use crate::{
    inventory::PassiveInventory,
    weapons::{magick_circle::MagickCircle, WeaponMeta},
};
use bevy::prelude::*;
use rand::{rngs::StdRng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use strum::IntoEnumIterator;

pub const CONFIG_PATH: &str = "run-config.ron";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponLoadout {
    pub weapon: WeaponType,
    pub level: u32,
    #[serde(default)]
    pub patterns: Vec<PatternType>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassiveLoadout {
    pub kind: PassiveType,
    pub level: u8,
}

#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunConfiguration {
    pub version: u32,
    pub stage: String,
    pub seed: u64,
    pub weapons: Vec<WeaponLoadout>,
    #[serde(default)]
    pub passives: Vec<PassiveLoadout>,
}

impl Default for RunConfiguration {
    fn default() -> Self {
        Self {
            version: 1,
            stage: "courtyard".into(),
            seed: 20260909,
            weapons: vec![WeaponLoadout {
                weapon: WeaponType::MagickCircle,
                level: 1,
                patterns: vec![],
            }],
            passives: vec![],
        }
    }
}

#[derive(Resource, Clone, Default)]
pub struct ActiveRunConfig(pub RunConfiguration);

#[derive(Resource)]
pub struct RunRng {
    pub waves: StdRng,
    pub loot: StdRng,
    pub upgrades: StdRng,
}
impl RunRng {
    pub fn new(seed: u64) -> Self {
        Self {
            waves: StdRng::seed_from_u64(seed ^ 0x5741_5645_535f_524e),
            loot: StdRng::seed_from_u64(seed ^ 0x4c4f_4f54_5f52_4e47),
            upgrades: StdRng::seed_from_u64(seed ^ 0x5550_4752_4144_4553),
        }
    }
}
impl Default for RunRng {
    fn default() -> Self {
        Self::new(RunConfiguration::default().seed)
    }
}

pub fn validate_configuration(world: &World, config: &RunConfiguration) -> Result<(), String> {
    if config.version != 1 {
        return Err("Unsupported run configuration version (expected 1).".into());
    }
    if !world
        .resource::<GameContent>()
        .stages
        .iter()
        .any(|s| s.id == config.stage)
    {
        return Err(format!("Unknown stage: {}", config.stage));
    }
    if config.weapons.is_empty()
        || config.weapons.len() > MAX_WEAPONS
        || config.passives.len() > MAX_PASSIVES
    {
        return Err("Choose 1-6 weapons and at most 6 passive items.".into());
    }
    let mut weapons = HashSet::new();
    for item in &config.weapons {
        if !weapons.insert(item.weapon) {
            return Err("Duplicate starting weapon.".into());
        }
        let maximum = world
            .resource::<WeaponUpgradeConfig>()
            .max_level(item.weapon);
        if item.level == 0 || item.level > maximum {
            return Err(format!("Invalid level for {}.", item.weapon));
        }
        if item.weapon != WeaponType::MagickCircle && !item.patterns.is_empty() {
            return Err("Only Magick Circle accepts circle patterns.".into());
        }
        let maximum_patterns = 1 + world.resource::<WeaponUpgradeConfig>().data[&item.weapon]
            .progression
            .iter()
            .flat_map(|step| &step.changes)
            .filter(|change| {
                matches!(
                    change,
                    crate::weapons::weapon_upgrade::WeaponUpgradeChange::AddCircle { .. }
                )
            })
            .count();
        if item.patterns.len() > maximum_patterns {
            return Err(format!("Magick Circle accepts up to {maximum_patterns} explicit patterns, or an empty default list."));
        }
    }
    let mut passives = HashSet::new();
    for item in &config.passives {
        if !passives.insert(item.kind) {
            return Err("Duplicate starting passive.".into());
        }
        let maximum = world
            .resource::<crate::inventory::PassiveCatalog>()
            .get(item.kind)
            .max_level();
        if item.level == 0 || item.level > maximum {
            return Err(format!("Passive levels must be 1-{maximum}."));
        }
    }
    Ok(())
}

pub fn export_configuration(world: &mut World) -> Result<(), String> {
    let config = world.resource::<RunConfiguration>();
    validate_configuration(world, config)?;
    write_configuration(config)
}

pub fn import_configuration(world: &mut World) -> Result<(), String> {
    let text = crate::storage::read_text(std::path::Path::new(CONFIG_PATH))?;
    import_configuration_text(world, &text)
}

/// Validate the complete recipe before replacing any staged or active state.
pub fn import_configuration_text(world: &mut World, text: &str) -> Result<(), String> {
    if text.len() > crate::storage::MAX_FILE_BYTES {
        return Err("Run config is larger than 64 KiB.".into());
    }
    let config: RunConfiguration = ron::from_str(text).map_err(|e| e.to_string())?;
    validate_configuration(world, &config)?;
    *world.resource_mut::<RunConfiguration>() = config;
    Ok(())
}

fn write_configuration(config: &RunConfiguration) -> Result<(), String> {
    write_configuration_to(config, std::path::Path::new(CONFIG_PATH))
}

pub(crate) fn write_configuration_to(
    config: &RunConfiguration,
    path: &std::path::Path,
) -> Result<(), String> {
    crate::storage::write_ron(config, path)
}

pub fn start_configured_run(world: &mut World) -> Result<(), String> {
    let config = world.resource::<RunConfiguration>().clone();
    validate_configuration(world, &config)?;
    let index = world
        .resource::<GameContent>()
        .stages
        .iter()
        .position(|s| s.id == config.stage)
        .unwrap();
    world.resource_mut::<ActiveStage>().0 = index;
    world.resource_mut::<ActiveRunConfig>().0 = config;
    world
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Restarting);
    Ok(())
}

#[cfg(feature = "dev-tools")]
pub fn export_current_build(world: &mut World) -> Result<(), String> {
    let config = current_build_configuration(world)?;
    write_configuration(&config)?;
    *world.resource_mut::<RunConfiguration>() = config;
    Ok(())
}

#[cfg(any(test, feature = "dev-tools"))]
pub fn current_build_configuration(world: &mut World) -> Result<RunConfiguration, String> {
    let player = world
        .query_filtered::<Entity, With<Player>>()
        .single(world)
        .map_err(|_| "No active player.")?;
    let mut config = world.resource::<ActiveRunConfig>().0.clone();
    config.weapons = world
        .query::<(&WeaponMeta, Option<&MagickCircle>, &ChildOf)>()
        .iter(world)
        .filter(|(_, _, parent)| parent.parent() == player)
        .map(|(meta, circle, _)| WeaponLoadout {
            weapon: meta.weapon_type,
            level: meta.level,
            patterns: circle.map(|c| c.patterns.clone()).unwrap_or_default(),
        })
        .collect();
    config.weapons.sort_by_key(|w| w.weapon.id());
    config.passives = world
        .get::<PassiveInventory>(player)
        .map(|inventory| {
            inventory
                .items
                .iter()
                .map(|item| PassiveLoadout {
                    kind: item.kind,
                    level: item.level,
                })
                .collect()
        })
        .unwrap_or_default();
    config.passives.sort_by_key(|item| item.kind.id());
    validate_configuration(world, &config)?;
    Ok(config)
}

pub fn apply_starting_loadout(world: &mut World) {
    let config = world.resource::<ActiveRunConfig>().0.clone();
    let Ok(player) = world.query_filtered::<Entity, With<Player>>().single(world) else {
        return;
    };
    world
        .entity_mut(player)
        .remove::<crate::weapons::StartingWeapon>();
    for item in &config.weapons {
        crate::weapons::grant_configured_weapon(
            world,
            player,
            item.weapon,
            item.level,
            &item.patterns,
        )
        .expect("validated starting weapon");
    }
    for item in &config.passives {
        for _ in 0..item.level {
            crate::inventory::grant_passive(world, player, item.kind)
                .expect("validated starting passive");
        }
    }
}

#[derive(Resource, Default)]
struct SetupStatus {
    message: String,
    seed_focused: bool,
    seed_text: String,
    seed_selected: bool,
    selected: Option<SetupAction>,
}
#[derive(Component)]
struct SetupRoot;
#[derive(Component)]
struct SetupSummary;
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum SetupAction {
    Stage(usize),
    Weapon(WeaponType),
    Seed,
    RandomSeed,
    Import,
    Export,
    Start,
    Quit,
}

impl SetupAction {
    fn order(self, stage_count: usize) -> usize {
        let weapon_count = WeaponType::iter().count();
        match self {
            Self::Stage(index) => index,
            Self::Weapon(kind) => {
                stage_count
                    + WeaponType::iter()
                        .position(|weapon| weapon == kind)
                        .unwrap_or(0)
            }
            Self::Seed => stage_count + weapon_count,
            Self::RandomSeed => stage_count + weapon_count + 1,
            Self::Import => stage_count + weapon_count + 2,
            Self::Export => stage_count + weapon_count + 3,
            Self::Start => stage_count + weapon_count + 4,
            Self::Quit => stage_count + weapon_count + 5,
        }
    }
}

pub struct SetupPlugin;
impl Plugin for SetupPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RunConfiguration>()
            .init_resource::<ActiveRunConfig>()
            .init_resource::<RunRng>()
            .init_resource::<SetupStatus>()
            .add_systems(OnEnter(GameState::MainMenu), spawn_setup)
            .add_systems(OnExit(GameState::MainMenu), cleanup_setup)
            .add_systems(
                Update,
                (setup_input, update_setup_summary, update_setup_buttons)
                    .chain()
                    .run_if(in_state(GameState::MainMenu)),
            );
    }
}

fn spawn_setup(
    mut commands: Commands,
    content: Res<GameContent>,
    config: Res<RunConfiguration>,
    mut status: ResMut<SetupStatus>,
) {
    status.seed_text = config.seed.to_string();
    status.seed_focused = false;
    status.seed_selected = false;
    status.selected = Some(SetupAction::Start);
    commands.spawn((SetupRoot, Node {
        width: Val::Percent(100.0), height: Val::Percent(100.0), justify_content: JustifyContent::Center,
        align_items: AlignItems::Center, ..default()
    }, BackgroundColor(Color::srgba(0.025,0.03,0.04,0.96))))
    .with_children(|root| {
        root.spawn((Node { width:Val::Px(880.0), padding:UiRect::all(Val::Px(24.0)), flex_direction:FlexDirection::Column,
            row_gap:Val::Px(12.0), ..default() }, BackgroundColor(Color::srgb(0.07,0.085,0.10))))
        .with_children(|panel| {
            panel.spawn((Text::new("OCCULT EXPEDITIONS"), TextFont {font_size:FontSize::Px(34.0),..default()},TextColor(Color::srgb(0.85,0.74,0.45))));
            panel.spawn((Text::new("Different traditions. One reality. Choose an expedition."), TextFont {font_size:FontSize::Px(18.0),..default()}));
            panel.spawn((SetupSummary, Text::new(""), TextFont {font_size:FontSize::Px(18.0),..default()},Node{min_height:Val::Px(125.0),..default()}));
            panel.spawn(Node{flex_direction:FlexDirection::Row,column_gap:Val::Px(8.0),..default()}).with_children(|row| {
                for (index, stage) in content.stages.iter().enumerate() { setup_button(row,&stage.name,SetupAction::Stage(index)); }
            });
            panel.spawn(Node{flex_direction:FlexDirection::Row,flex_wrap:FlexWrap::Wrap,column_gap:Val::Px(8.0),row_gap:Val::Px(8.0),..default()}).with_children(|row| {
                for weapon in WeaponType::iter() {
                    setup_button(row,&weapon.to_string(),SetupAction::Weapon(weapon));
                }
            });
            panel.spawn(Node{flex_direction:FlexDirection::Row,column_gap:Val::Px(8.0),..default()}).with_children(|row| {
                setup_button(row,"Edit seed",SetupAction::Seed); setup_button(row,"Random seed",SetupAction::RandomSeed);
                setup_button(row,"Import recipe",SetupAction::Import); setup_button(row,"Export recipe",SetupAction::Export);
            });
            let instructions = "Recipes use run-config.ron in the launch folder. Import validates before changing setup.\nSelecting a starting weapon resets the recipe to a fresh level-one build.\nArrow keys / D-pad / Tab: select. Enter / A: confirm.";
            #[cfg(feature = "dev-tools")]
            let instructions = format!("{instructions}\nF1: context-aware debug tools.");
            panel.spawn((Text::new(instructions),TextFont{font_size:FontSize::Px(15.0),..default()}));
            panel.spawn(Node{flex_direction:FlexDirection::Row,column_gap:Val::Px(12.0),..default()}).with_children(|row| {
                setup_button(row,"Begin expedition  [Enter]",SetupAction::Start); setup_button(row,"Quit",SetupAction::Quit);
            });
        });
    });
}

fn setup_button(parent: &mut ChildSpawnerCommands, label: &str, action: SetupAction) {
    parent
        .spawn((
            button(),
            action,
            Node {
                padding: UiRect::axes(Val::Px(12.0), Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.17, 0.21, 0.24)),
        ))
        .with_child((
            Text::new(label),
            TextFont {
                font_size: FontSize::Px(17.0),
                ..default()
            },
        ));
}

fn setup_input(world: &mut World) {
    #[cfg(feature = "dev-tools")]
    let (keyboard_blocked, pointer_blocked) = {
        let debug = world.resource::<crate::dev_tools::DebugSession>();
        (debug.blocks_keyboard(), debug.blocks_pointer())
    };
    #[cfg(not(feature = "dev-tools"))]
    let (keyboard_blocked, pointer_blocked) = (false, false);
    let clicked = if pointer_blocked {
        None
    } else {
        world
            .query_filtered::<&SetupAction, Added<Pressed>>()
            .iter(world)
            .next()
            .copied()
    };
    let keys = world.resource::<ButtonInput<KeyCode>>().clone();
    let (pad_next, pad_previous, pad_activate) = {
        let mut next = false;
        let mut previous = false;
        let mut activate = false;
        for pad in world.query::<&Gamepad>().iter(world) {
            next |= pad.just_pressed(GamepadButton::DPadDown)
                || pad.just_pressed(GamepadButton::DPadRight);
            previous |= pad.just_pressed(GamepadButton::DPadUp)
                || pad.just_pressed(GamepadButton::DPadLeft);
            activate |= pad.just_pressed(GamepadButton::South);
        }
        (next, previous, activate)
    };
    let editing = world.resource::<SetupStatus>().seed_focused;
    if editing && !keyboard_blocked {
        edit_seed(world, &keys);
        if clicked.is_none() && (keys.just_pressed(KeyCode::Enter) || pad_activate) {
            let result = commit_seed(world);
            world.resource_mut::<SetupStatus>().message =
                result.map_or_else(|error| error, |_| "Seed updated.".into());
            return;
        }
    }
    if !editing && !keyboard_blocked {
        let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        let forward = keys.just_pressed(KeyCode::ArrowDown)
            || keys.just_pressed(KeyCode::ArrowRight)
            || (keys.just_pressed(KeyCode::Tab) && !shift)
            || pad_next;
        let backward = keys.just_pressed(KeyCode::ArrowUp)
            || keys.just_pressed(KeyCode::ArrowLeft)
            || (keys.just_pressed(KeyCode::Tab) && shift)
            || pad_previous;
        if forward || backward {
            let stage_count = world.resource::<GameContent>().stages.len();
            let mut actions: Vec<_> = world.query::<&SetupAction>().iter(world).copied().collect();
            actions.sort_by_key(|action| action.order(stage_count));
            if !actions.is_empty() {
                let selected = world
                    .resource::<SetupStatus>()
                    .selected
                    .unwrap_or(SetupAction::Start);
                let index = actions
                    .iter()
                    .position(|action| *action == selected)
                    .unwrap_or(0);
                let next = if forward {
                    (index + 1) % actions.len()
                } else {
                    (index + actions.len() - 1) % actions.len()
                };
                world.resource_mut::<SetupStatus>().selected = Some(actions[next]);
            }
        }
    }
    let action = clicked.or_else(|| {
        if !editing && !keyboard_blocked && (keys.just_pressed(KeyCode::Enter) || pad_activate) {
            world.resource::<SetupStatus>().selected
        } else {
            None
        }
    });
    let Some(action) = action else {
        return;
    };
    if editing && matches!(action, SetupAction::Start | SetupAction::Export) {
        if let Err(error) = commit_seed(world) {
            world.resource_mut::<SetupStatus>().message = error;
            return;
        }
    }
    world.resource_mut::<SetupStatus>().selected = Some(action);
    let result: Result<(), String> = match action {
        SetupAction::Stage(index) => {
            let Some(stage) = world.resource::<GameContent>().stages.get(index) else {
                return;
            };
            let id = stage.id.clone();
            world.resource_mut::<RunConfiguration>().stage = id;
            Ok(())
        }
        SetupAction::Weapon(weapon) => {
            let mut config = world.resource_mut::<RunConfiguration>();
            config.weapons = vec![WeaponLoadout {
                weapon,
                level: 1,
                patterns: vec![],
            }];
            config.passives.clear();
            Ok(())
        }
        SetupAction::Seed => {
            let text = world.resource::<RunConfiguration>().seed.to_string();
            let mut status = world.resource_mut::<SetupStatus>();
            status.seed_text = text;
            status.seed_focused = true;
            status.seed_selected = true;
            Ok(())
        }
        SetupAction::RandomSeed => {
            world.resource_mut::<RunConfiguration>().seed = rand::random();
            world.resource_mut::<SetupStatus>().seed_focused = false;
            Ok(())
        }
        SetupAction::Import => {
            let result = import_configuration(world);
            if result.is_ok() {
                world.resource_mut::<SetupStatus>().seed_focused = false;
            }
            result
        }
        SetupAction::Export => export_configuration(world),
        SetupAction::Start => start_configured_run(world),
        SetupAction::Quit => {
            world
                .resource_mut::<NextState<GameState>>()
                .set(GameState::Quit);
            Ok(())
        }
    };
    world.resource_mut::<SetupStatus>().message = match result {
        Ok(()) => match action {
            SetupAction::Import => format!("Imported {CONFIG_PATH}."),
            SetupAction::Export => format!("Exported {CONFIG_PATH}."),
            _ => String::new(),
        },
        Err(error) => error,
    };
}

fn edit_seed(world: &mut World, keys: &ButtonInput<KeyCode>) {
    let digits = [
        (KeyCode::Digit0, KeyCode::Numpad0),
        (KeyCode::Digit1, KeyCode::Numpad1),
        (KeyCode::Digit2, KeyCode::Numpad2),
        (KeyCode::Digit3, KeyCode::Numpad3),
        (KeyCode::Digit4, KeyCode::Numpad4),
        (KeyCode::Digit5, KeyCode::Numpad5),
        (KeyCode::Digit6, KeyCode::Numpad6),
        (KeyCode::Digit7, KeyCode::Numpad7),
        (KeyCode::Digit8, KeyCode::Numpad8),
        (KeyCode::Digit9, KeyCode::Numpad9),
    ];
    let mut status = world.resource_mut::<SetupStatus>();
    if (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && keys.just_pressed(KeyCode::KeyA)
    {
        status.seed_selected = true;
    }
    if keys.just_pressed(KeyCode::Backspace)
        || (keys.just_pressed(KeyCode::Delete) && status.seed_selected)
    {
        if status.seed_selected {
            status.seed_text.clear();
            status.seed_selected = false;
        } else {
            status.seed_text.pop();
        }
    }
    for (index, (digit, numpad)) in digits.iter().enumerate() {
        if keys.just_pressed(*digit) || keys.just_pressed(*numpad) {
            if status.seed_selected {
                status.seed_text.clear();
                status.seed_selected = false;
            }
            if status.seed_text.len() < 20 {
                status.seed_text.push(char::from(b'0' + index as u8));
            }
        }
    }
}

fn commit_seed(world: &mut World) -> Result<(), String> {
    let seed = world
        .resource::<SetupStatus>()
        .seed_text
        .parse::<u64>()
        .map_err(|_| "Enter a valid unsigned 64-bit seed.".to_owned())?;
    world.resource_mut::<RunConfiguration>().seed = seed;
    let mut status = world.resource_mut::<SetupStatus>();
    status.seed_focused = false;
    status.seed_selected = false;
    Ok(())
}

fn update_setup_summary(
    config: Res<RunConfiguration>,
    content: Res<GameContent>,
    status: Res<SetupStatus>,
    mut text: Query<&mut Text, With<SetupSummary>>,
) {
    let stage = content.stages.iter().find(|s| s.id == config.stage);
    let stage_name = stage.map_or("Unknown stage", |s| s.name.as_str());
    let objective = stage.map_or_else(
        || "Unknown objective".into(),
        |stage| {
            let minutes = stage.duration as u32 / 60;
            let seconds = stage.duration as u32 % 60;
            match &stage.win_condition {
                crate::content::StageWinCondition::TimedSurvival => {
                    format!("Survive {minutes}:{seconds:02}")
                }
                crate::content::StageWinCondition::DefeatBoss { enemy } => format!(
                    "At {minutes}:{seconds:02}, defeat {} to win",
                    content
                        .enemy(enemy)
                        .map_or(enemy.as_str(), |enemy| enemy.name.as_str())
                ),
            }
        },
    );
    let weapons = config
        .weapons
        .iter()
        .map(|w| format!("{} Lv{}", w.weapon, w.level))
        .collect::<Vec<_>>()
        .join(", ");
    let seed = if status.seed_focused {
        format!(
            "{} {}  (digits / Ctrl+A / Backspace / Enter to apply)",
            status.seed_text,
            if status.seed_selected {
                "[all selected]"
            } else {
                "_"
            }
        )
    } else {
        config.seed.to_string()
    };
    let weapon_hint = if config.weapons.len() == 1 {
        let kind = config.weapons[0].weapon;
        format!("{}: {}", kind.family(), kind.summary())
    } else {
        "Six weapon slots per run. Inspect individual weapons in the pause or debug menu.".into()
    };
    for mut text in &mut text {
        text.0 = format!(
            "Stage: {stage_name}\nObjective: {objective}\nSeed: {seed}\nStarting build: {weapons} | {} passive items\n{weapon_hint}\n{}",
            config.passives.len(),
            status.message
        );
    }
}

fn update_setup_buttons(
    status: Res<SetupStatus>,
    mut buttons: Query<(&SetupAction, ButtonStateQuery, &mut BackgroundColor)>,
) {
    for (action, (pressed, hovered), mut color) in &mut buttons {
        let interaction = ButtonInteraction::new(pressed, hovered);
        color.0 = if interaction == ButtonInteraction::Pressed {
            Color::srgb(0.42, 0.38, 0.22)
        } else if status.selected == Some(*action) {
            Color::srgb(0.33, 0.31, 0.2)
        } else if interaction == ButtonInteraction::Hovered {
            Color::srgb(0.25, 0.3, 0.34)
        } else {
            Color::srgb(0.17, 0.21, 0.24)
        };
    }
}

fn cleanup_setup(mut commands: Commands, roots: Query<Entity, With<SetupRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
}
