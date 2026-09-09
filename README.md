# Survivors Prototype

An occult-themed 2D survivors prototype built with Bevy 0.19, Rapier, and Lyon.
The current loop includes enemy waves, experience and leveling, five Magick Circle
patterns, weapon upgrades, and pause/selection menus.

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

- **WASD:** move.
- **Escape:** pause/resume.
- **Up/Down or W/S:** navigate menus.
- **Enter or Space:** confirm the selected upgrade or pattern.
- **Mouse click:** select a menu option.

The game starts directly in play. A title screen, saves, audio, and game-over and
victory screens remain unfinished. See [the design document](design/DESIGN.md)
for the larger game concept.

## Verification

```sh
cargo fmt -- --check
cargo check --locked
cargo test --locked
```

The tests run the game plugins and Rapier without a window/GPU. They cover
movement and pause, damage/death and XP pickup, circle effects and cleanup,
overlap membership, and keyboard/mouse upgrade flows. Rendering and layout also
need a visual check after engine or UI changes. Some unused prototype scaffolding
still produces dead-code warnings.

## Migration checkpoints

The dependency upgrade is on `codex/dependency-upgrade`. The original uncommitted
source/design work is preserved in commit `8e81ab2`. A byte-verified local ZIP of
the pre-migration files, including editor metadata and the untracked `nul` file,
is stored at `.restart-backups/before-dependency-upgrade.zip` and ignored by Git.
It is outside `target/` so `cargo clean` does not remove it.

See [the migration record](design/DEPENDENCY-UPGRADE.md) for versions, checkpoints,
validation, and remaining cleanup. The legacy `src/weapon.rs` is preserved but is
not part of the active Rust module tree.
