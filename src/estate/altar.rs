//! The chapel altar: rites that make the next expedition harder and richer.
//!
//! A ritual is paid for when it is performed and stays prepared until the
//! next estate expedition departs, which uses it up. Only one can be prepared
//! at a time.
use super::content::{EstateContent, RitualDefinition};
use super::ui::{choice_row, heading, readout, set_row_detail, set_text, spawn_menu};
use super::{EstateAction, EstateScreen};
use crate::menu::{MenuAction, MenuActivation, MenuItem};
use crate::profile::{Profile, Stockpile};
use bevy::prelude::*;

/// Tags an altar row with the index of the ritual it performs.
#[derive(Component, Clone, Copy)]
pub struct RitualRow(pub usize);

#[derive(Component)]
pub struct AltarSummary;

/// The outcome of the latest attempt, shown until the menu closes.
#[derive(Resource, Default)]
struct AltarStatus(String);

pub fn plugin(app: &mut App) {
    app.init_resource::<AltarStatus>()
        .add_systems(OnEnter(EstateScreen::Altar), spawn_altar)
        .add_systems(
            Update,
            (handle_choices, update_rows, update_summary)
                .chain()
                .run_if(in_state(EstateScreen::Altar)),
        );
}

/// Whether `ritual` can be performed right now, or the reason it cannot.
pub fn can_perform(
    estate: &EstateContent,
    profile: &Profile,
    ritual: &RitualDefinition,
) -> Result<(), String> {
    if let Some(prepared) = estate.prepared_ritual(profile) {
        return Err(if prepared.id == ritual.id {
            format!("{} is already prepared.", ritual.name)
        } else {
            format!(
                "{} is already prepared. Undertake an expedition first.",
                prepared.name
            )
        });
    }
    let missing = estate.missing_requirements(&ritual.requires, profile);
    if !missing.is_empty() {
        return Err(format!(
            "{} requires {} first.",
            ritual.name,
            missing.join(" and ")
        ));
    }
    if !profile.stockpile.covers(&ritual.cost) {
        return Err(format!("{} needs {}.", ritual.name, ritual.cost));
    }
    Ok(())
}

/// Pay for `ritual` and prepare it. A refused attempt leaves the profile untouched.
pub fn perform(
    estate: &EstateContent,
    profile: &mut Profile,
    ritual: &RitualDefinition,
) -> Result<(), String> {
    can_perform(estate, profile, ritual)?;
    profile.stockpile.spend(&ritual.cost);
    profile.ritual = Some(ritual.id.clone());
    Ok(())
}

/// What a ritual does to the run, e.g. `Enemies have 150% health. Haul +50% Essence.`
pub fn terms(ritual: &RitualDefinition) -> String {
    let mut parts = Vec::new();
    let modifiers = ritual.modifiers;
    if modifiers.enemy_health_percent != 100 {
        parts.push(format!(
            "Enemies have {}% health.",
            modifiers.enemy_health_percent
        ));
    }
    if modifiers.damage_taken_percent != 100 {
        parts.push(format!(
            "You take {}% damage.",
            modifiers.damage_taken_percent
        ));
    }
    if ritual.haul_bonus != Stockpile::default() {
        parts.push(format!(
            "Haul bonus: {}%.",
            percent_list(&ritual.haul_bonus)
        ));
    }
    parts.join(" ")
}

/// `Essence +50, Seals +25` style list of a percentage stockpile.
pub fn percent_list(percent: &Stockpile) -> String {
    percent
        .to_string()
        .split(", ")
        .map(|part| part.replacen(' ', " +", 1))
        .collect::<Vec<_>>()
        .join("%, ")
}

fn spawn_altar(
    mut commands: Commands,
    estate: Res<EstateContent>,
    mut status: ResMut<AltarStatus>,
) {
    status.0.clear();
    spawn_menu(&mut commands, EstateScreen::Altar, "Altar", |panel| {
        readout(panel, AltarSummary);
        heading(panel, "RITES FOR THE NEXT EXPEDITION");
        for (index, ritual) in estate.rituals.iter().enumerate() {
            let row = choice_row(
                panel,
                &ritual.name,
                "",
                EstateAction::Perform(index),
                index == 0,
            );
            panel.commands().entity(row).insert(RitualRow(index));
        }
        heading(panel, "");
        choice_row(
            panel,
            "Step away",
            "",
            EstateAction::Close,
            estate.rituals.is_empty(),
        );
    });
}

fn handle_choices(
    mut activations: MessageReader<MenuActivation>,
    estate: Res<EstateContent>,
    mut profile: ResMut<Profile>,
    mut status: ResMut<AltarStatus>,
    mut next: ResMut<NextState<EstateScreen>>,
) {
    for activation in activations.read() {
        match activation.action {
            MenuAction::Estate(EstateAction::Close) => next.set(EstateScreen::Roaming),
            MenuAction::Estate(EstateAction::Perform(index)) => {
                let Some(ritual) = estate.rituals.get(index) else {
                    continue;
                };
                // A refused attempt must not mark the profile changed.
                let attempt = can_perform(&estate, &profile, ritual)
                    .and_then(|()| perform(&estate, &mut profile, ritual));
                status.0 = match attempt {
                    Ok(()) => format!("{} is prepared for the next expedition.", ritual.name),
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
    rows: Query<(&RitualRow, &Children)>,
    mut texts: Query<&mut Text>,
) {
    for (row, children) in &rows {
        let Some(ritual) = estate.rituals.get(row.0) else {
            continue;
        };
        let missing = estate.missing_requirements(&ritual.requires, &profile);
        let detail = if profile.ritual.as_ref() == Some(&ritual.id) {
            "Prepared".to_owned()
        } else if !missing.is_empty() {
            format!("Requires {}", missing.join(", "))
        } else {
            ritual.cost.to_string()
        };
        set_row_detail(children, &mut texts, &detail);
    }
}

fn update_summary(
    estate: Res<EstateContent>,
    profile: Res<Profile>,
    status: Res<AltarStatus>,
    rows: Query<(&RitualRow, &MenuItem)>,
    mut summary: Query<&mut Text, With<AltarSummary>>,
) {
    let highlighted = rows
        .iter()
        .find(|(_, item)| item.selected)
        .and_then(|(row, _)| estate.rituals.get(row.0))
        .map_or(String::new(), |ritual| {
            format!("{}\n{}", ritual.description, terms(ritual))
        });
    let prepared = estate
        .prepared_ritual(&profile)
        .map_or("none", |ritual| ritual.name.as_str());
    let value = format!(
        "In store: {}\nPrepared: {prepared}\n{highlighted}\n{}",
        profile.stockpile, status.0
    );
    for mut text in &mut summary {
        set_text(&mut text, &value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terms_list_only_what_changes() {
        let ritual: RitualDefinition = ron::from_str(
            "(id: \"r\", name: \"R\", description: \"D\", cost: (seals: 1), \
             modifiers: (enemy_health_percent: 150), haul_bonus: (essence: 50, seals: 25))",
        )
        .unwrap();
        assert_eq!(
            terms(&ritual),
            "Enemies have 150% health. Haul bonus: Essence +50%, Seals +25%."
        );
    }
}
