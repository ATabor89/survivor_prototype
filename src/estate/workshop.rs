//! The workshop: spend what expeditions bring home on lasting improvements.
use super::content::{EstateContent, UpgradeDefinition};
use super::ui::{choice_row, heading, readout, set_row_detail, set_text, spawn_menu};
use super::{EstateAction, EstateScreen};
use crate::menu::{MenuAction, MenuActivation, MenuItem};
use crate::profile::{Profile, Stockpile};
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

/// The price of the next rank of `upgrade` if it can be built right now, or
/// the reason it cannot.
pub fn can_build<'a>(
    estate: &EstateContent,
    profile: &Profile,
    upgrade: &'a UpgradeDefinition,
) -> Result<&'a Stockpile, String> {
    let Some(cost) = upgrade.next_cost(profile.rank(&upgrade.id)) else {
        return Err(format!("{} is already built.", upgrade.name));
    };
    let missing = estate.missing_requirements(&upgrade.requires, profile);
    if !missing.is_empty() {
        return Err(format!(
            "{} requires {} first.",
            upgrade.name,
            missing.join(" and ")
        ));
    }
    if !profile.stockpile.covers(cost) {
        return Err(format!("{} needs {cost}.", upgrade.name));
    }
    Ok(cost)
}

/// Pay for the next rank of `upgrade` and record it, returning the new rank.
/// A refused attempt leaves the profile untouched.
pub fn build(
    estate: &EstateContent,
    profile: &mut Profile,
    upgrade: &UpgradeDefinition,
) -> Result<u32, String> {
    let cost = *can_build(estate, profile, upgrade)?;
    profile.stockpile.spend(&cost);
    let rank = profile.upgrades.entry(upgrade.id.clone()).or_default();
    *rank += 1;
    Ok(*rank)
}

/// The detail beside an upgrade's row: its next price, or why it cannot be bought.
pub fn row_detail(
    estate: &EstateContent,
    profile: &Profile,
    upgrade: &UpgradeDefinition,
) -> String {
    let built = profile.rank(&upgrade.id);
    let ranked = upgrade.max_rank() > 1;
    let Some(cost) = upgrade.next_cost(built) else {
        return if ranked {
            format!("Rank {built}/{}", upgrade.max_rank())
        } else {
            "Built".to_owned()
        };
    };
    let missing = estate.missing_requirements(&upgrade.requires, profile);
    if !missing.is_empty() {
        return format!("Requires {}", missing.join(", "));
    }
    if ranked {
        format!("Rank {}/{}: {cost}", built + 1, upgrade.max_rank())
    } else {
        cost.to_string()
    }
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
                let attempt = can_build(&estate, &profile, upgrade)
                    .map(|_| ())
                    .and_then(|()| build(&estate, &mut profile, upgrade));
                status.0 = match attempt {
                    Ok(rank) if upgrade.max_rank() > 1 => {
                        format!("Built {} rank {rank}.", upgrade.name)
                    }
                    Ok(_) => format!("Built {}.", upgrade.name),
                    Err(reason) => reason,
                };
            }
            _ => {}
        }
    }
}

/// Each row's detail shows its next price, or why it cannot be built.
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
        set_row_detail(
            children,
            &mut texts,
            &row_detail(&estate, &profile, upgrade),
        );
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
        set_text(&mut text, &value);
    }
}
