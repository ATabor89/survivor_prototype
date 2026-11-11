use bevy::image::TextureAtlasLayout;
use bevy::prelude::*;
#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum GameState {
    #[default]
    MainMenu,
    Settings,
    /// The peaceful hub between expeditions; see `crate::estate`.
    Estate,
    Playing,
    LevelUp,
    Treasure,
    Paused,
    #[cfg(feature = "dev-tools")]
    DebugTools,
    GameOver,
    Victory,
    Restarting,
    Quit,
}

#[derive(Resource, Default)]
pub struct GameStats {
    pub enemies_killed: u32,
    pub time_elapsed: f32,
    pub gold: u32,
    pub bosses_defeated: u32,
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
    pub player_layout: Handle<TextureAtlasLayout>,
    pub enemies_layout: Handle<TextureAtlasLayout>,
}
