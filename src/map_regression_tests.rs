use crate::{
    arena::{camera_half_view, ArenaBounds, ArenaObstacle, ArenaRoot, GameCamera, WORLD_GROUP},
    content::{ActiveStage, GameContent, StageLayout, EMBEDDED_CONTENT},
    resources::GameState,
    tests::{count, game, player, step},
};
use bevy::{camera::CameraProjection, prelude::*};
use bevy_rapier2d::prelude::*;

#[test]
fn authored_content_has_three_distinct_layouts_and_subminute_wave_changes() {
    let content = GameContent::parse(EMBEDDED_CONTENT).unwrap();
    assert_eq!(content.stages.len(), 3);
    assert_eq!(content.stages[0].layout, StageLayout::Arena);
    assert_eq!(content.stages[1].layout, StageLayout::Horizontal);
    assert_eq!(content.stages[2].layout, StageLayout::Vertical);
    let stage = &content.stages[0];
    let early = stage
        .schedule
        .iter()
        .find(|phase| phase.start == 120.0)
        .unwrap();
    let later = stage
        .schedule
        .iter()
        .find(|phase| phase.start == 150.0)
        .unwrap();
    // Introducing the caster deliberately reduces pursuit pressure at 2:30.
    assert!(later.interval > early.interval);
    assert!(later.cap < early.cap);
    assert!(later.enemies.iter().any(|entry| entry.id == "caster"));
    assert!(matches!(
        stage.win_condition,
        crate::content::StageWinCondition::DefeatBoss { .. }
    ));
    assert_eq!(
        content.stages[1].win_condition,
        crate::content::StageWinCondition::TimedSurvival
    );
    assert!(stage
        .encounters
        .iter()
        .any(|encounter| encounter.ring.is_some() && encounter.reward_chest));
}

#[test]
fn content_validation_rejects_bad_references_timing_and_blocked_start() {
    let valid = GameContent::parse(EMBEDDED_CONTENT).unwrap();
    let mut content = valid.clone();
    content.stages[0].schedule[0].enemies[0].id = "missing-enemy".into();
    assert!(content.validate().unwrap_err().contains("unknown enemy"));
    let mut content = valid.clone();
    content.stages[0].schedule[1].start = 0.0;
    assert!(content
        .validate()
        .unwrap_err()
        .contains("start must increase"));
    let mut content = valid.clone();
    content.stages[0].schedule[0].minimum = 1001;
    assert!(content.validate().unwrap_err().contains("minimum <= cap"));
    let mut content = valid.clone();
    content.stages[0].obstacles[0].center = [0.0, 0.0];
    assert!(content
        .validate()
        .unwrap_err()
        .contains("starting position"));
    let mut content = valid;
    content.enemies[0].speed = f32::NAN;
    assert!(content.validate().unwrap_err().contains("finite"));
    let misspelled = EMBEDDED_CONTENT.replacen("duration: 600.0", "duratioon: 600.0", 1);
    assert!(GameContent::parse(&misspelled)
        .unwrap_err()
        .contains("duratioon"));
}

#[test]
fn stage_switch_replaces_geometry_and_rapier_obstacles_block_movement() {
    let mut app = game();
    for stage_index in 0..3 {
        let old_root = app
            .world_mut()
            .query_filtered::<Entity, With<ArenaRoot>>()
            .single(app.world())
            .unwrap();
        app.world_mut().resource_mut::<ActiveStage>().0 = stage_index;
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Restarting);
        step(&mut app, 4);
        assert!(app.world().get_entity(old_root).is_err());
        assert_eq!(count::<ArenaRoot>(&mut app), 1);
        assert_eq!(count::<GameCamera>(&mut app), 1);
        let stage = app.world().resource::<GameContent>().stages[stage_index].clone();
        assert_eq!(
            app.world().resource::<ArenaBounds>().half_size,
            Vec2::from_array(stage.half_size)
        );
        assert_eq!(count::<ArenaObstacle>(&mut app), stage.obstacles.len());
        for (body, groups) in app
            .world_mut()
            .query_filtered::<(&RigidBody, &CollisionGroups), With<ArenaObstacle>>()
            .iter(app.world())
        {
            assert_eq!(*body, RigidBody::Fixed);
            assert_eq!(groups.memberships, WORLD_GROUP);
        }

        let obstacle = &stage.obstacles[0];
        let obstacle_left = obstacle.center[0] - obstacle.half_size[0];
        let player = player(&mut app);
        app.world_mut()
            .get_mut::<Transform>(player)
            .unwrap()
            .translation = Vec3::new(obstacle_left - 30.0, obstacle.center[1], 0.0);
        step(&mut app, 2);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyD);
        step(&mut app, 30);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release_all();
        let position = app.world().get::<Transform>(player).unwrap().translation;
        assert!(
            position.x <= obstacle_left - 11.0,
            "crossed stage {stage_index} obstacle: {position}"
        );
        assert!(
            position.x > obstacle_left - 25.0,
            "did not approach stage {stage_index} obstacle"
        );
    }
}

#[test]
fn extreme_window_aspect_ratios_leave_space_to_spawn_along_each_stage() {
    let mut app = game();
    for stage_index in 0..3 {
        app.world_mut().resource_mut::<ActiveStage>().0 = stage_index;
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Restarting);
        step(&mut app, 4);
        let bounds = app.world().resource::<ArenaBounds>().half_size;
        let mut projection = app
            .world_mut()
            .query_filtered::<&Projection, With<GameCamera>>()
            .single(app.world())
            .unwrap()
            .clone();
        for (width, height) in [
            (1280.0, 720.0),
            (5120.0, 2880.0),
            (5120.0, 720.0),
            (720.0, 5120.0),
        ] {
            let Projection::Orthographic(ref mut ortho) = projection else {
                panic!("expected orthographic camera");
            };
            ortho.update(width, height);
            let view = camera_half_view(&projection);
            assert!(view.x <= 640.01 && view.y <= 360.01);
            let long_axis = if bounds.x >= bounds.y { 0 } else { 1 };
            assert!(view[long_axis] + 90.0 < bounds[long_axis]);
            if stage_index > 0 && width / height == 1280.0 / 720.0 {
                let short_axis = 1 - long_axis;
                assert!(
                    view[short_axis] >= bounds[short_axis] + 63.99,
                    "corridor short axis and both boundary walls should fit: stage {stage_index}, view {view}"
                );
            }
        }
    }
}
