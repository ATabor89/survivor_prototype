# Builds, run recipes, and persistent tools

## Worldbuilding contract

Within this setting all magick is real. Traditions are different systems for perceiving and manipulating the same reality, not competing explanations that the game ranks as true or false. The original vision in `project-summary.md` and the historical dashboard remains the source for the larger setting: Solomonic, Alchemical, Kabbalistic, Chaos, Vedic, Enochian, Hermetic, and other traditions can all have their own tools, practices, and discoveries.

Attack behavior and tradition are separate concerns. A projectile can be produced by an inscribed object, an alchemical preparation, or a different practice without requiring three collision implementations. A tradition may also contain circles, projectiles, summons, wards, or other behaviors. Current weapon family labels are provisional descriptive categories: Ceremonial, Talismanic, and Alchemy. In particular, “Talismanic” describes the fictional Needles' technique; it does not assign this invented weapon to one exclusive historical tradition. These props and descriptions are game inventions, not claims of historical authenticity. Detailed tradition-specific weapon design remains future work.

## Inventory and progression

The player has six weapon slots and six separate passive slots. Magick Circle occupies one weapon slot regardless of how many component circles it produces. Empty slots and occupied item levels are visible below health/XP. There are currently three weapons and four passive items; unused slots leave room for future content.

Level-up offers sample eligible acquisitions and upgrades. Already-owned weapons cannot be acquired twice. A full category excludes new acquisitions in that category while retaining upgrades to owned equipment. Capped equipment is excluded. Healing fills otherwise unavailable level-up choices; chests fall back to gold when owned equipment is capped. Normal weapon progression ends at level eight; the existing uncapped stat-change machinery remains for explicit development tests rather than appearing in ordinary capped offers.

Passive definitions live in `assets/data/items.ron`. Values are cumulative per level and recalculated from captured base player stats, so refreshing an inventory cannot compound the same bonus repeatedly. The initial items are Pilgrim's Boots (movement), Lodestone (pickup range), Vital Relic (maximum health), and Hourglass (global cooldown). Maximum-health increases also restore the added capacity. Chest rewards can upgrade owned weapons and passives, including several consecutive levels, and apply through the normal reward pathways. Evolutions remain unimplemented.

The pause screen shows the build, effective player stats, run totals, and damage by weapon. Armor and regeneration currently remain zero; no item claims to provide either effect. Attack size is the global radius multiplier used by current weapons. Per-weapon damage telemetry counts actual enemy health removed, excluding overkill, blocked hits, and prop damage; control and protection contributions are not represented as damage.

## Weapons and physics

- **Magick Circle:** existing automatic placement and independent effect composition remain. The first upgrade now gives +20% damage and +10% radius, so its base ten-damage hit becomes twelve. Later upgrades retain circle additions with more noticeable stat increases.
- **Inscribed Needles:** automatically selects the nearest living enemy within targeting range and fires a fast projectile. One needle can damage three distinct targets; world obstacles stop it. Piercing membership is tracked per projectile so remaining inside an enemy cannot repeatedly hit it.
- **Alchemical Vessel:** automatically throws toward the nearest target's position. It bursts at landing or collision, damaging enemies and breakable props in its area. The enemy can move away before it lands.

Rapier moves the projectile bodies and supplies shape casts and burst intersection queries. CCD is enabled, and swept Rapier queries cover fast sensor crossings. No hand-written segment/circle hit geometry or collision resolver was introduced. World colliders explicitly interact with attack groups. Projectile lifetimes, hits, and movement stop during gameplay pauses and reset with the run.

New weapon types use the same acquisition, ownership, effective-stat, upgrade, and telemetry interfaces. Their small behavior implementations remain explicit Rust systems rather than a speculative scripting engine.

Upgrade cards show effective before/after damage, radius, and cooldown using the same calculations as attacks, including global passive modifiers.

## Run setup and recipes

The executable starts at a main menu. Select a stage and starting weapon, enter or randomize a seed, then begin. Selecting a starting weapon produces a fresh level-one build. A recipe can also describe several starting weapons, levels, passive items, and an explicit Circle pattern override for development loadouts.

Import/export uses `run-config.ron` in the launch directory. The file is ignored by Git. Exporting from setup saves the draft recipe; exporting the current build in the Debug menu captures the current owned equipment while retaining the run's seed and stage. Import validates before replacing the draft. Starting the imported recipe creates a fresh run, not a mid-run save. Recipes do not preserve player position, current HP, elapsed time, enemies, or the state of random generators.

The schema is versioned. Unknown stages, duplicate items, invalid levels, excessive slots, inappropriate circle overrides, and unknown fields are rejected. A failed import does not change the active run. Explicit Circle overrides permit one through seven patterns independently of weapon level so debug presets can be saved faithfully; omitting the override uses normal progression defaults.

Separate seeded `StdRng` streams drive waves, loot, and upgrade/reward choices. Consuming a loot roll cannot perturb the wave stream. Starting again with the same recipe resets those streams. Repeatability assumes the same build, authored data, inputs, and relevant sequence of simulation events. This is not a deterministic replay guarantee: physics, frame timing, player choices, and later changes to the RNG implementation or content can change outcomes.

## Persistent debugging

F1 toggles the general panel and F2 selects Weapons. The panel can be opened on the main menu and retained across run starts, level-ups, treasure, pause, and result screens. Tabs and the pause preference persist. Run cheats and queued encounters reset for a fresh run.

Pause simulation affects active gameplay only. It does not replace a level-up, treasure, or other screen's state. In live mode, granting a level leaves the panel open while the ordinary selection screen handles the upgrade. Gameplay-only controls disable outside an appropriate run context. Egui input capture prevents clicks or typed keys from activating the screen behind the tools.

The Weapons tab can add/upgrade weapons and passives and edit Circle presets. Run recipe import/export controls support repeatable build experiments. Debug tools remain behind the `dev-tools` Cargo feature.

## Art

`assets/sprites/occult-items-v1.png` is a generated RGBA atlas used by inventory/upgrade icons, world pickups and props, and the thrown Vessel. It contains twelve cells in four columns and three rows. The original generated size is 1448 × 1086, with 362-pixel cells; runtime atlas coordinates use that actual output size. Corner and gutter alpha were checked. The generation tool, exact prompt, and index mapping are in `assets/sprites/ART-NOTES.md`.

## Validation and remaining work

Automated validation: 83 tests with `dev-tools`, 69 tests without it, strict Clippy across all targets in both configurations, and formatting checks. Coverage includes slot eligibility, passive recalculation, real Rapier projectile travel/piercing/walls/bursts, upgrade previews against fired attacks, recipe validation and replacement, seeded stream reset, telemetry, final-build capture, and debug input/state persistence.

Native Windows/Vulkan checks at 1920 x 1080 exercised seed entry and recipe import/export, starting from the main menu with live debug open, all three weapons together, passive acquisition through both debug tools and level-up choices, HUD slots/icons, pause stats, exported-build restart, chest rewards, and the debug-triggered victory summary. The egui/Bevy UI integration is explicitly enabled so the persistent tools render above full-screen menus. Keyboard/mouse were exercised; physical controller testing remains outstanding.

This pass focuses on builds and the tools to evaluate them. It does not add the village/hub mode, persistent progression, evolution recipes, new enemy attack behaviors, sound, or a deterministic replay system. Weapon balance, late-run pressure, and art remain provisional. The new menu and build interfaces give future modes an explicit place to connect without implementing their requirements prematurely.
