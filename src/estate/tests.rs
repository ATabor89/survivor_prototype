use super::content::{EstateContent, EMBEDDED_ESTATE};
use super::dialogue::{ActiveDialogue, DialogueLine};
use super::expedition_table::ExpeditionPlan;
use super::hud::{InteractionPrompt, LedgerReadout};
use super::interaction::{Focus, Interactable, InteractionKind};
use super::world::{EstateAvatar, EstateRoot, Npc, AVATAR_RADIUS, AVATAR_SPEED};
use super::EstateScreen;
use crate::arena::ArenaRoot;
use crate::components::Player;
use crate::expedition::{LastExpedition, RunOrigin};
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
