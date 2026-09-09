//! Optional playtest tools. The action layer is independent of egui for headless tests.
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};
use strum::IntoEnumIterator;

use crate::components::{Enemy, Health, Player};
use crate::experience::{calculate_experience_needed, Experience, ExperienceOrb};
use crate::resources::{GameState, GameTextures, SpawnTimer};
use crate::weapons::magick_circle::{MagickCircle, PatternType};
use crate::weapons::overlap::{ActiveOverlaps, EnemyCircleHits, OverlappingPatterns};
use crate::weapons::{Attack, BindingEffect, InvocationPull, ProtectionBuff, WeaponCooldown};
use crate::GameplaySets;

#[derive(Resource)]
pub struct DebugSession {
    pub waves: bool,
    pub invulnerable: bool,
    pub patterns: Vec<PatternType>,
    pub actions: Vec<DebugAction>,
    pub status: String,
}

impl Default for DebugSession {
    fn default() -> Self {
        Self {
            waves: true,
            invulnerable: false,
            patterns: vec![PatternType::Banishment],
            actions: Vec::new(),
            status: "Choose a build, apply it, then resume.".into(),
        }
    }
}

#[derive(Clone)]
pub enum DebugAction {
    ApplyBuild(Vec<PatternType>),
    ClearArena,
    SpawnTargets { moving: bool },
    Heal,
    GrantLevel,
    AdvanceWave,
    TestVictory,
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
impl Plugin for DebugUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(EguiPrimaryContextPass, panel);
    }
}

fn toggle_panel(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    mut session: ResMut<DebugSession>,
    weapons: Query<&MagickCircle>,
) {
    if keys.just_pressed(KeyCode::F1) {
        match state.get() {
            GameState::Playing => {
                if let Ok(circle) = weapons.single() {
                    session.patterns = circle.patterns.clone();
                }
                next.set(GameState::DebugTools);
            }
            GameState::DebugTools => next.set(GameState::Playing),
            _ => {}
        }
    }
    if *state.get() == GameState::DebugTools && keys.just_pressed(KeyCode::Escape) {
        next.set(GameState::Playing);
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
    let actions = std::mem::take(&mut world.resource_mut::<DebugSession>().actions);
    if *world.resource::<State<GameState>>().get() != GameState::DebugTools {
        return;
    }
    for action in actions {
        match action {
            DebugAction::ApplyBuild(patterns) => {
                if patterns.is_empty() || patterns.len() > 6 {
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
                    "Build applied. Fresh circles will spawn on resume.".into();
            }
            DebugAction::ClearArena => {
                clear_effects(world);
                let entities: Vec<_> = world
                    .query_filtered::<Entity, Or<(With<Enemy>, With<ExperienceOrb>)>>()
                    .iter(world)
                    .collect();
                for entity in entities {
                    world.despawn(entity);
                }
                world.resource_mut::<SpawnTimer>().0.reset();
                world.resource_mut::<DebugSession>().status =
                    "Arena cleared; build and progression retained.".into();
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
                world
                    .resource_mut::<NextState<GameState>>()
                    .set(GameState::Playing);
            }
            DebugAction::AdvanceWave => {
                let mut stats = world.resource_mut::<crate::resources::GameStats>();
                if stats.time_elapsed < 540.0 {
                    stats.time_elapsed =
                        ((stats.time_elapsed / 60.0).floor() * 60.0 + 60.0).min(540.0);
                }
                let wave = crate::waves::wave_index(stats.time_elapsed) + 1;
                world.resource_mut::<DebugSession>().status =
                    format!("Wave {wave} starts on resume.");
            }
            DebugAction::TestVictory => {
                world
                    .resource_mut::<crate::resources::GameStats>()
                    .time_elapsed = crate::run::SURVIVAL_SECONDS;
                world
                    .resource_mut::<NextState<GameState>>()
                    .set(GameState::Playing);
            }
            DebugAction::Resume => world
                .resource_mut::<NextState<GameState>>()
                .set(GameState::Playing),
        }
    }
}

fn panel(
    mut contexts: EguiContexts,
    state: Res<State<GameState>>,
    mut session: ResMut<DebugSession>,
    enemies: Query<&Health, With<Enemy>>,
    overlaps: Res<ActiveOverlaps>,
    mut debug_render: Option<ResMut<bevy_rapier2d::render::DebugRenderContext>>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    if *state.get() == GameState::Playing {
        egui::Area::new("lab_hint".into())
            .anchor(egui::Align2::LEFT_BOTTOM, [16.0, -16.0])
            .show(ctx, |ui| {
                ui.set_width(240.0);
                ui.label("F1  Circle Lab");
                if !session.waves || session.invulnerable {
                    ui.label("TEST OVERRIDES ACTIVE");
                }
            });
        return Ok(());
    }
    if *state.get() != GameState::DebugTools {
        return Ok(());
    }
    egui::Window::new("Circle Lab  |  simulation paused")
        .default_pos([18.0, 45.0]).default_width(430.0).default_height(600.0).resizable(true).vscroll(true)
        .show(ctx, |ui| {
            ui.label("F1 / Escape to resume. Build choices control the automatic layout.");
            ui.separator();
            ui.heading("Circle build");
            ui.horizontal(|ui| {
                if ui.button("Damage + Pull").clicked() { session.patterns = vec![PatternType::Banishment, PatternType::Invocation]; }
                if ui.button("Root + Eruption").clicked() { session.patterns = vec![PatternType::Binding, PatternType::Manifestation]; }
                if ui.button("All five").clicked() { session.patterns = PatternType::iter().collect(); }
            });
            for (index, pattern) in session.patterns.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(if index == 0 { "Center".to_string() } else { format!("Outer {index}") });
                    egui::ComboBox::from_id_salt(index).selected_text(pattern.to_string()).show_ui(ui, |ui| {
                        for choice in PatternType::iter() { ui.selectable_value(pattern, choice, choice.to_string()); }
                    });
                });
                ui.small(pattern.get_description());
            }
            ui.horizontal(|ui| {
                if ui.add_enabled(session.patterns.len() < 6, egui::Button::new("Add circle")).clicked() { session.patterns.push(PatternType::Banishment); }
                if ui.add_enabled(session.patterns.len() > 1, egui::Button::new("Remove last")).clicked() { session.patterns.pop(); }
                if ui.button("Apply build").clicked() {
                    let patterns = session.patterns.clone();
                    session.actions.push(DebugAction::ApplyBuild(patterns));
                }
            });
            ui.small("Duplicates allowed. Applying clears old circles and effects; weapon stats stay unchanged.");
            ui.separator();
            ui.heading("Test conditions");
            ui.checkbox(&mut session.waves, "Natural enemy spawns");
            ui.checkbox(&mut session.invulnerable, "Player invulnerable");
            if let Some(render) = debug_render.as_mut() { ui.checkbox(&mut render.enabled, "Show physics colliders"); }
            ui.horizontal(|ui| {
                if ui.button("Stationary targets").clicked() { session.actions.push(DebugAction::SpawnTargets { moving: false }); }
                if ui.button("Moving targets").clicked() { session.actions.push(DebugAction::SpawnTargets { moving: true }); }
                if ui.button("Clear arena").clicked() { session.actions.push(DebugAction::ClearArena); }
            });
            ui.horizontal(|ui| {
                if ui.button("Heal player").clicked() { session.actions.push(DebugAction::Heal); }
                if ui.button("Grant one level").clicked() { session.actions.push(DebugAction::GrantLevel); }
                if ui.button("Resume").clicked() { session.actions.push(DebugAction::Resume); }
            });
            ui.horizontal(|ui| {
                if ui.button("Next wave").clicked() { session.actions.push(DebugAction::AdvanceWave); }
                if ui.button("Test victory").clicked() { session.actions.push(DebugAction::TestVictory); }
            });
            ui.separator();
            ui.label(format!("Enemies: {}   Active overlap pairs: {}", enemies.iter().count(), overlaps.overlaps.len()));
            if let Some(lowest) = enemies.iter().map(|h| h.current).min() { ui.label(format!("Lowest enemy HP: {lowest}")); }
            ui.label(&session.status);
        });
    Ok(())
}
