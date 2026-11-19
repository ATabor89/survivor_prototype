//! The map room table: choose a destination and a weapon, then depart.
use super::content::EstateContent;
use super::ui::{choice_row, heading, readout, spawn_menu};
use super::{EstateAction, EstateScreen};
use crate::content::GameContent;
use crate::expedition::{HaulBonus, RunOrigin};
use crate::inventory::PassiveCatalog;
use crate::menu::{MenuAction, MenuActivation};
use crate::profile::{Profile, Stockpile};
use crate::resources::GameState;
use crate::setup::RunConfiguration;
use crate::weapons::{weapon_upgrade::WeaponUpgradeConfig, WeaponType};
use bevy::prelude::*;

/// The expedition being planned. Kept between visits to the table.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpeditionPlan {
    pub stage: usize,
    pub weapon: WeaponType,
}

impl FromWorld for ExpeditionPlan {
    fn from_world(world: &mut World) -> Self {
        Self {
            stage: 0,
            weapon: world.resource::<EstateContent>().expedition_weapons[0],
        }
    }
}

impl ExpeditionPlan {
    /// A fresh level-one run recipe for this plan with a new seed, and the
    /// haul bonus that goes with it.
    ///
    /// The estate shapes it: the weapon must be one the estate offers, and
    /// built upgrades, packed provisions, and the prepared ritual are included.
    pub fn recipe(&self, world: &World) -> Option<(RunConfiguration, Stockpile)> {
        let estate = world.resource::<EstateContent>();
        let profile = world.resource::<Profile>();
        if !estate.available_weapons(profile).contains(&self.weapon) {
            return None;
        }
        let upgrades = world.resource::<WeaponUpgradeConfig>();
        let outfit = estate.outfit(
            profile,
            self.weapon,
            world.resource::<PassiveCatalog>(),
            |weapon| upgrades.max_level(weapon),
        );
        let recipe = RunConfiguration {
            stage: world
                .resource::<GameContent>()
                .stages
                .get(self.stage)?
                .id
                .clone(),
            seed: rand::random(),
            weapons: outfit.weapons,
            passives: outfit.passives,
            modifiers: outfit.modifiers,
            ..default()
        };
        Some((recipe, outfit.haul_bonus))
    }
}

#[derive(Component)]
pub struct PlanSummary;

/// Shown on the table when an expedition could not start.
#[derive(Resource, Default)]
struct TableStatus(String);

pub fn plugin(app: &mut App) {
    app.init_resource::<ExpeditionPlan>()
        .init_resource::<TableStatus>()
        .add_systems(OnEnter(EstateScreen::ExpeditionTable), spawn_table)
        .add_systems(
            Update,
            (handle_choices, update_summary)
                .chain()
                .run_if(in_state(EstateScreen::ExpeditionTable)),
        )
        .add_systems(
            Update,
            depart_again.run_if(|state: Res<State<GameState>>| {
                matches!(state.get(), GameState::Victory | GameState::GameOver)
            }),
        );
}

fn spawn_table(
    mut commands: Commands,
    stages: Res<GameContent>,
    estate: Res<EstateContent>,
    profile: Res<Profile>,
    mut status: ResMut<TableStatus>,
) {
    status.0.clear();
    spawn_menu(
        &mut commands,
        EstateScreen::ExpeditionTable,
        "Map room",
        |panel| {
            readout(panel, PlanSummary);
            heading(panel, "DESTINATION");
            for (index, stage) in stages.stages.iter().enumerate() {
                choice_row(
                    panel,
                    &stage.name,
                    &stage.objective(&stages),
                    EstateAction::SelectStage(index),
                    false,
                );
            }
            heading(panel, "STARTING WEAPON");
            for weapon in estate.available_weapons(&profile) {
                choice_row(
                    panel,
                    &weapon.to_string(),
                    weapon.family(),
                    EstateAction::SelectWeapon(weapon),
                    false,
                );
            }
            heading(panel, "");
            choice_row(
                panel,
                "Depart",
                "What you bring back is banked here",
                EstateAction::Depart,
                true,
            );
            choice_row(panel, "Step away", "", EstateAction::Close, false);
        },
    );
}

fn handle_choices(
    mut commands: Commands,
    mut activations: MessageReader<MenuActivation>,
    mut plan: ResMut<ExpeditionPlan>,
    mut next: ResMut<NextState<EstateScreen>>,
) {
    for activation in activations.read() {
        let MenuAction::Estate(action) = &activation.action else {
            continue;
        };
        match *action {
            EstateAction::SelectStage(stage) => plan.stage = stage,
            EstateAction::SelectWeapon(weapon) => plan.weapon = weapon,
            EstateAction::Close => next.set(EstateScreen::Roaming),
            EstateAction::Depart => commands.queue(depart),
            EstateAction::Build(_) | EstateAction::Buy(_) | EstateAction::Perform(_) => {}
        }
    }
}

/// "Depart again" on an estate result screen plans a fresh expedition, so
/// provisions and rituals are never reused for free.
fn depart_again(mut commands: Commands, mut activations: MessageReader<MenuActivation>) {
    for activation in activations.read() {
        if matches!(activation.action, MenuAction::Estate(EstateAction::Depart)) {
            commands.queue(depart);
        }
    }
}

/// Start the planned expedition and use up what was prepared for it. Starting
/// a run needs the whole world to validate the recipe, so this runs as a
/// queued command.
fn depart(world: &mut World) {
    let plan = *world.resource::<ExpeditionPlan>();
    let result = plan
        .recipe(world)
        .ok_or_else(|| "That destination or weapon is not available.".to_owned())
        .and_then(|(recipe, bonus)| {
            crate::setup::start_run(world, recipe)?;
            Ok(bonus)
        });
    match result {
        Ok(bonus) => {
            *world.resource_mut::<RunOrigin>() = RunOrigin::Estate;
            *world.resource_mut::<HaulBonus>() = HaulBonus { percent: bonus };
            let mut profile = world.resource_mut::<Profile>();
            if !profile.provisions.is_empty() || profile.ritual.is_some() {
                profile.provisions.clear();
                profile.ritual = None;
            }
        }
        Err(error) => {
            warn!("Expedition could not start: {error}");
            world.resource_mut::<TableStatus>().0 = error;
            // From a result screen there is no table to show the reason on.
            if !matches!(
                world.resource::<State<GameState>>().get(),
                GameState::Estate
            ) {
                world
                    .resource_mut::<NextState<GameState>>()
                    .set(GameState::Estate);
            }
        }
    }
}

fn update_summary(
    plan: Res<ExpeditionPlan>,
    stages: Res<GameContent>,
    estate: Res<EstateContent>,
    profile: Res<Profile>,
    passives: Res<crate::inventory::PassiveCatalog>,
    status: Res<TableStatus>,
    mut summary: Query<&mut Text, With<PlanSummary>>,
) {
    let stage = stages
        .stages
        .get(plan.stage)
        .map_or("Unknown destination", |stage| stage.name.as_str());
    let mut value = format!("Destination: {stage}\nStarting weapon: {}", plan.weapon);
    let provisions: Vec<_> = estate
        .starting_passives(&profile)
        .iter()
        .map(|passive| format!("{} {}", passives.get(passive.kind).name, passive.level))
        .collect();
    if !provisions.is_empty() {
        value.push_str(&format!("\nEstate provisions: {}", provisions.join(", ")));
    }
    let packed = super::provisioner::packed_names(&estate, &profile);
    if !packed.is_empty() {
        value.push_str(&format!("\nPacked: {}", packed.join(", ")));
    }
    if let Some(ritual) = estate.prepared_ritual(&profile) {
        value.push_str(&format!(
            "\nRitual: {}. {}",
            ritual.name,
            super::altar::terms(ritual)
        ));
    }
    let bonus = estate.upgrade_haul_bonus(&profile);
    if bonus != Stockpile::default() {
        value.push_str(&format!(
            "\nEstate haul bonus: {}%",
            super::altar::percent_list(&bonus)
        ));
    }
    if !status.0.is_empty() {
        value.push('\n');
        value.push_str(&status.0);
    }
    for mut text in &mut summary {
        if text.0 != value {
            text.0.clone_from(&value);
        }
    }
}
