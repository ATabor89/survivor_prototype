//! Authored finite stages. Rapier handles walls and obstacles; bounds describe layout.
use crate::{
    components::Player,
    content::{ActiveStage, GameContent, StageDefinition, StageLayout},
    resources::GameState,
};
use bevy::prelude::*;
use bevy_prototype_lyon::{
    geometry::{ShapeBuilder, ShapeBuilderBase},
    shapes,
};
use bevy_rapier2d::prelude::*;

pub const WORLD_GROUP: Group = Group::GROUP_5;

#[derive(Resource, Clone, Copy)]
pub struct ArenaBounds {
    pub half_size: Vec2,
}
impl Default for ArenaBounds {
    fn default() -> Self {
        Self {
            half_size: Vec2::new(1600.0, 1200.0),
        }
    }
}

#[derive(Component)]
pub struct ArenaRoot;
#[derive(Component)]
pub struct ArenaObstacle;
#[derive(Component)]
pub struct GameCamera;

pub struct ArenaPlugin;
impl Plugin for ArenaPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameContent>()
            .init_resource::<ActiveStage>()
            .init_resource::<ArenaBounds>()
            .add_systems(Startup, setup_arena)
            .add_systems(OnEnter(GameState::Restarting), rebuild_arena)
            .add_systems(
                PostUpdate,
                follow_camera
                    .after(PhysicsSet::Writeback)
                    .before(bevy::transform::TransformSystems::Propagate),
            );
    }
}

fn setup_arena(
    mut commands: Commands,
    content: Res<GameContent>,
    active: Res<ActiveStage>,
    mut bounds: ResMut<ArenaBounds>,
) {
    let stage = content.stage(&active);
    bounds.half_size = Vec2::from_array(stage.half_size);
    commands.spawn((
        Camera2d,
        stage_projection(stage),
        GameCamera,
        Name::new("Stage camera"),
    ));
    spawn_stage(&mut commands, stage);
}

/// Geometry has a separate lifetime from run entities. Rebuilding applies stage
/// selection and ensures old obstacle colliders cannot survive a restart.
pub fn rebuild_arena(
    mut commands: Commands,
    content: Res<GameContent>,
    active: Res<ActiveStage>,
    mut bounds: ResMut<ArenaBounds>,
    roots: Query<Entity, With<ArenaRoot>>,
    mut cameras: Query<&mut Projection, With<GameCamera>>,
) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    let stage = content.stage(&active);
    bounds.half_size = Vec2::from_array(stage.half_size);
    for mut projection in &mut cameras {
        *projection = stage_projection(stage);
    }
    spawn_stage(&mut commands, stage);
}

fn stage_projection(stage: &StageDefinition) -> Projection {
    // Bound both world-view dimensions even at extreme window aspect ratios.
    // Narrow stages frame their short axis; a wide display must not expose an
    // entire long axis and eliminate all offscreen spawn locations.
    Projection::Orthographic(OrthographicProjection {
        scaling_mode: bevy::camera::ScalingMode::AutoMax {
            max_width: (stage.half_size[0] * 2.0 + 128.0).min(1280.0),
            max_height: (stage.half_size[1] * 2.0 + 128.0).min(720.0),
        },
        ..OrthographicProjection::default_2d()
    })
}

fn spawn_stage(commands: &mut Commands, stage: &StageDefinition) {
    let half = Vec2::from_array(stage.half_size);
    let base_color = match stage.layout {
        StageLayout::Arena => Vec3::new(0.115, 0.135, 0.135),
        StageLayout::Horizontal => Vec3::new(0.15, 0.125, 0.105),
        StageLayout::Vertical => Vec3::new(0.105, 0.12, 0.16),
    };
    commands
        .spawn((
            ArenaRoot,
            Name::new(stage.name.clone()),
            Transform::default(),
            Visibility::default(),
        ))
        .with_children(|arena| {
            // Tile counts and clipped edge sizes follow authored bounds. Visual
            // tiles are independent of collision geometry.
            let tile = 160.0;
            let counts = (half * 2.0 / tile).ceil().as_uvec2();
            for x in 0..counts.x {
                for y in 0..counts.y {
                    let lower = -half + Vec2::new(x as f32, y as f32) * tile;
                    let size = (half - lower).min(Vec2::splat(tile));
                    let shade = ((x * 17 + y * 31) % 5) as f32 * 0.006;
                    let color = base_color + Vec3::splat(shade);
                    arena.spawn((
                        Sprite::from_color(
                            Color::srgb(color.x, color.y, color.z),
                            size - Vec2::splat(2.0),
                        ),
                        Transform::from_translation((lower + size * 0.5).extend(-20.0)),
                    ));
                }
            }
            if stage.layout != StageLayout::Vertical {
                arena.spawn((
                    Sprite::from_color(
                        Color::srgb(0.19, 0.18, 0.16),
                        Vec2::new(half.x * 2.0, 120.0),
                    ),
                    Transform::from_xyz(0.0, 0.0, -19.0),
                ));
            }
            if stage.layout != StageLayout::Horizontal {
                arena.spawn((
                    Sprite::from_color(
                        Color::srgb(0.19, 0.18, 0.16),
                        Vec2::new(120.0, half.y * 2.0),
                    ),
                    Transform::from_xyz(0.0, 0.0, -19.0),
                ));
            }
            for landmark in &stage.landmarks {
                let position = Vec2::from_array(landmark.position);
                arena.spawn((
                    Name::new(landmark.name.clone()),
                    ShapeBuilder::with(&shapes::Circle {
                        radius: 75.0,
                        center: Vec2::ZERO,
                    })
                    .stroke((Color::srgb(0.38, 0.34, 0.23), 3.0))
                    .build(),
                    Transform::from_translation(position.extend(-18.0)),
                ));
                arena.spawn((
                    Text2d::new(landmark.name.clone()),
                    TextFont {
                        font_size: FontSize::Px(18.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.62, 0.56, 0.41)),
                    Transform::from_translation((position + Vec2::Y * 98.0).extend(-17.0)),
                ));
            }
            let thickness = 64.0;
            for (center, wall_half) in [
                (
                    Vec2::new(half.x + thickness / 2.0, 0.0),
                    Vec2::new(thickness / 2.0, half.y + thickness),
                ),
                (
                    Vec2::new(-half.x - thickness / 2.0, 0.0),
                    Vec2::new(thickness / 2.0, half.y + thickness),
                ),
                (
                    Vec2::new(0.0, half.y + thickness / 2.0),
                    Vec2::new(half.x + thickness, thickness / 2.0),
                ),
                (
                    Vec2::new(0.0, -half.y - thickness / 2.0),
                    Vec2::new(half.x + thickness, thickness / 2.0),
                ),
            ] {
                arena.spawn((
                    Name::new("Stage boundary"),
                    RigidBody::Fixed,
                    Collider::cuboid(wall_half.x, wall_half.y),
                    CollisionGroups::new(WORLD_GROUP, Group::GROUP_1 | Group::GROUP_2),
                    Sprite::from_color(Color::srgb(0.29, 0.29, 0.26), wall_half * 2.0),
                    Transform::from_translation(center.extend(-2.0)),
                ));
            }
            for obstacle in &stage.obstacles {
                let obstacle_half = Vec2::from_array(obstacle.half_size);
                arena.spawn((
                    ArenaObstacle,
                    Name::new("Stone plinth"),
                    RigidBody::Fixed,
                    Collider::cuboid(obstacle_half.x, obstacle_half.y),
                    CollisionGroups::new(WORLD_GROUP, Group::GROUP_1 | Group::GROUP_2),
                    Sprite::from_color(Color::srgb(0.3, 0.31, 0.3), obstacle_half * 2.0),
                    Transform::from_translation(Vec2::from_array(obstacle.center).extend(-1.0)),
                ));
            }
        });
}

pub fn camera_half_view(projection: &Projection) -> Vec2 {
    match projection {
        Projection::Orthographic(ortho) if ortho.area.size().min_element() > 16.0 => {
            ortho.area.size() * 0.5
        }
        _ => Vec2::new(640.0, 360.0),
    }
}

fn follow_camera(
    bounds: Res<ArenaBounds>,
    player: Query<&Transform, (With<Player>, Without<GameCamera>)>,
    mut camera: Query<(&mut Transform, &Projection), With<GameCamera>>,
) {
    let Ok(player) = player.single() else {
        return;
    };
    for (mut transform, projection) in &mut camera {
        let limit =
            (bounds.half_size + Vec2::splat(64.0) - camera_half_view(projection)).max(Vec2::ZERO);
        let position = player.translation.truncate().clamp(-limit, limit);
        transform.translation.x = position.x;
        transform.translation.y = position.y;
    }
}
