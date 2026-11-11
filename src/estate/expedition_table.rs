//! The map room table: choose a destination and a weapon, then depart.
use super::content::EstateContent;
use super::ui::{choice_row, heading, readout, spawn_menu};
use super::{EstateAction, EstateScreen};
use crate::content::GameContent;
use crate::expedition::RunOrigin;
use crate::menu::{MenuAction, MenuActivation};
use crate::setup::{RunConfiguration, WeaponLoadout};
use crate::weapons::WeaponType;
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
    /// A fresh level-one run recipe for this plan with a new seed.
    pub fn recipe(&self, content: &GameContent) -> Option<RunConfiguration> {
        Some(RunConfiguration {
            stage: content.stages.get(self.stage)?.id.clone(),
            seed: rand::random(),
            weapons: vec![WeaponLoadout {
                weapon: self.weapon,
                level: 1,
                patterns: vec![],
            }],
            passives: vec![],
            ..default()
        })
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
        );
}

fn spawn_table(
    mut commands: Commands,
    stages: Res<GameContent>,
    estate: Res<EstateContent>,
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
            for weapon in &estate.expedition_weapons {
                choice_row(
                    panel,
                    &weapon.to_string(),
                    weapon.family(),
                    EstateAction::SelectWeapon(*weapon),
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
        }
    }
}

/// Start the planned expedition. Starting a run needs the whole world to
/// validate the recipe, so this runs as a queued command.
fn depart(world: &mut World) {
    let plan = *world.resource::<ExpeditionPlan>();
    let result = plan
        .recipe(world.resource::<GameContent>())
        .ok_or_else(|| "That destination no longer exists.".to_owned())
        .and_then(|recipe| crate::setup::start_run(world, recipe));
    match result {
        Ok(()) => *world.resource_mut::<RunOrigin>() = RunOrigin::Estate,
        Err(error) => {
            warn!("Expedition could not start: {error}");
            world.resource_mut::<TableStatus>().0 = error;
        }
    }
}

fn update_summary(
    plan: Res<ExpeditionPlan>,
    stages: Res<GameContent>,
    status: Res<TableStatus>,
    mut summary: Query<&mut Text, With<PlanSummary>>,
) {
    let stage = stages
        .stages
        .get(plan.stage)
        .map_or("Unknown destination", |stage| stage.name.as_str());
    let mut value = format!("Destination: {stage}\nStarting weapon: {}", plan.weapon);
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
