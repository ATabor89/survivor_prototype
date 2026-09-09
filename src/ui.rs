use crate::components::{Health, Player};
use crate::experience::{calculate_experience_needed, Experience};
use crate::resources::{GameStats, WaveConfig};
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
                    margin: UiRect::left(Val::Px(-56.0)),
                    ..default()
                },
                RunStatus,
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
        });
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
    xp: Query<&Experience, With<Player>>,
    mut labels: ParamSet<(
        Query<&mut Text, With<RunStatus>>,
        Query<&mut Text, With<ExperienceStatus>>,
    )>,
) {
    if let Ok(mut text) = labels.p0().single_mut() {
        text.0 = format!("Wave {} / 10", wave.current_wave.max(1));
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
        text.0 = format!("Kills: {}", game_stats.enemies_killed);
    }
}
