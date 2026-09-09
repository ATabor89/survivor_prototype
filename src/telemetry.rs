//! Actual health removed, excluding overkill, blocked hits, and props.
use crate::weapons::WeaponType;
use bevy::{platform::collections::HashMap, prelude::*};

#[derive(Resource, Default)]
pub struct RunTelemetry {
    pub damage_by_weapon: HashMap<WeaponType, u64>,
    pub hits_by_weapon: HashMap<WeaponType, u64>,
    pub damage_taken: u64,
}

impl RunTelemetry {
    pub fn record_weapon(&mut self, weapon: WeaponType, removed: u64) {
        let damage = self.damage_by_weapon.entry(weapon).or_default();
        *damage = damage.saturating_add(removed);
        let hits = self.hits_by_weapon.entry(weapon).or_default();
        *hits = hits.saturating_add(1);
    }
}
