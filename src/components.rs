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
pub struct Health {
    pub current: i32,
    pub maximum: i32,
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