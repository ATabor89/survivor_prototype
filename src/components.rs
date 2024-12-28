use bevy::prelude::*;

#[derive(Component)]
pub struct Player {
    pub speed: f32,
    pub magnet_strength: f32,
    pub magnet_speed: f32,
}

#[derive(Component)]
pub struct Enemy {
    pub speed: f32,
    pub experience_value: u32,
}

#[derive(Component)]
pub struct Projectile {
    pub speed: f32,
}

#[derive(Component)]
pub struct Health {
    pub current: i32,
    pub maximum: i32,
}

#[derive(Component)]
pub struct Combat {
    pub attack_damage: f32,
    pub attack_speed: f32,
    pub last_attack: f32, // Time tracker for attack cooldown
}

#[derive(Component)]
pub struct Experience {
    pub current: u32,
    pub level: u32,
}

#[derive(Component)]
pub struct ExperienceOrb {
    pub value: u32,
}

#[derive(Component)]
pub struct Vacuumable {
    pub base_speed: f32,
}

impl Default for Vacuumable {
    fn default() -> Self {
        Self {
            base_speed: 300.0, // Some reasonable default speed
        }
    }
}

/// Player-specific components that affect weapons
#[derive(Component)]
pub struct CooldownReduction {
    pub percent: f32, // e.g., 0.20 for 20% reduction
}

#[derive(Component)]
pub struct DamageMultiplier {
    pub factor: f32, // e.g., 1.5 for 150% damage
}

#[derive(Component)]
pub struct AreaMultiplier {
    pub factor: f32, // e.g., 1.2 for 120% area
}

#[derive(Component)]
pub struct Luck(pub i32);

impl Default for CooldownReduction {
    fn default() -> Self {
        Self { percent: 0.0 }
    }
}

impl Default for DamageMultiplier {
    fn default() -> Self {
        Self { factor: 1.0 }
    }
}

impl Default for AreaMultiplier {
    fn default() -> Self {
        Self { factor: 1.0 }
    }
}

impl Default for Luck {
    fn default() -> Self {
        Self(20)
    }
}

#[derive(Component)]
pub struct PlayerStats {
    pub level: u32,
    pub experience: u32,
    pub health: f32,
    pub speed: f32,
    pub attack: f32,
    pub defense: f32,
    pub luck: f32,
}

// Physics-related components
#[derive(Component, Default)]
pub struct IntendedMovement(pub Vec3);

#[derive(Component, Default)]
pub struct PhysicsBody {
    pub radius: f32,
    pub mass: f32,
}
