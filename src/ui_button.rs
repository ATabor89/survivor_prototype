//! Shared button state for Bevy 0.20's headless UI widgets.
//!
//! Bevy 0.20 deprecated `bevy::ui::Interaction` and the old `Button` marker.
//! Buttons now use `ui_widgets::Button`, which inserts [`Pressed`] while the
//! pointer holds it down, and picking's [`Hovered`] for hover state.
//! [`ButtonInteraction`] folds both back into the three states the menus style
//! against, so the input and styling systems keep their existing behavior.
//!
//! A very fast click (press and release delivered in the same frame, as with a
//! touchpad tap) can leave `Pressed` on the button: the press registers but the
//! release does not clear it, so the button looks held and ignores the next
//! click. [`UiButtonPlugin`] clears any `Pressed` left over once no pointer is
//! held. It runs in `Last`, so input systems in `Update` still see the press.

use bevy::{picking::hover::Hovered, prelude::*};

pub use bevy::ui::Pressed;
pub use bevy::ui_widgets::Button;

pub struct UiButtonPlugin;

impl Plugin for UiButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Last, clear_stale_presses);
    }
}

fn clear_stale_presses(
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    touches: Option<Res<Touches>>,
    pressed: Query<Entity, (With<Button>, With<Pressed>)>,
    mut commands: Commands,
) {
    let mouse_held = mouse.is_some_and(|mouse| mouse.pressed(MouseButton::Left));
    let touch_held = touches.is_some_and(|touches| touches.iter().next().is_some());
    if mouse_held || touch_held {
        return;
    }
    for entity in &pressed {
        commands.entity(entity).remove::<Pressed>();
    }
}

/// Components every clickable UI button spawns with.
pub fn button() -> impl Bundle {
    (Button, Hovered::default())
}

/// A button's visible state: pressed wins over hovered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonInteraction {
    Pressed,
    Hovered,
    None,
}

impl ButtonInteraction {
    pub fn new(pressed: bool, hovered: Option<&Hovered>) -> Self {
        if pressed {
            Self::Pressed
        } else if hovered.is_some_and(Hovered::get) {
            Self::Hovered
        } else {
            Self::None
        }
    }
}

/// Query data for [`ButtonInteraction::new`].
pub type ButtonStateQuery = (Has<Pressed>, Option<&'static Hovered>);

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<ButtonInput<MouseButton>>()
            .add_plugins(UiButtonPlugin);
        app
    }

    #[test]
    fn press_left_after_a_released_click_is_cleared() {
        let mut app = app();
        let button = app.world_mut().spawn((button(), Pressed)).id();
        app.update();
        assert!(app.world().get::<Pressed>(button).is_none());
    }

    #[test]
    fn press_is_kept_while_the_mouse_is_held() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        let button = app.world_mut().spawn((button(), Pressed)).id();
        app.update();
        assert!(app.world().get::<Pressed>(button).is_some());
    }
}
