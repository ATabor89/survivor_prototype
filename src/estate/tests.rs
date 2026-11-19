use super::altar::AltarSummary;
use super::content::EstateEffect;
use super::content::{EstateContent, EMBEDDED_ESTATE};
use super::dialogue::{ActiveDialogue, DialogueLine};
use super::expedition_table::ExpeditionPlan;
use super::hud::{InteractionPrompt, LedgerReadout};
use super::interaction::{Focus, Interactable, InteractionKind};
use super::provisioner::ProvisionerSummary;
use super::workshop::{build, UpgradeRow, WorkshopSummary};
use super::world::{EstateAvatar, EstateRoot, Npc, AVATAR_RADIUS, AVATAR_SPEED};
use super::EstateScreen;
use crate::arena::ArenaRoot;
use crate::components::Player;
use crate::expedition::{HaulBonus, LastExpedition, RunOrigin};
use crate::menu::{MenuItem, MenuRoot};
use crate::profile::{Profile, Stockpile};
use crate::resources::{GameState, GameStats};
use crate::setup::ActiveRunConfig;
use crate::tests::{count, game, key, state, step};
use crate::weapons::{WeaponMeta, WeaponType};
use bevy::prelude::*;

fn estate() -> App {
    let mut app = game();
    enter_estate(&mut app);
    app
}

fn enter_estate(app: &mut App) {
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Estate);
    step(app, 3);
    assert_eq!(state(app), GameState::Estate);
}

fn screen(app: &App) -> EstateScreen {
    *app.world().resource::<State<EstateScreen>>().get()
}

fn avatar(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<EstateAvatar>>()
        .single(app.world())
        .unwrap()
}

fn avatar_position(app: &mut App) -> Vec2 {
    let avatar = avatar(app);
    app.world()
        .get::<Transform>(avatar)
        .unwrap()
        .translation
        .truncate()
}

fn place_avatar(app: &mut App, position: Vec2) {
    let avatar = avatar(app);
    app.world_mut()
        .get_mut::<Transform>(avatar)
        .unwrap()
        .translation = position.extend(0.0);
    step(app, 2);
}

fn hold(app: &mut App, key: KeyCode, frames: usize) {
    for _ in 0..frames {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        step(app, 1);
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(key);
}

fn position_of(app: &mut App, kind: InteractionKind) -> Vec2 {
    app.world_mut()
        .query::<(&Interactable, &Transform)>()
        .iter(app.world())
        .find(|(interactable, _)| interactable.kind == kind)
        .map(|(_, transform)| transform.translation.truncate())
        .unwrap()
}

fn text_of<T: Component>(app: &mut App) -> String {
    app.world_mut()
        .query_filtered::<&Text, With<T>>()
        .single(app.world())
        .unwrap()
        .0
        .clone()
}

fn selected_label(app: &mut App) -> String {
    let (_, children) = app
        .world_mut()
        .query::<(&MenuItem, &Children)>()
        .iter(app.world())
        .find(|(item, _)| item.selected)
        .unwrap();
    let title = children[0];
    app.world().get::<Text>(title).unwrap().0.clone()
}

fn open_table(app: &mut App) {
    let table = position_of(app, InteractionKind::ExpeditionTable);
    place_avatar(app, table + Vec2::new(0.0, -40.0));
    key(app, KeyCode::KeyE);
    step(app, 2);
    assert_eq!(screen(app), EstateScreen::ExpeditionTable);
}

fn open_workshop(app: &mut App) {
    open_station(app, InteractionKind::Workshop, Vec2::new(0.0, -40.0));
    assert_eq!(screen(app), EstateScreen::Workshop);
}

fn open_station(app: &mut App, kind: InteractionKind, offset: Vec2) {
    let station = position_of(app, kind);
    place_avatar(app, station + offset);
    key(app, KeyCode::KeyE);
    step(app, 2);
}

fn upgrade_named(app: &App, id: &str) -> super::content::UpgradeDefinition {
    app.world()
        .resource::<EstateContent>()
        .upgrades
        .iter()
        .find(|upgrade| upgrade.id == id)
        .cloned()
        .unwrap()
}

fn grant(app: &mut App, upgrade: &str, rank: u32) {
    app.world_mut()
        .resource_mut::<Profile>()
        .upgrades
        .insert(upgrade.to_owned(), rank);
}

/// Leave the table with Depart selected and start the planned expedition.
fn depart_from_table(app: &mut App) {
    open_table(app);
    assert_eq!(selected_label(app), "Depart");
    key(app, KeyCode::Enter);
    step(app, 4);
    assert_eq!(state(app), GameState::Playing);
}

/// The index and definition of the first upgrade with a matching effect.
fn upgrade_where(
    app: &App,
    matches: impl Fn(EstateEffect) -> bool,
) -> (usize, super::content::UpgradeDefinition) {
    app.world()
        .resource::<EstateContent>()
        .upgrades
        .iter()
        .cloned()
        .enumerate()
        .find(|(_, upgrade)| matches(upgrade.effect))
        .expect("the bundled estate has an upgrade of every kind")
}

fn row_detail(app: &mut App, index: usize) -> String {
    let (_, children) = app
        .world_mut()
        .query::<(&UpgradeRow, &Children)>()
        .iter(app.world())
        .find(|(row, _)| row.0 == index)
        .unwrap();
    let detail = children[1];
    app.world().get::<Text>(detail).unwrap().0.clone()
}

fn kill_player(app: &mut App) {
    let player = crate::tests::player(app);
    app.world_mut()
        .get_mut::<crate::combat::DamageCooldown>(player)
        .unwrap()
        .time = -1.0;
    app.world_mut().write_message(crate::combat::DamageEvent {
        target: player,
        amount: 10_000,
        source: None,
    });
    step(app, 3);
    assert_eq!(state(app), GameState::GameOver);
}

#[test]
fn bundled_estate_is_valid_and_offers_real_weapons() {
    let content = EstateContent::parse(EMBEDDED_ESTATE).unwrap();
    assert!(!content.npcs.is_empty());
    assert!(content
        .expedition_weapons
        .contains(&WeaponType::MagickCircle));
}

#[test]
fn invalid_estates_are_rejected_with_a_reason() {
    let valid = |body: &str| {
        format!(
            "(half_size: (400.0, 300.0), spawn: (0.0, 0.0), expedition_weapons: [magick_circle], \
             stations: [(kind: ExpeditionTable, name: \"Table\", position: (0.0, 100.0))], {body})"
        )
    };
    EstateContent::parse(&valid("")).unwrap();
    for (body, reason) in [
        (
            "buildings: [(name: \"Hall\", center: (0.0, 0.0), half_size: (50.0, 50.0), color: (0.2, 0.2, 0.2))]",
            "spawn is inside",
        ),
        (
            "buildings: [(name: \"Hall\", center: (380.0, 0.0), half_size: (50.0, 50.0), color: (0.2, 0.2, 0.2))]",
            "inside the estate",
        ),
        (
            "npcs: [(id: \"a\", name: \"A\", position: (0.0, -100.0), color: (1.0, 1.0, 1.0), lines: [])]",
            "at least one line",
        ),
        (
            "npcs: [(id: \"a\", name: \"A\", position: (900.0, 0.0), color: (1.0, 1.0, 1.0), lines: [\"Hi\"])]",
            "inside the estate walls",
        ),
        (
            "npcs: [(id: \"a\", name: \"A\", position: (0.0, -100.0), color: (1.0, 1.0, 1.0), lines: [\"Hi\"]), \
             (id: \"a\", name: \"B\", position: (50.0, -100.0), color: (1.0, 1.0, 1.0), lines: [\"Hi\"])]",
            "unique",
        ),
    ] {
        let error = EstateContent::parse(&valid(body)).unwrap_err();
        assert!(error.contains(reason), "{body}: {error}");
    }
    let no_table = "(half_size: (400.0, 300.0), spawn: (0.0, 0.0), \
                    expedition_weapons: [magick_circle], stations: [])";
    assert!(EstateContent::parse(no_table)
        .unwrap_err()
        .contains("ExpeditionTable"));
}

#[test]
fn entering_the_estate_replaces_the_run_and_leaving_cleans_up() {
    let mut app = estate();
    let npcs = app.world().resource::<EstateContent>().npcs.len();
    assert_eq!(count::<EstateAvatar>(&mut app), 1);
    assert_eq!(count::<EstateRoot>(&mut app), 1);
    assert_eq!(count::<Npc>(&mut app), npcs);
    assert_eq!(count::<Player>(&mut app), 0);
    assert_eq!(count::<ArenaRoot>(&mut app), 0);
    assert_eq!(screen(&app), EstateScreen::Roaming);

    key(&mut app, KeyCode::Escape);
    step(&mut app, 2);
    assert_eq!(state(&app), GameState::MainMenu);
    assert_eq!(count::<EstateAvatar>(&mut app), 0);
    assert_eq!(count::<EstateRoot>(&mut app), 0);
    assert_eq!(count::<Interactable>(&mut app), 0);
    assert_eq!(count::<LedgerReadout>(&mut app), 0);
}

#[test]
fn avatar_walks_at_its_speed_and_is_stopped_by_walls_and_buildings() {
    let mut app = estate();
    place_avatar(&mut app, Vec2::new(0.0, -100.0));
    let start = avatar_position(&mut app);
    hold(&mut app, KeyCode::KeyD, 30);
    let moved = avatar_position(&mut app);
    assert!(
        (moved.x - start.x - AVATAR_SPEED * 0.5).abs() < 2.0,
        "{start} -> {moved}"
    );
    assert!((moved.y - start.y).abs() < 0.01);

    // The east wall holds however long the key is held.
    let half = Vec2::from_array(app.world().resource::<EstateContent>().half_size);
    place_avatar(&mut app, Vec2::new(half.x - 40.0, -100.0));
    hold(&mut app, KeyCode::KeyD, 60);
    assert!(avatar_position(&mut app).x <= half.x - AVATAR_RADIUS + 0.5);

    // Walking north from the spawn point runs into the manor, not through it.
    let manor = app.world().resource::<EstateContent>().buildings[0].clone();
    let south_face = manor.center[1] - manor.half_size[1];
    place_avatar(
        &mut app,
        Vec2::new(manor.center[0] + 120.0, south_face - 60.0),
    );
    hold(&mut app, KeyCode::KeyW, 90);
    assert!(avatar_position(&mut app).y <= south_face - AVATAR_RADIUS + 0.5);
}

#[test]
fn the_nearest_interactable_in_reach_is_prompted_and_only_while_roaming() {
    let mut app = estate();
    place_avatar(&mut app, Vec2::new(0.0, -100.0));
    assert!(app.world().resource::<Focus>().0.is_none());
    assert_eq!(text_of::<InteractionPrompt>(&mut app), "");

    let npc = position_of(&mut app, InteractionKind::Talk(0));
    place_avatar(&mut app, npc + Vec2::new(30.0, 0.0));
    let name = app.world().resource::<EstateContent>().npcs[0].name.clone();
    assert_eq!(
        text_of::<InteractionPrompt>(&mut app),
        format!("E: Talk to {name}")
    );

    key(&mut app, KeyCode::KeyE);
    step(&mut app, 2);
    assert_eq!(screen(&app), EstateScreen::Dialogue);
    assert_eq!(text_of::<InteractionPrompt>(&mut app), "");
    // Movement keys do nothing during a conversation.
    let before = avatar_position(&mut app);
    hold(&mut app, KeyCode::KeyD, 10);
    assert_eq!(avatar_position(&mut app), before);
}

#[test]
fn dialogue_advances_line_by_line_then_returns_to_roaming() {
    let mut app = estate();
    let npc = position_of(&mut app, InteractionKind::Talk(0));
    place_avatar(&mut app, npc + Vec2::new(0.0, -30.0));
    key(&mut app, KeyCode::Enter);
    step(&mut app, 2);

    let lines = app.world().resource::<EstateContent>().npcs[0]
        .lines
        .clone();
    assert!(lines.len() >= 2, "the test needs a multi-line villager");
    for (index, line) in lines.iter().enumerate() {
        assert_eq!(screen(&app), EstateScreen::Dialogue);
        assert_eq!(
            *app.world().resource::<ActiveDialogue>(),
            ActiveDialogue {
                npc: 0,
                line: index
            }
        );
        assert_eq!(&text_of::<DialogueLine>(&mut app), line);
        key(&mut app, KeyCode::Space);
        step(&mut app, 2);
    }
    assert_eq!(screen(&app), EstateScreen::Roaming);
    assert!(app.world().get_resource::<ActiveDialogue>().is_none());
    assert_eq!(count::<DialogueLine>(&mut app), 0);

    // Escape leaves a conversation without leaving the estate.
    key(&mut app, KeyCode::KeyE);
    step(&mut app, 2);
    assert_eq!(screen(&app), EstateScreen::Dialogue);
    key(&mut app, KeyCode::Escape);
    step(&mut app, 2);
    assert_eq!(screen(&app), EstateScreen::Roaming);
    assert_eq!(state(&app), GameState::Estate);
}

#[test]
fn villagers_speak_differently_after_an_expedition_has_returned() {
    let mut app = estate();
    app.world_mut().resource_mut::<Profile>().expeditions = 1;
    let npc = position_of(&mut app, InteractionKind::Talk(0));
    place_avatar(&mut app, npc + Vec2::new(0.0, -30.0));
    key(&mut app, KeyCode::KeyE);
    step(&mut app, 2);
    let definition = app.world().resource::<EstateContent>().npcs[0].clone();
    assert_ne!(definition.lines, definition.returning_lines);
    assert_eq!(
        text_of::<DialogueLine>(&mut app),
        definition.returning_lines[0]
    );
}

#[test]
fn the_table_plans_a_stage_and_weapon_and_departs_as_an_estate_run() {
    let mut app = estate();
    open_table(&mut app);
    assert_eq!(count::<MenuRoot>(&mut app), 1);
    assert_eq!(selected_label(&mut app), "Depart");

    // Walk the selection up from Depart: the last weapon, then the others,
    // then the stages. Choose the last weapon and the last stage.
    let weapons = app
        .world()
        .resource::<EstateContent>()
        .expedition_weapons
        .clone();
    let stages = app
        .world()
        .resource::<crate::content::GameContent>()
        .stages
        .clone();
    key(&mut app, KeyCode::ArrowUp);
    key(&mut app, KeyCode::Enter);
    for _ in 0..weapons.len() {
        key(&mut app, KeyCode::ArrowUp);
    }
    assert_eq!(selected_label(&mut app), stages.last().unwrap().name);
    key(&mut app, KeyCode::Enter);
    step(&mut app, 1);
    let plan = *app.world().resource::<ExpeditionPlan>();
    assert_eq!(plan.weapon, *weapons.last().unwrap());
    assert_eq!(plan.stage, stages.len() - 1);
    assert_eq!(screen(&app), EstateScreen::ExpeditionTable);

    // Return to Depart and go.
    for _ in 0..=weapons.len() {
        key(&mut app, KeyCode::ArrowDown);
    }
    assert_eq!(selected_label(&mut app), "Depart");
    key(&mut app, KeyCode::Enter);
    step(&mut app, 4);

    assert_eq!(state(&app), GameState::Playing);
    assert_eq!(*app.world().resource::<RunOrigin>(), RunOrigin::Estate);
    let config = app.world().resource::<ActiveRunConfig>().0.clone();
    assert_eq!(config.stage, stages.last().unwrap().id);
    assert_eq!(config.weapons.len(), 1);
    assert_eq!(config.weapons[0].weapon, plan.weapon);
    let owned: Vec<_> = app
        .world_mut()
        .query::<&WeaponMeta>()
        .iter(app.world())
        .map(|meta| meta.weapon_type)
        .collect();
    assert_eq!(owned, vec![plan.weapon]);
    // The estate is gone and the stage is back.
    assert_eq!(count::<EstateAvatar>(&mut app), 0);
    assert_eq!(count::<MenuRoot>(&mut app), 0);
    assert_eq!(count::<ArenaRoot>(&mut app), 1);
    assert_eq!(count::<Player>(&mut app), 1);
}

#[test]
fn stepping_away_from_the_table_keeps_the_plan_and_the_estate() {
    let mut app = estate();
    open_table(&mut app);
    key(&mut app, KeyCode::ArrowDown); // Depart -> Step away
    assert_eq!(selected_label(&mut app), "Step away");
    key(&mut app, KeyCode::Enter);
    step(&mut app, 2);
    assert_eq!(screen(&app), EstateScreen::Roaming);
    assert_eq!(count::<MenuRoot>(&mut app), 0);
    assert_eq!(count::<EstateAvatar>(&mut app), 1);
}

#[test]
fn an_expedition_returns_to_the_estate_with_its_haul_banked() {
    let mut app = estate();
    open_table(&mut app);
    key(&mut app, KeyCode::Enter);
    step(&mut app, 4);
    assert_eq!(state(&app), GameState::Playing);

    {
        let mut stats = app.world_mut().resource_mut::<GameStats>();
        stats.enemies_killed = 50;
        stats.gold = 4;
    }
    kill_player(&mut app);
    assert_eq!(selected_label(&mut app), "Return to estate");
    key(&mut app, KeyCode::Enter);
    step(&mut app, 4);

    assert_eq!(state(&app), GameState::Estate);
    assert_eq!(screen(&app), EstateScreen::Roaming);
    assert_eq!(count::<EstateAvatar>(&mut app), 1);
    assert_eq!(count::<Player>(&mut app), 0);
    assert_eq!(count::<crate::components::Enemy>(&mut app), 0);
    let expected = Stockpile {
        gold: 4,
        essence: 5,
        seals: 0,
    };
    let profile = app.world().resource::<Profile>().clone();
    assert_eq!(profile.stockpile, expected);
    assert_eq!(profile.expeditions, 1);
    let ledger = text_of::<LedgerReadout>(&mut app);
    assert!(ledger.contains("Gold 4, Essence 5"), "{ledger}");
    assert!(ledger.contains("Last expedition: Defeat"), "{ledger}");
    // The run's totals do not leak into the next one.
    assert_eq!(app.world().resource::<GameStats>().enemies_killed, 0);
}

#[test]
fn abandoning_an_estate_expedition_returns_home_with_nothing() {
    let mut app = estate();
    open_table(&mut app);
    key(&mut app, KeyCode::Enter);
    step(&mut app, 4);
    app.world_mut().resource_mut::<GameStats>().gold = 9;

    key(&mut app, KeyCode::Escape);
    step(&mut app, 2);
    assert_eq!(state(&app), GameState::Paused);
    key(&mut app, KeyCode::ArrowDown); // Resume -> Abandon run
    assert_eq!(selected_label(&mut app), "Abandon run");
    key(&mut app, KeyCode::Enter);
    step(&mut app, 4);

    assert_eq!(state(&app), GameState::Estate);
    assert_eq!(*app.world().resource::<Profile>(), Profile::default());
    assert!(app.world().resource::<LastExpedition>().0.is_none());
}

#[test]
fn sandbox_runs_keep_their_own_result_and_pause_menus() {
    let mut app = game();
    key(&mut app, KeyCode::Escape);
    step(&mut app, 2);
    key(&mut app, KeyCode::ArrowDown);
    assert_eq!(selected_label(&mut app), "Main menu");
    key(&mut app, KeyCode::Escape);
    step(&mut app, 2);

    kill_player(&mut app);
    assert_eq!(selected_label(&mut app), "Play again");
}

#[test]
fn a_sandbox_run_started_after_an_estate_run_is_not_banked() {
    let mut app = estate();
    open_table(&mut app);
    key(&mut app, KeyCode::Enter);
    step(&mut app, 4);
    assert_eq!(*app.world().resource::<RunOrigin>(), RunOrigin::Estate);

    // Abandon, go to the title, and begin a setup-screen run.
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::MainMenu);
    step(&mut app, 3);
    key(&mut app, KeyCode::Enter); // "Begin expedition" is selected by default.
    step(&mut app, 4);
    assert_eq!(state(&app), GameState::Playing);
    assert_eq!(*app.world().resource::<RunOrigin>(), RunOrigin::Sandbox);

    app.world_mut().resource_mut::<GameStats>().gold = 30;
    kill_player(&mut app);
    assert_eq!(*app.world().resource::<Profile>(), Profile::default());
}

#[test]
fn the_title_screen_leads_into_the_estate() {
    let mut app = game();
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::MainMenu);
    step(&mut app, 3);
    key(&mut app, KeyCode::ArrowDown); // Begin expedition -> Enter the estate
    key(&mut app, KeyCode::Enter);
    step(&mut app, 3);
    assert_eq!(state(&app), GameState::Estate);
    assert_eq!(count::<EstateAvatar>(&mut app), 1);
}

#[test]
fn building_spends_exactly_the_cost_once_and_refusals_change_nothing() {
    let app = estate();
    let estate = app.world().resource::<EstateContent>().clone();
    let (_, upgrade) = upgrade_where(&app, |effect| {
        matches!(effect, EstateEffect::UnlockWeapon(_))
    });
    assert!(upgrade.requires.is_empty());
    let cost = upgrade.ranks[0];
    let mut profile = Profile::default();
    let error = build(&estate, &mut profile, &upgrade).unwrap_err();
    assert!(error.contains(&cost.to_string()), "{error}");
    assert_eq!(profile, Profile::default());

    profile.stockpile = cost;
    profile.stockpile.add(&Stockpile {
        gold: 1,
        essence: 2,
        seals: 3,
    });
    assert_eq!(build(&estate, &mut profile, &upgrade), Ok(1));
    assert!(profile.owns(&upgrade.id));
    assert_eq!(
        profile.stockpile,
        Stockpile {
            gold: 1,
            essence: 2,
            seals: 3
        }
    );

    let rich = profile.clone();
    assert!(build(&estate, &mut profile, &upgrade)
        .unwrap_err()
        .contains("already built"));
    assert_eq!(profile, rich);
}

#[test]
fn the_workshop_menu_builds_rank_by_rank_and_explains_refusals() {
    let mut app = estate();
    let first = app.world().resource::<EstateContent>().upgrades[0].clone();
    assert!(first.max_rank() > 1, "the first bundled upgrade is ranked");
    open_workshop(&mut app);
    assert_eq!(selected_label(&mut app), first.name);
    assert_eq!(
        row_detail(&mut app, 0),
        format!("Rank 1/{}: {}", first.max_rank(), first.ranks[0])
    );
    assert!(text_of::<WorkshopSummary>(&mut app).contains(&first.description));

    // Too poor: nothing is built and the reason is shown.
    key(&mut app, KeyCode::Enter);
    step(&mut app, 1);
    assert_eq!(*app.world().resource::<Profile>(), Profile::default());
    assert!(text_of::<WorkshopSummary>(&mut app).contains("needs"));

    app.world_mut().resource_mut::<Profile>().stockpile = first.ranks[0];
    key(&mut app, KeyCode::Enter);
    step(&mut app, 1);
    let profile = app.world().resource::<Profile>().clone();
    assert_eq!(profile.rank(&first.id), 1);
    assert_eq!(profile.stockpile, Stockpile::default());
    assert_eq!(
        row_detail(&mut app, 0),
        format!("Rank 2/{}: {}", first.max_rank(), first.ranks[1])
    );
    assert!(text_of::<WorkshopSummary>(&mut app).contains(&format!("Built {} rank 1", first.name)));

    // The first rank's price does not buy the second.
    app.world_mut().resource_mut::<Profile>().stockpile = first.ranks[0];
    key(&mut app, KeyCode::Enter);
    step(&mut app, 1);
    assert_eq!(app.world().resource::<Profile>().rank(&first.id), 1);
    assert_eq!(app.world().resource::<Profile>().stockpile, first.ranks[0]);

    // Paying for every remaining rank fills the upgrade, and then it stops.
    for rank in 1..first.max_rank() {
        app.world_mut().resource_mut::<Profile>().stockpile = first.ranks[rank as usize];
        key(&mut app, KeyCode::Enter);
        step(&mut app, 1);
    }
    assert_eq!(
        app.world().resource::<Profile>().rank(&first.id),
        first.max_rank()
    );
    assert_eq!(
        row_detail(&mut app, 0),
        format!("Rank {0}/{0}", first.max_rank())
    );

    key(&mut app, KeyCode::Escape);
    step(&mut app, 2);
    assert_eq!(screen(&app), EstateScreen::Roaming);
    assert_eq!(count::<UpgradeRow>(&mut app), 0);
}

#[test]
fn prerequisites_must_be_built_first() {
    let mut app = estate();
    let estate = app.world().resource::<EstateContent>().clone();
    let (index, locked) = estate
        .upgrades
        .iter()
        .cloned()
        .enumerate()
        .find(|(_, upgrade)| !upgrade.requires.is_empty())
        .expect("the bundled estate has an upgrade with a prerequisite");
    let required = upgrade_named(&app, &locked.requires[0]);
    let mut profile = Profile::default();
    profile.stockpile.add(&locked.ranks[0]);
    let error = build(&estate, &mut profile, &locked).unwrap_err();
    assert!(error.contains(&required.name), "{error}");
    assert_eq!(profile.rank(&locked.id), 0);

    open_workshop(&mut app);
    assert_eq!(
        row_detail(&mut app, index),
        format!("Requires {}", required.name)
    );
    grant(&mut app, &required.id, 1);
    step(&mut app, 1);
    assert_eq!(
        row_detail(&mut app, index),
        format!("Rank 1/{}: {}", locked.max_rank(), locked.ranks[0])
    );

    profile.upgrades.insert(required.id.clone(), 1);
    assert_eq!(build(&estate, &mut profile, &locked), Ok(1));
}

#[test]
fn an_unlocked_weapon_appears_at_the_table_and_can_start_an_expedition() {
    let mut app = estate();
    let (_, upgrade) = upgrade_where(&app, |effect| {
        matches!(effect, EstateEffect::UnlockWeapon(_))
    });
    let EstateEffect::UnlockWeapon(weapon) = upgrade.effect else {
        unreachable!()
    };
    let content = app.world().resource::<EstateContent>().clone();
    assert!(!content
        .available_weapons(app.world().resource::<Profile>())
        .contains(&weapon));
    // Without the upgrade the weapon cannot be smuggled into a plan.
    let plan = ExpeditionPlan { stage: 0, weapon };
    assert!(plan.recipe(app.world()).is_none());

    grant(&mut app, &upgrade.id, 1);
    open_table(&mut app);
    // Depart sits below the weapons; the newly unlocked one is listed last.
    key(&mut app, KeyCode::ArrowUp);
    assert_eq!(selected_label(&mut app), weapon.to_string());
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::ArrowDown);
    key(&mut app, KeyCode::Enter);
    step(&mut app, 4);

    assert_eq!(state(&app), GameState::Playing);
    let owned: Vec<_> = app
        .world_mut()
        .query::<&WeaponMeta>()
        .iter(app.world())
        .map(|meta| meta.weapon_type)
        .collect();
    assert_eq!(owned, vec![weapon]);
}

#[test]
fn a_starting_passive_upgrade_equips_estate_expeditions_only() {
    use crate::inventory::PassiveInventory;
    let mut app = estate();
    let (_, upgrade) = upgrade_where(&app, |effect| {
        matches!(effect, EstateEffect::StartingPassive(_))
    });
    let EstateEffect::StartingPassive(kind) = upgrade.effect else {
        unreachable!()
    };
    assert!(upgrade.max_rank() >= 2);
    grant(&mut app, &upgrade.id, 2);
    depart_from_table(&mut app);
    let player = crate::tests::player(&mut app);
    assert_eq!(
        app.world()
            .get::<PassiveInventory>(player)
            .unwrap()
            .level(kind),
        2,
        "each rank is one level"
    );

    // The sandbox recipe is untouched by estate progress.
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::MainMenu);
    step(&mut app, 3);
    key(&mut app, KeyCode::Enter);
    step(&mut app, 4);
    assert_eq!(state(&app), GameState::Playing);
    let player = crate::tests::player(&mut app);
    assert_eq!(
        app.world()
            .get::<PassiveInventory>(player)
            .unwrap()
            .level(kind),
        0
    );
}

#[test]
fn upgrade_definitions_are_validated() {
    let estate = |stations: &str, upgrades: &str| {
        format!(
            "(half_size: (400.0, 300.0), spawn: (0.0, 0.0), expedition_weapons: [magick_circle], \
             stations: [(kind: ExpeditionTable, name: \"Table\", position: (0.0, 100.0)){stations}], \
             upgrades: [{upgrades}])"
        )
    };
    let workshop = ", (kind: Workshop, name: \"Bench\", position: (0.0, -100.0))";
    let upgrade = |id: &str, cost: &str, effect: &str| {
        format!(
            "(id: \"{id}\", name: \"N\", description: \"D\", ranks: [{cost}], effect: {effect}),"
        )
    };
    let knife = upgrade("a", "(gold: 1)", "UnlockWeapon(knife_of_the_art)");
    EstateContent::parse(&estate(workshop, &knife)).unwrap();
    for (stations, upgrades, reason) in [
        ("", knife.clone(), "Workshop"),
        (workshop, format!("{knife}{knife}"), "unique"),
        (
            workshop,
            upgrade("a", "()", "UnlockWeapon(knife_of_the_art)"),
            "cost something",
        ),
        (
            workshop,
            upgrade("a", "(gold: 1), (gold: 2)", "UnlockWeapon(knife_of_the_art)"),
            "one rank",
        ),
        (
            workshop,
            upgrade("a", "(gold: 1)", "HaulBonus(gold, 0)"),
            "positive haul bonus",
        ),
        (
            workshop,
            upgrade("a", "(gold: 1), (gold: 1), (gold: 1)", "HaulBonus(gold, 90)"),
            "more than 200%",
        ),
        (
            workshop,
            format!(
                "(id: \"a\", name: \"N\", description: \"D\", ranks: [(gold: 1)], requires: [\"b\"], effect: HaulBonus(gold, 5)),{}",
                upgrade("b", "(gold: 1)", "HaulBonus(essence, 5)")
            ),
            "listed before it",
        ),
        (
            workshop,
            upgrade("a", "(gold: 1)", "UnlockWeapon(magick_circle)"),
            "already granted",
        ),
        (
            workshop,
            format!(
                "{}{}",
                upgrade("a", "(gold: 1)", "StartingPassive(cooldown)"),
                upgrade("b", "(gold: 1)", "StartingPassive(cooldown)")
            ),
            "already granted",
        ),
    ] {
        let error = EstateContent::parse(&estate(stations, &upgrades)).unwrap_err();
        assert!(error.contains(reason), "{upgrades}: {error}");
    }
}

#[test]
fn authored_ranks_and_provisions_fit_within_passive_and_weapon_levels() {
    let app = estate();
    let estate = app.world().resource::<EstateContent>();
    let catalog = app.world().resource::<crate::inventory::PassiveCatalog>();
    for upgrade in &estate.upgrades {
        if let EstateEffect::StartingPassive(kind) = upgrade.effect {
            // Leave room for a provision on top of a fully built upgrade.
            assert!(
                upgrade.max_rank() < u32::from(catalog.get(kind).max_level()),
                "{} has more ranks than {kind:?} has levels",
                upgrade.id
            );
        }
    }
}

#[test]
fn provisions_are_packed_up_to_the_limit_and_used_by_the_next_expedition() {
    use super::content::{ProvisionEffect, PROVISION_SLOTS};
    use super::provisioner::{buy, can_buy};
    use crate::inventory::PassiveInventory;
    let mut app = estate();
    let estate = app.world().resource::<EstateContent>().clone();
    let passive = estate
        .provisions
        .iter()
        .find(|provision| matches!(provision.effect, ProvisionEffect::Passive(_)))
        .unwrap()
        .clone();
    let weapon = estate
        .provisions
        .iter()
        .find(|provision| matches!(provision.effect, ProvisionEffect::Weapon(_)))
        .unwrap()
        .clone();
    let ProvisionEffect::Passive(kind) = passive.effect else {
        unreachable!()
    };
    let ProvisionEffect::Weapon(extra) = weapon.effect else {
        unreachable!()
    };

    let mut profile = Profile::default();
    assert!(buy(&mut profile, &passive).unwrap_err().contains("needs"));
    profile.stockpile = Stockpile {
        gold: 500,
        essence: 500,
        seals: 0,
    };
    buy(&mut profile, &passive).unwrap();
    assert!(can_buy(&profile, &passive)
        .unwrap_err()
        .contains("already packed"));
    buy(&mut profile, &weapon).unwrap();
    assert_eq!(profile.provisions.len(), PROVISION_SLOTS);
    let third = estate
        .provisions
        .iter()
        .find(|provision| !profile.provisions.contains(&provision.id))
        .unwrap();
    let full = profile.clone();
    assert!(buy(&mut profile, third).unwrap_err().contains("fit"));
    assert_eq!(profile, full);

    // A passive provision stacks on the matching upgrade, and the weapon is
    // carried beside the chosen one.
    let upgrade = estate
        .upgrades
        .iter()
        .find(|upgrade| upgrade.effect == EstateEffect::StartingPassive(kind))
        .map(|upgrade| upgrade.id.clone());
    let expected_level = if let Some(id) = upgrade {
        profile.upgrades.insert(id, 1);
        2
    } else {
        1
    };
    *app.world_mut().resource_mut::<Profile>() = profile;
    let starting = estate.expedition_weapons[0];
    assert_ne!(starting, extra);
    depart_from_table(&mut app);

    let player = crate::tests::player(&mut app);
    assert_eq!(
        app.world()
            .get::<PassiveInventory>(player)
            .unwrap()
            .level(kind),
        expected_level
    );
    let mut owned: Vec<_> = app
        .world_mut()
        .query::<&WeaponMeta>()
        .iter(app.world())
        .map(|meta| meta.weapon_type)
        .collect();
    owned.sort_by_key(|weapon| weapon.to_string());
    let mut expected = vec![starting, extra];
    expected.sort_by_key(|weapon| weapon.to_string());
    assert_eq!(owned, expected);
    assert!(app.world().resource::<Profile>().provisions.is_empty());
}

#[test]
fn a_weapon_provision_matching_the_starting_weapon_levels_it_up() {
    use super::content::ProvisionEffect;
    let mut app = estate();
    let estate = app.world().resource::<EstateContent>().clone();
    let provision = estate
        .provisions
        .iter()
        .find(|provision| matches!(provision.effect, ProvisionEffect::Weapon(_)))
        .unwrap();
    let ProvisionEffect::Weapon(weapon) = provision.effect else {
        unreachable!()
    };
    assert!(estate.expedition_weapons.contains(&weapon));
    app.world_mut()
        .resource_mut::<Profile>()
        .provisions
        .push(provision.id.clone());
    app.world_mut().resource_mut::<ExpeditionPlan>().weapon = weapon;
    let (recipe, _) = app
        .world()
        .resource::<ExpeditionPlan>()
        .recipe(app.world())
        .unwrap();
    assert_eq!(recipe.weapons.len(), 1);
    assert_eq!(recipe.weapons[0].level, 2);
}

#[test]
fn the_provisioner_and_altar_menus_open_from_their_stations() {
    let mut app = estate();
    open_station(
        &mut app,
        InteractionKind::Provisioner,
        Vec2::new(0.0, -40.0),
    );
    assert_eq!(screen(&app), EstateScreen::Provisioner);
    let first = app.world().resource::<EstateContent>().provisions[0].clone();
    assert_eq!(selected_label(&mut app), first.name);
    app.world_mut().resource_mut::<Profile>().stockpile = first.cost;
    key(&mut app, KeyCode::Enter);
    step(&mut app, 1);
    assert_eq!(app.world().resource::<Profile>().provisions, vec![first.id]);
    let summary = text_of::<ProvisionerSummary>(&mut app);
    assert!(
        summary.contains(&format!("Packed (1/2): {}", first.name)),
        "{summary}"
    );
    key(&mut app, KeyCode::Escape);
    step(&mut app, 2);
    assert_eq!(screen(&app), EstateScreen::Roaming);

    open_station(&mut app, InteractionKind::Altar, Vec2::new(0.0, 40.0));
    assert_eq!(screen(&app), EstateScreen::Altar);
    let ritual = app.world().resource::<EstateContent>().rituals[0].clone();
    assert_eq!(selected_label(&mut app), ritual.name);
    assert!(text_of::<AltarSummary>(&mut app).contains(&super::altar::terms(&ritual)));
    app.world_mut().resource_mut::<Profile>().stockpile = ritual.cost;
    key(&mut app, KeyCode::Enter);
    step(&mut app, 1);
    assert_eq!(app.world().resource::<Profile>().ritual, Some(ritual.id));
    assert!(text_of::<AltarSummary>(&mut app).contains("prepared for the next expedition"));
}

#[test]
fn rituals_are_prepared_one_at_a_time_and_respect_prerequisites() {
    use super::altar::{can_perform, perform};
    let app = estate();
    let estate = app.world().resource::<EstateContent>().clone();
    let open = estate
        .rituals
        .iter()
        .find(|ritual| ritual.requires.is_empty())
        .unwrap();
    let locked = estate
        .rituals
        .iter()
        .find(|ritual| !ritual.requires.is_empty())
        .expect("the bundled estate has a ritual behind an upgrade");
    let mut profile = Profile {
        stockpile: Stockpile {
            gold: 999,
            essence: 999,
            seals: 999,
        },
        ..default()
    };
    let error = can_perform(&estate, &profile, locked).unwrap_err();
    assert!(error.contains("requires"), "{error}");

    perform(&estate, &mut profile, open).unwrap();
    let prepared = profile.clone();
    for ritual in &estate.rituals {
        assert!(perform(&estate, &mut profile, ritual)
            .unwrap_err()
            .contains("already prepared"));
    }
    assert_eq!(profile, prepared);
}

#[test]
fn a_ritual_shapes_one_expedition_and_its_haul_then_is_spent() {
    let mut app = estate();
    let estate = app.world().resource::<EstateContent>().clone();
    let ritual = estate
        .rituals
        .iter()
        .find(|ritual| ritual.haul_bonus.essence > 0 && ritual.requires.is_empty())
        .unwrap()
        .clone();
    assert!(!ritual.modifiers.is_neutral());
    app.world_mut().resource_mut::<Profile>().ritual = Some(ritual.id.clone());
    depart_from_table(&mut app);

    let config = app.world().resource::<ActiveRunConfig>().0.clone();
    assert_eq!(config.modifiers, ritual.modifiers);
    assert_eq!(
        app.world().resource::<HaulBonus>().percent,
        ritual.haul_bonus
    );
    assert_eq!(app.world().resource::<Profile>().ritual, None);

    // Enemies that appear during the run carry the ritual's health.
    let enemy = app
        .world_mut()
        .spawn((
            crate::components::Enemy {
                speed: 0.0,
                experience_value: 0,
            },
            crate::components::Health {
                current: 100,
                maximum: 100,
            },
        ))
        .id();
    let health = app.world().get::<crate::components::Health>(enemy).unwrap();
    let expected = ritual.modifiers.enemy_health(100);
    assert_eq!((health.current, health.maximum), (expected, expected));
    app.world_mut().despawn(enemy);

    app.world_mut().resource_mut::<GameStats>().enemies_killed = 100;
    kill_player(&mut app);
    let essence = 10 + 10 * ritual.haul_bonus.essence / 100;
    assert_eq!(app.world().resource::<Profile>().stockpile.essence, essence);
    let report = app.world().resource::<LastExpedition>().0.clone().unwrap();
    assert_eq!(report.bonus.essence, essence - 10);

    // "Depart again" plans afresh: the spent ritual does not come back.
    assert_eq!(selected_label(&mut app), "Return to estate");
    key(&mut app, KeyCode::ArrowDown);
    assert_eq!(selected_label(&mut app), "Depart again");
    key(&mut app, KeyCode::Enter);
    step(&mut app, 4);
    assert_eq!(state(&app), GameState::Playing);
    assert!(app
        .world()
        .resource::<ActiveRunConfig>()
        .0
        .modifiers
        .is_neutral());
    assert_eq!(
        app.world().resource::<HaulBonus>().percent,
        Stockpile::default()
    );
}

#[test]
fn haul_bonus_upgrades_add_to_estate_expeditions_only() {
    let mut app = estate();
    let (_, upgrade) = upgrade_where(&app, |effect| {
        matches!(
            effect,
            EstateEffect::HaulBonus(crate::profile::ResourceKind::Gold, _)
        )
    });
    let EstateEffect::HaulBonus(_, percent) = upgrade.effect else {
        unreachable!()
    };
    grant(&mut app, &upgrade.id, 2);
    depart_from_table(&mut app);
    app.world_mut().resource_mut::<GameStats>().gold = 40;
    kill_player(&mut app);
    assert_eq!(
        app.world().resource::<Profile>().stockpile.gold,
        40 + 40 * percent * 2 / 100
    );
}

#[test]
fn provision_and_ritual_definitions_are_validated() {
    let estate = |stations: &str, body: &str| {
        format!(
            "(half_size: (400.0, 300.0), spawn: (0.0, 0.0), expedition_weapons: [magick_circle], \
             stations: [(kind: ExpeditionTable, name: \"Table\", position: (0.0, 100.0)){stations}], {body})"
        )
    };
    let shop = ", (kind: Provisioner, name: \"Shop\", position: (0.0, -100.0))";
    let altar = ", (kind: Altar, name: \"Altar\", position: (100.0, -100.0))";
    let provision = |cost: &str| {
        format!(
            "provisions: [(id: \"p\", name: \"P\", description: \"D\", cost: {cost}, effect: Passive(cooldown))]"
        )
    };
    let ritual = |extra: &str| {
        format!("rituals: [(id: \"r\", name: \"R\", description: \"D\", cost: (gold: 1){extra})]")
    };
    EstateContent::parse(&estate(shop, &provision("(gold: 1)"))).unwrap();
    EstateContent::parse(&estate(
        altar,
        &ritual(", modifiers: (enemy_health_percent: 120)"),
    ))
    .unwrap();
    for (stations, body, reason) in [
        ("", provision("(gold: 1)"), "Provisioner station"),
        (shop, provision("()"), "cost something"),
        ("", ritual(", haul_bonus: (gold: 5)"), "Altar station"),
        (altar, ritual(""), "change the run"),
        (
            altar,
            ritual(", modifiers: (damage_taken_percent: 900)"),
            "between",
        ),
        (
            altar,
            ritual(", requires: [\"nowhere\"], haul_bonus: (gold: 5)"),
            "unknown upgrade",
        ),
    ] {
        let error = EstateContent::parse(&estate(stations, &body)).unwrap_err();
        assert!(error.contains(reason), "{body}: {error}");
    }
}
