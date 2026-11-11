# Design documents

The README describes how to build, run, and test the game. These documents record
design intent and the reasoning behind each implementation pass. Older passes are
kept as history, so read the status column before trusting implementation details.

## Current

| Document | Covers |
| --- | --- |
| [BUILD-DIVERSITY.md](BUILD-DIVERSITY.md) | Inventory slots, passives, run recipes, seeded streams, debug tools, and the worldbuilding contract |
| [STAGE-CONTENT.md](STAGE-CONTENT.md) | Authoring stages, phases, encounters, breakables, and treasure in `assets/data/stages.ron` |
| [COMBAT-ENCOUNTERS.md](COMBAT-ENCOUNTERS.md) | Map win conditions, enemy attack behaviors, pacing, combat feedback, and audio |
| [OCCULT-ARSENAL.md](OCCULT-ARSENAL.md) | The eight automatic weapons added after Circle, Needles, and Vessel |
| [CIRCLE-COMPOSITION.md](CIRCLE-COMPOSITION.md) | How Magick Circle effects stack and overlap |

## Concepts and research

| Document | Covers |
| --- | --- |
| [project-summary.md](project-summary.md) | The original vision: village hub, expeditions, traditions, Magnum Opus progression |
| [magic-dashboard-refined.tsx](magic-dashboard-refined.tsx) | The original dashboard of traditions, weapons, items, and mechanics |
| [OCCULT-WEAPON-RESEARCH.md](OCCULT-WEAPON-RESEARCH.md) | Historical inspiration behind the arsenal, separated from invented mechanics |
| [CHRONOPHAGE.md](CHRONOPHAGE.md) | Deferred enemy concept; not scheduled |

## Historical records

| Document | Covers |
| --- | --- |
| [DESIGN.md](DESIGN.md) | The long-term vision plus an implementation snapshot from the single-weapon era. Sections 1 and 8 are still useful; the rest is superseded by the current documents above |
| [SYSTEMS-REVIEW.md](SYSTEMS-REVIEW.md) | The first courtyard/run refactor, superseded by the stage-content pass |
| [DEPENDENCY-UPGRADE.md](DEPENDENCY-UPGRADE.md) | The Bevy 0.15 to 0.20 migration record, including branch names that no longer matter |
