use crate::menu::WeaponUpgradeConfirmedEvent;
use crate::weapons::weapon_upgrade::WeaponUpgradeChange;
use crate::weapons::{
    Area, AreaEffect, Attack, Damage, Lifetime, Orbits, Rotates, Sigil, WeaponArea,
    WeaponCooldown, WeaponDamage, WeaponMeta, WeaponMovement, WeaponType,
};
use bevy::color::Color;
use bevy::log::info;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::*;
use bevy_prototype_lyon::draw::Fill;
use bevy_prototype_lyon::entity::ShapeBundle;
use bevy_prototype_lyon::geometry::GeometryBuilder;
use bevy_prototype_lyon::prelude::RectangleOrigin;
use bevy_prototype_lyon::shapes;
use bevy_rapier2d::geometry::{ActiveEvents, Collider, CollisionGroups, Group, Sensor};

/// Specialized MagickCircle components
#[derive(Component)]
pub struct MagickCircle {
    pub patterns: Vec<PatternType>,
    pub num_sigils: u32,
}

#[derive(Component, Debug, Copy, Clone, Eq, PartialEq)]
pub enum PatternType {
    Protection,    // Basic defensive circle
    Binding,       // Slows/holds enemies
    Banishment,    // Damages/pushes enemies
    Invocation,    // Attracts/pulls enemies
    Manifestation, // Creates effects over time
}

impl std::fmt::Display for PatternType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PatternType::Protection => write!(f, "Protection"),
            PatternType::Binding => write!(f, "Binding"),
            PatternType::Banishment => write!(f, "Banishment"),
            PatternType::Invocation => write!(f, "Invocation"),
            PatternType::Manifestation => write!(f, "Manifestation"),
        }
    }
}

/// Spawns a magick circle weapon with default configuration
pub fn spawn_magick_circle(commands: &mut Commands, player_entity: Entity) {
    info!("Spawning magick circle for player: {:?}", player_entity);
    commands.entity(player_entity).with_children(|parent| {
        // Spawn the base weapon entity
        parent.spawn((
            // Core weapon components
            WeaponMeta {
                weapon_type: WeaponType::MagickCircle,
                level: 1,
            },
            WeaponCooldown {
                timer: Timer::from_seconds(3.5, TimerMode::Repeating),
                base_duration: 3.5,
                cooldown_bonus: 0,
            },
            WeaponDamage {
                base_amount: 10,
                damage_bonus: 0,
            },
            WeaponArea {
                base_radius: 64.0,
                area_bonus: 0,
            },
            // MagickCircle specific components
            MagickCircle {
                patterns: vec![PatternType::Banishment],
                num_sigils: 4,
            },
            // Optional modifiers
            AreaEffect {
                duration: 0.5,
                tick_rate: 0.5,
                last_tick: 0.0,
            },
            // Could add other modifiers like PiercingAttack or Knockback
            // based on configuration
        ));
    });
}

/// Helper function to spawn a magick circle attack
pub fn spawn_magick_circle_attack(
    commands: &mut Commands,
    center_pos: Vec3,
    damage: i32,
    radius: f32,
    pattern_type: PatternType,
    num_sigils: u32,
    offset_angle: Option<f32>,
) -> Entity {
    // Only calculate offset if angle is provided
    let spawn_pos = if let Some(angle) = offset_angle {
        let offset_distance = radius * 1.5;
        let offset = Vec3::new(
            angle.cos() * offset_distance,
            angle.sin() * offset_distance,
            0.0,
        );
        center_pos + offset
    } else {
        center_pos // No offset for centered circle
    };

    info!("Spawning attack at position: {:?}", spawn_pos);
    // First spawn the attack entity
    let attack_entity = commands
        .spawn((
            Attack,
            Lifetime {
                timer: Timer::from_seconds(3.0, TimerMode::Once),
            },
            Rotates {
                speed: 1.0,
                current_angle: 0.0,
            },
            Damage { amount: damage },
            Area { radius },
            AreaEffect {
                duration: 0.5,
                tick_rate: 0.5,
                last_tick: 0.0,
            },
            ShapeBundle {
                path: GeometryBuilder::new()
                    .add(&shapes::Circle {
                        radius,
                        center: Vec2::ZERO,
                    })
                    .build(),
                transform: Transform::from_translation(spawn_pos),
                ..default()
            },
            Fill::color(Color::srgba(0.5, 0.5, 1.0, 0.3)),
            Sensor,
            Collider::ball(radius),
            ActiveEvents::COLLISION_EVENTS,
            CollisionGroups::new(Group::GROUP_3, Group::GROUP_2),
            pattern_type,
            WeaponMovement::Stationary,
        ))
        .id();

    // Then spawn sigils as children of the attack
    let sigil_size = radius * 0.25;
    for i in 0..num_sigils {
        let sigil_entity = commands
            .spawn((
                Sigil {
                    index: i,
                    base_size: sigil_size,
                },
                Orbits {
                    radius,
                    speed: 1.0,
                    current_angle: (i as f32) * std::f32::consts::TAU / num_sigils as f32,
                },
                ShapeBundle {
                    path: GeometryBuilder::new()
                        .add(&shapes::Rectangle {
                            extents: Vec2::splat(sigil_size),
                            origin: RectangleOrigin::Center,
                            ..default()
                        })
                        .build(),
                    transform: Transform::default(),
                    ..default()
                },
                Fill::color(Color::srgba(0.7, 0.7, 1.0, 0.8)),
            ))
            .id();

        commands.entity(attack_entity).add_child(sigil_entity);
    }

    attack_entity
}

pub fn apply_magick_circle_weapon_upgrades(
    mut upgrade_events: EventReader<WeaponUpgradeConfirmedEvent>,
    mut weapon_query: Query<(&mut MagickCircle, &WeaponMeta)>,
) {
    for upgrade_event in upgrade_events.read() {
        // We already have the final `upgrade_spec` in `upgrade_event`
        for (mut circle, meta) in weapon_query.iter_mut() {
            if meta.weapon_type == upgrade_event.weapon_type {
                for change in &upgrade_event.upgrade_spec.changes {
                    match &change {
                        WeaponUpgradeChange::AddCircle { pattern } => {
                            info!(
                                "Adding new Magick Circle pattern: {:?} at level {}",
                                pattern, meta.level
                            );
                            circle.patterns.push(*pattern);
                        }
                        _ => (),
                    }
                }
            }
        }
    }
}
