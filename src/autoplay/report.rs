//! Turn finished runs into a summary table and a per-run CSV.
use super::simulate::{Ending, Finished, RunResult, Scenario};
use crate::weapons::WeaponType;
use std::fmt::Write as _;

/// The weapons a scenario starts with, e.g. `magick_circle+knife_of_the_art`.
pub fn loadout(scenario: &Scenario) -> String {
    let ids: Vec<_> = scenario
        .recipe
        .weapons
        .iter()
        .map(|loadout| loadout.weapon.id())
        .collect();
    ids.join("+")
}

/// `m:ss` for a number of simulated seconds.
pub fn clock(seconds: f32) -> String {
    let whole = seconds.max(0.0) as u32;
    format!("{}:{:02}", whole / 60, whole % 60)
}

/// One line for a finished run, for progress output.
pub fn headline(finished: &Finished) -> String {
    let scenario = &finished.scenario;
    let outcome = match &finished.result {
        Ok(result) => format!(
            "{} at {}, level {}, {} kills",
            result.ending.name(),
            clock(result.seconds),
            result.level,
            result.kills
        ),
        Err(error) => format!("error: {error}"),
    };
    format!(
        "{} {} {} {} seed {}: {outcome} ({:.1}s)",
        scenario.recipe.stage,
        loadout(scenario),
        scenario.movement.name(),
        scenario.policy.name(),
        scenario.recipe.seed,
        finished.wall.as_secs_f32()
    )
}

/// Runs that share a stage, starting loadout, and pilot, differing by seed.
struct Group<'a> {
    stage: &'a str,
    loadout: String,
    movement: &'static str,
    policy: &'static str,
    starting: Vec<WeaponType>,
    results: Vec<&'a RunResult>,
    errors: usize,
}

fn groups(finished: &[Finished]) -> Vec<Group<'_>> {
    let mut groups: Vec<Group> = Vec::new();
    for run in finished {
        let scenario = &run.scenario;
        let loadout = loadout(scenario);
        let (movement, policy) = (scenario.movement.name(), scenario.policy.name());
        let position = groups.iter().position(|group| {
            group.stage == scenario.recipe.stage
                && group.loadout == loadout
                && group.movement == movement
                && group.policy == policy
        });
        let index = position.unwrap_or_else(|| {
            groups.push(Group {
                stage: &scenario.recipe.stage,
                loadout,
                movement,
                policy,
                starting: scenario.recipe.weapons.iter().map(|w| w.weapon).collect(),
                results: Vec::new(),
                errors: 0,
            });
            groups.len() - 1
        });
        match &run.result {
            Ok(result) => groups[index].results.push(result),
            Err(_) => groups[index].errors += 1,
        }
    }
    groups
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let (sum, count) = values.fold((0.0, 0u32), |(sum, count), value| (sum + value, count + 1));
    if count == 0 {
        0.0
    } else {
        sum / f64::from(count)
    }
}

impl Group<'_> {
    fn cells(&self) -> [String; 11] {
        let runs = &self.results;
        let wins = runs
            .iter()
            .filter(|run| run.ending == Ending::Victory)
            .count();
        // Damage dealt by the weapons the run started with, as a share of all
        // weapon damage across the group's runs.
        let (mut starting, mut total) = (0u64, 0u64);
        for (weapon, damage) in runs.iter().flat_map(|run| &run.damage_by_weapon) {
            total += damage;
            if self.starting.contains(weapon) {
                starting += damage;
            }
        }
        let share = if total == 0 {
            "-".to_owned()
        } else {
            format!("{:.0}%", starting as f64 * 100.0 / total as f64)
        };
        [
            self.stage.to_owned(),
            self.loadout.clone(),
            self.movement.to_owned(),
            self.policy.to_owned(),
            format!("{wins}/{}", runs.len()),
            clock(mean(runs.iter().map(|run| f64::from(run.seconds))) as f32),
            format!("{:.1}", mean(runs.iter().map(|run| f64::from(run.level)))),
            format!("{:.0}", mean(runs.iter().map(|run| f64::from(run.kills)))),
            format!(
                "{:.0}",
                mean(runs.iter().map(|run| run.damage_taken as f64))
            ),
            share,
            if self.errors == 0 {
                String::new()
            } else {
                self.errors.to_string()
            },
        ]
    }
}

const HEADINGS: [&str; 11] = [
    "stage",
    "start",
    "pilot",
    "policy",
    "wins",
    "survived",
    "level",
    "kills",
    "hurt",
    "start dmg",
    "errors",
];

/// A table with one row per stage, starting loadout, and pilot, averaged over
/// its seeds. `hurt` is damage taken; `start dmg` is the share of weapon damage
/// dealt by the weapons the run started with.
pub fn summary(finished: &[Finished]) -> String {
    let rows: Vec<_> = groups(finished).iter().map(Group::cells).collect();
    let widths: Vec<usize> = (0..HEADINGS.len())
        .map(|column| {
            rows.iter()
                .map(|row| row[column].len())
                .chain([HEADINGS[column].len()])
                .max()
                .unwrap_or(0)
        })
        .collect();
    let mut table = String::new();
    let headings = HEADINGS.map(str::to_owned);
    for row in std::iter::once(&headings).chain(&rows) {
        let mut line = String::new();
        for (column, cell) in row.iter().enumerate() {
            // Names read best left-aligned, numbers right-aligned.
            if column < 4 {
                let _ = write!(line, "{cell:<width$}  ", width = widths[column]);
            } else {
                let _ = write!(line, "{cell:>width$}  ", width = widths[column]);
            }
        }
        table.push_str(line.trim_end());
        table.push('\n');
    }
    table
}

pub const CSV_HEADER: &str =
    "stage,start,pilot,policy,seed,ending,seconds,level,kills,bosses,gold,\
essence,seals,damage_taken,lowest_health,build,damage_by_weapon,level_by_minute,error";

/// Quote a field only if it needs it.
fn field(text: &str) -> String {
    if text.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_owned()
    }
}

fn pairs<T: std::fmt::Display>(pairs: &[(WeaponType, T)]) -> String {
    let parts: Vec<_> = pairs
        .iter()
        .map(|(weapon, value)| format!("{}:{value}", weapon.id()))
        .collect();
    parts.join("|")
}

/// One row per run. Lists inside a cell are separated by `|`.
pub fn csv(finished: &[Finished]) -> String {
    let mut text = format!("{CSV_HEADER}\n");
    for run in finished {
        let scenario = &run.scenario;
        let mut cells = vec![
            field(&scenario.recipe.stage),
            loadout(scenario),
            scenario.movement.name().to_owned(),
            scenario.policy.name().to_owned(),
            scenario.recipe.seed.to_string(),
        ];
        match &run.result {
            Ok(result) => {
                let levels: Vec<_> = result.level_by_minute.iter().map(u32::to_string).collect();
                cells.extend([
                    result.ending.name().to_owned(),
                    format!("{:.1}", result.seconds),
                    result.level.to_string(),
                    result.kills.to_string(),
                    result.bosses.to_string(),
                    result.haul.gold.to_string(),
                    result.haul.essence.to_string(),
                    result.haul.seals.to_string(),
                    result.damage_taken.to_string(),
                    result
                        .lowest_health
                        .map_or(String::new(), |health| health.to_string()),
                    pairs(&result.build),
                    pairs(&result.damage_by_weapon),
                    levels.join("|"),
                    String::new(),
                ]);
            }
            Err(error) => {
                cells.push("error".to_owned());
                cells.extend(std::iter::repeat_n(String::new(), 12));
                cells.push(field(error));
            }
        }
        text.push_str(&cells.join(","));
        text.push('\n');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::autoplay::policy::{MovementStyle, UpgradePolicy};
    use crate::profile::Stockpile;
    use crate::setup::RunConfiguration;
    use std::time::Duration;

    fn run(seed: u64, result: Result<RunResult, String>) -> Finished {
        Finished {
            scenario: Scenario {
                recipe: RunConfiguration {
                    seed,
                    ..Default::default()
                },
                movement: MovementStyle::Bold,
                policy: UpgradePolicy::Solo,
                max_seconds: 900.0,
            },
            result,
            wall: Duration::from_secs(2),
        }
    }

    fn result(ending: Ending, seconds: f32, circle: u64, knife: u64) -> RunResult {
        RunResult {
            ending,
            seconds,
            level: 10,
            level_by_minute: vec![3, 6],
            kills: 200,
            bosses: 1,
            damage_taken: 80,
            lowest_health: Some(12),
            damage_by_weapon: vec![
                (WeaponType::MagickCircle, circle),
                (WeaponType::KnifeOfTheArt, knife),
            ],
            build: vec![
                (WeaponType::MagickCircle, 4),
                (WeaponType::KnifeOfTheArt, 2),
            ],
            haul: Stockpile {
                gold: 5,
                essence: 20,
                seals: 3,
            },
        }
    }

    #[test]
    fn the_summary_averages_a_group_and_counts_wins_and_errors() {
        let finished = [
            run(1, Ok(result(Ending::Victory, 600.0, 300, 100))),
            run(2, Ok(result(Ending::Defeat, 300.0, 300, 300))),
            run(3, Err("stuck".into())),
        ];
        let table = summary(&finished);
        let lines: Vec<_> = table.lines().collect();
        assert_eq!(lines.len(), 2, "{table}");
        assert_eq!(
            lines[1].split_whitespace().collect::<Vec<_>>().join(" "),
            "courtyard magick_circle bold solo 1/2 7:30 10.0 200 80 60% 1"
        );
    }

    #[test]
    fn the_csv_has_one_complete_row_per_run_and_quotes_awkward_errors() {
        let finished = [
            run(1, Ok(result(Ending::TimedOut, 61.25, 300, 100))),
            run(2, Err("bad, \"very\" bad".into())),
        ];
        let text = csv(&finished);
        let lines: Vec<_> = text.lines().collect();
        assert_eq!(lines[0], CSV_HEADER);
        assert_eq!(
            lines[1],
            "courtyard,magick_circle,bold,solo,1,timed_out,61.2,10,200,1,5,20,3,80,12,\
             magick_circle:4|knife_of_the_art:2,magick_circle:300|knife_of_the_art:100,3|6,"
        );
        assert_eq!(
            lines[2],
            "courtyard,magick_circle,bold,solo,2,error,,,,,,,,,,,,,\"bad, \"\"very\"\" bad\""
        );
        let columns = CSV_HEADER.split(',').count();
        assert_eq!(lines[1].split(',').count(), columns);
    }

    #[test]
    fn clocks_and_headlines_read_naturally() {
        assert_eq!(clock(0.0), "0:00");
        assert_eq!(clock(125.9), "2:05");
        let line = headline(&run(9, Ok(result(Ending::Defeat, 125.0, 1, 0))));
        assert_eq!(
            line,
            "courtyard magick_circle bold solo seed 9: defeat at 2:05, level 10, 200 kills (2.0s)"
        );
    }
}
