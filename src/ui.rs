use crate::components::{Health, Player, PlayerStats};
use crate::experience::{calculate_experience_needed, Experience};
use crate::inventory::{PassiveCatalog, PassiveInventory, PassiveType, MAX_PASSIVES, MAX_WEAPONS};
use crate::resources::{GameStats, WaveConfig};
use crate::weapons::WeaponMeta;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

// Root node marker
#[derive(Component)]
pub struct GameUI;

#[derive(Component)]
pub struct HealthBar;

#[derive(Component)]
pub struct HealthText;

#[derive(Component)]
pub struct GameTimer;

#[derive(Component)]
pub struct KillCounter;
#[derive(Component)]
pub struct RunStatus;
#[derive(Component)]
pub struct ExperienceStatus;
#[derive(Component)]
pub struct BossStatus;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildSlotKind {
    Weapon,
    Passive,
}

#[derive(Component)]
pub struct BuildSlot {
    pub kind: BuildSlotKind,
    pub index: usize,
}

#[derive(Component)]
pub struct BuildSlotLabel;

#[derive(Component)]
pub(crate) struct BuildSlotIcon;

#[derive(Clone)]
pub(crate) struct BuildEntry {
    pub name: String,
    pub level: u32,
    pub icon: usize,
    pub weapon: Option<crate::weapons::WeaponType>,
}

#[derive(Clone)]
pub(crate) struct BuildSnapshot {
    pub weapons: Vec<BuildEntry>,
    pub passives: Vec<BuildEntry>,
    pub stats: String,
}

/// Captured before dead players and their weapon children are removed.
#[derive(Resource, Default)]
pub(crate) struct FinalBuild(pub Option<BuildSnapshot>);

pub(crate) fn capture_final_build(
    view: BuildView,
    next: Res<NextState<crate::resources::GameState>>,
    mut final_build: ResMut<FinalBuild>,
) {
    if !matches!(
        next.as_ref(),
        NextState::Pending(
            crate::resources::GameState::GameOver | crate::resources::GameState::Victory
        )
    ) {
        return;
    }
    if let Some(snapshot) = view.snapshot() {
        final_build.0 = Some(snapshot);
    }
}

pub(crate) fn reset_final_build(mut final_build: ResMut<FinalBuild>) {
    final_build.0 = None;
}

#[derive(SystemParam)]
pub(crate) struct BuildView<'w, 's> {
    player: Query<
        'w,
        's,
        (
            &'static Health,
            &'static PlayerStats,
            &'static PassiveInventory,
            &'static Children,
        ),
        With<Player>,
    >,
    weapons: Query<'w, 's, &'static WeaponMeta>,
    catalog: Res<'w, PassiveCatalog>,
}

impl BuildView<'_, '_> {
    pub(crate) fn snapshot(&self) -> Option<BuildSnapshot> {
        let Ok((health, stats, passives, children)) = self.player.single() else {
            return None;
        };
        let weapons = children
            .iter()
            .filter_map(|entity| self.weapons.get(entity).ok())
            .map(|weapon| BuildEntry {
                name: weapon.weapon_type.to_string(),
                level: weapon.level,
                weapon: Some(weapon.weapon_type),
                icon: match weapon.weapon_type.id() {
                    "inscribed_needles" => 1,
                    "alchemical_vessel" => 2,
                    _ => 0,
                },
            })
            .collect();
        let passives = passives
            .items
            .iter()
            .map(|item| BuildEntry {
                name: self.catalog.get(item.kind).name.clone(),
                level: item.level as u32,
                weapon: None,
                icon: match item.kind {
                    PassiveType::MoveSpeed => 3,
                    PassiveType::PickupRange => 4,
                    PassiveType::MaxHealth => 5,
                    PassiveType::Cooldown => 6,
                },
            })
            .collect();
        Some(BuildSnapshot {
            weapons, passives,
            stats: format!(
                "Health  {} / {}\nMovement speed  {:.0}\nPickup range  {:.0}\nDamage  {:.0}%\nArea  {:.0}%\nCooldown reduction  {:.0}%\nLuck  {}\nArmor  {:.0}\nRegeneration  {:.1} / s",
                health.current, health.maximum, stats.move_speed, stats.pickup_range,
                stats.damage_multiplier * 100.0, stats.area_multiplier * 100.0,
                stats.cooldown_reduction * 100.0, stats.luck, stats.armor, stats.regeneration,
            ),
        })
    }
}

pub fn spawn_ui(mut commands: Commands) {
    // Root node with marker component
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            GameUI,
        ))
        .with_children(|parent| {
            spawn_build_slots(parent, BuildSlotKind::Weapon, "WEAPONS", 78.0, MAX_WEAPONS);
            spawn_build_slots(
                parent,
                BuildSlotKind::Passive,
                "PASSIVES",
                178.0,
                MAX_PASSIVES,
            );
            // Health bar container
            parent
                .spawn((
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(30.0),
                        border: UiRect::all(Val::Px(2.0)),

                        ..default()
                    },
                    BorderColor::all(Color::srgb(0.7, 0.7, 0.7)),
                    BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                ))
                .with_children(|parent| {
                    // The actual health bar
                    parent.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.8, 0.2, 0.2)),
                        HealthBar,
                    ));
                });

            // Health text
            parent.spawn((
                Text::new("100/100"),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(220.0),
                    top: Val::Px(2.0),
                    ..default()
                },
                TextFont {
                    font_size: FontSize::Px(24.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                HealthText,
            ));

            // Game Timer
            parent.spawn((
                Text::new("00:00"),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Px(10.0),
                    // Center the text horizontally
                    margin: UiRect {
                        left: Val::Px(-40.0), // Approximately half the text width
                        ..default()
                    },
                    ..default()
                },
                TextFont {
                    font_size: FontSize::Px(32.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                GameTimer,
            ));

            // Kill Counter
            parent.spawn((
                Text::new("Kills: 0"),
                TextFont {
                    font_size: FontSize::Px(24.0),
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(10.0),
                    top: Val::Px(10.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                KillCounter,
            ));
            parent.spawn((
                Text::new("Wave 1 / 10"),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::srgb(0.82, 0.76, 0.55)),
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(48.0),
                    left: Val::Percent(50.0),
                    width: Val::Px(440.0),
                    margin: UiRect::left(Val::Px(-220.0)),
                    ..default()
                },
                RunStatus,
                TextLayout::justify(Justify::Center),
            ));
            parent.spawn((
                Text::new("Level 1   0 / 100 XP"),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::srgb(0.65, 0.82, 0.95)),
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(48.0),
                    left: Val::Px(10.0),
                    ..default()
                },
                ExperienceStatus,
            ));
            parent.spawn((
                Text::new(""),
                BossStatus,
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.65, 0.35)),
                TextLayout::justify(Justify::Center),
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(24.0),
                    left: Val::Percent(25.0),
                    width: Val::Percent(50.0),
                    ..default()
                },
            ));
        });
}

fn spawn_build_slots(
    parent: &mut ChildSpawnerCommands,
    kind: BuildSlotKind,
    title: &str,
    top: f32,
    count: usize,
) {
    parent
        .spawn((Node {
            position_type: PositionType::Absolute,
            top: Val::Px(top),
            left: Val::Px(10.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            ..default()
        },))
        .with_children(|row| {
            row.spawn((
                Text::new(title),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.78, 0.7)),
            ));
            row.spawn(Node {
                column_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|slots| {
                for index in 0..count {
                    slots
                        .spawn((
                            BuildSlot { kind, index },
                            Node {
                                width: Val::Px(88.0),
                                height: Val::Px(74.0),
                                padding: UiRect::all(Val::Px(4.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                row_gap: Val::Px(2.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.07, 0.08, 0.10, 0.90)),
                            BorderColor::all(Color::srgb(0.25, 0.25, 0.28)),
                        ))
                        .with_children(|slot| {
                            slot.spawn((
                                ImageNode::default(),
                                crate::art::ItemIcon(0),
                                BuildSlotIcon,
                                Node {
                                    width: Val::Px(26.0),
                                    height: Val::Px(26.0),
                                    display: Display::None,
                                    ..default()
                                },
                            ));
                            slot.spawn((
                                Text::new("Empty"),
                                BuildSlotLabel,
                                TextFont {
                                    font_size: FontSize::Px(11.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.72, 0.73, 0.76)),
                                TextLayout::justify(Justify::Center),
                            ));
                        });
                }
            });
        });
}

pub(crate) fn update_build_slots(
    build: BuildView,
    slots: Query<(&BuildSlot, &Children)>,
    mut labels: Query<&mut Text, With<BuildSlotLabel>>,
    mut icons: Query<(&mut Node, &mut crate::art::ItemIcon), With<BuildSlotIcon>>,
) {
    let snapshot = build.snapshot();
    for (slot, children) in &slots {
        let entry = snapshot.as_ref().and_then(|build| match slot.kind {
            BuildSlotKind::Weapon => build.weapons.get(slot.index),
            BuildSlotKind::Passive => build.passives.get(slot.index),
        });
        for child in children.iter() {
            if let Ok(mut text) = labels.get_mut(child) {
                let value = entry.map_or_else(
                    || "Empty".to_string(),
                    |entry| format!("{}\nLv. {}", entry.name, entry.level),
                );
                if text.0 != value {
                    text.0 = value;
                }
            }
            if let Ok((mut node, mut icon)) = icons.get_mut(child) {
                node.display = if entry.is_some() {
                    Display::Flex
                } else {
                    Display::None
                };
                if let Some(entry) = entry {
                    if icon.0 != entry.icon {
                        icon.0 = entry.icon;
                    }
                }
            }
        }
    }
}

pub fn cleanup_ui(mut commands: Commands, ui_query: Query<Entity, With<GameUI>>) {
    for entity in ui_query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn update_game_timer(
    stats: Res<GameStats>,
    mut timer_query: Query<&mut Text, With<GameTimer>>,
) {
    if let Ok(mut text) = timer_query.single_mut() {
        let total_secs = stats.time_elapsed as u32;
        let minutes = total_secs / 60;
        let seconds = total_secs % 60;
        text.0 = format!("{:02}:{:02}", minutes, seconds);
    }
}

#[expect(
    clippy::type_complexity,
    reason = "ParamSet safely shares mutable Text access between HUD labels."
)]
pub fn update_run_status(
    wave: Res<WaveConfig>,
    content: Res<crate::content::GameContent>,
    active: Res<crate::content::ActiveStage>,
    stats: Res<GameStats>,
    director: Res<crate::waves::WaveDirector>,
    xp: Query<&Experience, With<Player>>,
    mut labels: ParamSet<(
        Query<&mut Text, With<RunStatus>>,
        Query<&mut Text, With<ExperienceStatus>>,
    )>,
) {
    if let Ok(mut text) = labels.p0().single_mut() {
        let stage = content.stage(&active);
        text.0 = if stats.time_elapsed >= stage.duration
            && matches!(
                stage.win_condition,
                crate::content::StageWinCondition::DefeatBoss { .. }
            ) {
            match director.finale {
                crate::waves::FinaleProgress::Awaiting
                | crate::waves::FinaleProgress::Telegraphing => {
                    "FINAL BOSS - Prepare for the encounter".into()
                }
                crate::waves::FinaleProgress::Active(_) => "FINAL BOSS - Defeat it to win".into(),
                crate::waves::FinaleProgress::Defeated => "FINAL BOSS - Defeated".into(),
            }
        } else {
            format!(
                "{} - Wave {} / {}",
                stage.name,
                wave.current_wave.max(1),
                (stage.duration / 60.0).ceil() as u32
            )
        };
    }
    if let Ok(xp) = xp.single() {
        if let Ok(mut text) = labels.p1().single_mut() {
            text.0 = format!(
                "Level {}   {} / {} XP",
                xp.level,
                xp.current,
                calculate_experience_needed(xp.level)
            );
        }
    }
}

pub fn update_health_ui(
    mut health_bar_query: Query<&mut Node, With<HealthBar>>,
    mut health_text_query: Query<&mut Text, With<HealthText>>,
    player_query: Query<&Health, With<Player>>,
) {
    if let Ok(player_health) = player_query.single() {
        // Update health bar width
        if let Ok(mut style) = health_bar_query.single_mut() {
            let health_percent =
                (player_health.current as f32 / player_health.maximum as f32 * 100.0).max(0.0);
            style.width = Val::Percent(health_percent);
        }

        // Update health text
        if let Ok(mut text) = health_text_query.single_mut() {
            text.0 = format!("{}/{}", player_health.current, player_health.maximum);
        }
    }
}

pub fn update_kill_counter(
    game_stats: Res<GameStats>,
    mut kill_counter_query: Query<&mut Text, With<KillCounter>>,
) {
    if let Ok(mut text) = kill_counter_query.single_mut() {
        text.0 = format!(
            "Kills: {}  Gold: {}",
            game_stats.enemies_killed, game_stats.gold
        );
    }
}

pub fn update_boss_status(
    bosses: Query<(&Name, &Health), With<crate::waves::Boss>>,
    mut text: Query<&mut Text, With<BossStatus>>,
) {
    if let Ok(mut text) = text.single_mut() {
        text.0 = bosses
            .iter()
            .filter(|(_, h)| h.current > 0)
            .take(3)
            .map(|(name, health)| format!("{}  {} / {}", name, health.current, health.maximum))
            .collect::<Vec<_>>()
            .join("  |  ");
    }
}
