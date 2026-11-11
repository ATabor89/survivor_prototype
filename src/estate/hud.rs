//! Estate overlay: the ledger of banked resources and the interaction prompt.
use super::interaction::{Focus, Interactable};
use super::EstateScreen;
use crate::expedition::{LastExpedition, Outcome};
use crate::profile::Profile;
use crate::resources::GameState;
use bevy::prelude::*;

#[derive(Component)]
pub struct LedgerReadout;

#[derive(Component)]
pub struct InteractionPrompt;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Estate), spawn_hud)
        .add_systems(
            Update,
            (update_ledger, update_prompt).run_if(in_state(GameState::Estate)),
        );
}

fn spawn_hud(mut commands: Commands) {
    commands
        .spawn((
            DespawnOnExit(GameState::Estate),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(18.0)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            // The overlay must never intercept clicks meant for menus.
            Pickable::IGNORE,
        ))
        .with_children(|root| {
            root.spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            })
            .with_children(|top| {
                top.spawn((
                    LedgerReadout,
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(18.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.92, 0.86, 0.66)),
                ));
                top.spawn((
                    Text::new("WASD / left stick: walk      E / A: interact      Esc: title"),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.62, 0.62, 0.66)),
                ));
            });
            root.spawn(Node {
                justify_content: JustifyContent::Center,
                padding: UiRect::bottom(Val::Px(24.0)),
                ..default()
            })
            .with_child((
                InteractionPrompt,
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(22.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// The ledger text: banked totals, history, and the most recent return.
pub fn ledger_text(profile: &Profile, last: &LastExpedition) -> String {
    let mut text = format!(
        "ESTATE LEDGER\n{}\nExpeditions: {}   Victories: {}",
        profile.stockpile, profile.expeditions, profile.victories
    );
    if let Some(report) = last.0.as_ref().filter(|report| report.banked) {
        let outcome = match report.outcome {
            Outcome::Victory => "Victory",
            Outcome::Defeat => "Defeat",
        };
        text.push_str(&format!(
            "\nLast expedition: {outcome}, brought back {}",
            report.haul
        ));
    }
    text
}

fn update_ledger(
    profile: Res<Profile>,
    last: Res<LastExpedition>,
    mut readout: Query<&mut Text, With<LedgerReadout>>,
) {
    let value = ledger_text(&profile, &last);
    for mut text in &mut readout {
        if text.0 != value {
            text.0.clone_from(&value);
        }
    }
}

fn update_prompt(
    screen: Res<State<EstateScreen>>,
    focus: Res<Focus>,
    interactables: Query<&Interactable>,
    mut prompt: Query<&mut Text, With<InteractionPrompt>>,
) {
    let value = match (
        screen.get(),
        focus.0.and_then(|e| interactables.get(e).ok()),
    ) {
        (EstateScreen::Roaming, Some(target)) => format!("E: {}", target.prompt),
        _ => String::new(),
    };
    for mut text in &mut prompt {
        if text.0 != value {
            text.0.clone_from(&value);
        }
    }
}
