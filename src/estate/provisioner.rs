//! The provisioner: supplies bought for the next expedition only.
//!
//! Workshop upgrades last forever; provisions are packed into the profile and
//! used up when the next estate expedition departs.
use super::content::{EstateContent, ProvisionDefinition, PROVISION_SLOTS};
use super::ui::{choice_row, heading, readout, set_row_detail, set_text, spawn_menu};
use super::{EstateAction, EstateScreen};
use crate::menu::{MenuAction, MenuActivation, MenuItem};
use crate::profile::Profile;
use bevy::prelude::*;

/// Tags a shop row with the index of the provision it sells.
#[derive(Component, Clone, Copy)]
pub struct ProvisionRow(pub usize);

#[derive(Component)]
pub struct ProvisionerSummary;

/// The outcome of the latest purchase, shown until the menu closes.
#[derive(Resource, Default)]
struct ProvisionerStatus(String);

pub fn plugin(app: &mut App) {
    app.init_resource::<ProvisionerStatus>()
        .add_systems(OnEnter(EstateScreen::Provisioner), spawn_shop)
        .add_systems(
            Update,
            (handle_choices, update_rows, update_summary)
                .chain()
                .run_if(in_state(EstateScreen::Provisioner)),
        );
}

/// Whether `provision` can be packed right now, or the reason it cannot.
pub fn can_buy(profile: &Profile, provision: &ProvisionDefinition) -> Result<(), String> {
    if profile.provisions.contains(&provision.id) {
        return Err(format!("{} is already packed.", provision.name));
    }
    if profile.provisions.len() >= PROVISION_SLOTS {
        return Err(format!(
            "Only {PROVISION_SLOTS} provisions fit in one expedition's pack."
        ));
    }
    if !profile.stockpile.covers(&provision.cost) {
        return Err(format!("{} needs {}.", provision.name, provision.cost));
    }
    Ok(())
}

/// Pay for `provision` and pack it. A refused attempt leaves the profile untouched.
pub fn buy(profile: &mut Profile, provision: &ProvisionDefinition) -> Result<(), String> {
    can_buy(profile, provision)?;
    profile.stockpile.spend(&provision.cost);
    profile.provisions.push(provision.id.clone());
    Ok(())
}

/// Names of the packed provisions, for summaries.
pub fn packed_names(estate: &EstateContent, profile: &Profile) -> Vec<String> {
    profile
        .provisions
        .iter()
        .filter_map(|id| estate.provision(id))
        .map(|provision| provision.name.clone())
        .collect()
}

fn spawn_shop(
    mut commands: Commands,
    estate: Res<EstateContent>,
    mut status: ResMut<ProvisionerStatus>,
) {
    status.0.clear();
    spawn_menu(
        &mut commands,
        EstateScreen::Provisioner,
        "Provisioner",
        |panel| {
            readout(panel, ProvisionerSummary);
            heading(panel, "FOR THE NEXT EXPEDITION");
            for (index, provision) in estate.provisions.iter().enumerate() {
                let row = choice_row(
                    panel,
                    &provision.name,
                    "",
                    EstateAction::Buy(index),
                    index == 0,
                );
                panel.commands().entity(row).insert(ProvisionRow(index));
            }
            heading(panel, "");
            choice_row(
                panel,
                "Step away",
                "",
                EstateAction::Close,
                estate.provisions.is_empty(),
            );
        },
    );
}

fn handle_choices(
    mut activations: MessageReader<MenuActivation>,
    estate: Res<EstateContent>,
    mut profile: ResMut<Profile>,
    mut status: ResMut<ProvisionerStatus>,
    mut next: ResMut<NextState<EstateScreen>>,
) {
    for activation in activations.read() {
        match activation.action {
            MenuAction::Estate(EstateAction::Close) => next.set(EstateScreen::Roaming),
            MenuAction::Estate(EstateAction::Buy(index)) => {
                let Some(provision) = estate.provisions.get(index) else {
                    continue;
                };
                // A refused attempt must not mark the profile changed.
                let attempt =
                    can_buy(&profile, provision).and_then(|()| buy(&mut profile, provision));
                status.0 = match attempt {
                    Ok(()) => format!("Packed {}.", provision.name),
                    Err(reason) => reason,
                };
            }
            _ => {}
        }
    }
}

fn update_rows(
    estate: Res<EstateContent>,
    profile: Res<Profile>,
    rows: Query<(&ProvisionRow, &Children)>,
    mut texts: Query<&mut Text>,
) {
    for (row, children) in &rows {
        let Some(provision) = estate.provisions.get(row.0) else {
            continue;
        };
        let detail = if profile.provisions.contains(&provision.id) {
            "Packed".to_owned()
        } else {
            provision.cost.to_string()
        };
        set_row_detail(children, &mut texts, &detail);
    }
}

fn update_summary(
    estate: Res<EstateContent>,
    profile: Res<Profile>,
    status: Res<ProvisionerStatus>,
    rows: Query<(&ProvisionRow, &MenuItem)>,
    mut summary: Query<&mut Text, With<ProvisionerSummary>>,
) {
    let highlighted = rows
        .iter()
        .find(|(_, item)| item.selected)
        .and_then(|(row, _)| estate.provisions.get(row.0))
        .map_or("", |provision| provision.description.as_str());
    let packed = packed_names(&estate, &profile);
    let packed = if packed.is_empty() {
        "nothing".to_owned()
    } else {
        packed.join(", ")
    };
    let value = format!(
        "In store: {}\nPacked ({}/{PROVISION_SLOTS}): {packed}\n{highlighted}\n{}",
        profile.stockpile,
        profile.provisions.len(),
        status.0
    );
    for mut text in &mut summary {
        set_text(&mut text, &value);
    }
}
