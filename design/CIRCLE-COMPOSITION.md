# Circle composition and playtesting

Circles compose their ordinary effects. There are no named pair bonuses or pair-specific dispatch rules. The player's decisions are primarily build choices; circles retain their automatic positions and three-second lifetime. An Invocation circle pulling an enemy out of another circle is an intentional spatial tradeoff.

## Effects

| Pattern | Current rule |
| --- | --- |
| Banishment | Base damage every 0.5 seconds |
| Binding | Root refreshed every 0.5 seconds; expires one second after the last refresh |
| Invocation | Continuous pull at 120 pixels/second per circle; vector sum capped at 180 |
| Manifestation | Charging seal; 3x base damage after 1.25 seconds and again at 2.5 seconds |
| Protection | 50% incoming damage reduction while the player occupies the circle |

Tuning is centralized in `src/weapons/effects.rs`. Damage cadence is measured from each circle's spawn time, including circles spawned late in a run. Only occupants present at a pulse receive that pulse. Manifestation's inner seal expands while charging and flares briefly on eruption. These are initial tuning values, not a completed balance pass.

Each damage source contributes independently, including duplicate patterns. Roots refresh rather than multiply, and disable pursuit without blocking forced movement. Pull vectors are accumulated in stable source order; opposing pulls cancel, and pulls slow close to their centers to avoid overshooting. An active pull, including a cancelled pull, replaces pursuit velocity. Protection uses the strongest ward and never adds reductions together. Three or more circles follow the same rules.

`ActiveOverlaps` retains physical circle pairs for diagnostics. Its pattern helpers return unique unordered combinations. Damage and control effects do not depend on those helpers, so deduplication does not suppress damage from duplicate circles.

## Circle Lab

Run `cargo run --locked --features dev-tools`, then press **F1 while playing**. The panel pauses virtual time and physics. F1, Escape, or Resume closes it and resumes play. Ordinary pause and level-up menus remain separate.

- Select one to six patterns, including duplicates. The first occupies the center; the others use the existing automatic outer positions.
- Presets populate the draft build. **Apply build** replaces the current pattern list, clears old circles and effects, and schedules fresh circles on resume. It retains weapon stats and progression.
- Disable natural spawns and enable invulnerability to isolate tests. A visible reminder remains during play while these overrides are active.
- Spawn eight stationary targets at the overlap regions or eight moving pursuers. Targets have 1,000 HP and give no XP.
- Clear arena removes enemies, gems, circles, and effects without resetting the selected weapon build or progression.
- Heal restores player HP. Grant one level resumes through the real XP/upgrade flow.
- Show physics colliders toggles Rapier's debug geometry in development builds.

Normal enemies now award 10 XP. Previously the listed reward was 50 and the death system multiplied it by 66, yielding 3,300 XP per gem. Both testing values were removed; gems now preserve the configured enemy reward exactly. The first level still costs 100 XP. This removes the testing shortcut without claiming the overall progression is balanced.

The panel uses optional `bevy_egui` 0.42, whose [support table](https://github.com/vladbat00/bevy_egui#bevy-support-table) includes Bevy 0.19. [bevy-inspector-egui](https://github.com/jakobhellermann/bevy-inspector-egui) was considered, but its entity inspection does not replace the game-specific build and arena controls. Normal builds do not enable either debug UI or cheat controls.

## Verification

The headless suite exercises all ten distinct pairs, five duplicate pairs, representative triples, pull cancellation and caps, root/pull interaction, non-stacking protection, delayed eruptions and pause behavior. With `dev-tools`, it also exercises panel state transitions, build application, target spawning, invulnerability, disabling natural spawns, arena clearing, and the real level-up shortcut.

Native Windows/Vulkan smoke testing verified F1 open/resume, preset selection and application, stationary target spawning, spawn/invulnerability toggles, arena clearing, panel scrolling, and the level-up shortcut. Generated sprites and Manifestation's charging/eruption seal rendered without asset or renderer errors. Long-run balance and other platforms remain unverified.

Provisional generated character sprites replace the primitive shapes; the original assets remain in the repository. See `assets/sprites/ART-NOTES.md` for provenance and the generation prompt. No new enemy behavior is implied by the artwork.
