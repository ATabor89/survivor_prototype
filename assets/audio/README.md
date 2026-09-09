# Sound credits and provenance

These eight Ogg Vorbis sounds were created by **Kenney** and downloaded from the
official Kenney website on 2026-09-09. Both source packs are **Creative Commons
Zero (CC0 1.0)**. The pack-specific license files are preserved verbatim beside
this document. Attribution is optional; we credit the creator here regardless.

- [Interface Sounds 1.0](https://kenney.nl/assets/interface-sounds), released 2020.
  [Original ZIP](https://kenney.nl/media/pages/assets/interface-sounds/fa43c1dd4d-1677589452/kenney_interface-sounds.zip).
  See `LICENSE-Kenney-Interface.txt`.
- [Impact Sounds 1.0](https://kenney.nl/assets/impact-sounds), released 2019.
  [Original ZIP](https://kenney.nl/media/pages/assets/impact-sounds/87b4ddecda-1677589768/kenney_impact-sounds.zip).
  See `LICENSE-Kenney-Impact.txt`.
- [CC0 1.0 public-domain dedication](https://creativecommons.org/publicdomain/zero/1.0/).

Files are copied unmodified from each archive's `Audio/` directory and renamed
for their in-game cue. There is no synthesized audio, recompression, trimming, or
external runtime download. The selection totals 85,252 bytes; each clip is under
0.65 seconds at 44.1 kHz.

| Local file | Pack | Original file | Cue |
| --- | --- | --- | --- |
| `enemy_hit.ogg` | Impact | `impactSoft_medium_000.ogg` | Nearby target takes damage |
| `enemy_defeated.ogg` | Impact | `impactPunch_medium_000.ogg` | Nearby target takes lethal damage |
| `player_hit.ogg` | Impact | `impactPunch_heavy_000.ogg` | Player takes damage |
| `level_up.ogg` | Interface | `confirmation_001.ogg` | Upgrade selection opens |
| `treasure.ogg` | Interface | `confirmation_002.ogg` | Treasure reward opens |
| `victory.ogg` | Interface | `confirmation_004.ogg` | Run victory |
| `game_over.ogg` | Interface | `error_003.ogg` | Run defeat |
| `encounter_warning.ogg` | Interface | `question_001.ogg` | Encounter warning appears |

## Playback

`src/game_audio.rs` uses Bevy's built-in `AudioPlayer`, `PlaybackSettings`, and
`AudioSink`. Register `GameAudioPlugin` only in the executable. Its decision and
decoder tests require no audio device. `AudioSettings` exposes a persistent mute
flag and linear volume, defaulting to 25%; each cue also has a quieter mix gain.

Only committed damage events produce impacts. Enemy impacts farther than 900
world units from the player are ignored. Simultaneous kills become one defeat
cue, superseding the ordinary hit cue that frame. Per-cue cooldowns and a cap of
eight voices prevent crowded scenes from creating an unbounded mix: ordinary
impacts may occupy four voices, gameplay cues six, leaving room for player,
encounter, and reward feedback. Muted or throttled cues are discarded.

Gameplay sounds pause during menu/reward states and resume with gameplay;
interface cues remain audible during those pauses. Result screens and run/title
resets clear existing voices and pending requests. Preferences survive resets.
Bevy despawns completed one-shots; a timeout also reclaims sources when an audio
device or asset is unavailable. No music, continuous loops, or per-frame attack
sound barrage is included. Listening balance remains a first pass.

## SHA-256

```text
7d3ba0bb5e60a11b5d3e558c141303dcf494256675fbf753c0d252d2cf0481e3  enemy_hit.ogg
486988aa2d6440ffc4c62a0e8ccf3c23673ba84424bd4723378d451b7255eb5c  enemy_defeated.ogg
b33a8f14068aec24ec69ba85e5e87fdc41228975f6a1a3e44a6e7d6fc3d9f8d8  player_hit.ogg
063564703b6094d70718a3e787a55cc9141611e4ecd6b6637f8828f79b4a8c3a  level_up.ogg
33b17a9a9a2397c62b285c52c33a907fdffb476909c99e42dde603f6a7a8b12c  treasure.ogg
568967a3d9f8a8f6af54ea01729c4882284308f2a27d78c07ffd7ee0d6951661  victory.ogg
885b28175c7b511118bcffab95675b48bbac9005cf6514656ebdad979276b3ab  game_over.ogg
abb8f9e4eb2071491b0b15f84ee302386b1847609310c1b000ae174639a6cc5a  encounter_warning.ogg
```
