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
    pub current: f32,
    pub maximum: f32,
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
