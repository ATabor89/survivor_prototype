# Authored stages, encounters, and rewards

The current combat and objective pass is documented in [Combat encounters](COMBAT-ENCOUNTERS.md): enemy attack behaviors, per-map timed/boss victory, health bars, feedback, and audio. The research below records the earlier prototype investigation; example phase values are illustrative rather than the current Courtyard balance.

## Research findings

The reference is the community-maintained Vampire Survivors Wiki, inspected September 9, 2026. These are documented mechanics, not a claim that we inspected the current game's source. The tables and descriptions can lag game updates.

Ordinary waves and special events are distinct. The documented director attempts spawns periodically, fills a minimum population when below it, and otherwise adds enemies from the active mix. It also has a separate global limit. That is more flexible than increasing a single batch count every minute. [Enemy spawning mechanics](https://vampire-survivors.fandom.com/wiki/Enemies)

A small Mad Forest sample illustrates the separation:

| Time | Ordinary mix | Minimum | Interval | Separate activity |
| --- | --- | --- | --- | --- |
| 0:00 | Red-eyed bats | 15 | 1.0 s | — |
| 1:00 | Zombies and bats | 30 | 1.0 s | Glowing Bat boss |
| 2:00 | Two bat variants | 50 | 0.5 s | Bat-swarm event with its own delay/repeats |
| 3:00 | Skeletons | 40 | 0.25 s | Glowing Bat boss |

These are reference values, not our balance settings. The examined table does not establish a universal acceleration at every `:30`. Our schedule therefore accepts arbitrary timestamps rather than imposing minute or half-minute slots. [Mad Forest wave table](https://vampire-survivors.fandom.com/wiki/Mad_Forest)

The Flower Wall event is documented as an ellipse of enemies creeping toward the player. I could confirm encirclement, but not a universal repeated expansion/contraction rule. Our pulsing formation is a deliberate prototype encounter inspired by the user's description. [Flower Wall event](https://vampire-survivors.fandom.com/wiki/Flower_Wall_%28event%29)

Destructibles use stage-specific spawn chances and caps. Some maps have candidate locations; others spawn around the player just outside the view. The documented system can replace an existing light source when at its cap, so revisiting a region need not leave it permanently empty. This supports treating replenishment separately from static map objects. [Light sources](https://vampire-survivors.fandom.com/wiki/Light_source)

Chests commonly award one upgrade, with three- and five-item outcomes; they generally upgrade owned weapons/items. Evolution eligibility is a separate mechanic with exceptions. Our initial treasure system uses owned weapon progression; evolutions remain deferred until their weapon requirements exist. [Treasure chests](https://vampire-survivors.fandom.com/wiki/Treasure_Chest)

## Authoring content

Edit `assets/data/stages.ron` and relaunch. The executable loads the file at runtime; changing stats, positions, schedules, or enemy references does not require rebuilding Rust. RON and Serde supply parsing rather than a custom file format.

Load order: an explicit `BEVY_ASSET_ROOT` project root, `assets/` beside the executable, then the source project's `assets/`. If none exists, bundled defaults are available with a warning. Invalid supplied data fails with its path and a useful parse/validation error instead of silently selecting different content.

`GameContent` separates a shared enemy catalogue from stage definitions:

- Enemies have stable IDs, names, health, speed, appearance tint/size, and XP. Several stages can reference the same ID.
- Stages define their name, duration, dimensions, travel layout, obstacles, landmarks, fixed pickups, breakable theme/settings, spawn phases, and timed encounters.
- A phase specifies `start`, `interval`, `batch`, `minimum`, `cap`, `health_scale`, and weighted enemy references. Phases are sorted and can start at any second.
- An encounter specifies a timestamp, enemy/count, chest reward flag, and an optional ring formation. Ring data supplies its enemy/count, radius, pulse fraction, period, and duration.

For example, this phase begins exactly at 2:30:

```ron
(start: 150.0, interval: 1.2, batch: 3, cap: 54, minimum: 12,
 health_scale: 1.15,
 enemies: [(id: "acolyte", weight: 3), (id: "runner", weight: 3)]),
```

Validation rejects unknown enemy IDs, empty pools, duplicate IDs, non-finite or non-positive rates, unsorted/out-of-run timestamps, invalid counts, misplaced obstacles, and blocked starting/pickup positions. Fixed-size vectors use RON tuples, e.g. `half_size: (1600.0, 1200.0)`.

## Runtime responsibilities

`arena.rs` builds stage geometry and camera framing. Walls and authored obstacles are fixed Rapier colliders. Decorative tiles and landmarks have no invisible collision. Stage changes rebuild geometry through the normal restart path. The current camera survives; run entities, drops, messages, encounter history, and debug overrides reset.

`waves.rs` selects the active phase and samples weighted enemy definitions. The population minimum and cap are separate. At most 32 regular enemies are added per spawn attempt, so a large deficit is filled over multiple attempts. Far-away ordinary enemies are retired without rewards to avoid exhausting the cap behind the player; bosses and formation members are not treated as ordinary spawns.

Normal spawns use offscreen edges appropriate to the map: all sides in the courtyard, left/right in the processional, and top/bottom in the descent. Rapier checks candidate positions against colliders. Camera framing leaves spawn room even for extreme window aspect ratios. Failed placement attempts are bounded and skipped.

Timed encounters fire once per run, independently of normal spawn intervals/caps. A two-second gold outline warns of arrival. Placement rechecks current Rapier collisions, including the player. Formation members steer toward a pulsing ellipse with normal Rapier velocities; no custom collision resolver or position clamping moves actors through obstacles. Root and pull effects still influence them. A ring expires on its authored timeout or when its leader dies. Obstacles can leave gaps in a ring when individual placements are blocked.

`loot.rs` owns breakables, pickups, and treasure. Props are fixed sensors, so they are damageable clutter rather than additional movement blockers. Banishment and Manifestation damage them through Rapier intersections; they do not receive enemy-only effects or count as kills/XP. Static obstacles remain solid.

Props replenish outside the camera, retire when far away, and use per-stage caps, health, drop chance, names, and timing. The first drop table is 35% healing/65% gold, followed by the stage's drop-chance roll. Loose healing/gold drops have a 96-item safety cap; boss chests are retained. Healing restores up to 20 health and gold increases the run total. Static healing/gold/chest placements are supported independently.

Bosses marked for chest drops leave one chest. Touching it pauses play and reveals one, three, or five upgrades (70%/25%/5% prototype odds). Confirming early reveals the list; confirming the revealed list claims it once. Rewards follow each owned weapon's actual configured next levels, including randomly selected circle patterns, so a chest does not skip circle-add progression. Multiple rewards simulate successive levels before being applied through the normal weapon-upgrade messages.

## Playable examples and tools

The Sealed Courtyard remains the default ten-minute run. The Ash Processional is a long horizontal, nine-minute test stage; The Bell Descent is a long vertical, eight-minute test stage. These are intentionally finite examples, not an endless streaming implementation. They include solid obstacles, landmarks, and distant authored pickups.

The window launches at 1920×1080. With `--features dev-tools`:

- F1 opens the general Debug menu; F2 jumps straight to Weapons in the same panel.
- Run, Weapons, and World are one-click tabs. Opening pauses by default. Uncheck **Pause simulation** to leave the panel open during play.
- Run can heal, grant a level, toggle natural spawns/invulnerability, advance a minute or authored phase, test victory, or queue a named encounter. Queued encounters survive pausing and begin when simulation resumes.
- World selects a stage (fresh run), displays colliders, clears enemies/effects, and creates a test chest, healing pickup, or urn beside the player.
- Escape/F1/Start closes the panel. Level-up, treasure, death, and victory screens take input priority. Egui keyboard focus prevents typing from moving the player.

## Validation

Automated checks pass with 43 tests in the normal build and 52 with `dev-tools`. Both configurations pass `cargo clippy --all-targets -- -D warnings`; formatting and the native debug build also pass. Coverage includes validated content, phase transitions, corridor spawn directions and camera framing, Rapier obstacle collisions, encounter pause/cleanup, breakable damage and population caps, pickup collection, chest progression, and debug input/state transitions.

Native Windows/Vulkan playtesting confirmed the 1920×1080 client size, paused and live debug controls, F2 weapon access, boss warning and pulsing formation, boss health display, and ring removal on boss death. Collecting that boss's chest opened the treasure overlay and claiming it added the configured next circle. Switching to each corridor stage reset the run and framed both boundaries across its short axis. No runtime errors appeared in the playtest log. These checks supplement the earlier full-run victory and death/restart playtests; they are not a balance assessment of every encounter.

## Boundaries of this pass

No new weapon family, evolution recipes, character-unlock/coffin system, persistent currency, audio, pathfinding, or infinite terrain streaming was added. The schemas support shared enemies, per-stage schedules and placed content; adding a genuinely new behavior still requires an explicit implementation. Enemies can accumulate against obstacles because pursuit remains simple. Art uses the existing character atlas and small code-authored placeholder sprites for props and drops. Chest timing, boss difficulty, drop odds, and the alternate stages need ordinary playtesting and balance work.
