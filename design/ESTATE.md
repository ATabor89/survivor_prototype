# The estate and the expedition loop

The estate is the hub between expeditions. This pass is a grey-box proof of the
loop, not a finished village: placeholder art, three villagers, one workshop.

## The loop

1. **Walk the estate.** Talk to villagers, check the ledger of banked resources.
2. **Plan at the map room.** Pick a destination and a starting weapon, then depart.
3. **Run the expedition.** This is the existing survivors gameplay, unchanged.
4. **Come home.** The result screen banks the haul and leads back to the estate.
5. **Build at the workshop.** Spend the haul on upgrades that change later expeditions.

The title screen's existing setup controls remain as a **sandbox**: any weapon,
any seed, recipe import/export. Sandbox runs show their haul but never touch
saved progress, so they stay useful for testing and balance work.

## Controls

| Input | Action |
| --- | --- |
| WASD / left stick | Walk |
| E, Enter, Space / South button | Interact with the highlighted villager or station; advance dialogue |
| Up/Down or W/S, Enter, mouse | Navigate and confirm estate menus |
| Escape / Start | Close the open menu or conversation; from the estate itself, return to the title |

## How the two modes talk to each other

The modes share two small pieces of data and nothing else.

- **Into an expedition: a run recipe.** The map room builds an ordinary
  `RunConfiguration` (stage, seed, starting weapon, starting passives) and starts
  it through `setup::start_run`. Estate upgrades only ever change that recipe.
  Expedition code has no knowledge of the estate.
- **Out of an expedition: an `ExpeditionReport`.** When a run reaches a result
  screen, `expedition.rs` converts its totals into a haul. `RunOrigin` records
  who launched the run: `Estate` runs bank the haul into the `Profile`; `Sandbox`
  runs only display it.

Abandoning an estate expedition from the pause menu returns home with nothing.

## Resources

| Resource | Source | Rate |
| --- | --- | --- |
| Gold | Breakables and treasure, as before | 1:1 with the run's gold |
| Essence | Defeated enemies | 1 per 10 kills |
| Seals | Bosses, plus a victory bonus | 1 per boss, +2 for winning |

Everything collected is kept on defeat. The rates are constants at the top of
`src/expedition.rs` and are placeholders until the estate has a real economy.

## Saved progress

`Profile` holds the banked stockpile, expedition and victory counts, and the ids
of built upgrades. It is written to `save.ron` in the launch directory whenever
it changes (ignored by Git, like `run-config.ron`). Delete the file to start over.

A save that cannot be read, or that comes from a newer version, is copied to
`save.ron.bak` before a fresh profile replaces it. Fields added later use serde
defaults, so older saves keep loading. Only the executable names a save file;
headless tests keep the profile in memory.

## Authoring the estate

Edit `assets/data/estate.ron` and relaunch. It is validated at startup and
follows the same lookup rules as `stages.ron`.

- `half_size`, `spawn`: the walled area and where the avatar appears.
- `buildings`: solid labelled rectangles. Interiors are not modelled.
- `stations`: usable spots. `ExpeditionTable` opens the map room menu and
  `Workshop` opens the upgrade menu.
- `npcs`: a name, position, tint, `lines`, and optional `returning_lines` used
  once at least one expedition has come home.
- `expedition_weapons`: starting weapons offered before any upgrades.
- `upgrades`: one-time purchases with a `cost` and an `effect`:
  - `UnlockWeapon(<weapon id>)` adds a starting-weapon choice at the map room.
  - `StartingPassive(<passive kind>)` equips every estate expedition with that
    passive at level one.

Validation rejects points inside buildings or outside the walls, duplicate ids,
villagers with nothing to say, free upgrades, upgrades that repeat a weapon or
passive, and upgrades without a workshop to build them.

## Code map

| File | Responsibility |
| --- | --- |
| `src/estate/mod.rs` | Plugin, the `EstateScreen` sub-states, Escape handling |
| `src/estate/content.rs` | `estate.ron` schema, validation, and what a profile has unlocked |
| `src/estate/world.rs` | Map, villagers, stations, and the walking avatar |
| `src/estate/interaction.rs` | Nearest interactable in reach and what using it does |
| `src/estate/dialogue.rs` | Line-by-line conversations |
| `src/estate/expedition_table.rs` | Map room menu and launching an expedition |
| `src/estate/workshop.rs` | Upgrade purchases |
| `src/estate/hud.rs` | Ledger readout and interaction prompt |
| `src/estate/ui.rs` | Shared layout for estate menus |
| `src/expedition.rs` | Run origin, haul conversion, and banking |
| `src/profile.rs` | Saved progress |
| `src/storage.rs` | Bounded reads and atomic writes for small RON files |

`GameState::Estate` is one state in the existing machine, and `EstateScreen`
(`Roaming`, `Dialogue`, `ExpeditionTable`, `Workshop`) is a Bevy sub-state of it.
Everything the estate spawns is scoped to one of those states with
`DespawnOnExit`, so there is no hand-written teardown. Estate menus reuse the
existing menu navigation and activation pipeline through `MenuAction::Estate`.

The avatar uses the same Rapier character controller and movement input as the
expedition player but is a separate `EstateAvatar` entity with no health,
weapons, or experience, so expedition systems never see it. The camera follows
whichever entity carries `CameraFocus`.

## Verification

Headless tests drive the real plugins and Rapier. They cover content validation,
entering and leaving the estate, walking speed and collision with walls and
buildings, interaction focus and prompts, dialogue flow and returning lines,
planning and departing, the full round trip with banking, abandoning, sandbox
isolation, purchases, and both upgrade effects reaching an actual run.

The estate has not had a visual check. Layout, text sizes, and whether the
map reads clearly need eyes on a real window.

## Limits and next steps

- No day cycle, shops, villager schedules, or building interiors.
- Villagers do not block movement and have no portraits or branching dialogue.
- Upgrades are one-time purchases with two effect kinds. Ranks, prerequisites,
  and effects on stages or rewards are natural extensions of `EstateEffect`.
- The economy is untuned. Prices and conversion rates are guesses.
- One save slot, written beside the executable.
