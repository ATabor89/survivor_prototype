//! Optional playtest tools. The action layer is independent of egui for headless tests.
use bevy::ecs::system::{RunSystemOnce, SystemParam};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};
use bevy_rapier2d::prelude::ReadRapierContext;
use strum::IntoEnumIterator;

use crate::components::{Enemy, Health, Player};
use crate::content::{ActiveStage, GameContent};
use crate::experience::{calculate_experience_needed, Experience, ExperienceOrb};
use crate::inventory::{PassiveCatalog, PassiveInventory, PassiveType};
use crate::loot::LootKind;
use crate::resources::{GameState, GameStats, GameTextures, SpawnTimer};
use crate::weapons::magick_circle::{MagickCircle, PatternType};
use crate::weapons::overlap::{ActiveOverlaps, EnemyCircleHits, OverlappingPatterns};
use crate::weapons::weapon_upgrade::{WeaponUpgradeChange, WeaponUpgradeConfig};
use crate::weapons::{Attack, BindingEffect, InvocationPull, ProtectionBuff, WeaponCooldown};
use crate::weapons::{WeaponMeta, WeaponType};
use crate::GameplaySets;

#[derive(Resource)]
pub struct DebugSession {
    pub open: bool,
    pub pause_simulation: bool,
    pub tab: DebugTab,
    /// Updated by egui after drawing; gameplay input respects widget focus.
    pub wants_keyboard_input: bool,
    pub wants_pointer_input: bool,
    /// The overlay consumed this frame's Escape/Start, even if it just closed.
    pub consumed_pause_input: bool,
    pub queued_encounters: Vec<usize>,
    pub waves: bool,
    pub invulnerable: bool,
    pub patterns: Vec<PatternType>,
    pub actions: Vec<DebugAction>,
    pub status: String,
}

impl Default for DebugSession {
    fn default() -> Self {
        Self {
            open: false,
            pause_simulation: true,
            tab: DebugTab::Run,
            wants_keyboard_input: false,
            wants_pointer_input: false,
            consumed_pause_input: false,
            queued_encounters: Vec::new(),
            waves: true,
            invulnerable: false,
            patterns: vec![PatternType::Banishment],
            actions: Vec::new(),
            status: "Choose Run, Weapons, or World to test the current run.".into(),
        }
    }
}

impl DebugSession {
    pub fn blocks_keyboard(&self) -> bool {
        (self.open && self.wants_keyboard_input) || self.consumed_pause_input
    }

    pub fn blocks_pointer(&self) -> bool {
        self.open && self.wants_pointer_input
    }
}

/// Reset run-specific debug data without discarding the user's tool workspace.
pub fn reset_debug_run(world: &mut World) {
    let mut session = world.resource_mut::<DebugSession>();
    session.waves = true;
    session.invulnerable = false;
    session.queued_encounters.clear();
    session.actions.clear();
    session.wants_keyboard_input = false;
    session.wants_pointer_input = false;
    session.status = "Fresh run ready; debug layout and pause preference retained.".into();
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DebugTab {
    #[default]
    Run,
    Weapons,
    World,
}

#[derive(Clone)]
pub enum DebugAction {
    ApplyBuild(Vec<PatternType>),
    ClearArena,
    SpawnTargets { moving: bool },
    Heal,
    GrantLevel,
    AdvanceWave,
    AdvancePhase,
    TriggerEncounter(usize),
    TestVictory,
    TriggerFinale,
    SpawnEnemy(String),
    SelectStage(usize),
    SpawnPickup(LootKind),
    SpawnBreakable,
    GrantWeapon(WeaponType),
    GrantPassive(PassiveType),
    ExportBuild,
    ExportConfiguration,
    ImportConfiguration,
    StartConfiguredRun,
    Resume,
}

pub struct DebugActionsPlugin;
impl Plugin for DebugActionsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugSession>().add_systems(
            Update,
            (toggle_panel, apply_actions)
                .chain()
                .before(GameplaySets::Input),
        );
    }
}

pub struct DebugUiPlugin;

#[derive(SystemParam)]
struct DebugPreferences<'w> {
    config: Res<'w, crate::setup::ActiveRunConfig>,
    audio: ResMut<'w, crate::game_audio::AudioSettings>,
}

impl Plugin for DebugUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(EguiPrimaryContextPass, panel).add_systems(
            PreUpdate,
            refresh_input_capture.after(bevy_egui::EguiPreUpdateSet::BeginPass),
        );
    }
}

fn refresh_input_capture(mut contexts: EguiContexts, mut session: ResMut<DebugSession>) {
    if !session.open {
        session.wants_keyboard_input = false;
        session.wants_pointer_input = false;
        return;
    }
    if let Ok(ctx) = contexts.ctx_mut() {
        session.wants_keyboard_input = ctx.egui_wants_keyboard_input();
        session.wants_pointer_input = ctx.egui_wants_pointer_input();
    }
}

fn toggle_panel(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    mut session: ResMut<DebugSession>,
    weapons: Query<&MagickCircle>,
) {
    session.consumed_pause_input = false;
    let close = session.open
        && (keys.just_pressed(KeyCode::F1)
            || keys.just_pressed(KeyCode::Escape)
            || gamepads
                .iter()
                .any(|pad| pad.just_pressed(GamepadButton::Start)));
    if close {
        session.open = false;
        session.wants_keyboard_input = false;
        session.wants_pointer_input = false;
        session.consumed_pause_input = true;
        if *state.get() == GameState::DebugTools {
            next.set(GameState::Playing);
        }
        return;
    }
    if keys.just_pressed(KeyCode::F1) || keys.just_pressed(KeyCode::F2) {
        if !session.open {
            session.open = true;
            if let Ok(circle) = weapons.single() {
                session.patterns = circle.patterns.clone();
            }
        }
        if keys.just_pressed(KeyCode::F2) {
            session.tab = DebugTab::Weapons;
        }
    }
    if session.open {
        match (state.get(), session.pause_simulation) {
            (GameState::Playing, true) => next.set(GameState::DebugTools),
            (GameState::DebugTools, false) => next.set(GameState::Playing),
            _ => {}
        }
    }
}

fn clear_effects(world: &mut World) {
    let attacks: Vec<_> = world
        .query_filtered::<Entity, With<Attack>>()
        .iter(world)
        .collect();
    for entity in attacks {
        world.despawn(entity);
    }
    let affected: Vec<_> = world
        .query_filtered::<Entity, Or<(
            With<BindingEffect>,
            With<InvocationPull>,
            With<ProtectionBuff>,
            With<OverlappingPatterns>,
        )>>()
        .iter(world)
        .collect();
    for entity in affected {
        world.entity_mut(entity).remove::<(
            BindingEffect,
            InvocationPull,
            ProtectionBuff,
            OverlappingPatterns,
        )>();
    }
    world.resource_mut::<EnemyCircleHits>().hits.clear();
    world.resource_mut::<ActiveOverlaps>().overlaps.clear();
}

pub fn apply_actions(world: &mut World) {
    flush_encounter_requests(world);
    let actions = std::mem::take(&mut world.resource_mut::<DebugSession>().actions);
    if !world.resource::<DebugSession>().open {
        return;
    }
    for action in actions {
        let state = world.resource::<State<GameState>>().get();
        let has_run = matches!(
            state,
            GameState::Playing
                | GameState::DebugTools
                | GameState::Paused
                | GameState::LevelUp
                | GameState::Treasure
        );
        let available = match &action {
            DebugAction::SelectStage(_)
            | DebugAction::Resume
            | DebugAction::ExportBuild
            | DebugAction::ExportConfiguration
            | DebugAction::ImportConfiguration
            | DebugAction::StartConfiguredRun => true,
            DebugAction::TestVictory => matches!(state, GameState::Playing | GameState::DebugTools),
            _ => has_run,
        };
        if !available {
            continue;
        }
        match action {
            DebugAction::ApplyBuild(patterns) => {
                if patterns.is_empty() || patterns.len() > 7 {
                    continue;
                }
                if world.query::<&MagickCircle>().iter(world).next().is_none() {
                    world.resource_mut::<DebugSession>().status =
                        "This build does not own Magick Circle.".into();
                    continue;
                }
                for mut circle in world.query::<&mut MagickCircle>().iter_mut(world) {
                    circle.patterns = patterns.clone();
                }
                clear_effects(world);
                for mut cooldown in world.query::<&mut WeaponCooldown>().iter_mut(world) {
                    let duration = cooldown.timer.duration();
                    cooldown.timer.set_elapsed(duration);
                }
                world.resource_mut::<DebugSession>().status =
                    "Build applied. Fresh circles spawn when the simulation runs.".into();
            }
            DebugAction::ClearArena => {
                clear_effects(world);
                let entities: Vec<_> = world
                    .query_filtered::<Entity, (
                        Or<(
                            With<Enemy>,
                            With<ExperienceOrb>,
                            With<crate::enemy_behavior::HostileProjectile>,
                        )>,
                        Without<crate::waves::FinalBoss>,
                    )>()
                    .iter(world)
                    .collect();
                for entity in entities {
                    world.despawn(entity);
                }
                world.resource_mut::<SpawnTimer>().0.reset();
                world.resource_mut::<DebugSession>().status =
                    "Arena cleared; build, progression, and required final boss retained.".into();
            }
            DebugAction::SpawnTargets { moving } => {
                let Ok(player) = world
                    .query_filtered::<&Transform, With<Player>>()
                    .single(world)
                else {
                    continue;
                };
                let center = player.translation;
                let textures = world.resource::<GameTextures>();
                let image = textures.enemies.clone();
                let layout = textures.enemies_layout.clone();
                for i in 0..8 {
                    let angle = i as f32 * std::f32::consts::TAU / 8.0;
                    // Stationary targets straddle the central/outer overlap regions.
                    let distance = if moving { 200.0 } else { 48.0 };
                    world.spawn((
                        Name::new(if moving {
                            "Test pursuer"
                        } else {
                            "Test target"
                        }),
                        Enemy {
                            speed: if moving { 70.0 } else { 0.0 },
                            experience_value: 0,
                        },
                        Health {
                            current: 1000,
                            maximum: 1000,
                        },
                        Sprite {
                            image: image.clone(),
                            custom_size: Some(Vec2::splat(48.0)),
                            texture_atlas: Some(TextureAtlas {
                                layout: layout.clone(),
                                index: 1,
                            }),
                            ..default()
                        },
                        Transform::from_translation(
                            center + Vec3::new(angle.cos(), angle.sin(), 0.0) * distance,
                        ),
                    ));
                }
                world.resource_mut::<DebugSession>().status =
                    "Spawned eight targets (1,000 HP, no XP reward).".into();
            }
            DebugAction::Heal => {
                for mut health in world
                    .query_filtered::<&mut Health, With<Player>>()
                    .iter_mut(world)
                {
                    health.current = health.maximum;
                }
            }
            DebugAction::GrantLevel => {
                for mut xp in world
                    .query_filtered::<&mut Experience, With<Player>>()
                    .iter_mut(world)
                {
                    xp.current = xp.current.max(calculate_experience_needed(xp.level));
                }
                // Only release a debug pause. Natural selection/treasure/pause
                // states retain ownership and process the banked XP on resume.
                if *world.resource::<State<GameState>>().get() == GameState::DebugTools {
                    world
                        .resource_mut::<NextState<GameState>>()
                        .set(GameState::Playing);
                }
            }
            DebugAction::AdvanceWave => {
                let duration = world
                    .resource::<GameContent>()
                    .stage(world.resource::<ActiveStage>())
                    .duration;
                let mut stats = world.resource_mut::<crate::resources::GameStats>();
                let next_minute = (stats.time_elapsed / 60.0).floor() * 60.0 + 60.0;
                if next_minute < duration {
                    stats.time_elapsed = next_minute;
                }
                let wave = crate::waves::wave_index(stats.time_elapsed) + 1;
                world.resource_mut::<DebugSession>().status =
                    format!("Wave {wave} starts when the simulation runs.");
            }
            DebugAction::AdvancePhase => {
                let elapsed = world.resource::<GameStats>().time_elapsed;
                let next = world
                    .resource::<GameContent>()
                    .stage(world.resource::<ActiveStage>())
                    .schedule
                    .iter()
                    .find(|phase| phase.start > elapsed)
                    .map(|phase| phase.start);
                if let Some(start) = next {
                    world.resource_mut::<GameStats>().time_elapsed = start;
                    world.resource_mut::<DebugSession>().status =
                        "Advanced to the next authored spawn phase.".into();
                } else {
                    world.resource_mut::<DebugSession>().status =
                        "Already in the final spawn phase.".into();
                }
            }
            DebugAction::TriggerEncounter(index) => {
                if index
                    < world
                        .resource::<GameContent>()
                        .stage(world.resource::<ActiveStage>())
                        .encounters
                        .len()
                {
                    let mut session = world.resource_mut::<DebugSession>();
                    session.queued_encounters.push(index);
                    session.status =
                        "Encounter queued; resume the simulation to begin its warning.".into();
                }
            }
            DebugAction::TestVictory => {
                let duration = world
                    .resource::<GameContent>()
                    .stage(world.resource::<ActiveStage>())
                    .duration;
                world
                    .resource_mut::<crate::resources::GameStats>()
                    .time_elapsed = duration;
                // Explicit debug bypass. Normal boss-map victory still requires a kill.
                world.resource_mut::<crate::waves::WaveDirector>().finale =
                    crate::waves::FinaleProgress::Defeated;
                world
                    .resource_mut::<NextState<GameState>>()
                    .set(GameState::Playing);
            }
            DebugAction::TriggerFinale => {
                let stage = world
                    .resource::<GameContent>()
                    .stage(world.resource::<ActiveStage>());
                if matches!(
                    stage.win_condition,
                    crate::content::StageWinCondition::DefeatBoss { .. }
                ) {
                    let duration = stage.duration;
                    let mut stats = world.resource_mut::<GameStats>();
                    stats.time_elapsed = stats.time_elapsed.max(duration);
                    world.resource_mut::<DebugSession>().status =
                        "Final boss warning begins when the simulation runs.".into();
                }
            }
            DebugAction::SpawnEnemy(id) => spawn_enemy_test(world, &id),
            DebugAction::SelectStage(index) => {
                if let Some(stage) = world
                    .resource::<GameContent>()
                    .stages
                    .get(index)
                    .map(|stage| stage.id.clone())
                {
                    world.resource_mut::<crate::setup::RunConfiguration>().stage = stage;
                    configuration_result(world, crate::setup::start_configured_run);
                }
            }
            DebugAction::SpawnPickup(kind) => spawn_world_test_object(world, Some(kind)),
            DebugAction::SpawnBreakable => spawn_world_test_object(world, None),
            DebugAction::GrantWeapon(kind) => {
                let result = grant_debug_weapon(world, kind);
                world.resource_mut::<DebugSession>().status =
                    result.map_or_else(|error| error, |()| format!("{kind} added or upgraded."));
            }
            DebugAction::GrantPassive(kind) => {
                let player = world.query_filtered::<Entity, With<Player>>().single(world);
                let result = player
                    .map_err(|_| "No active player.".to_string())
                    .and_then(|player| crate::inventory::grant_passive(world, player, kind));
                world.resource_mut::<DebugSession>().status =
                    result.map_or_else(|error| error, |()| "Passive added or upgraded.".into());
            }
            DebugAction::ExportBuild => {
                configuration_result(world, crate::setup::export_current_build)
            }
            DebugAction::ExportConfiguration => {
                configuration_result(world, crate::setup::export_configuration)
            }
            DebugAction::ImportConfiguration => {
                configuration_result(world, crate::setup::import_configuration)
            }
            DebugAction::StartConfiguredRun => {
                configuration_result(world, crate::setup::start_configured_run)
            }
            DebugAction::Resume => close_panel(world),
        }
    }
    flush_encounter_requests(world);
}

fn grant_debug_weapon(world: &mut World, kind: WeaponType) -> Result<(), String> {
    let player = world
        .query_filtered::<Entity, With<Player>>()
        .single(world)
        .map_err(|_| "No active player.".to_string())?;
    let level = world
        .query::<(&WeaponMeta, &ChildOf)>()
        .iter(world)
        .find(|(weapon, parent)| weapon.weapon_type == kind && parent.parent() == player)
        .map(|(weapon, _)| weapon.level);
    if let Some(level) = level {
        let Some(mut spec) = world
            .resource::<WeaponUpgradeConfig>()
            .get_next_upgrades(kind, level)
            .into_iter()
            .next()
        else {
            return Err(format!("{kind} is already at its maximum level."));
        };
        for change in &mut spec.changes {
            if let WeaponUpgradeChange::AddCircle { pattern } = change {
                if pattern.is_none() {
                    *pattern = Some(PatternType::Banishment);
                }
            }
        }
        world.write_message(crate::menu::WeaponUpgradeConfirmedEvent {
            weapon_type: kind,
            upgrade_spec: spec,
        });
        Ok(())
    } else {
        crate::weapons::grant_configured_weapon(world, player, kind, 1, &[])
    }
}

fn configuration_result(world: &mut World, action: fn(&mut World) -> Result<(), String>) {
    let result = action(world);
    world.resource_mut::<DebugSession>().status = match result {
        Ok(()) => "Configuration action completed.".into(),
        Err(error) => error,
    };
}

fn spawn_world_test_object(world: &mut World, pickup: Option<LootKind>) {
    let spawned = world.run_system_once(
        move |mut commands: Commands,
              player: Query<(Entity, &Transform), With<Player>>,
              context: bevy_rapier2d::prelude::ReadRapierContext,
              bounds: Res<crate::arena::ArenaBounds>| {
            let Ok((player_entity, player)) = player.single() else {
                return false;
            };
            let Ok(context) = context.single() else {
                return false;
            };
            let distance = if pickup.is_some() { 20.0 } else { 60.0 };
            for direction in [Vec2::X, Vec2::Y, Vec2::NEG_X, Vec2::NEG_Y] {
                let position = player.translation.truncate() + direction * distance;
                if position
                    .abs()
                    .cmple(bounds.half_size - Vec2::splat(16.0))
                    .all()
                    && crate::waves::position_is_free(
                        &context,
                        position,
                        16.0,
                        pickup.map(|_| player_entity),
                    )
                {
                    if let Some(kind) = pickup {
                        crate::loot::spawn_loot(&mut commands, position, kind);
                    } else {
                        crate::loot::spawn_breakable(
                            &mut commands,
                            position,
                            10,
                            1.0,
                            "Test urn",
                            LootKind::Healing,
                        );
                    }
                    return true;
                }
            }
            false
        },
    );
    world.resource_mut::<DebugSession>().status = if spawned.unwrap_or(false) {
        "Test object spawned beside the player; resume to interact.".into()
    } else {
        "No clear test position nearby. Move away from walls or enemies and retry.".into()
    };
}

fn flush_encounter_requests(world: &mut World) {
    if *world.resource::<State<GameState>>().get() != GameState::Playing {
        return;
    }
    let requests = std::mem::take(&mut world.resource_mut::<DebugSession>().queued_encounters);
    if !requests.is_empty() {
        world.resource_mut::<DebugSession>().status = "Encounter warning triggered.".into();
    }
    for index in requests {
        world.write_message(crate::waves::TriggerEncounter(index));
    }
}

fn spawn_enemy_test(world: &mut World, id: &str) {
    let Some(definition) = world.resource::<GameContent>().enemy(id).cloned() else {
        return;
    };
    let result = world.run_system_once(
        move |mut commands: Commands,
              textures: Res<GameTextures>,
              player: Query<&Transform, With<Player>>,
              bounds: Res<crate::arena::ArenaBounds>,
              context: ReadRapierContext| {
            let Ok(player) = player.single() else {
                return false;
            };
            let Ok(context) = context.single() else {
                return false;
            };
            let center = player.translation.truncate();
            for index in 0..24 {
                let position =
                    center + Vec2::from_angle(index as f32 * std::f32::consts::TAU / 24.0) * 280.0;
                if position
                    .abs()
                    .cmplt(bounds.half_size - Vec2::splat(32.0))
                    .all()
                    && crate::waves::position_is_free(&context, position, 20.0, None)
                {
                    crate::waves::spawn_enemy(&mut commands, &textures, &definition, position, 1.0);
                    return true;
                }
            }
            false
        },
    );
    world.resource_mut::<DebugSession>().status = if matches!(result, Ok(true)) {
        format!("Spawned {id} using its authored behavior.")
    } else {
        "No clear space for that test enemy.".into()
    };
}

fn close_panel(world: &mut World) {
    let mut session = world.resource_mut::<DebugSession>();
    session.open = false;
    session.wants_keyboard_input = false;
    session.wants_pointer_input = false;
    session.consumed_pause_input = true;
    if *world.resource::<State<GameState>>().get() == GameState::DebugTools {
        world
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Playing);
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "The debug window reads distinct game resources and egui context."
)]
fn panel(
    mut contexts: EguiContexts,
    state: Res<State<GameState>>,
    mut session: ResMut<DebugSession>,
    enemies: Query<&Health, With<Enemy>>,
    overlaps: Res<ActiveOverlaps>,
    mut debug_render: Option<ResMut<bevy_rapier2d::render::DebugRenderContext>>,
    content: Res<GameContent>,
    active_stage: Res<ActiveStage>,
    stats: Res<GameStats>,
    players: Query<&Health, With<Player>>,
    circles: Query<&MagickCircle>,
    weapons: Query<&WeaponMeta>,
    passives: Query<&PassiveInventory, With<Player>>,
    passive_catalog: Res<PassiveCatalog>,
    upgrade_config: Res<WeaponUpgradeConfig>,
    mut preferences: DebugPreferences,
) -> Result {
    session.wants_keyboard_input = false;
    session.wants_pointer_input = false;
    let ctx = contexts.ctx_mut()?;
    if *state.get() == GameState::Playing {
        egui::Area::new("debug_hint".into())
            .anchor(egui::Align2::LEFT_BOTTOM, [16.0, -16.0])
            .show(ctx, |ui| {
                ui.set_width(270.0);
                ui.label("F1  Debug menu    F2  Weapons");
                if !session.waves || session.invulnerable {
                    ui.label("TEST OVERRIDES ACTIVE");
                }
            });
    }
    if !session.open {
        return Ok(());
    }
    let has_player = players.single().is_ok_and(|health| health.current > 0);
    let active_run = has_player
        && matches!(
            state.get(),
            GameState::Playing
                | GameState::DebugTools
                | GameState::Paused
                | GameState::LevelUp
                | GameState::Treasure
        );
    let mut window_open = true;
    egui::Window::new("Debug menu")
        .open(&mut window_open)
        .default_pos([18.0, 45.0])
        .default_width(500.0)
        .default_height(650.0)
        .resizable(true)
        .vscroll(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (tab, name) in [
                    (DebugTab::Run, "Run"),
                    (DebugTab::Weapons, "Weapons"),
                    (DebugTab::World, "World"),
                ] {
                    ui.selectable_value(&mut session.tab, tab, name);
                }
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut session.pause_simulation, "Pause simulation");
                ui.label(if *state.get() == GameState::Playing {
                    "Live"
                } else {
                    "Paused"
                });
                if ui.button("Close menu").clicked() {
                    session.actions.push(DebugAction::Resume);
                }
            });
            ui.small("F1 / Escape closes. F2 opens Weapons. Live mode keeps this menu open.");
            ui.small(format!("Run seed: {}", preferences.config.0.seed));
            ui.horizontal(|ui| {
                ui.checkbox(&mut preferences.audio.muted, "Mute sound");
                ui.add(egui::Slider::new(&mut preferences.audio.volume, 0.0..=1.0).text("Volume"));
            });
            if !matches!(state.get(), GameState::Playing | GameState::DebugTools) {
                ui.small("The game screen keeps its own pause. Debug controls remain available where applicable.");
            }
            ui.separator();
            match session.tab {
                DebugTab::Run => {
                    draw_run_controls(ui, &mut session, &content, &active_stage, &stats, active_run, has_player, matches!(state.get(), GameState::Playing | GameState::DebugTools))
                }
                DebugTab::Weapons => {
                    ui.add_enabled_ui(active_run, |ui| draw_inventory_controls(ui, &mut session, &weapons, passives.single().ok(), &passive_catalog, &upgrade_config));
                    ui.separator();
                    if circles.is_empty() { ui.label("Acquire Magick Circle to edit its pattern layout. Other weapons retain their own attack behavior."); }
                    ui.add_enabled_ui(active_run && !circles.is_empty(), |ui| draw_weapon_controls(ui, &mut session));
                }
                DebugTab::World => draw_world_controls(
                    ui,
                    &mut session,
                    debug_render.as_deref_mut(),
                    &content,
                    active_stage.0,
                    active_run,
                ),
            }
            ui.separator();
            ui.label(format!(
                "Enemies: {}   Active overlap pairs: {}",
                enemies.iter().count(),
                overlaps.overlaps.len()
            ));
            if let Some(lowest) = enemies.iter().map(|health| health.current).min() {
                ui.label(format!("Lowest enemy HP: {lowest}"));
            }
            ui.label(&session.status);
        });
    if !window_open {
        session.actions.push(DebugAction::Resume);
    }
    session.wants_keyboard_input = window_open && ctx.egui_wants_keyboard_input();
    session.wants_pointer_input = window_open && ctx.egui_wants_pointer_input();
    Ok(())
}

#[expect(
    clippy::too_many_arguments,
    reason = "Run controls expose contextual capabilities separately from their data."
)]
fn draw_run_controls(
    ui: &mut egui::Ui,
    session: &mut DebugSession,
    content: &GameContent,
    active: &ActiveStage,
    stats: &GameStats,
    active_run: bool,
    has_player: bool,
    can_test_victory: bool,
) {
    ui.heading("Run controls");
    let stage = content.stage(active);
    let seconds = stats.time_elapsed as u32;
    let phase = crate::waves::phase_index(&stage.schedule, stats.time_elapsed) + 1;
    ui.label(format!(
        "{}  |  {:02}:{:02}  |  Phase {phase}/{}",
        stage.name,
        seconds / 60,
        seconds % 60,
        stage.schedule.len()
    ));
    ui.add_enabled_ui(active_run, |ui| {
    ui.checkbox(&mut session.waves, "Natural enemy spawns");
    ui.checkbox(&mut session.invulnerable, "Player invulnerable");
    ui.horizontal(|ui| {
        if ui.button("Heal player").clicked() {
            session.actions.push(DebugAction::Heal);
        }
        if ui.button("Grant one level").clicked() {
            session.actions.push(DebugAction::GrantLevel);
        }
    });
    ui.horizontal(|ui| {
        if ui.button("Next wave").clicked() {
            session.actions.push(DebugAction::AdvanceWave);
        }
        if ui.button("Next phase").clicked() {
            session.actions.push(DebugAction::AdvancePhase);
        }
        if ui.add_enabled(can_test_victory, egui::Button::new("Test victory")).clicked() {
            session.actions.push(DebugAction::TestVictory);
        }
    });
    ui.small("Grant level keeps this menu open. Selection and treasure screens keep their pending choices.");
    if matches!(stage.win_condition, crate::content::StageWinCondition::DefeatBoss { .. })
        && ui.button("Start final boss encounter").clicked() {
        session.actions.push(DebugAction::TriggerFinale);
    }
    ui.collapsing("Test an enemy", |ui| {
        for enemy in &content.enemies {
            if ui.button(format!("Spawn {}", enemy.name)).clicked() {
                session.actions.push(DebugAction::SpawnEnemy(enemy.id.clone()));
            }
        }
    });
    if !stage.encounters.is_empty() {
        ui.separator();
        ui.label("Authored encounters");
        for (index, encounter) in stage.encounters.iter().enumerate() {
            let name = content
                .enemy(&encounter.enemy)
                .map_or(encounter.enemy.as_str(), |enemy| enemy.name.as_str());
            let seconds = encounter.at as u32;
            if ui
                .button(format!(
                    "Test {name} · {:02}:{:02}",
                    seconds / 60,
                    seconds % 60
                ))
                .clicked()
            {
                session.actions.push(DebugAction::TriggerEncounter(index));
            }
        }
    }
    });
    ui.separator();
    ui.label("Run configuration (run-config.ron)");
    ui.horizontal(|ui| {
        if ui
            .add_enabled(has_player, egui::Button::new("Export current build"))
            .clicked()
        {
            session.actions.push(DebugAction::ExportBuild);
        }
        if ui.button("Export configuration").clicked() {
            session.actions.push(DebugAction::ExportConfiguration);
        }
    });
    ui.horizontal(|ui| {
        if ui.button("Import configuration").clicked() {
            session.actions.push(DebugAction::ImportConfiguration);
        }
        if ui.button("Start configured run").clicked() {
            session.actions.push(DebugAction::StartConfiguredRun);
        }
    });
}

fn draw_inventory_controls(
    ui: &mut egui::Ui,
    session: &mut DebugSession,
    weapons: &Query<&WeaponMeta>,
    passives: Option<&PassiveInventory>,
    catalog: &PassiveCatalog,
    config: &WeaponUpgradeConfig,
) {
    ui.heading("Weapon slots");
    for kind in WeaponType::iter() {
        let level = weapons
            .iter()
            .find(|weapon| weapon.weapon_type == kind)
            .map_or(0, |weapon| weapon.level);
        let max = config.max_level(kind);
        ui.horizontal(|ui| {
            ui.label(format!("{kind}   Lv. {level}/{max}"));
            if ui
                .add_enabled(
                    level < max
                        && (level > 0 || weapons.iter().count() < crate::inventory::MAX_WEAPONS),
                    egui::Button::new(if level == 0 { "Add" } else { "Upgrade" }),
                )
                .clicked()
            {
                session.actions.push(DebugAction::GrantWeapon(kind));
            }
        });
        ui.small(format!("{} · {}", kind.family(), kind.summary()));
    }
    ui.separator();
    ui.heading("Passive slots");
    for kind in PassiveType::iter() {
        let definition = catalog.get(kind);
        let level = passives.map_or(0, |inventory| inventory.level(kind));
        let available = passives.is_some_and(|inventory| inventory.can_upgrade(kind, catalog));
        ui.horizontal(|ui| {
            ui.label(format!(
                "{}   Lv. {level}/{}",
                definition.name,
                definition.max_level()
            ));
            if ui
                .add_enabled(
                    available,
                    egui::Button::new(if level == 0 { "Add" } else { "Upgrade" }),
                )
                .clicked()
            {
                session.actions.push(DebugAction::GrantPassive(kind));
            }
        });
        ui.small(&definition.description);
    }
}

fn draw_weapon_controls(ui: &mut egui::Ui, session: &mut DebugSession) {
    ui.heading("Circle build");
    ui.label("Build choices control the automatic layout.");
    ui.horizontal(|ui| {
        if ui.button("Damage + Pull").clicked() {
            session.patterns = vec![PatternType::Banishment, PatternType::Invocation];
        }
        if ui.button("Root + Eruption").clicked() {
            session.patterns = vec![PatternType::Binding, PatternType::Manifestation];
        }
        if ui.button("All five").clicked() {
            session.patterns = PatternType::iter().collect();
        }
    });
    for (index, pattern) in session.patterns.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label(if index == 0 {
                "Center".to_string()
            } else {
                format!("Outer {index}")
            });
            egui::ComboBox::from_id_salt(("circle-pattern", index))
                .selected_text(pattern.to_string())
                .show_ui(ui, |ui| {
                    for choice in PatternType::iter() {
                        ui.selectable_value(pattern, choice, choice.to_string());
                    }
                });
        });
        ui.small(pattern.get_description());
    }
    ui.horizontal(|ui| {
        if ui
            .add_enabled(session.patterns.len() < 7, egui::Button::new("Add circle"))
            .clicked()
        {
            session.patterns.push(PatternType::Banishment);
        }
        if ui
            .add_enabled(session.patterns.len() > 1, egui::Button::new("Remove last"))
            .clicked()
        {
            session.patterns.pop();
        }
        if ui.button("Apply build").clicked() {
            session
                .actions
                .push(DebugAction::ApplyBuild(session.patterns.clone()));
        }
    });
    ui.small(
        "Duplicates allowed. Applying clears circles and effects; weapon stats stay unchanged.",
    );
    ui.horizontal(|ui| {
        if ui.button("Stationary targets").clicked() {
            session
                .actions
                .push(DebugAction::SpawnTargets { moving: false });
        }
        if ui.button("Moving targets").clicked() {
            session
                .actions
                .push(DebugAction::SpawnTargets { moving: true });
        }
    });
}

fn draw_world_controls(
    ui: &mut egui::Ui,
    session: &mut DebugSession,
    debug_render: Option<&mut bevy_rapier2d::render::DebugRenderContext>,
    content: &GameContent,
    active_stage: usize,
    active_run: bool,
) {
    ui.heading("World controls");
    ui.label("Stage (starts a fresh run)");
    for (index, stage) in content.stages.iter().enumerate() {
        if ui
            .selectable_label(index == active_stage, &stage.name)
            .clicked()
        {
            session.actions.push(DebugAction::SelectStage(index));
        }
    }
    ui.separator();
    ui.add_enabled_ui(active_run, |ui| {
        ui.horizontal(|ui| {
            if ui.button("Test chest").clicked() {
                session
                    .actions
                    .push(DebugAction::SpawnPickup(LootKind::Chest));
            }
            if ui.button("Healing pickup").clicked() {
                session
                    .actions
                    .push(DebugAction::SpawnPickup(LootKind::Healing));
            }
            if ui.button("Test urn").clicked() {
                session.actions.push(DebugAction::SpawnBreakable);
            }
        });
        if let Some(render) = debug_render {
            ui.checkbox(&mut render.enabled, "Show physics colliders");
        }
        if ui.button("Clear arena").clicked() {
            session.actions.push(DebugAction::ClearArena);
        }
        ui.small("Clears enemies, gems, circles, and effects. Your build and progression remain.");
    });
}
