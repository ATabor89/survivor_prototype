//! Optional presentation layer: short CC0 cues played through Bevy audio.
//! Register GameAudioPlugin only in the executable, not the headless game plugin.
use bevy::audio::{AudioSinkPlayback, Volume};
use bevy::prelude::*;

use crate::{combat::DamageApplied, components::Player, resources::GameState, GameplaySets};

/// User preferences survive run resets. Volume is linear, in the range 0..=1.
#[derive(Resource, Clone, Copy, Debug)]
pub struct AudioSettings {
    pub muted: bool,
    pub volume: f32,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            muted: false,
            volume: 0.25,
        }
    }
}

impl AudioSettings {
    fn effective_volume(self) -> f32 {
        if self.muted || !self.volume.is_finite() {
            0.0
        } else {
            self.volume.clamp(0.0, 1.0)
        }
    }
}

pub struct GameAudioPlugin;
impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioSettings>()
            .init_resource::<CueQueue>()
            .init_resource::<CueLimits>()
            .add_systems(Startup, load_sounds)
            .add_systems(
                Update,
                (
                    collect_damage_cues,
                    collect_encounter_cues,
                    play_cues,
                    update_playback,
                )
                    .chain()
                    .after(GameplaySets::Spawning)
                    .before(GameplaySets::Cleanup),
            )
            .add_systems(OnEnter(GameState::LevelUp), queue_state_cue)
            .add_systems(OnEnter(GameState::Treasure), queue_state_cue)
            .add_systems(
                OnEnter(GameState::Victory),
                (clear_sounds, queue_state_cue).chain(),
            )
            .add_systems(
                OnEnter(GameState::GameOver),
                (clear_sounds, queue_state_cue).chain(),
            )
            .add_systems(OnEnter(GameState::MainMenu), clear_sounds)
            .add_systems(OnEnter(GameState::Restarting), clear_sounds);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cue {
    EnemyHit,
    EnemyDefeated,
    PlayerHit,
    LevelUp,
    Treasure,
    Victory,
    GameOver,
    EncounterWarning,
}

impl Cue {
    const ALL: [Self; 8] = [
        Self::EnemyHit,
        Self::EnemyDefeated,
        Self::PlayerHit,
        Self::LevelUp,
        Self::Treasure,
        Self::Victory,
        Self::GameOver,
        Self::EncounterWarning,
    ];

    fn path(self) -> &'static str {
        match self {
            Self::EnemyHit => "audio/enemy_hit.ogg",
            Self::EnemyDefeated => "audio/enemy_defeated.ogg",
            Self::PlayerHit => "audio/player_hit.ogg",
            Self::LevelUp => "audio/level_up.ogg",
            Self::Treasure => "audio/treasure.ogg",
            Self::Victory => "audio/victory.ogg",
            Self::GameOver => "audio/game_over.ogg",
            Self::EncounterWarning => "audio/encounter_warning.ogg",
        }
    }

    fn gameplay(self) -> bool {
        matches!(
            self,
            Self::EnemyHit | Self::EnemyDefeated | Self::PlayerHit | Self::EncounterWarning
        )
    }

    fn priority(self) -> u8 {
        match self {
            Self::Victory | Self::GameOver | Self::LevelUp | Self::Treasure => 0,
            Self::PlayerHit => 1,
            Self::EncounterWarning => 2,
            Self::EnemyDefeated => 3,
            Self::EnemyHit => 4,
        }
    }

    fn gain(self) -> f32 {
        match self {
            Self::EnemyHit => 0.24,
            Self::EnemyDefeated => 0.34,
            Self::PlayerHit => 0.80,
            Self::EncounterWarning => 0.65,
            _ => 0.70,
        }
    }

    fn interval(self) -> f64 {
        match self {
            Self::EnemyHit => 0.10,
            Self::EnemyDefeated => 0.18,
            Self::PlayerHit => 0.20,
            Self::EncounterWarning => 1.0,
            _ => 0.10,
        }
    }
}

#[derive(Resource)]
struct SoundBank([Handle<AudioSource>; 8]);

fn load_sounds(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(SoundBank(Cue::ALL.map(|cue| assets.load(cue.path()))));
}

#[derive(Resource, Default)]
struct CueQueue(Vec<Cue>);

#[derive(Resource)]
struct CueLimits {
    next_allowed: [f64; 8],
}

impl Default for CueLimits {
    fn default() -> Self {
        Self {
            next_allowed: [f64::NEG_INFINITY; 8],
        }
    }
}

/// Includes pending sources, so unavailable audio devices cannot grow a queue.
#[derive(Component)]
struct SoundVoice {
    cue: Cue,
    age: f32,
    pending_age: f32,
}

fn collect_damage_cues(
    mut damage: MessageReader<DamageApplied>,
    state: Res<State<GameState>>,
    players: Query<&Transform, With<Player>>,
    mut queue: ResMut<CueQueue>,
) {
    if *state.get() != GameState::Playing {
        damage.clear();
        return;
    }
    let position = players
        .single()
        .ok()
        .map(|transform| transform.translation.truncate());
    let (mut hit, mut defeated, mut player_hit) = (false, false, false);
    for event in damage.read() {
        if event.amount == 0 {
            continue;
        }
        if event.is_player {
            player_hit = true;
        } else if position
            .is_some_and(|player| player.distance_squared(event.position) <= 900.0_f32.powi(2))
        {
            if event.lethal {
                defeated = true;
            } else {
                hit = true;
            }
        }
    }
    if player_hit {
        queue.0.push(Cue::PlayerHit);
    }
    // A cluster of simultaneous kills reads as one defeat, rather than a chord
    // of every target's impact plus death sound.
    if defeated {
        queue.0.push(Cue::EnemyDefeated);
    } else if hit {
        queue.0.push(Cue::EnemyHit);
    }
}

fn collect_encounter_cues(
    telegraphs: Query<(), Added<crate::waves::EncounterTelegraph>>,
    state: Res<State<GameState>>,
    mut queue: ResMut<CueQueue>,
) {
    if *state.get() == GameState::Playing && !telegraphs.is_empty() {
        queue.0.push(Cue::EncounterWarning);
    }
}

fn queue_state_cue(state: Res<State<GameState>>, mut queue: ResMut<CueQueue>) {
    let cue = match state.get() {
        GameState::LevelUp => Cue::LevelUp,
        GameState::Treasure => Cue::Treasure,
        GameState::Victory => Cue::Victory,
        GameState::GameOver => Cue::GameOver,
        _ => return,
    };
    queue.0.push(cue);
}

fn play_cues(
    mut commands: Commands,
    bank: Res<SoundBank>,
    settings: Res<AudioSettings>,
    time: Res<Time<Real>>,
    voices: Query<&SoundVoice>,
    mut queue: ResMut<CueQueue>,
    mut limits: ResMut<CueLimits>,
) {
    let volume = settings.effective_volume();
    let now = time.elapsed_secs_f64();
    let mut total = voices.iter().count();
    let mut gameplay = voices.iter().filter(|voice| voice.cue.gameplay()).count();
    let mut impacts = voices
        .iter()
        .filter(|voice| matches!(voice.cue, Cue::EnemyHit | Cue::EnemyDefeated))
        .count();
    queue.0.sort_by_key(|cue| cue.priority());
    queue.0.dedup();
    for cue in queue.0.drain(..) {
        let ambient_impact = matches!(cue, Cue::EnemyHit | Cue::EnemyDefeated);
        if volume == 0.0
            || total >= 8
            || (cue.gameplay() && gameplay >= 6)
            || (ambient_impact && impacts >= 4)
            || now < limits.next_allowed[cue as usize]
        {
            continue;
        }
        commands.spawn((
            AudioPlayer::new(bank.0[cue as usize].clone()),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume * cue.gain())),
            SoundVoice {
                cue,
                age: 0.0,
                pending_age: 0.0,
            },
        ));
        limits.next_allowed[cue as usize] = now + cue.interval();
        total += 1;
        gameplay += usize::from(cue.gameplay());
        impacts += usize::from(ambient_impact);
    }
}

fn update_playback(
    mut commands: Commands,
    state: Res<State<GameState>>,
    settings: Res<AudioSettings>,
    time: Res<Time<Real>>,
    mut voices: Query<(
        Entity,
        &mut SoundVoice,
        &mut PlaybackSettings,
        Option<&mut AudioSink>,
    )>,
) {
    for (entity, mut voice, mut playback, sink) in &mut voices {
        let paused = voice.cue.gameplay() && *state.get() != GameState::Playing;
        let volume = Volume::Linear(settings.effective_volume() * voice.cue.gain());
        // PlaybackSettings applies before sink creation; AudioSink controls a
        // source already playing. Updating both also covers asset-load delays.
        playback.paused = paused;
        playback.volume = volume;
        if let Some(mut sink) = sink {
            sink.set_volume(volume);
            if paused {
                sink.pause();
            } else {
                sink.play();
            }
        } else {
            voice.pending_age += time.delta_secs();
        }
        if !paused {
            voice.age += time.delta_secs();
        }
        // DESPAWN handles normal completion. These limits also reclaim sources
        // when a device or asset is unavailable, without replaying stale cues.
        if voice.age > 4.0 || voice.pending_age > 2.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn clear_sounds(
    mut commands: Commands,
    voices: Query<Entity, With<SoundVoice>>,
    mut queue: ResMut<CueQueue>,
    mut limits: ResMut<CueLimits>,
) {
    for entity in &voices {
        commands.entity(entity).despawn();
    }
    queue.0.clear();
    *limits = CueLimits::default();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_cues_decode_without_an_audio_device() {
        use bevy::audio::Decodable;
        for cue in Cue::ALL {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join(cue.path());
            let source = AudioSource {
                bytes: std::fs::read(path).unwrap().into(),
            };
            let samples: Vec<_> = source.decoder().collect();
            assert!(!samples.is_empty(), "Empty cue: {cue:?}");
            assert!(
                samples.len() < 44100 * 2 * 2,
                "Cue exceeds two seconds: {cue:?}"
            );
            assert!(samples.iter().all(|sample| sample.is_finite()));
            assert!(
                samples.iter().any(|sample| sample.abs() > 0.001),
                "Silent cue: {cue:?}"
            );
        }
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(State::new(GameState::Playing))
            .init_resource::<AudioSettings>()
            .init_resource::<CueQueue>()
            .init_resource::<CueLimits>()
            .insert_resource(SoundBank(std::array::from_fn(|_| Handle::default())));
        app
    }

    #[test]
    fn crowded_cues_are_coalesced_and_muted_cues_do_not_accumulate() {
        let mut app = app();
        app.add_systems(Update, play_cues);
        app.world_mut().resource_mut::<CueQueue>().0 = vec![Cue::EnemyHit; 200];
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&SoundVoice>()
                .iter(app.world())
                .count(),
            1
        );
        app.world_mut().resource_mut::<CueQueue>().0 = vec![Cue::EnemyHit; 200];
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&SoundVoice>()
                .iter(app.world())
                .count(),
            1
        );
        app.world_mut().resource_mut::<AudioSettings>().muted = true;
        app.world_mut()
            .resource_mut::<CueQueue>()
            .0
            .push(Cue::Victory);
        app.update();
        assert!(app.world().resource::<CueQueue>().0.is_empty());
        assert_eq!(
            app.world_mut()
                .query::<&SoundVoice>()
                .iter(app.world())
                .count(),
            1
        );
    }

    #[test]
    fn impact_cap_reserves_voices_for_player_and_reward_feedback() {
        let mut app = app();
        app.add_systems(Update, play_cues);
        for _ in 0..4 {
            app.world_mut().spawn(SoundVoice {
                cue: Cue::EnemyHit,
                age: 0.0,
                pending_age: 0.0,
            });
        }
        app.world_mut().resource_mut::<CueQueue>().0 =
            vec![Cue::EnemyHit, Cue::PlayerHit, Cue::Treasure];
        app.update();
        let cues: Vec<_> = app
            .world_mut()
            .query::<&SoundVoice>()
            .iter(app.world())
            .map(|voice| voice.cue)
            .collect();
        assert_eq!(cues.len(), 6);
        assert!(cues.contains(&Cue::PlayerHit));
        assert!(cues.contains(&Cue::Treasure));
    }

    #[test]
    fn pause_only_stops_gameplay_audio_and_reset_preserves_preferences() {
        let mut app = app();
        app.add_systems(Update, update_playback);
        let hit = app
            .world_mut()
            .spawn((
                SoundVoice {
                    cue: Cue::EnemyHit,
                    age: 0.0,
                    pending_age: 0.0,
                },
                PlaybackSettings::DESPAWN,
            ))
            .id();
        let reward = app
            .world_mut()
            .spawn((
                SoundVoice {
                    cue: Cue::LevelUp,
                    age: 0.0,
                    pending_age: 0.0,
                },
                PlaybackSettings::DESPAWN,
            ))
            .id();
        app.world_mut()
            .insert_resource(State::new(GameState::LevelUp));
        app.update();
        assert!(app.world().get::<PlaybackSettings>(hit).unwrap().paused);
        assert!(!app.world().get::<PlaybackSettings>(reward).unwrap().paused);
        app.world_mut()
            .insert_resource(State::new(GameState::Playing));
        app.update();
        assert!(!app.world().get::<PlaybackSettings>(hit).unwrap().paused);
        app.world_mut().resource_mut::<AudioSettings>().volume = 0.42;
        use bevy::ecs::system::RunSystemOnce;
        app.world_mut().run_system_once(clear_sounds).unwrap();
        assert_eq!(
            app.world_mut()
                .query::<&SoundVoice>()
                .iter(app.world())
                .count(),
            0
        );
        assert_eq!(app.world().resource::<AudioSettings>().volume, 0.42);
    }
}
