# Survivors Prototype

An occult-themed 2D survivors prototype built with Bevy 0.19, Rapier, and Lyon.
Build an inventory of Magick Circles, Inscribed Needles, Alchemical Vessels, and
passive items while collecting experience, breaking props for supplies, and
surviving authored enemy phases and boss encounters.
Three bounded stages offer arena, horizontal, and vertical layouts. Death and
victory both lead to a run summary and a fresh-run option.

## Build and run

Use Rust **1.95 or newer** with the Windows MSVC toolchain and Visual Studio C++
build tools. The dependency migration was built and tested with Rust **1.96.0**.
Run commands from the repository root so the game can find `assets/`:

```sh
cargo run --locked
```

For a direct `.exe` launch, set `BEVY_ASSET_ROOT` to the repository root or place
`assets/` beside the executable. `cargo run` supplies the project root automatically.

The first build takes longer because development dependencies are optimized.
Bevy's 2D and UI feature groups are enabled; audio and 3D can be enabled when
those systems are introduced.

The game launches in a resizable **1920 × 1080** window. The camera follows the
player rather than zooming out to fit the whole stage.

## Controls

- **WASD:** move.
- **Escape:** pause/resume.
- **Up/Down or W/S:** navigate menus.
- **Enter or Space:** confirm a menu selection; reveal/claim treasure rewards.
- **Mouse click:** select a menu option.
- **Controller left stick:** move; **D-pad Up/Down:** navigate menus;
  **South face button** (A/Cross): confirm; **Start:** pause/resume.

The game starts at expedition setup: choose a stage, seed, and starting weapon.
Import/export a run recipe through `run-config.ron` in the launch directory;
the Debug menu can also export the current build as a starting loadout.
Pause, level-up selection,
treasure reveals, and the paused Debug menu freeze the run clock. Chests reveal
weapon upgrades before they are claimed; confirm again to claim and resume.

Choose **Play again** after death or victory to reset the run, including XP,
weapon upgrades, enemies, gems, and debug overrides. Controller input has automated
coverage; a physical controller has not been verified. The Debug menu uses
keyboard/mouse.

## Debug menu

```sh
cargo run --locked --features dev-tools
```

**F1** opens the persistent Debug menu, including on the main menu. **F2** opens its
Weapons tab directly. The initial preference pauses the simulation; uncheck **Pause simulation**
to keep playing with the menu visible. F1, Escape, Start, or **Close menu** closes
the overlay. Text-widget focus blocks WASD while the live menu needs keyboard input.
Level-up, treasure, and result screens retain their own state while the debug
panel stays open. Its visibility, tab, and pause preference survive run changes.

| Tab | Controls |
| --- | --- |
| Run | Heal, grant a level, toggle natural spawns/invulnerability, skip a minute or the next authored phase, test victory, and trigger named encounters |
| Weapons | Circle patterns and presets, weapon/passive grants, apply builds, spawn stationary/moving test targets |
| World | Select a stage and restart, spawn test chests/healing/urns, show Rapier colliders, clear the arena |

Encounter requests made while paused remain queued until the simulation resumes.
**Next phase** uses the stage's authored timestamps; **Next wave** advances a minute
and stops before the stage's ending. **Test victory** uses that stage's duration.
Choosing a stage starts a fresh run. **Clear arena** preserves the current build
and progression. See [circle composition](design/CIRCLE-COMPOSITION.md) for effect rules.

## Builds and run recipes

Six weapon slots and six passive slots appear below health/XP, including empty
slots and item levels. Magick Circle takes one weapon slot. Level-ups can offer
new equipment while that category has room; owned and capped items are filtered
appropriately. Chests upgrade owned weapons/passives and fall back to gold for
fully capped builds. The pause screen shows effective stats, the full build,
run totals, and actual enemy damage by weapon.

Passive tuning is in [items.ron](assets/data/items.ron). Run recipes are versioned
and validated before use. Separate seeded wave, loot, and reward streams support
repeatable experiments; they are not a deterministic physics replay or mid-run save.
See [the build and recipe guide](design/BUILD-DIVERSITY.md) for mechanics, the
worldbuilding contract, configuration behavior, art provenance, and verification.

## Stages and content

| Stage | Layout | Playable size | Survival time |
| --- | --- | --- | --- |
| The Sealed Courtyard | Arena | 3,200 × 2,400 | 10 minutes |
| The Ash Processional | Horizontal | 12,800 × 560 | 9 minutes |
| The Bell Descent | Vertical | 960 × 11,200 | 8 minutes |

Rapier handles walls, obstacles, character sliding, and pickup overlaps. Enemy
phases have authored start times, spawn mixes, population targets, and caps.
Timed encounters announce themselves before spawning; some include a Warden and
a temporary ring of enemies. Defeating configured bosses drops a reward chest.
Breakable props can drop healing or gold, and authored pickups provide landmarks
worth visiting. Gold is recorded for the current run; there is no persistent shop
or currency progression yet.

Edit [assets/data/stages.ron](assets/data/stages.ron) and relaunch to change stages,
enemy definitions, encounter schedules, obstacles, pickups, and breakables without
editing Rust. Content is validated at startup. See [stage content and playtesting](design/STAGE-CONTENT.md)
for authoring details and the current verification record.

Encounter schedules and XP progression are an initial balance pass. Mid-run
saves, audio, and persistent progression remain future work. The
[systems review](design/SYSTEMS-REVIEW.md) records the earlier refactor baseline;
[the design document](design/DESIGN.md) preserves the larger concept.

## Verification

```sh
cargo fmt -- --check
cargo check --locked
cargo test --locked
cargo test --locked --features dev-tools
cargo clippy --locked --all-targets --all-features -- -D warnings
```

The tests run the game plugins and Rapier without a window/GPU. They cover
movement and pause, damage/death and XP pickup, circle effects and cleanup,
overlap membership and unique combinations, continuous Invocation pull and cleanup,
Binding expiry, Protection damage reduction, keyboard/mouse/controller menu flows,
physics-backed wall movement, XP collection across pauses, authored stages and
encounters, loot/treasure, debug pause/live behavior, and run endings/restarts.
Rendering and layout also need a visual check after engine or UI changes. See
the stage-content document for final integrated validation and remaining limits.

## Migration checkpoints

The dependency upgrade is merged into `magick_circle_pattern_select`, the baseline
for further work. Migration checkpoints remain on `codex/dependency-upgrade`. The original uncommitted
source/design work is preserved in commit `8e81ab2`. A byte-verified local ZIP of
the pre-migration files, including editor metadata and the untracked `nul` file,
is stored at `.restart-backups/before-dependency-upgrade.zip` and ignored by Git.
It is outside `target/` so `cargo clean` does not remove it.

See [the migration record](design/DEPENDENCY-UPGRADE.md) for versions, checkpoints,
validation, and remaining cleanup. The legacy `src/weapon.rs` is preserved but is
not part of the active Rust module tree.
