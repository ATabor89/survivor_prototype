# Occult arsenal prototype

Eight new weapons extend the roster to eleven. All are automatic, take one of six weapon slots, use player damage/area/cooldown modifiers, and participate in normal level-up offers, chest upgrades, recipes, debug grants, and damage telemetry. Each new weapon has eight levels. This is an initial behavior and tuning pass.

The [research notes](OCCULT-WEAPON-RESEARCH.md) separate historical inspiration from invented combat mechanics. Family labels describe inspiration and worldbuilding; there are no family restrictions or extra bonuses.

| Weapon | Automatic behavior | Positioning and limits |
| --- | --- | --- |
| Knife of the Art | A 120-degree fan in the last movement direction, retained when standing still. | One hit per target per swing; 0.22-second visual. |
| Planetary Talismans | Two orbiting medals, increasing to five at levels 3/5/7. | Perimeter at 105 units; area grows contact size. Each cooldown refresh recharges contact hits; repeated contact has a 0.35-second interval. One controller, five reusable visuals; expires after 3.25 seconds without refresh. |
| Angelic Inscription | Chooses a nearby enemy position, then draws a fixed grid: three rows followed by three columns. | Each enemy can take half the cast damage from each orientation. Damage does not multiply at line intersections. Up to four active patterns per weapon. |
| Circulating Pelican | Two curved streams travel outward and return toward the moving owner. | Each stream hits each target once per leg. Rapier walls stop streams. Outward leg 0.8 seconds; total lifetime at most 3 seconds. |
| Forgotten Sigil | Automatically places a sigil at each cooldown, leaving a trail while moving. | Arms after 0.6 seconds or 60 units of owner movement. Nearby enemies trigger it; otherwise it erupts at 4 seconds. Explosion visual lasts 0.28 seconds. Stationary players still place sigils; all expire. |
| Bound Intention | A single wax-and-paper familiar prioritizes the highest-maximum-health enemy within range. | One pending strike per cooldown. It obeys Rapier walls and follows its owner when idle. Direct steering only; navigating around obstacles and a larger familiar system remain future work. |
| Emanation Lattice | Selects nearby enemies, branches through them, and charges staggered pulses. | Living endpoints track until discharge. Starts with three targets, grows to six at levels 2/4/6. A fixed total cast-damage budget is divided among nodes and overlapping victims. Extra branches distribute damage; they do not multiply it. |
| Resonant Recitation | An expanding pulse centered on the cast location; every fourth pulse deals double damage with 25% more radius. | Each enemy is hit once per pulse. Pulses last 0.7 seconds. This deals damage without knockback. |

## Progression and implementation

Base damage / radius / cooldown values are in src/weapons/arsenal.rs; level changes are in src/weapons/weapon_upgrade.rs. The new weapons share a conservative seven-upgrade schedule: damage, area, cooldown, damage+area, cooldown, damage, then all three. Orbit count and Lattice target-count increases are included in upgrade descriptions.

The shared Cast data is dispatched to three focused modules: rituals, inscriptions, and mobile. Rapier supplies convex/overlap/shape-cast hit queries and wall stopping; movement paths are authored weapon behavior. Damage uses the existing combat event and source-attribution pipeline. Attack sources remain present through damage resolution, and run cleanup removes effect hierarchies. Virtual-time updates stop with the run.

Eight individually generated transparent sprites are stored in assets/sprites/arsenal. Their [provenance and exact generation prompts](../assets/sprites/arsenal/README.md) are recorded alongside them. These are placeholders; there are no actual sacred-language inscriptions in the art.

## Playtesting

Choose any weapon on the title screen; its family and behavior appear in the setup summary. F2 opens the Weapons debug tab to grant or upgrade equipment. Start a fresh run to try a different six-slot build. Disable natural waves in F1 for isolated testing, then use stationary/moving targets from the weapon controls. Pause shows actual enemy damage by weapon.

Useful comparisons:
- Knife + Talismans + Recitation for directional and close-range coverage.
- Inscription + Lattice to compare fixed ground attacks with tracked branches.
- Pelican + Sigil + Bound Intention while moving around obstacles.

Existing version-one recipes remain valid. New weapon identifiers use snake_case, e.g. knife_of_the_art and emanation_lattice. Import/export uses the existing run-config.ron workflow. The local recipe is not overwritten by this work.

Headless regression coverage includes all-weapon recipe/max-level construction, six-slot offer filtering, retained facing, actual Rapier fan/stream/wall/inscription hits, contact cadence, branching damage conservation and moving targets, forced sigil detonation, familiar uniqueness, attribution, bounded effects, pause, and restart.
## Validation for this pass

- 122 tests passed with dev-tools; 107 with default features.
- Strict Clippy passed for all targets and features; formatting passed.
- Native 1920 × 1080 checks covered all eleven setup choices, a six-weapon mixed build, generated inventory icons, tracked Lattice/ground Inscription visuals, familiar and sigils, pause damage attribution, return to title, a fresh Knife/Talismans/Sigil run, and a real Knife level-up with effective damage preview.
- Native logs contained no gameplay or asset-loading errors. These were short smoke tests; long-run balance remains provisional.
