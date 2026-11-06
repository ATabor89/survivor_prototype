//! Small shared sound controls on setup, pause, and result screens.
use crate::ui_button::{button, Pressed};
use crate::{game_audio::AudioSettings, resources::GameState};
use bevy::prelude::*;

#[derive(Component)]
struct SoundControls;
#[derive(Component)]
struct SoundLabel;
#[derive(Component)]
enum SoundAction {
    Toggle,
    Quieter,
    Louder,
}

pub struct AudioControlsPlugin;
impl Plugin for AudioControlsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_controls)
            .add_systems(Update, (sound_input, update_controls).chain());
    }
}

fn spawn_controls(mut commands: Commands) {
    commands
        .spawn((
            SoundControls,
            GlobalZIndex(110),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                bottom: Val::Px(16.0),
                padding: UiRect::all(Val::Px(8.0)),
                column_gap: Val::Px(10.0),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.08, 0.09, 0.12)),
        ))
        .with_children(|row| {
            row.spawn((
                SoundLabel,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
            ));
            for (label, action) in [
                ("Mute", SoundAction::Toggle),
                ("-", SoundAction::Quieter),
                ("+", SoundAction::Louder),
            ] {
                row.spawn((
                    button(),
                    action,
                    Node {
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(7.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.23, 0.25, 0.29)),
                ))
                .with_child((
                    Text::new(label),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                ));
            }
        });
}

fn sound_input(
    mut settings: ResMut<AudioSettings>,
    buttons: Query<&SoundAction, Added<Pressed>>,
    #[cfg(feature = "dev-tools")] debug: Res<crate::dev_tools::DebugSession>,
) {
    #[cfg(feature = "dev-tools")]
    if debug.blocks_pointer() {
        return;
    }
    for action in &buttons {
        match action {
            SoundAction::Toggle => settings.muted = !settings.muted,
            SoundAction::Quieter => settings.volume = (settings.volume - 0.1).max(0.0),
            SoundAction::Louder => settings.volume = (settings.volume + 0.1).min(1.0),
        }
    }
}

fn update_controls(
    state: Res<State<GameState>>,
    settings: Res<AudioSettings>,
    mut roots: Query<&mut Node, With<SoundControls>>,
    mut labels: Query<&mut Text, With<SoundLabel>>,
    buttons: Query<(&SoundAction, &Children)>,
    mut button_text: Query<&mut Text, Without<SoundLabel>>,
) {
    let visible = matches!(
        state.get(),
        GameState::MainMenu | GameState::Paused | GameState::Victory | GameState::GameOver
    );
    for mut node in &mut roots {
        node.display = if visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    for mut text in &mut labels {
        text.0 = format!(
            "Sound: {}{:.0}%",
            if settings.muted { "muted / " } else { "" },
            settings.volume * 100.0
        );
    }
    for (action, children) in &buttons {
        if matches!(action, SoundAction::Toggle) {
            for child in children.iter() {
                if let Ok(mut text) = button_text.get_mut(child) {
                    text.0 = if settings.muted { "Unmute" } else { "Mute" }.into();
                }
            }
        }
    }
}
