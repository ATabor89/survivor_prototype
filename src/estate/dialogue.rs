//! Line-by-line conversations with villagers.
use super::content::EstateContent;
use super::{confirm_pressed, keyboard_free, EstateScreen};
use crate::profile::Profile;
use bevy::prelude::*;

/// The conversation in progress. Present only during [`EstateScreen::Dialogue`].
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveDialogue {
    pub npc: usize,
    pub line: usize,
}

#[derive(Component)]
pub struct DialogueSpeaker;

#[derive(Component)]
pub struct DialogueLine;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(EstateScreen::Dialogue), spawn_panel)
        .add_systems(OnExit(EstateScreen::Dialogue), end_dialogue)
        .add_systems(
            Update,
            (advance.run_if(keyboard_free), show_line)
                .chain()
                .run_if(in_state(EstateScreen::Dialogue)),
        );
}

fn spawn_panel(mut commands: Commands) {
    commands
        .spawn((
            DespawnOnExit(EstateScreen::Dialogue),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                bottom: Val::Px(36.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(50),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(860.0),
                    max_width: Val::Percent(92.0),
                    min_height: Val::Px(150.0),
                    padding: UiRect::all(Val::Px(20.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BorderColor::all(Color::srgb(0.62, 0.56, 0.41)),
                BackgroundColor(Color::srgba(0.06, 0.06, 0.08, 0.94)),
            ))
            .with_children(|panel| {
                panel.spawn((
                    DialogueSpeaker,
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(20.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.94, 0.82, 0.52)),
                ));
                panel.spawn((
                    DialogueLine,
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(22.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
                panel.spawn((
                    Text::new("Enter / Space / A: continue      Esc: leave"),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.6, 0.6, 0.64)),
                ));
            });
        });
}

fn advance(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    content: Res<EstateContent>,
    profile: Res<Profile>,
    dialogue: Option<ResMut<ActiveDialogue>>,
    mut next: ResMut<NextState<EstateScreen>>,
) {
    let Some(mut dialogue) = dialogue else {
        // Nothing to say: never strand the player on an empty panel.
        next.set(EstateScreen::Roaming);
        return;
    };
    if !(keyboard.just_pressed(KeyCode::KeyE) || confirm_pressed(&keyboard, &gamepads)) {
        return;
    }
    let lines = content
        .npcs
        .get(dialogue.npc)
        .map_or(0, |npc| npc.lines_for(profile.expeditions).len());
    if dialogue.line + 1 < lines {
        dialogue.line += 1;
    } else {
        next.set(EstateScreen::Roaming);
    }
}

fn show_line(
    content: Res<EstateContent>,
    profile: Res<Profile>,
    dialogue: Option<Res<ActiveDialogue>>,
    mut speaker: Query<&mut Text, (With<DialogueSpeaker>, Without<DialogueLine>)>,
    mut line: Query<&mut Text, With<DialogueLine>>,
) {
    let Some(npc) = dialogue
        .as_ref()
        .and_then(|dialogue| content.npcs.get(dialogue.npc))
    else {
        return;
    };
    let index = dialogue.map_or(0, |dialogue| dialogue.line);
    let spoken = npc
        .lines_for(profile.expeditions)
        .get(index)
        .map_or("", String::as_str);
    for mut text in &mut speaker {
        if text.0 != npc.name {
            text.0.clone_from(&npc.name);
        }
    }
    for mut text in &mut line {
        if text.0 != spoken {
            text.0 = spoken.to_owned();
        }
    }
}

fn end_dialogue(mut commands: Commands) {
    commands.remove_resource::<ActiveDialogue>();
}
