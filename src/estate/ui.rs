//! Shared layout for estate menus.
//!
//! Estate menus are single-column lists that reuse the game's menu
//! navigation: every row is a `MenuItem` under one `MenuRoot`, so keyboard,
//! mouse, and controller input all behave as they do on other menus.
use super::{EstateAction, EstateScreen};
use crate::menu::{MenuAction, MenuActionComponent, MenuItem, MenuRoot, MenuType};
use crate::ui_button::button;
use bevy::prelude::*;

/// Spawn a full-screen modal menu that lives for the duration of `screen`.
pub fn spawn_menu(
    commands: &mut Commands,
    screen: EstateScreen,
    title: &str,
    content: impl FnOnce(&mut ChildSpawnerCommands),
) {
    commands
        .spawn((
            DespawnOnExit(screen),
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
                menu_type: MenuType::Estate,
            },
            MenuType::Estate,
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: Val::Px(760.0),
                        max_width: Val::Percent(94.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        padding: UiRect::all(Val::Px(24.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgb(0.7, 0.7, 0.7)),
                    BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new(title),
                        TextFont {
                            font_size: FontSize::Px(34.0),
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.8, 0.0)),
                    ));
                    content(panel);
                });
        });
}

/// A small caption above a group of rows.
pub fn heading(panel: &mut ChildSpawnerCommands, text: &str) {
    panel.spawn((
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::srgb(0.75, 0.68, 0.48)),
        Node {
            margin: UiRect::top(Val::Px(10.0)),
            ..default()
        },
    ));
}

/// Free text that a system keeps up to date, tagged with `marker`.
pub fn readout(panel: &mut ChildSpawnerCommands, marker: impl Component) {
    panel.spawn((
        marker,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(17.0),
            ..default()
        },
        TextColor(Color::srgb(0.85, 0.85, 0.88)),
    ));
}

/// One selectable row: a title on the left and a dimmer detail beside it.
/// Returns the row so callers can tag it for later label updates.
pub fn choice_row(
    panel: &mut ChildSpawnerCommands,
    title: &str,
    detail: &str,
    action: EstateAction,
    selected: bool,
) -> Entity {
    panel
        .spawn((
            button(),
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(7.0)),
                column_gap: Val::Px(16.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
            MenuItem { selected },
            MenuActionComponent {
                action: MenuAction::Estate(action),
            },
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(title),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            row.spawn((
                Text::new(detail),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ));
        })
        .id()
}
