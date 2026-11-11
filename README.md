# Survivors Prototype

An occult-themed 2D survivors prototype built with Bevy 0.20, Rapier, and Lyon.
Expeditions are survivors-style runs; between them, a small estate hub banks what
you bring home and spends it on upgrades that change later expeditions.

## The estate loop

Choose **Enter the estate** on the title screen. Walk with WASD, press **E** next
to a villager or station to use it, and **Escape** to back out. The **Map room**
plans and launches an expedition; its result screen returns you to the estate
with the haul banked. The **Workshop** spends gold, essence, and seals on
upgrades that unlock starting weapons or equip every expedition with a passive.
Progress is saved to `save.ron` in the launch directory; delete it to start over.

The rest of the title screen is a sandbox: any weapon, any seed, nothing banked.
See [the estate guide](design/ESTATE.md) for the loop, resources, and authoring
`assets/data/estate.ron`.

## Expeditions

Build an inventory of Magick Circles, Inscribed Needles, Alchemical Vessels, and
passive items while collecting experience, breaking props for supplies, and
surviving authored enemy phases and boss encounters.
Three bounded stages offer arena, horizontal, and vertical layouts. The Courtyard
culminates in a final boss at ten minutes; defeating it wins the run. The other
maps demonstrate timed survival. Death and victory both lead to a run summary.

## Build and run

Use Rust **1.97.1 or newer** (Bevy 0.20's minimum; run `rustup update` if needed)
with the Windows MSVC toolchain and Visual Studio C++ build tools. The Bevy 0.20
migration was built and tested with Rust **1.99.1**.
Run commands from the repository root so the game can find `assets/`:

```sh
cargo run --locked
```

For a direct `.exe` launch, set `BEVY_ASSET_ROOT` to the repository root or place
`assets/` beside the executable. `cargo run` supplies the project root automatically.

The first build takes longer because development dependencies are optimized.
Bevy's 2D, UI, and audio features are enabled. Short CC0 placeholder sounds come
from Kenney; [sound credits and licenses](assets/audio/README.md) are included.
Setup, pause, result screens, and debug tools offer mute/volume controls.

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
| Run | Heal, grant a level, toggle natural spawns/invulnerability, skip a minute or phase, spawn authored enemies, trigger encounters/finale, or bypass to victory |
| Weapons | Circle patterns and presets, weapon/passive grants, apply builds, spawn stationary/moving test targets |
| World | Select a stage and restart, spawn test chests/healing/urns, show Rapier colliders, clear the arena |

Encounter requests made while paused remain queued until the simulation resumes.
**Next phase** uses the stage's authored timestamps; **Next wave** advances a minute
and stops before the stage's ending. **Start final boss encounter** advances to
the authored boss objective and waits for simulation resume. **Test victory**
explicitly bypasses the objective.
Choosing a stage starts a fresh run. **Clear arena** preserves the current build
and progression. See [circle composition](design/CIRCLE-COMPOSITION.md) for effect rules.

## Builds and run recipes

The roster now includes eleven weapons. See the [occult arsenal guide](design/OCCULT-ARSENAL.md)
for the eight new automatic behaviors, upgrade rules, limitations, and focused playtests.
Every weapon can be selected at run setup or acquired through level-ups and F2 debug tools.

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
| The Sealed Courtyard | Arena | 3,200 × 2,400 | Final boss at 10 minutes |
| The Ash Processional | Horizontal | 12,800 × 560 | 9 minutes |
| The Bell Descent | Vertical | 960 × 11,200 | 8 minutes |

Rapier handles walls, obstacles, character sliding, and pickup overlaps. Enemy
phases have authored start times, spawn mixes, population targets, and caps.
Timed encounters announce themselves before spawning; some include a Warden and
a temporary ring of enemies. Defeating configured bosses drops a reward chest.
Breakable props can drop healing or gold, and authored pickups provide landmarks
worth visiting. Gold from an estate expedition is banked when the run ends.

Edit [assets/data/stages.ron](assets/data/stages.ron) and relaunch to change stages,
enemy definitions, encounter schedules, obstacles, pickups, and breakables without
editing Rust. Content is validated at startup. See [stage content and playtesting](design/STAGE-CONTENT.md)
for authoring details. The [combat encounter guide](design/COMBAT-ENCOUNTERS.md)
documents map win conditions, enemy attacks, pacing, and combat feedback.

Encounter schedules and XP progression are an initial balance pass. Mid-run
saves remain future work. The
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

## Project history

`main` is the development baseline. The [design index](design/README.md) says which
documents describe the current game and which are historical records, including
[the dependency migration record](design/DEPENDENCY-UPGRADE.md).

Every push and pull request runs formatting, strict Clippy, and both test
configurations in [GitHub Actions](.github/workflows/ci.yml).
