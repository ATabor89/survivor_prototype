use crate::components::{Health, Player};
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

pub fn spawn_ui(mut commands: Commands) {
    // Root node with marker component
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    padding: UiRect::all(Val::Px(10.0)),
                    ..default()
                },
                ..default()
            },
            GameUI,
        ))
        .with_children(|parent| {
            // Health bar container
            parent.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(200.0),
                    height: Val::Px(30.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                border_color: BorderColor(Color::rgb(0.7, 0.7, 0.7)),
                background_color: BackgroundColor(Color::rgb(0.1, 0.1, 0.1)),
                ..default()
            })
                .with_children(|parent| {
                    // The actual health bar
                    parent.spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::rgb(0.8, 0.2, 0.2)),
                            ..default()
                        },
                        HealthBar,
                    ));
                });

            // Health text
            parent.spawn((
                TextBundle::from_section(
                    "100/100",
                    TextStyle {
                        font_size: 24.0,
                        color: Color::WHITE,
                        ..default()
                    },
                )
                    .with_style(Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(220.0),
                        top: Val::Px(2.0),
                        ..default()
                    }),
                HealthText,
            ));

            // Game Timer
            parent.spawn((
                TextBundle::from_section(
                    "00:00",
                    TextStyle {
                        font_size: 32.0,
                        color: Color::WHITE,
                        ..default()
                    },
                )
                    .with_style(Style {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(50.0),
                        top: Val::Px(10.0),
                        // Center the text horizontally
                        margin: UiRect {
                            left: Val::Px(-40.0), // Approximately half the text width
                            ..default()
                        },
                        ..default()
                    }),
                GameTimer,
            ));
        });
}

pub fn cleanup_ui(
    mut commands: Commands,
    ui_query: Query<Entity, With<GameUI>>,
) {
    for entity in ui_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

pub fn update_game_timer(
    time: Res<Time<Virtual>>,
    mut timer_query: Query<&mut Text, With<GameTimer>>,
) {
    if let Ok(mut text) = timer_query.get_single_mut() {
        let total_secs = time.elapsed_seconds() as u32;
        let minutes = total_secs / 60;
        let seconds = total_secs % 60;
        text.sections[0].value = format!("{:02}:{:02}", minutes, seconds);
    }
}

pub fn update_health_ui(
    mut health_bar_query: Query<&mut Style, With<HealthBar>>,
    mut health_text_query: Query<&mut Text, With<HealthText>>,
    player_query: Query<&Health, With<Player>>,
) {
    if let Ok(player_health) = player_query.get_single() {
        // Update health bar width
        if let Ok(mut style) = health_bar_query.get_single_mut() {
            let health_percent = (player_health.current / player_health.maximum * 100.0).max(0.0);
            style.width = Val::Percent(health_percent);
        }

        // Update health text
        if let Ok(mut text) = health_text_query.get_single_mut() {
            text.sections[0].value = format!(
                "{}/{}",
                player_health.current.ceil() as i32,
                player_health.maximum as i32
            );
        }
    }
}