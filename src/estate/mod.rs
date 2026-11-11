//! The estate: the peaceful hub between expeditions.
//!
//! The player walks a small walled map, talks to villagers, and uses the map
//! room to launch an expedition. Finished expeditions return here with their
//! haul banked in the [`crate::profile::Profile`].
//!
//! Everything spawned here is scoped to [`GameState::Estate`] or one of the
//! [`EstateScreen`] sub-states, so leaving cleans up without bespoke teardown.
mod content;
mod dialogue;
mod expedition_table;
mod hud;
mod interaction;
mod ui;
mod world;

#[cfg(test)]
mod tests;

pub use content::EstateContent;

use crate::resources::GameState;
use crate::weapons::WeaponType;
use bevy::prelude::*;

/// What the player is doing inside the estate.
#[derive(SubStates, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[source(GameState = GameState::Estate)]
pub enum EstateScreen {
    /// Walking around; the only screen in which the avatar moves.
    #[default]
    Roaming,
    Dialogue,
    ExpeditionTable,
}

/// Choices offered by estate menus, carried by `MenuAction::Estate`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EstateAction {
    SelectStage(usize),
    SelectWeapon(WeaponType),
    Depart,
    Close,
}

pub struct EstatePlugin;

impl Plugin for EstatePlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<EstateScreen>()
            .init_resource::<EstateContent>()
            .add_plugins((
                world::plugin,
                interaction::plugin,
                dialogue::plugin,
                expedition_table::plugin,
                hud::plugin,
            ))
            .add_systems(
                Update,
                step_back
                    .run_if(in_state(GameState::Estate))
                    .run_if(keyboard_free),
            );
    }
}

/// Escape (or Start) closes the open screen, or leaves a quiet estate for the title.
fn step_back(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    screen: Res<State<EstateScreen>>,
    mut next_screen: ResMut<NextState<EstateScreen>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let pressed = keyboard.just_pressed(KeyCode::Escape)
        || gamepads
            .iter()
            .any(|pad| pad.just_pressed(GamepadButton::Start));
    if !pressed {
        return;
    }
    match screen.get() {
        EstateScreen::Roaming => next_state.set(GameState::MainMenu),
        EstateScreen::Dialogue | EstateScreen::ExpeditionTable => {
            next_screen.set(EstateScreen::Roaming);
        }
    }
}

/// True when the confirm input was pressed this frame.
fn confirm_pressed(keyboard: &ButtonInput<KeyCode>, gamepads: &Query<&Gamepad>) -> bool {
    keyboard.just_pressed(KeyCode::Enter)
        || keyboard.just_pressed(KeyCode::Space)
        || gamepads
            .iter()
            .any(|pad| pad.just_pressed(GamepadButton::South))
}

/// Run condition: the debug overlay is not capturing this frame's input.
#[cfg(feature = "dev-tools")]
fn keyboard_free(debug: Res<crate::dev_tools::DebugSession>) -> bool {
    !debug.open && !debug.consumed_pause_input
}

/// Run condition: nothing else captures input in builds without debug tools.
#[cfg(not(feature = "dev-tools"))]
fn keyboard_free() -> bool {
    true
}
