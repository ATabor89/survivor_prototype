# Autoplay: simulated runs for balance work

`survivors_prototype simulate` plays whole expeditions without a window. A
scripted pilot moves the player and answers level-ups and treasure, and each
run reports how long it lasted, how strong it got, and which weapons did the
work. It exists because the number of weapon, stage, and upgrade combinations
is far more than one person can play.

It measures the game as it is. It does not judge whether a number is right.

## Quick start

```sh
# Five seeds of the default build on the courtyard
cargo run --locked -- simulate

# Compare every weapon as a lone starting weapon on every stage
cargo run --locked -- simulate --stage all --weapon all --seeds 3

# How does a build hold up for careful, average, and idle play?
cargo run --locked -- simulate --weapon knife_of_the_art --pilot all --policy greedy

# Play a recipe exported from the setup screen, and keep per-run rows
cargo run --locked -- simulate --recipe run-config.ron --seeds 10 --csv runs.csv
```

`cargo run --locked -- simulate --help` lists every option. Progress goes to
standard error and the summary table to standard output, so the table can be
redirected on its own.

A run of 10 to 15 simulated minutes takes roughly 15 to 30 seconds per CPU core
in a development build, and runs are played in parallel, one per core by default.

## What a scenario is

Every combination of the following is played once per seed.

| Option | Meaning |
| --- | --- |
| `--stage` | A stage id from `assets/data/stages.ron`, or `all` |
| `--weapon` | The single starting weapon at level 1, or `all` |
| `--recipe` | A full run recipe instead of `--stage`/`--weapon`: several weapons, levels, Circle patterns, starting passives |
| `--pilot` | How the pilot moves (below) |
| `--policy` | How the pilot spends level-ups (below) |
| `--seeds`, `--seed` | How many runs, and the first seed; later runs count up |
| `--minutes` | Simulated time after which a run still alive is stopped (default 15) |

Estate upgrades only ever change a run's recipe, so their effect is measured
the same way: export or write a recipe with the passive or weapon an upgrade
grants and compare it against one without.

### Pilots

| Pilot | Behavior | What it tells you |
| --- | --- | --- |
| `evasive` | Keeps well clear of everything and circles the crowd | Killing power and boss time with survival taken out of the picture; it is rarely hit |
| `bold` (default) | Walks to pickups and only dodges what is about to connect | The closest of the three to ordinary play: it levels quickly and takes real damage |
| `still` | Never moves | Whether a build can hold the stage by itself, and how soon pressure overwhelms it |

Movement is a potential field: threats push with a strength that rises as they
close, walls push inward near the edges, and the nearest pickup pulls. The pilot
steers through a virtual gamepad's left stick and confirms menus with its South
button, so it goes through the same input, physics, and menu systems as a player.
It cannot do anything a person could not.

### Upgrade policies

| Policy | Behavior | What it tells you |
| --- | --- | --- |
| `solo` (default) | Invests in the starting weapon, then passives; takes other weapons only when nothing else is offered | How far one weapon carries a run |
| `greedy` | Fills every weapon slot, then upgrades weapons, then passives | A broad build, and how much the starting weapon still matters in one |
| `random` | Picks uniformly | A baseline with no preference |

Magick Circle pattern choices are always random. Treasure is always accepted.

## Reading the results

The table has one row per stage, starting loadout, pilot, and policy, averaged
over its seeds.

| Column | Meaning |
| --- | --- |
| `wins` | Victories out of completed runs |
| `survived` | Mean simulated time until the run ended or was stopped |
| `level`, `kills` | Mean player level and kills at the end |
| `hurt` | Mean damage taken |
| `start dmg` | Share of all weapon damage dealt by the starting weapons |
| `errors` | Runs that could not be played; the reason is in the progress output |

A run that reaches the time limit counts as neither a win nor a loss; raise
`--minutes` if a boss stage needs longer.

`--csv` writes one row per run with the seed, ending, time, level, kills,
bosses, the haul the run would bring back to the estate (gold, essence, seals),
damage taken, lowest health seen, the final build, damage by weapon, and level
at the end of each minute. Lists inside a cell are separated by `|`.

## Repeatability

The same scenario gives the same result on the same build of the game. The run's
wave, loot, and reward streams and the pilot's own choices are all seeded from
the recipe's seed, and simulated runs execute their systems one at a time so the
order never depends on thread timing. A test plays a scenario twice and compares
every reported number.

Results can differ between machines or compilers, because floating-point physics
is not guaranteed to be identical across them. Compare numbers produced by the
same build.

## How it fits together

| Piece | Role |
| --- | --- |
| `src/headless.rs` | The real game plugins and Rapier with no window, shared with the regression tests; time advances 1/60 s per update |
| `src/autoplay/pilot.rs` | `AutoplayPlugin`: steering, menu answers, and the `FlightLog` of what the pilot observed |
| `src/autoplay/policy.rs` | Movement styles and upgrade policies, as pure functions |
| `src/autoplay/simulate.rs` | `simulate` plays one `Scenario` to a `RunResult`; `simulate_all` plays a batch on worker threads |
| `src/autoplay/report.rs` | The summary table and CSV |
| `src/autoplay/cli.rs` | Argument parsing and the scenario matrix |

Runs start through `setup::start_run`, the same entry point as the setup screen
and the estate's map room, and never read or write `save.ron`.

## Limits

- The pilot has no plan. It does not seek out landmarks, lead a boss, or save a
  chest for later, so it understates what a deliberate player achieves.
- The pilot faces where it walks, and it mostly walks away from enemies. Weapons
  aimed by movement direction, such as Knife of the Art, score far below what a
  player who turns to face the crowd gets from them.
- Three pilots bracket human play; none of them is a model of it. Treat results
  as comparisons between builds under the same pilot, not as predictions.
- A handful of seeds is noisy. Differences of a few percent need more runs.
- Nothing is rendered, so visual clarity and feel still need a person.
