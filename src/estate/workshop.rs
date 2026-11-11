//! The workshop: spend what expeditions bring home on lasting improvements.
use super::content::{EstateContent, UpgradeDefinition};
use super::ui::{choice_row, heading, readout, spawn_menu};
use super::{EstateAction, EstateScreen};
use crate::menu::{MenuAction, MenuActivation, MenuItem};
use crate::profile::Profile;
use bevy::prelude::*;

/// Tags a workshop row with the index of the upgrade it builds.
#[derive(Component, Clone, Copy)]
pub struct UpgradeRow(pub usize);

#[derive(Component)]
pub struct WorkshopSummary;

/// The outcome of the latest build attempt, shown until the menu closes.
#[derive(Resource, Default)]
struct WorkshopStatus(String);

pub fn plugin(app: &mut App) {
    app.init_resource::<WorkshopStatus>()
        .add_systems(OnEnter(EstateScreen::Workshop), spawn_workshop)
        .add_systems(
            Update,
            (handle_choices, update_rows, update_summary)
                .chain()
                .run_if(in_state(EstateScreen::Workshop)),
        );
}

/// Whether `upgrade` can be built right now, or the reason it cannot.
pub fn can_build(profile: &Profile, upgrade: &UpgradeDefinition) -> Result<(), String> {
    if profile.owns(&upgrade.id) {
        return Err(format!("{} is already built.", upgrade.name));
    }
    if !profile.stockpile.covers(&upgrade.cost) {
        return Err(format!("{} needs {}.", upgrade.name, upgrade.cost));
    }
    Ok(())
}

/// Pay for `upgrade` and record it as built. A refused attempt leaves the
/// profile untouched.
pub fn build(profile: &mut Profile, upgrade: &UpgradeDefinition) -> Result<(), String> {
    can_build(profile, upgrade)?;
    profile.stockpile.spend(&upgrade.cost);
    profile.upgrades.push(upgrade.id.clone());
    Ok(())
}

fn spawn_workshop(
    mut commands: Commands,
    estate: Res<EstateContent>,
    mut status: ResMut<WorkshopStatus>,
) {
    status.0.clear();
    spawn_menu(&mut commands, EstateScreen::Workshop, "Workshop", |panel| {
        readout(panel, WorkshopSummary);
        heading(panel, "IMPROVEMENTS");
        for (index, upgrade) in estate.upgrades.iter().enumerate() {
            let row = choice_row(
                panel,
                &upgrade.name,
                "",
                EstateAction::Build(index),
                index == 0,
            );
            panel.commands().entity(row).insert(UpgradeRow(index));
        }
        heading(panel, "");
        choice_row(
            panel,
            "Step away",
            "",
            EstateAction::Close,
            estate.upgrades.is_empty(),
        );
    });
}

fn handle_choices(
    mut activations: MessageReader<MenuActivation>,
    estate: Res<EstateContent>,
    mut profile: ResMut<Profile>,
    mut status: ResMut<WorkshopStatus>,
    mut next: ResMut<NextState<EstateScreen>>,
) {
    for activation in activations.read() {
        match activation.action {
            MenuAction::Estate(EstateAction::Close) => next.set(EstateScreen::Roaming),
            MenuAction::Estate(EstateAction::Build(index)) => {
                let Some(upgrade) = estate.upgrades.get(index) else {
                    continue;
                };
                // Checking through a shared reference first means a refused
                // attempt never marks the profile changed or triggers a save.
                let attempt =
                    can_build(&profile, upgrade).and_then(|()| build(&mut profile, upgrade));
                status.0 = match attempt {
                    Ok(()) => format!("Built {}.", upgrade.name),
                    Err(reason) => reason,
                };
            }
            _ => {}
        }
    }
}

/// Each row's detail shows its price until it is built.
fn update_rows(
    estate: Res<EstateContent>,
    profile: Res<Profile>,
    rows: Query<(&UpgradeRow, &Children)>,
    mut texts: Query<&mut Text>,
) {
    for (row, children) in &rows {
        let Some(upgrade) = estate.upgrades.get(row.0) else {
            continue;
        };
        let detail = if profile.owns(&upgrade.id) {
            "Built".to_owned()
        } else {
            upgrade.cost.to_string()
        };
        // The detail is the row's second text; see `ui::choice_row`.
        if let Some(mut text) = children.get(1).and_then(|child| texts.get_mut(*child).ok()) {
            if text.0 != detail {
                text.0 = detail;
            }
        }
    }
}

fn update_summary(
    estate: Res<EstateContent>,
    profile: Res<Profile>,
    status: Res<WorkshopStatus>,
    rows: Query<(&UpgradeRow, &MenuItem)>,
    mut summary: Query<&mut Text, With<WorkshopSummary>>,
) {
    let highlighted = rows
        .iter()
        .find(|(_, item)| item.selected)
        .and_then(|(row, _)| estate.upgrades.get(row.0))
        .map_or("", |upgrade| upgrade.description.as_str());
    let value = format!(
        "In store: {}\n{highlighted}\n{}",
        profile.stockpile, status.0
    );
    for mut text in &mut summary {
        if text.0 != value {
            text.0.clone_from(&value);
        }
    }
}
