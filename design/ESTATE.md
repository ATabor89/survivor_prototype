# The estate and the expedition loop

The estate is the hub between expeditions. It is still grey-box: placeholder art,
three villagers, and four stations. The second pass deepened the loop with
ranked upgrades, prerequisites, a provisioner, rituals, and prices set against
the autoplay haul.

## The loop

1. **Walk the estate.** Talk to villagers, check the ledger of banked resources.
2. **Prepare (optional).** Pack up to two provisions at the provisioner's counter,
   and perform one ritual at the chapel altar. Both last for one expedition.
3. **Plan at the map room.** Pick a destination and a starting weapon, then depart.
   Departing uses up the packed provisions and the prepared ritual.
4. **Run the expedition.** The survivors gameplay, shaped by the recipe the estate built.
5. **Come home.** The result screen banks the haul, plus any estate haul bonus.
   "Depart again" plans a fresh expedition from the map room's current plan, so
   spent provisions and rituals are not reused.
6. **Build at the workshop.** Spend the haul on lasting upgrades, one rank at a time.

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

- **Into an expedition: a run recipe and a haul bonus.** The map room builds an
  ordinary `RunConfiguration` (stage, seed, starting weapons, starting passives,
  and `modifiers`) and starts it through `setup::start_run`. Alongside it, the
  `HaulBonus` resource is set to the percentages the estate grants. Everything
  the estate does reaches a run through these two values. Expedition code has
  no knowledge of the estate.
- **Out of an expedition: an `ExpeditionReport`.** When a run reaches a result
  screen, `expedition.rs` converts its totals into a haul and, for estate runs,
  adds the haul bonus (reported separately as `bonus`). `RunOrigin` records who
  launched the run: `Estate` runs bank the haul into the `Profile`; `Sandbox`
  runs only display it and never get a bonus.

`RunModifiers` (`src/run_modifiers.rs`) are general run difficulty settings:
`enemy_health_percent` scales every enemy's health when it spawns, bosses
included, and `damage_taken_percent` scales damage to the player before
protection. Both default to 100, accept 25 to 400, and are left out of
`run-config.ron` when unchanged, so sandbox recipes can use them too.

Abandoning an estate expedition from the pause menu returns home with nothing.

## Resources

| Resource | Source | Rate |
| --- | --- | --- |
| Gold | Breakables and treasure, as before | 1:1 with the run's gold |
| Essence | Defeated enemies | 1 per 10 kills |
| Seals | Bosses, plus a victory bonus | 1 per boss, +2 for winning |

Everything collected is kept on defeat. The rates are constants at the top of
`src/expedition.rs`.

## Economy

Prices are set against the 2026-11-11 autoplay shakedown (bold pilot, 66 runs
over three stages), whose median run brought home **14 gold, 81 essence, and 8
seals**. Under the first pass the whole workshop cost 25 gold, 150 essence, and
3 seals, so two or three runs bought everything.

| Total across every workshop rank | Amount | Median runs |
| --- | --- | --- |
| Gold | 280 | about 20 |
| Essence | 1,730 | about 21 |
| Seals | 111 | about 14 |

So buying out the workshop takes roughly 20 runs at the bot's pace, fewer with
the Counting House and Alembic bonuses or with rituals, and fewer for a player
who wins. Gold is the scarcest resource and is also what the provisioner
mostly wants, so each trip is a choice between saving and spending. Early
purchases (the first Larder rank, a provision, Lesser Banishing) are affordable
after one run.

These are deliberately rough. The plan is to retune with autoplay data once
there are more stages and estate effects, not by hand now. Seals are plentiful
because every warden counts as a boss; their prices reflect that.

## Workshop upgrades

An upgrade has one or more ranks, each with its own price. Ranks are bought in
order. `requires` lists upgrades that must be built (at any rank) first; they
must appear earlier in the file, which also rules out cycles.

| Effect | What a rank does |
| --- | --- |
| `UnlockWeapon(weapon)` | Adds a starting weapon at the map room. One rank only. |
| `StartingPassive(kind)` | Every estate expedition starts with that passive, one level per rank. |
| `HaulBonus(resource, percent)` | Estate expeditions bring home `percent` more of that resource per rank. At most 200% per resource from upgrades. |

## Provisions

The provisioner's counter sells supplies for the next expedition only. Two fit
in a pack, each at most once, and departing uses them up. They are paid for
when bought; abandoning the expedition does not refund them.

| Effect | Result |
| --- | --- |
| `Passive(kind)` | One more level of that passive, on top of any upgrade. |
| `Weapon(weapon)` | Carry that weapon too, or start one level higher if it is the chosen weapon. |

Levels are capped at the passive's or weapon's maximum.

## Rituals

The chapel altar prepares one ritual at a time for the next expedition. It is
paid for when performed and used up on departure. A ritual sets the run's
`modifiers` and adds a `haul_bonus` (percent per resource) on top of the
upgrade bonuses. `requires` hides a ritual until those upgrades are built.

The bundled rites cover three shapes: Lesser Banishing makes the run easier
for no reward, Vigil of Ash trades tougher enemies for more essence and gold,
and the Rite of Thorns (after Chapel Bells) raises enemy health and damage
taken for a large bonus to everything.

## Saved progress

`Profile` holds the banked stockpile, expedition and victory counts, the rank
of each built upgrade, the packed provisions, and the prepared ritual. It is written to `save.ron` in the launch directory whenever
it changes (ignored by Git, like `run-config.ron`). Delete the file to start over.

A save that cannot be read, or that comes from a newer version, is copied to
`save.ron.bak` before a fresh profile replaces it. Fields added later use serde
defaults, so older saves keep loading. Version 1 saves (upgrades as a list of
ids) are upgraded in memory with every listed upgrade at rank one, and are
written as version 2 the next time the profile changes. Only the executable names a save file;
headless tests keep the profile in memory.

## Authoring the estate

Edit `assets/data/estate.ron` and relaunch. It is validated at startup and
follows the same lookup rules as `stages.ron`.

- `half_size`, `spawn`: the walled area and where the avatar appears.
- `buildings`: solid labelled rectangles. Interiors are not modelled.
- `stations`: usable spots. `ExpeditionTable` opens the map room menu,
  `Workshop` the upgrade menu, `Provisioner` the provision shop, and `Altar`
  the ritual menu.
- `npcs`: a name, position, tint, `lines`, and optional `returning_lines` used
  once at least one expedition has come home.
- `expedition_weapons`: starting weapons offered before any upgrades.
- `upgrades`: `ranks` (one price per rank), optional `requires`, and an
  `effect`; see "Workshop upgrades".
- `provisions`: a `cost` and an `effect`; see "Provisions".
- `rituals`: a `cost`, optional `requires`, `modifiers`, and `haul_bonus`; see
  "Rituals".

Validation rejects points inside buildings or outside the walls, duplicate ids,
villagers with nothing to say, anything free, weapon unlocks with more than one
rank, upgrades that repeat a weapon or passive, prerequisites that are unknown
or listed later, modifiers out of range, rituals that change nothing, and
upgrades, provisions, or rituals without the station that offers them.

## Code map

| File | Responsibility |
| --- | --- |
| `src/estate/mod.rs` | Plugin, the `EstateScreen` sub-states, Escape handling |
| `src/estate/content.rs` | `estate.ron` schema, validation, and what a profile has unlocked |
| `src/estate/world.rs` | Map, villagers, stations, and the walking avatar |
| `src/estate/interaction.rs` | Nearest interactable in reach and what using it does |
| `src/estate/dialogue.rs` | Line-by-line conversations |
| `src/estate/expedition_table.rs` | Map room menu and launching an expedition |
| `src/estate/workshop.rs` | Upgrade ranks and prerequisites |
| `src/estate/provisioner.rs` | Provisions packed for the next expedition |
| `src/estate/altar.rs` | Rituals prepared for the next expedition |
| `src/estate/hud.rs` | Ledger readout and interaction prompt |
| `src/estate/ui.rs` | Shared layout for estate menus |
| `src/expedition.rs` | Run origin, haul conversion, haul bonus, and banking |
| `src/run_modifiers.rs` | Run difficulty modifiers applied to enemies and damage |
| `src/profile.rs` | Saved progress |
| `src/storage.rs` | Bounded reads and atomic writes for small RON files |

`GameState::Estate` is one state in the existing machine, and `EstateScreen`
(`Roaming`, `Dialogue`, `ExpeditionTable`, `Workshop`, `Provisioner`, `Altar`)
is a Bevy sub-state of it.
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
isolation, ranked purchases and prerequisites, every upgrade effect reaching an
actual run, provisions stacking and being used up, rituals scaling enemies and
the haul, "Depart again" not reusing them, and migrating version 1 saves.

The estate has not had a visual check. Layout, text sizes, and whether the
map reads clearly need eyes on a real window.

## Limits and next steps

- No day cycle, villager schedules, or building interiors. The provisioner is a counter, not a stocked shop with inventory.
- Villagers do not block movement and have no portraits or branching dialogue.
- Effects on stages themselves (new destinations, map changes) are not modelled.
- Autoplay does not yet play estate expeditions, so ritual and provision
  effects have no balance data. A `--ritual`/`--provision` option for
  `simulate` is the natural next step.
- Prices are rough; see "Economy".
- One save slot, written beside the executable.
