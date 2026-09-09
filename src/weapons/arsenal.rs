//! Shared cast data and Rapier membership for the expanded occult arsenal.
use super::{WeaponArea, WeaponCooldown, WeaponDamage, WeaponMeta, WeaponType};
use crate::{
    components::Enemy,
    death::{MarkedForDeath, MarkedForDespawn},
    loot::{Breakable, BREAKABLE_GROUP},
};
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

#[derive(Clone, Copy)]
pub struct Cast {
    pub owner: Entity,
    pub weapon: Entity,
    pub kind: WeaponType,
    pub origin: Vec2,
    pub facing: Vec2,
    pub target: Option<(Entity, Vec2)>,
    pub damage: i32,
    pub radius: f32,
    pub level: u32,
}

pub type DamageTargets<'w, 's> = Query<
    'w,
    's,
    (),
    (
        Or<(With<Enemy>, With<Breakable>)>,
        Without<MarkedForDeath>,
        Without<MarkedForDespawn>,
    ),
>;

pub fn damage_shape(
    context: &RapierContext<'_>,
    shape: &Collider,
    position: Vec2,
    angle: f32,
    targets: &DamageTargets,
) -> Vec<Entity> {
    let mut hits = Vec::new();
    context.intersect_shape(
        position,
        angle,
        shape.raw.as_ref(),
        QueryFilter::default().groups(CollisionGroups::new(
            Group::GROUP_3,
            Group::GROUP_2 | BREAKABLE_GROUP,
        )),
        |entity| {
            if targets.contains(entity) {
                hits.push(entity);
            }
            true
        },
    );
    hits.sort_unstable();
    hits.dedup();
    hits
}

/// Last intentional movement direction, retained while stationary.
#[derive(Component)]
pub struct Facing(pub Vec2);
impl Default for Facing {
    fn default() -> Self {
        Self(Vec2::X)
    }
}

pub fn spawn_weapon(commands: &mut Commands, player: Entity, kind: WeaponType) {
    let (damage, radius, cooldown) = match kind {
        WeaponType::KnifeOfTheArt => (30, 110.0, 1.1),
        WeaponType::PlanetaryTalismans => (10, 20.0, 3.0),
        WeaponType::AngelicInscription => (36, 140.0, 3.8),
        WeaponType::CirculatingPelican => (20, 14.0, 2.6),
        WeaponType::ForgottenSigil => (32, 90.0, 1.5),
        WeaponType::BoundIntention => (16, 360.0, 1.3),
        WeaponType::EmanationLattice => (45, 280.0, 2.8),
        WeaponType::ResonantRecitation => (12, 150.0, 2.0),
        _ => unreachable!("Legacy weapons have their own constructors"),
    };
    commands.entity(player).with_children(|parent| {
        parent.spawn((
            Name::new(kind.to_string()),
            WeaponMeta {
                weapon_type: kind,
                level: 1,
            },
            WeaponDamage {
                base_amount: damage,
                damage_bonus: 0,
            },
            WeaponArea {
                base_radius: radius,
                area_bonus: 0,
            },
            WeaponCooldown {
                timer: Timer::from_seconds(cooldown, TimerMode::Repeating),
                base_duration: cooldown,
                cooldown_bonus: 0,
            },
        ));
    });
}

pub fn cast(commands: &mut Commands, cast: Cast) {
    match cast.kind {
        WeaponType::KnifeOfTheArt
        | WeaponType::PlanetaryTalismans
        | WeaponType::ResonantRecitation => super::rituals::cast(commands, cast),
        WeaponType::AngelicInscription | WeaponType::EmanationLattice => {
            super::inscriptions::cast(commands, cast)
        }
        WeaponType::CirculatingPelican
        | WeaponType::ForgottenSigil
        | WeaponType::BoundIntention => super::mobile::cast(commands, cast),
        _ => unreachable!("Legacy weapons have their own casts"),
    }
}
