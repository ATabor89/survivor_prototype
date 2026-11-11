//! Finding the nearest thing the avatar can use, and using it.
use super::dialogue::ActiveDialogue;
use super::world::EstateAvatar;
use super::{confirm_pressed, keyboard_free, EstateScreen};
use bevy::prelude::*;

/// How close the avatar must stand to use something.
pub const REACH: f32 = 72.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteractionKind {
    /// Speak with the NPC at this index in the estate content.
    Talk(usize),
    ExpeditionTable,
    Workshop,
}

#[derive(Component)]
pub struct Interactable {
    pub kind: InteractionKind,
    /// Completes "E: ..." in the on-screen prompt.
    pub prompt: String,
}

/// The interactable in reach, if any; the nearest one when several are.
#[derive(Resource, Default)]
pub struct Focus(pub Option<Entity>);

pub fn plugin(app: &mut App) {
    app.init_resource::<Focus>().add_systems(
        Update,
        (update_focus, interact.run_if(keyboard_free))
            .chain()
            .run_if(in_state(EstateScreen::Roaming)),
    );
}

fn update_focus(
    avatar: Query<&Transform, With<EstateAvatar>>,
    targets: Query<(Entity, &Transform), With<Interactable>>,
    mut focus: ResMut<Focus>,
) {
    let nearest = avatar.single().ok().and_then(|avatar| {
        let position = avatar.translation.truncate();
        targets
            .iter()
            .map(|(entity, transform)| {
                (
                    entity,
                    position.distance_squared(transform.translation.truncate()),
                )
            })
            .filter(|(_, distance)| *distance <= REACH * REACH)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(entity, _)| entity)
    });
    // Avoid marking the resource changed every frame; the prompt reacts to it.
    if focus.0 != nearest {
        focus.0 = nearest;
    }
}

fn interact(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    focus: Res<Focus>,
    interactables: Query<&Interactable>,
    mut next: ResMut<NextState<EstateScreen>>,
) {
    if !(keyboard.just_pressed(KeyCode::KeyE) || confirm_pressed(&keyboard, &gamepads)) {
        return;
    }
    let Some(target) = focus.0.and_then(|entity| interactables.get(entity).ok()) else {
        return;
    };
    match target.kind {
        InteractionKind::Talk(npc) => {
            commands.insert_resource(ActiveDialogue { npc, line: 0 });
            next.set(EstateScreen::Dialogue);
        }
        InteractionKind::ExpeditionTable => next.set(EstateScreen::ExpeditionTable),
        InteractionKind::Workshop => next.set(EstateScreen::Workshop),
    }
}
