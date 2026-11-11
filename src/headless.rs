//! The real game plugins and Rapier without a window, GPU, or audio device.
//!
//! Regression tests and the autoplay simulator both run the game this way, so
//! what they exercise is the same code a player runs. Time advances by a fixed
//! step per update, which makes a run as fast as the CPU allows.
use bevy::asset::AssetPlugin;
use bevy::gizmos::GizmoPlugin;
use bevy::mesh::MeshPlugin;
use bevy::prelude::*;
use bevy::scene::ScenePlugin;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use bevy::transform::TransformPlugin;
use std::time::Duration;

/// Simulated time per update.
pub const STEP_SECONDS: f64 = 1.0 / 60.0;

/// The game with every gameplay plugin, ready for more plugins to be added.
/// Call [`finish`] before the first update.
pub fn build() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: format!("{}/assets", env!("CARGO_MANIFEST_DIR")),
            ..default()
        },
        TransformPlugin,
        StatesPlugin,
        MeshPlugin,
        ScenePlugin,
        GizmoPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<TextureAtlasLayout>()
    .init_resource::<ButtonInput<KeyCode>>()
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        STEP_SECONDS,
    )))
    .add_plugins(crate::SurvivorsGamePlugin);
    app
}

/// Complete plugin setup; the app can be updated afterwards.
pub fn finish(app: &mut App) {
    app.finish();
    app.cleanup();
}
