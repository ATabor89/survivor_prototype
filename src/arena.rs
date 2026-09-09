//! Persistent arena geometry. Rapier owns collision response; bounds also describe layout.
use crate::components::Player;
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
pub struct GameCamera;

pub struct ArenaPlugin;
impl Plugin for ArenaPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ArenaBounds>()
            .add_systems(Startup, setup_arena)
            // Follow the completed physics step, then propagate the camera's transform.
            .add_systems(
                PostUpdate,
                follow_camera
                    .after(PhysicsSet::Writeback)
                    .before(bevy::transform::TransformSystems::Propagate),
            );
    }
}

fn setup_arena(mut commands: Commands, bounds: Res<ArenaBounds>) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: bevy::camera::ScalingMode::AutoMin {
                min_width: 1280.0,
                min_height: 720.0,
            },
            ..OrthographicProjection::default_2d()
        }),
        GameCamera,
        Name::new("Arena camera"),
    ));
    commands
        .spawn((
            ArenaRoot,
            Name::new("The sealed courtyard"),
            Transform::default(),
            Visibility::default(),
        ))
        .with_children(|arena| {
            // A small, deterministic tile palette gives motion and position reference.
            let tile = 160.0;
            for x in -10i32..10 {
                for y in -8i32..8 {
                    let center = Vec2::new((x as f32 + 0.5) * tile, (y as f32 + 0.5) * tile);
                    if center.y.abs() > bounds.half_size.y {
                        continue;
                    }
                    let shade = ((x * 17 + y * 31).rem_euclid(5)) as f32 * 0.006;
                    arena.spawn((
                        Sprite::from_color(
                            Color::srgb(0.115 + shade, 0.135 + shade, 0.135 + shade),
                            Vec2::splat(tile - 2.0),
                        ),
                        Transform::from_translation(center.extend(-20.0)),
                    ));
                }
            }
            // Crossed paths and a central seal are landmarks, not invisible obstacles.
            for size in [
                Vec2::new(bounds.half_size.x * 2.0, 120.0),
                Vec2::new(120.0, bounds.half_size.y * 2.0),
            ] {
                arena.spawn((
                    Sprite::from_color(Color::srgb(0.19, 0.18, 0.16), size),
                    Transform::from_xyz(0.0, 0.0, -19.0),
                ));
            }
            for radius in [100.0, 135.0] {
                arena.spawn((
                    ShapeBuilder::with(&shapes::Circle {
                        radius,
                        center: Vec2::ZERO,
                    })
                    .stroke((Color::srgb(0.38, 0.34, 0.23), 3.0))
                    .build(),
                    Transform::from_xyz(0.0, 0.0, -18.0),
                ));
            }
            let thickness = 64.0;
            let h = bounds.half_size;
            for (center, half) in [
                (
                    Vec2::new(h.x + thickness / 2.0, 0.0),
                    Vec2::new(thickness / 2.0, h.y + thickness),
                ),
                (
                    Vec2::new(-h.x - thickness / 2.0, 0.0),
                    Vec2::new(thickness / 2.0, h.y + thickness),
                ),
                (
                    Vec2::new(0.0, h.y + thickness / 2.0),
                    Vec2::new(h.x + thickness, thickness / 2.0),
                ),
                (
                    Vec2::new(0.0, -h.y - thickness / 2.0),
                    Vec2::new(h.x + thickness, thickness / 2.0),
                ),
            ] {
                arena.spawn((
                    Name::new("Courtyard wall"),
                    RigidBody::Fixed,
                    Collider::cuboid(half.x, half.y),
                    CollisionGroups::new(WORLD_GROUP, Group::GROUP_1 | Group::GROUP_2),
                    Sprite::from_color(Color::srgb(0.29, 0.29, 0.26), half * 2.0),
                    Transform::from_translation(center.extend(-2.0)),
                ));
            }
            for x in [-1.0, 1.0] {
                for y in [-1.0, 1.0] {
                    // Purely visual corner plinths stay out of the playable interior.
                    arena.spawn((
                        Sprite::from_color(Color::srgb(0.42, 0.37, 0.27), Vec2::splat(96.0)),
                        Transform::from_xyz(x * (h.x + 32.0), y * (h.y + 32.0), -1.0),
                    ));
                }
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
