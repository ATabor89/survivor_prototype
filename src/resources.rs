use bevy::prelude::*;
use bevy::sprite::TextureAtlasLayout;
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
