# Survivors Prototype

An occult-themed 2D survivors prototype built with Bevy 0.19, Rapier, and Lyon.
Survive ten minutes in a large, enclosed courtyard. Build a weapon from five
Magick Circle patterns while collecting experience and facing increasingly dense
waves. Death and victory both lead to a run summary and a fresh-run option.

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

## Controls

For the optional **Circle Lab** debug menu, run:

```sh
cargo run --locked --features dev-tools
```

Press **F1 while playing** to pause and edit circle builds, spawn targets, toggle
invulnerability/natural spawns, grant a level, skip to the next wave, or test the
victory flow. Apply the build, then resume with
F1, Escape, or the Resume button. See [composition and playtesting](design/CIRCLE-COMPOSITION.md)
for effect rules and the available controls.

- **WASD:** move.
- **Escape:** pause/resume.
- **Up/Down or W/S:** navigate menus.
- **Enter or Space:** confirm the selected upgrade or pattern.
- **Mouse click:** select a menu option.
- **Controller left stick:** move; **D-pad Up/Down:** navigate menus;
  **South face button** (A/Cross): confirm; **Start:** pause/resume.

The game starts directly in play. The camera follows the player through a
3,200 × 2,400 courtyard; Rapier walls contain players and enemies. Waves change
each minute, introducing Runners in minute two and Brutes in minute four.
Pause, level-up selection, and Circle Lab freeze the run clock.

Choose **Play again** after death or victory to reset the run, including XP,
weapon upgrades, enemies, gems, and debug overrides. Controller input has automated
coverage; a physical controller has not been verified. Circle Lab remains a
keyboard/mouse tool.

The encounter schedule and XP progression are an initial balance pass. A title
screen, saves, audio, and persistent progression remain future work. See the
[systems review](design/SYSTEMS-REVIEW.md) for the current architecture, changes,
and limitations, and [the design document](design/DESIGN.md) for the larger concept.

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
physics-backed wall movement, XP collection across pauses, wave progression,
and run endings/restarts. Rendering and layout also need a visual check after
engine or UI changes. See the systems review for the final validation record.

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
