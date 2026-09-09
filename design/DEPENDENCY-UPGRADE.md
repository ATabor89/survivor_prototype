# Project restart and dependency upgrade

Assessment date: 2026-09-08. The assessment below records the **pre-migration** checkout. Implementation progress is recorded at the end of this document.

## Current checkout

- Branch: `magick_circle_pattern_select`, tracking the same branch on `origin` (remote was not fetched during this assessment).
- Latest commit: `bf62d76`, 2026-02-11, “Updating patterns to spawn different circles with different powers”.
- Existing tracked changes: `src/weapons/mod.rs`, integrating circle-overlap tracking.
- Existing untracked paths: `.claude/`, `.idea/`, `design/`, `nul`, `src/weapon.rs`, and `src/weapons/overlap.rs`.
- Active application: one Rust binary, 17 source files and approximately 3,733 lines including comments and whitespace.
- `src/weapon.rs` is an additional 1,018-line legacy implementation that is not declared by `main.rs`; it is outside the active module tree. Preserve it until its intended disposition is decided, but do not migrate it as active code.
- No repository tests, CI workflows, or pinned Rust toolchain were found. Installed Rust/Cargo: 1.96.0. Package edition: 2021.

## Baseline validation

`cargo check --locked` passed with Rust 1.96.0 on Windows, including the uncommitted overlap module. The first offline attempt could not retrieve the fork; the subsequent online check successfully fetched the locked Git revision and missing crates. No manifest or lockfile changes were needed.

The application generated 27 warnings: mostly unused prototype types/fields, two unused imports, and deprecated `Color::rgba`. Compilation was checked; linking, launching, and interactive gameplay were not tested during this assessment. Existing source changes were preserved; the only project file added by this assessment is this document.

## Implemented scope

The source implements movement, enemy waves and chasing, collision damage, XP drops and collection, leveling, pause and upgrade menus, a health/timer/kill HUD, and one weapon family. Magick Circle has five patterns: Banishment, Binding, Invocation, Manifestation, and Protection. Circle upgrades can open a pattern-selection submenu, with keyboard and mouse interaction.

Overlap bookkeeping is integrated into the uncommitted weapon changes. Combo effects are not implemented. Main menu, settings, game-over/victory screens, saves, audio, additional weapons, hub/expeditions, and meta-progression remain future work or placeholders.

`DESIGN.md` and `project-summary.md` are useful context, but some implementation claims in the design document have drifted:

- `GameStats::victory_threshold` is initialized to 200 but never read by active code; there is no implemented victory trigger.
- Damage cooldown is stored on the target entity, rather than separately for each damage-source type.
- Death events originate in `death_system`; XP orbs are spawned by `experience.rs`.
- Movement uses `player.speed`; 150 is its initial value, not a hard-coded speed in the movement system.

## Dependency targets

Versions below were checked against published package documentation on the assessment date. These are migration targets, not a tested dependency resolution for this project.

| Dependency | Current lockfile | Published target | Notes |
| --- | --- | --- | --- |
| Bevy | 0.15.0 | 0.19.1 | Four engine release transitions |
| bevy_rapier2d | Git fork reporting 0.27.0, commit `926a8bfc` | 0.36.0 | Published release depends on Bevy 0.19; remove the temporary fork |
| bevy_prototype_lyon | 0.13.0 | 0.17.0 | Published release depends on Bevy 0.19 |
| rand | 0.8.5 | 0.10.2 | Separate RNG migration after the engine/plugins |
| strum / strum_macros | 0.26.3 / 0.26.4 | 0.28 / 0.28 | Can use `strum`'s `derive` feature to consolidate imports |

Sources: [Bevy](https://docs.rs/crate/bevy/0.19.1), [Rapier](https://docs.rs/crate/bevy_rapier2d/0.36.0), [Lyon](https://docs.rs/crate/bevy_prototype_lyon/0.17.0), [Rand](https://docs.rs/crate/rand/0.10.2), [Strum](https://docs.rs/strum/0.28.0/strum/).

## Migration work mapped to this code

| Area | Local impact | Migration work |
| --- | --- | --- |
| Query/error APIs | Gameplay, UI, weapons, physics | Replace removed `get_single*` APIs and handle fallible `single*`/entity commands explicitly |
| Entity relationships | Menus, weapon ownership, damage sensors, cleanup | Migrate `Parent`, `ChildBuilder`, hierarchy imports, and recursive despawning |
| Buffered events | Damage, deaths, upgrades, weapon effects, collisions, exit | Migrate application event registration/readers/writers to messages; preserve delivery and scheduling across state changes |
| Collections | Weapon configuration and overlap bookkeeping | Replace `bevy::utils` collection imports |
| Rapier | `physics.rs`, `systems.rs`, `experience.rs`, `weapons/mod.rs` | Migrate context access/configuration and changed velocity APIs; validate sensor intersections, XP collision messages, zero gravity, and pause behavior |
| Lyon | `weapons/magick_circle.rs` | Migrate `ShapeBundle`/`GeometryBuilder` construction to the current shape API; compare all five patterns visually |
| UI/text | `menu.rs`, `ui.rs`, `upgrade.rs` | Update font/border/layout APIs and inspect wrapping, scrolling, focus, and hierarchy cleanup |
| RNG | `upgrade.rs`, `systems.rs` | Update RNG entry points/traits and sampling calls while preserving upgrade-choice behavior |

Bevy migration guides: [0.15 → 0.16](https://bevy.org/learn/migration-guides/0-15-to-0-16/), [0.16 → 0.17](https://bevy.org/learn/migration-guides/0-16-to-0-17/), [0.17 → 0.18](https://bevy.org/learn/migration-guides/0-17-to-0-18/), [0.18 → 0.19](https://bevy.org/learn/migration-guides/0-18-to-0-19/). Also consult the [Rapier changelog](https://github.com/dimforge/bevy_rapier/blob/master/CHANGELOG.md) and [Lyon examples and compatibility table](https://github.com/rparrett/bevy_prototype_lyon).

## Existing behavior to track separately

1. Overlap tracking clears each frame, then records only circles whose damage tick is due. Consequently, physically overlapping circles with staggered ticks may never be recorded together. Continuous spatial membership should be independent of damage timing if combos require persistent overlaps.
2. Physics setup only supplies zero gravity when `RapierConfiguration` is absent. If Rapier creates its own configuration, that path does not overwrite its gravity. Verify actual startup behavior before and after replacing the fork.
3. Damage/death, weapon effects, and XP processing are not all explicitly ordered relative to one another. Compilation cannot establish that frame timing and message delivery remain correct during migration.
4. Debug builds log every detected overlap and many damage-processing steps at INFO level. This may obscure useful migration diagnostics.

These are source-review observations, not results of an interactive playtest.

## Suggested implementation sequence

1. Establish the baseline: run the locked dependency check and a gameplay smoke test. Preserve the current tracked and untracked work in a deliberate checkpoint before migration commits; avoid indiscriminately staging editor directories or `nul`.
2. Replace the Rapier fork with a published Bevy-0.15-compatible release as a small first checkpoint. Rapier 0.28 is the first published 0.15 release; verify the chosen patch and compatibility before editing the manifest.
3. Move through Bevy 0.16, 0.17, 0.18, and 0.19 with compatible Rapier/Lyon releases at each checkpoint. Apply each release's migration guide and check compilation before advancing. This makes physics or UI regressions easier to locate.
4. Update Rand and Strum separately. Review lockfile changes and inspect `cargo tree -d` for unintended duplicate Bevy versions.
5. Record a tested toolchain, add concise build/run instructions, and introduce focused regression tests for overlap membership and upgrade/state transitions. Consider reducing Bevy's default features to the required 2D/UI functionality as a separate build improvement.

## Verification checklist

- `cargo check --locked` succeeds on each migration checkpoint.
- Build and launch the final binary with all sprites loading and no missing-resource, query-conflict, or render errors.
- WASD movement, enemy pursuit, contact damage, death, XP attraction and collection work.
- Each of the five circle patterns renders, orbits, expires, and applies its intended effect.
- Level-up selection, adding a circle, pattern selection, healing, and continued play work with keyboard and mouse; no duplicate confirmation or missing upgrade.
- Pause/resume and level-up menus freeze/resume both virtual time and physics; HUD does not duplicate on re-entry.
- Test overlap behavior with circles whose effect ticks are deliberately offset, distinguishing the known baseline limitation from migration regressions.
- Run targeted tests and formatting checks on the completed migration; document remaining warnings and deferred gameplay issues.

## Implementation record

The user approved the migration after the assessment. Work continues on `codex/dependency-upgrade`.

| Checkpoint | Result |
| --- | --- |
| `8e81ab2` | Preserved existing source, legacy weapon file, overlap work, and design files |
| `4975827` | Replaced the Git fork with published Rapier 0.28; compilation passed |
| `3ad2572` | Bevy 0.16.1 / Rapier 0.30 / Lyon 0.14.1; compilation passed |
| `85f4861` | Bevy 0.17.3 / Rapier 0.32 / Lyon 0.15; compilation passed |
| `a0887f9` | Bevy 0.18.1 / Rapier 0.33 / Lyon 0.16; compilation passed |
| `d23cd6b` | Bevy 0.19.1 / Rapier 0.36 / Lyon 0.17; compilation passed |

All 41 original project files outside `.git` and `target` were also archived and verified byte-for-byte. The archive is now `.restart-backups/before-dependency-upgrade.zip`, ignored by Git and outside Cargo's build directory. Editor metadata and `nul` were left in place. The original branch was not advanced.

Bevy 0.19.1 declares Rust 1.95 as its minimum. Installed Rust 1.96.0 satisfies that requirement; no toolchain update was needed. The manifest now records the minimum. Bevy is configured for 2D and UI rather than its full default 3D/audio feature set.

The lockfile was refreshed after the old `wide` 0.7.28 dependency failed to provide a SIMD transpose required by current Rapier. `wide` 0.7.33 resolved that compile failure. Rand is now 0.10.2 and Strum is 0.28 with its derive feature, replacing the separate direct macro dependency.

### Regression fixes

The six headless regression tests run the real game plugins and Rapier. Their initial failures exposed the following issues, which are now fixed:

- **Bevy 0.19 transform propagation:** Rapier's early transform pass needs `mark_dirty_trees` before it. Without this ordering, movement of the player (which has child entities) could be reset during physics writeback. The test verified that the local position changed before physics and was then overwritten, and now verifies movement survives the full frame.
- **Overlap membership:** Spatial membership is now collected every frame, independently of damage tick timing. The test offsets two circle timers, checks their overlap over multiple frames, verifies damage still ticks at 0.5 seconds, then checks cleanup when the enemy exits.
- **Protection:** Circle sensors now include kinematic/static collision detection so they can detect the kinematic player. Other collision groups and enemy detection remain in place.
- **Upgrade progression:** Progression index zero is the level-1-to-2 upgrade. The original index skipped that first damage/area upgrade and reached limit breaks early. Keyboard and mouse tests now exercise the intended progression.
- **Pattern selection:** The pending upgrade resource is removed after confirmation. The existing guard against consuming the opening keypress twice is retained and tested.
- **Diagnostics:** Per-frame overlap reports use DEBUG rather than INFO.

The suite also verifies pause/resume of time and physics, one XP orb/kill per enemy death, real XP sensor pickup, all five circle effects, recursive sigil cleanup, healing, and a single HUD after menu transitions. Combo effects themselves remain unfinished. The preserved legacy `src/weapon.rs` was not rewritten.

### Final validation

- `cargo test --locked`: 6 passed, including the default parallel test runner.
- `cargo build --locked`: passed for the normal Windows executable.
- Formatting and locked compilation checks passed; the dependency tree contains one Bevy version, 0.19.1.
- Native visual playtest on Windows/Vulkan: player and enemy sprites, HUD, pause/resume, level-up and five-option pattern menus rendered; keyboard confirmation and mouse upgrade selection worked.
- The first direct executable launch lacked the asset-root environment used by Cargo. Repeating with `BEVY_ASSET_ROOT` set to the repository root loaded the sprites; both corrected launch logs contained no asset or renderer errors. Direct launch instructions are in `README.md`.
- The visual pass also replaced unsupported emoji placeholders with ASCII symbols and replaced raw upgrade-struct debug text with readable change descriptions.
- There are 24 remaining dead-code warnings from prototype scaffolding. No dependency/API deprecation warnings remain in the application build.

Interactive inspection was a smoke test, not a full balance or long-duration playtest. Linux, macOS, WebAssembly, and release-profile builds were not verified. Larger cleanup, persistent progression, complete end-game flows, and circle combo design remain future work.
