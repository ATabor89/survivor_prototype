use bevy::prelude::*;
use bevy::sprite::TextureAtlasLayout;
use crate::menu::{UpgradeChoice, UpgradeType};
use crate::types::{EquipmentType, Rarity, StatType, WeaponType};

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum GameState {
    #[default]
    MainMenu,
    Settings,
    Playing,
    LevelUp,
    Paused,
    GameOver,
    Quit,
}

#[derive(Resource, Default)]
pub struct GameStats {
    pub enemies_killed: u32,
    pub time_elapsed: f32,
    pub victory_threshold: u32,
}

#[derive(Resource)]
pub struct SpawnTimer(pub Timer);

impl Default for SpawnTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(2.0, TimerMode::Repeating))
    }
}

#[derive(Resource)]
pub struct WaveConfig {
    pub max_enemies: u32,
    pub current_wave: u32,
}

impl Default for WaveConfig {
    fn default() -> Self {
        Self {
            max_enemies: 20,
            current_wave: 0,
        }
    }
}

// Resource to hold our sprite sheets and layouts
#[derive(Resource)]
pub struct GameTextures {
    pub player: Handle<Image>,
    pub enemies: Handle<Image>,
    pub projectiles: Handle<Image>,
    pub player_layout: Handle<TextureAtlasLayout>,
    pub enemies_layout: Handle<TextureAtlasLayout>,
    pub projectiles_layout: Handle<TextureAtlasLayout>,
}

#[derive(Resource)]
pub struct UpgradePool {
    weapons: Vec<(WeaponType, Rarity)>,
    equipment: Vec<(EquipmentType, Rarity)>,
    stats: Vec<(StatType, Rarity)>,
}

impl Default for UpgradePool {
    fn default() -> Self {
        Self::new()
    }
}

impl UpgradePool {
    pub fn new() -> Self {
        Self {
            weapons: vec![
                (WeaponType::Sword, Rarity::Common),
                (WeaponType::Axe, Rarity::Common),
                (WeaponType::Spear, Rarity::Uncommon),
                (WeaponType::Bow, Rarity::Uncommon),
                (WeaponType::Magic, Rarity::Rare),
            ],
            equipment: vec![
                (EquipmentType::Armor, Rarity::Common),
                (EquipmentType::Ring, Rarity::Uncommon),
                (EquipmentType::Amulet, Rarity::Rare),
                (EquipmentType::Boots, Rarity::Common),
                (EquipmentType::Gloves, Rarity::Uncommon),
            ],
            stats: vec![
                (StatType::Health, Rarity::Common),
                (StatType::Speed, Rarity::Common),
                (StatType::Attack, Rarity::Uncommon),
                (StatType::Defense, Rarity::Common),
                (StatType::Luck, Rarity::Rare),
            ],
        }
    }

    pub fn generate_choices(&self, count: usize) -> Vec<UpgradeChoice> {
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        let mut choices = Vec::new();

        while choices.len() < count {
            let choice = match rand::random::<f32>() {
                x if x < 0.4 => {
                    // Weapon choice
                    if let Some((ref weapon_type, ref rarity)) = self.weapons.choose(&mut rng) {
                        UpgradeChoice {
                            upgrade_type: UpgradeType::Weapon(weapon_type.clone()),
                            description: format!("Add a {} to your arsenal", weapon_type),
                            rarity: rarity.clone(),
                        }
                    } else {
                        continue;
                    }
                }
                x if x < 0.7 => {
                    // Equipment choice
                    if let Some((ref equipment_type, ref rarity)) = self.equipment.choose(&mut rng) {
                        UpgradeChoice {
                            upgrade_type: UpgradeType::Equipment(equipment_type.clone()),
                            description: format!("Equip {} for enhanced protection", equipment_type),
                            rarity: rarity.clone(),
                        }
                    } else {
                        continue;
                    }
                }
                _ => {
                    // Stat choice
                    if let Some((ref stat_type, ref rarity)) = self.stats.choose(&mut rng) {
                        UpgradeChoice {
                            upgrade_type: UpgradeType::Stat(stat_type.clone()),
                            description: format!("Increase your {} by 10%", stat_type),
                            rarity: rarity.clone(),
                        }
                    } else {
                        continue;
                    }
                }
            };
            choices.push(choice);
        }

        choices
    }
}