use crate::menu::WeaponUpgradeConfirmedEvent;
use crate::weapons::weapon_upgrade::WeaponUpgradeChange;
use crate::weapons::{
    Area, AreaEffect, Attack, Damage, Lifetime, Orbits, Rotates, Sigil, WeaponArea, WeaponCooldown,
    WeaponDamage, WeaponMeta, WeaponMovement, WeaponType,
};
use bevy::color::Color;
use bevy::log::info;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::*;
use bevy_prototype_lyon::entity::Shape;
use bevy_prototype_lyon::geometry::{ShapeBuilder, ShapeBuilderBase};
use bevy_prototype_lyon::prelude::RegularPolygonFeature;
use bevy_prototype_lyon::shapes;
use bevy_rapier2d::geometry::{
    ActiveCollisionTypes, ActiveEvents, Collider, CollisionGroups, Group, Sensor,
};
use strum::EnumIter;

/// Specialized MagickCircle components
#[derive(Component)]
pub struct MagickCircle {
    pub patterns: Vec<PatternType>,
    pub num_sigils: u32,
}

#[derive(Component)]
pub struct ManifestationCharge;

pub fn animate_manifestation_charge(
    circles: Query<(&AreaEffect, &Lifetime)>,
    mut seals: Query<(&ChildOf, &mut Transform), With<ManifestationCharge>>,
) {
    for (parent, mut transform) in &mut seals {
        if let Ok((effect, lifetime)) = circles.get(parent.parent()) {
            let erupting = effect.timer.just_finished()
                || (lifetime.timer.elapsed() >= effect.timer.duration()
                    && effect.timer.elapsed_secs() < 0.15);
            let scale = if erupting {
                1.0 + (0.15 - effect.timer.elapsed_secs()).max(0.0) * 2.0
            } else {
                0.1 + 0.9 * effect.timer.fraction()
            };
            transform.scale = Vec3::new(scale, scale, 1.0);
        }
    }
}

#[derive(Component, Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, EnumIter)]
pub enum PatternType {
    Banishment,    // Damages/pushes enemies
    Binding,       // Slows/holds enemies
    Invocation,    // Attracts/pulls enemies
    Manifestation, // Creates effects over time
    Protection,    // Basic defensive circle
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

/// Visual configuration for a pattern type — colors, sigil shape, motion.
pub struct PatternVisuals {
    /// RGBA fill color for the circle (the big translucent disc)
    pub circle_color: Color,
    /// RGBA fill color for the orbiting sigils
    pub sigil_color: Color,
    /// Number of sides for the sigil shape.
    /// 0 = 6-pointed star (custom polygon), 3+ = regular polygon.
    pub sigil_sides: u32,
    /// Orbit speed for sigils (negative = reverse direction)
    pub orbit_speed: f32,
    /// Rotation speed for the main circle (negative = reverse)
    pub rotation_speed: f32,
    /// Number of sigils orbiting the circle
    pub num_sigils: u32,
}

impl PatternType {
    pub fn get_description(&self) -> String {
        match self {
            PatternType::Protection => {
                "Creates a defensive circle that reduces incoming damage".to_string()
            }
            PatternType::Binding => "Roots enemies, lingering briefly after they leave".to_string(),
            PatternType::Banishment => "Damages enemies within the circle".to_string(),
            PatternType::Invocation => "Pulls enemies toward the circle's center".to_string(),
            PatternType::Manifestation => {
                "Charges for 1.25s, then erupts for triple damage".to_string()
            }
        }
    }

    /// Returns the visual configuration for this pattern type.
    pub fn visuals(&self) -> PatternVisuals {
        match self {
            // Red-orange — fire, destruction, aggressive
            PatternType::Banishment => PatternVisuals {
                circle_color: Color::srgba(0.9, 0.2, 0.1, 0.3),
                sigil_color: Color::srgba(1.0, 0.4, 0.1, 0.8),
                sigil_sides: 3,   // triangles — sharp, aggressive
                orbit_speed: 1.5, // fast orbit
                rotation_speed: 1.5,
                num_sigils: 4,
            },
            // Deep purple — arcane chains, restraint
            PatternType::Binding => PatternVisuals {
                circle_color: Color::srgba(0.5, 0.1, 0.7, 0.3),
                sigil_color: Color::srgba(0.7, 0.3, 1.0, 0.8),
                sigil_sides: 6,   // hexagons — stable, locking
                orbit_speed: 0.5, // slow, deliberate
                rotation_speed: 0.4,
                num_sigils: 6, // forming a cage
            },
            // Teal/cyan — summoning, otherworldly pull
            PatternType::Invocation => PatternVisuals {
                circle_color: Color::srgba(0.1, 0.7, 0.7, 0.3),
                sigil_color: Color::srgba(0.2, 0.9, 0.8, 0.8),
                sigil_sides: 5,    // pentagons — summoning geometry
                orbit_speed: -1.0, // reverse orbit — pulling inward
                rotation_speed: -0.8,
                num_sigils: 5,
            },
            // Green/lime — growth, creation, periodic effects
            PatternType::Manifestation => PatternVisuals {
                circle_color: Color::srgba(0.2, 0.7, 0.1, 0.3),
                sigil_color: Color::srgba(0.5, 0.9, 0.2, 0.8),
                sigil_sides: 0, // 6-pointed star — mystical
                orbit_speed: 0.8,
                rotation_speed: 1.2,
                num_sigils: 3,
            },
            // Silver-blue/white — wards, shields, purity
            PatternType::Protection => PatternVisuals {
                circle_color: Color::srgba(0.7, 0.8, 1.0, 0.35),
                sigil_color: Color::srgba(0.9, 0.95, 1.0, 0.85),
                sigil_sides: 4, // diamonds — solid, stable
                orbit_speed: 0.7,
                rotation_speed: 0.6,
                num_sigils: 4,
            },
        }
    }
}

/// Spawns a magick circle weapon with default configuration
pub fn spawn_magick_circle(commands: &mut Commands, player_entity: Entity, pickup_id: u8) {
    info!("Spawning magick circle for player: {:?}", player_entity);
    commands.entity(player_entity).with_children(|parent| {
        // Spawn the base weapon entity
        parent.spawn((
            // Core weapon components
            WeaponMeta {
                weapon_type: WeaponType::MagickCircle,
                level: 1,
                pickup_id,
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
            // Could add other modifiers like PiercingAttack or Knockback
            // based on configuration
        ));
    });
}

/// Builds a filled sigil shape based on the number of sides.
/// `sides == 0` produces a 6-pointed star; any other value produces a regular polygon.
fn build_sigil_shape(sigil_size: f32, sigil_sides: u32, color: Color) -> Shape {
    if sigil_sides == 0 {
        // 6-pointed star: alternating outer and inner vertices
        let outer_r = sigil_size;
        let inner_r = sigil_size * 0.5;
        let points: Vec<Vec2> = (0..12)
            .map(|i| {
                let angle = (i as f32) * std::f32::consts::TAU / 12.0 - std::f32::consts::FRAC_PI_2;
                let r = if i % 2 == 0 { outer_r } else { inner_r };
                Vec2::new(angle.cos() * r, angle.sin() * r)
            })
            .collect();
        ShapeBuilder::new()
            .add(&shapes::Polygon {
                points,
                closed: true,
            })
            .fill(color)
            .build()
    } else {
        ShapeBuilder::new()
            .add(&shapes::RegularPolygon {
                sides: sigil_sides as usize,
                center: Vec2::ZERO,
                feature: RegularPolygonFeature::Radius(sigil_size),
            })
            .fill(color)
            .build()
    }
}

/// Helper function to spawn a magick circle attack
pub fn spawn_magick_circle_attack(
    commands: &mut Commands,
    center_pos: Vec3,
    damage: i32,
    radius: f32,
    pattern_type: PatternType,
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

    let visuals = pattern_type.visuals();

    info!(
        "Spawning {:?} attack at position: {:?}",
        pattern_type, spawn_pos
    );
    // First spawn the attack entity
    let attack_entity = commands
        .spawn((
            Attack,
            Lifetime {
                timer: Timer::from_seconds(3.0, TimerMode::Once),
            },
            Rotates {
                speed: visuals.rotation_speed,
                current_angle: 0.0,
            },
            Damage { amount: damage },
            Area { radius },
            AreaEffect::new(pattern_type),
            ShapeBuilder::with(&shapes::Circle {
                radius,
                center: Vec2::ZERO,
            })
            .fill(visuals.circle_color)
            .build(),
            Transform::from_translation(spawn_pos),
            Sensor,
            Collider::ball(radius),
            ActiveEvents::COLLISION_EVENTS,
            // Protection circles must also detect the kinematic player.
            ActiveCollisionTypes::default() | ActiveCollisionTypes::KINEMATIC_STATIC,
            CollisionGroups::new(Group::GROUP_3, Group::GROUP_1 | Group::GROUP_2),
            pattern_type,
            WeaponMovement::Stationary,
        ))
        .id();

    // Then spawn sigils as children of the attack
    if pattern_type == PatternType::Manifestation {
        commands.entity(attack_entity).with_children(|parent| {
            parent.spawn((
                ManifestationCharge,
                ShapeBuilder::with(&shapes::Circle {
                    radius: radius * 0.85,
                    center: Vec2::ZERO,
                })
                .stroke((Color::srgb(0.7, 1.0, 0.3), 2.0))
                .fill(Color::srgba(0.5, 0.9, 0.2, 0.12))
                .build(),
                Transform::from_xyz(0.0, 0.0, 0.1).with_scale(Vec3::splat(0.1)),
            ));
        });
    }
    let sigil_size = radius * 0.25;
    let actual_num_sigils = visuals.num_sigils;
    for i in 0..actual_num_sigils {
        let sigil_entity = commands
            .spawn((
                Sigil {
                    index: i,
                    base_size: sigil_size,
                },
                Orbits {
                    radius,
                    speed: visuals.orbit_speed,
                    current_angle: (i as f32) * std::f32::consts::TAU / actual_num_sigils as f32,
                },
                build_sigil_shape(sigil_size, visuals.sigil_sides, visuals.sigil_color),
            ))
            .id();

        commands.entity(attack_entity).add_child(sigil_entity);
    }

    attack_entity
}

pub fn apply_magick_circle_weapon_upgrades(
    mut upgrade_events: MessageReader<WeaponUpgradeConfirmedEvent>,
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
                            circle.patterns.push(pattern.expect("Unable to add new Magick Circle pattern; upgrade event's pattern was `None`"));
                        }
                        _ => (),
                    }
                }
            }
        }
    }
}

pub struct PatternChoice {
    pattern: PatternType,
    description: String,
    // Maybe icon/visual indicator
}
