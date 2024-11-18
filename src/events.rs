use bevy::prelude::*;

#[derive(Event)]
pub struct EntityDeathEvent {
    pub entity: Entity,
    pub position: Vec2,
    pub exp_value: Option<u32>,  // Only some entities give experience
}