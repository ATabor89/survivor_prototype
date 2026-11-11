//! The estate map and the avatar that walks it.
use super::content::{EstateContent, StationKind};
use super::interaction::{Interactable, InteractionKind};
use super::{keyboard_free, EstateScreen};
use crate::arena::{
    spawn_boundary_walls, view_projection, world_collision_groups, ArenaBounds, ArenaRoot,
    GameCamera, WORLD_GROUP,
};
use crate::components::CameraFocus;
use crate::physics::top_down_controller;
use crate::resources::{GameState, GameTextures};
use crate::systems::movement_input;
use crate::GameplaySets;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

pub const AVATAR_SPEED: f32 = 170.0;
pub const AVATAR_RADIUS: f32 = 12.0;

/// The player's body in the estate. It carries no health, weapons, or XP, so
/// expedition systems that query `Player` never see it.
#[derive(Component)]
#[require(CameraFocus)]
pub struct EstateAvatar;

#[derive(Component)]
pub struct EstateRoot;

#[derive(Component)]
pub struct Npc;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Estate), build_estate)
        .add_systems(
            Update,
            walk.in_set(GameplaySets::Movement)
                .run_if(in_state(EstateScreen::Roaming))
                .run_if(keyboard_free),
        );
}

/// Replace the expedition stage with the estate. The stage is rebuilt by the
/// normal restart path when the next expedition begins.
fn build_estate(
    mut commands: Commands,
    content: Res<EstateContent>,
    textures: Res<GameTextures>,
    mut bounds: ResMut<ArenaBounds>,
    stage: Query<Entity, With<ArenaRoot>>,
    mut cameras: Query<&mut Projection, With<GameCamera>>,
) {
    for entity in &stage {
        commands.entity(entity).despawn();
    }
    let half = Vec2::from_array(content.half_size);
    bounds.half_size = half;
    for mut projection in &mut cameras {
        *projection = view_projection(half);
    }

    let character = |color: Color| Sprite {
        image: textures.player.clone(),
        custom_size: Some(Vec2::splat(48.0)),
        texture_atlas: Some(TextureAtlas {
            layout: textures.player_layout.clone(),
            index: 0,
        }),
        color,
        ..default()
    };

    commands.spawn((
        EstateAvatar,
        DespawnOnExit(GameState::Estate),
        Name::new("Estate avatar"),
        character(Color::WHITE),
        Transform::from_translation(Vec2::from_array(content.spawn).extend(0.0)),
        RigidBody::KinematicPositionBased,
        top_down_controller(),
        Collider::ball(AVATAR_RADIUS),
        CollisionGroups::new(Group::GROUP_1, WORLD_GROUP),
    ));

    commands
        .spawn((
            EstateRoot,
            DespawnOnExit(GameState::Estate),
            Name::new("Estate"),
            Transform::default(),
            Visibility::default(),
        ))
        .with_children(|estate| {
            spawn_ground(estate, half);
            spawn_boundary_walls(estate, half);
            for building in &content.buildings {
                let center = Vec2::from_array(building.center);
                let building_half = Vec2::from_array(building.half_size);
                let [red, green, blue] = building.color;
                estate.spawn((
                    Name::new(building.name.clone()),
                    RigidBody::Fixed,
                    Collider::cuboid(building_half.x, building_half.y),
                    world_collision_groups(),
                    Sprite::from_color(Color::srgb(red, green, blue), building_half * 2.0),
                    Transform::from_translation(center.extend(-1.0)),
                ));
                estate.spawn(label(&building.name, 20.0, center.extend(1.0)));
            }
            for station in &content.stations {
                let position = Vec2::from_array(station.position);
                let (kind, prompt) = match station.kind {
                    StationKind::ExpeditionTable => (
                        InteractionKind::ExpeditionTable,
                        format!("Plan an expedition at the {}", station.name),
                    ),
                    StationKind::Workshop => (
                        InteractionKind::Workshop,
                        format!("Build improvements at the {}", station.name),
                    ),
                };
                estate.spawn((
                    Name::new(station.name.clone()),
                    Interactable { kind, prompt },
                    Sprite::from_color(Color::srgb(0.80, 0.68, 0.36), Vec2::new(54.0, 34.0)),
                    Transform::from_translation(position.extend(-0.5)),
                ));
                estate.spawn(label(
                    &station.name,
                    16.0,
                    (position + Vec2::Y * 36.0).extend(1.0),
                ));
            }
            for (index, npc) in content.npcs.iter().enumerate() {
                let position = Vec2::from_array(npc.position);
                let [red, green, blue] = npc.color;
                estate.spawn((
                    Npc,
                    Name::new(npc.name.clone()),
                    Interactable {
                        kind: InteractionKind::Talk(index),
                        prompt: format!("Talk to {}", npc.name),
                    },
                    character(Color::srgb(red, green, blue)),
                    Transform::from_translation(position.extend(-0.25)),
                ));
                estate.spawn(label(
                    &npc.name,
                    15.0,
                    (position + Vec2::Y * 40.0).extend(1.0),
                ));
            }
        });
}

fn spawn_ground(estate: &mut ChildSpawnerCommands, half: Vec2) {
    // Tile counts and clipped edge sizes follow the authored bounds, matching
    // how expedition stages draw their floors.
    let tile = 160.0;
    let counts = (half * 2.0 / tile).ceil().as_uvec2();
    for x in 0..counts.x {
        for y in 0..counts.y {
            let lower = -half + Vec2::new(x as f32, y as f32) * tile;
            let size = (half - lower).min(Vec2::splat(tile));
            let shade = ((x * 13 + y * 29) % 5) as f32 * 0.007;
            estate.spawn((
                Sprite::from_color(
                    Color::srgb(0.13 + shade, 0.17 + shade, 0.12 + shade),
                    size - Vec2::splat(2.0),
                ),
                Transform::from_translation((lower + size * 0.5).extend(-20.0)),
            ));
        }
    }
}

fn label(text: &str, size: f32, position: Vec3) -> impl Bundle {
    (
        Text2d::new(text),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(Color::srgb(0.86, 0.82, 0.70)),
        Transform::from_translation(position),
    )
}

fn walk(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    time: Res<Time>,
    mut avatar: Query<&mut KinematicCharacterController, With<EstateAvatar>>,
) {
    let direction = movement_input(&keyboard, &gamepads, true);
    for mut controller in &mut avatar {
        controller.translation = Some(direction * AVATAR_SPEED * time.delta_secs());
    }
}
