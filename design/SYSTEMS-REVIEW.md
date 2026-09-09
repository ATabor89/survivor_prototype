# Systems review and playable-run baseline

This pass keeps the existing circle-composition rules and automatic weapon placement.
It addresses concrete state, input, progression, and physics issues, then adds a
bounded map and a complete ten-minute survival loop. It does not replace Bevy's ECS
or Rapier's collision handling with another framework.

## What changed

| Area | Change and reason |
| --- | --- |
| Movement and boundaries | Player motion now goes through Rapier's kinematic character controller. Fixed wall colliders contain the player and dynamic enemies; the controller handles sliding along walls. Camera limits affect framing, not collision response. |
| Contact damage | Damage-sensor groups explicitly target enemies. Damage handling uses a single target query and rejects non-positive damage and already-dying targets. Independent circle damage still stacks; the player's damage cooldown still limits repeated hits. |
| Experience | Orbs have velocity from spawn, and the vacuum writes it directly. Zero-distance attraction stays finite. Collection reads Rapier's current intersections rather than relying on short-lived collision-start messages, so a pause cannot strand an already-touching gem. |
| State ordering | Combat resolves before XP and run progress. A lethal hit wins over a simultaneous level-up or victory. The living-player checks prevent a level-up screen from opening after the player has died. |
| Menu input | A single activation pipeline follows the active menu's child order. It ignores unrelated buttons, detects mouse press edges, gives a click precedence over simultaneous keyboard confirmation, and requires fresh input for replacement menus. |
| Upgrade offers | Removed unimplemented Void Shards and Duration offers. Early menus may contain only a weapon upgrade and healing rather than a fake third choice. Descriptions now identify percentage bonuses and show cooldown reduction with the correct sign. |
| Weapon bounds and cleanup | Effective cooldown has a 0.1-second minimum. Removed inactive event/component scaffolding and the unused pickup counter; trimmed hot-path logging. Circle effects and overlap diagnostics remain separate. |
| Run lifetime | Per-run elapsed time replaces the global virtual clock in the HUD. Death and victory show a summary with Play again/Quit. Restart clears run entities, progression, pending gameplay messages, overlap tracking, spawn progress, and debug overrides. |

Damage and area upgrades remain percentage-based. Small damage increases can still
round down to the same integer hit value; changing that progression is a balance
decision rather than an incidental refactor.

The inactive `src/weapon.rs` is still preserved as historical checkpoint material.
It is not declared in the Rust module tree and does not run alongside `src/weapons/`.
The original source/design checkpoint and `.restart-backups` archive remain
documented in [the migration record](DEPENDENCY-UPGRADE.md).

## Map and run structure

`src/arena.rs` owns a persistent 3,200 × 2,400 courtyard, its fixed Rapier walls,
visual floor landmarks, and the following camera. The orthographic view retains
a consistent minimum world view instead of zooming out to show the entire map.
The arena and camera survive a restart; the player and run contents are recreated.
Paths, floor tiles, and the central seal are visual landmarks. There are no hidden
interior obstacles or hand-written collision exceptions.

`src/run.rs` owns the 600-second survival target and run reset. Only active gameplay
advances the timer; pause, level-up menus, Circle Lab, and terminal screens freeze
it. A living player reaching ten minutes wins. Death takes precedence on a lethal
final frame. The game still starts directly in play.

`src/waves.rs` owns the schedule and enemy spawning. Enemies arrive outside the
camera view with a margin, inside the arena. Spawn candidates are checked against
Rapier's world/enemy colliders. Failed attempts are bounded and skipped; the
director does not force a spawn into a wall. Wave transitions are time-based and
do not wait for every enemy from the previous minute to die.

## Initial wave schedule

The `WAVES` table is the tuning source. Health scaling applies to newly spawned
enemies; surviving enemies retain their existing stats.

| Run minute | Spawn interval | Batch | Active-enemy cap | Health multiplier |
| --- | --- | --- | --- | --- |
| 1 | 2.0 s | 1 | 24 | 1.0× |
| 2 | 1.7 s | 2 | 36 | 1.0× |
| 3 | 1.5 s | 2 | 48 | 1.1× |
| 4 | 1.3 s | 3 | 60 | 1.2× |
| 5 | 1.1 s | 3 | 72 | 1.3× |
| 6 | 1.0 s | 4 | 84 | 1.4× |
| 7 | 0.9 s | 4 | 96 | 1.5× |
| 8 | 0.8 s | 5 | 108 | 1.6× |
| 9 | 0.7 s | 5 | 120 | 1.7× |
| 10 | 0.6 s | 6 | 132 | 1.8× |

Acolytes form the base population. Faster, lighter Runners enter the mixture in
minute two; slower, tougher Brutes enter in minute four. These use the same
pursuit behavior, with tint and size differences for recognition. Acolytes and
Runners award 10 XP; Brutes award 20 XP. They have no bespoke attacks or pathfinding.
The schedule is deliberately small and editable, not a procedural encounter system.

## Controls and fast checks

- WASD moves; Escape pauses/resumes.
- Up/Down or W/S navigates menus; Enter/Space confirms; a mouse click selects directly.
- Controller left stick moves; D-pad Up/Down navigates; South face button confirms;
  Start pauses/resumes. Circle Lab uses keyboard/mouse.
- With `--features dev-tools`, F1 while playing opens Circle Lab. Existing build,
  target, health, level, spawn, and invulnerability controls remain available.
- **Next wave** advances the run clock to the next minute for encounter checks;
  it leaves the clock unchanged once the final wave has started.
- **Test victory** exercises the real end-of-run transition without waiting ten minutes.
- **Play again** on either result screen resets the run and restores normal debug defaults.

The lab's **Clear arena** remains a playtest operation that preserves the current
build/progression; it is distinct from restarting a run.

## Verification

Validated on Windows with Rust 1.96.0:

- `cargo fmt -- --check` passed.
- `cargo test --locked`: 30 passed.
- `cargo test --locked --features dev-tools`: 32 passed.
- Strict Clippy (`--all-targets -- -D warnings`) passed both with and without
  `--features dev-tools`.
- `cargo build --locked --features dev-tools` passed.
- Native Vulkan smoke test on an NVIDIA RTX 3090: courtyard and HUD rendering,
  live enemy pursuit/circle damage/XP, Circle Lab pause and next-wave control,
  the victory screen, and keyboard Play again all worked. Restart visibly
  restored health, XP, kills, clock, first wave, and normal debug defaults.
  The playtest logs contained no warnings or errors.

The victory check used the Lab shortcut; it was not a full ten-minute balance run.

The automated regressions exercise the real game plugins and Rapier headlessly.
They cover the existing composition behavior plus menu ordering and mixed input,
consecutive pattern selection, controller input, finite orb attraction, paused
pickup collection, terminal-state priority, physics-backed movement, wave rules,
and restart cleanup. They are regression checks, not evidence of game balance.

## Remaining work and limits

- Play a complete ten-minute run and tune XP, weapon scaling, enemy mixes, caps,
  and late-run pressure. The initial schedule has not had that balance validation.
- Verify a physical controller and decide whether analog menu navigation, input
  rebinding, or controller support inside Circle Lab is useful.
- Add title/settings screens, sound, save data, and persistent progression when
  their gameplay requirements are clear.
- Expand map content and enemy behavior deliberately. The current courtyard has
  walls and landmarks, without interior obstacle navigation, bosses, objectives,
  or a map-selection system.
- Review long-session performance and other platforms separately. Headless tests
  and Windows smoke tests do not establish Linux/macOS/wasm support or performance
  at substantially larger enemy counts.
- Character art remains provisional; enemy variants currently share an atlas cell.
  The preserved legacy weapon file can be archived or removed in a later cleanup
  once its historical value is no longer needed.
