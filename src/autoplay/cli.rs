//! `survivors_prototype simulate ...`: play batches of runs from a terminal.
use super::policy::{MovementStyle, UpgradePolicy};
use super::report;
use super::simulate::{simulate_all, Scenario};
use crate::content::GameContent;
use crate::setup::{RunConfiguration, WeaponLoadout};
use crate::storage;
use crate::weapons::WeaponType;
use std::path::PathBuf;
use std::process::ExitCode;
use strum::IntoEnumIterator;

pub const USAGE: &str = "\
Play runs with a scripted pilot and report how they went.

Usage: survivors_prototype simulate [options]

Every combination of stage, weapon, pilot, and policy is played once per seed.

  --stage <id|all>     Stage to play; repeat for several (default: courtyard)
  --weapon <id|all>    Starting weapon; repeat to compare several
                       (default: magick_circle)
  --recipe <path>      Play a run recipe exported from the setup screen
                       instead of --stage and --weapon
  --pilot <name|all>   How the pilot moves: evasive (keeps its distance),
                       bold (chases pickups, dodges late), or still (never
                       moves); repeat for several (default: bold)
  --policy <name|all>  How level-ups are spent: solo (only the starting
                       weapon), greedy (fill weapon slots first), or random;
                       repeat for several (default: solo)
  --seeds <n>          Runs per combination (default: 5)
  --seed <n>           First seed; later runs count up from it (default: 1,
                       or the recipe's seed)
  --minutes <m>        Stop a run still alive after this much simulated time
                       (default: 15)
  --jobs <n>           Runs to play at once (default: one per CPU)
  --csv <path>         Also write one row per run to this file
  --help               Show this message
";

/// A parsed `simulate` command line.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// Stage ids; `all` is expanded once the game's stages are known.
    pub stages: Vec<String>,
    pub weapons: Vec<WeaponType>,
    pub recipe: Option<PathBuf>,
    pub pilots: Vec<MovementStyle>,
    pub policies: Vec<UpgradePolicy>,
    pub seeds: u32,
    pub first_seed: Option<u64>,
    pub minutes: f32,
    pub jobs: Option<usize>,
    pub csv: Option<PathBuf>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            stages: Vec::new(),
            weapons: Vec::new(),
            recipe: None,
            pilots: Vec::new(),
            policies: Vec::new(),
            seeds: 5,
            first_seed: None,
            minutes: 15.0,
            jobs: None,
            csv: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Help,
    Simulate(Options),
}

fn number<T: std::str::FromStr>(flag: &str, value: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("{flag} needs a number, not `{value}`"))
}

fn weapon(id: &str) -> Result<WeaponType, String> {
    WeaponType::iter()
        .find(|weapon| weapon.id() == id)
        .ok_or_else(|| {
            let known: Vec<_> = WeaponType::iter().map(WeaponType::id).collect();
            format!("unknown weapon `{id}`; choose from: {}", known.join(", "))
        })
}

/// Parse the arguments that follow `simulate`.
pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let mut options = Options::default();
    let mut args = args.into_iter();
    while let Some(flag) = args.next() {
        if flag == "--help" || flag == "-h" {
            return Ok(Command::Help);
        }
        let mut value = || {
            args.next()
                .ok_or_else(|| format!("{flag} needs a value; see --help"))
        };
        match flag.as_str() {
            "--stage" => options.stages.push(value()?),
            "--weapon" => match value()?.as_str() {
                "all" => options.weapons.extend(WeaponType::iter()),
                id => options.weapons.push(weapon(id)?),
            },
            "--recipe" => options.recipe = Some(value()?.into()),
            "--pilot" => match value()?.as_str() {
                "all" => options.pilots.extend(MovementStyle::ALL),
                name => options
                    .pilots
                    .push(MovementStyle::parse(name).ok_or_else(|| {
                        format!("unknown pilot `{name}`; choose evasive, bold, still, or all")
                    })?),
            },
            "--policy" => match value()?.as_str() {
                "all" => options.policies.extend(UpgradePolicy::ALL),
                name => options
                    .policies
                    .push(UpgradePolicy::parse(name).ok_or_else(|| {
                        format!("unknown policy `{name}`; choose solo, greedy, random, or all")
                    })?),
            },
            "--seeds" => options.seeds = number(&flag, &value()?)?,
            "--seed" => options.first_seed = Some(number(&flag, &value()?)?),
            "--minutes" => options.minutes = number(&flag, &value()?)?,
            "--jobs" => options.jobs = Some(number(&flag, &value()?)?),
            "--csv" => options.csv = Some(value()?.into()),
            _ => return Err(format!("unknown option `{flag}`; see --help")),
        }
    }
    if options.seeds == 0 || options.jobs == Some(0) {
        return Err("--seeds and --jobs must be at least 1".into());
    }
    if !(options.minutes > 0.0 && options.minutes.is_finite()) {
        return Err("--minutes must be greater than zero".into());
    }
    if options.recipe.is_some() && !(options.stages.is_empty() && options.weapons.is_empty()) {
        return Err("--recipe replaces --stage and --weapon; use one or the other".into());
    }
    options.weapons.dedup();
    options.pilots.dedup();
    options.policies.dedup();
    Ok(Command::Simulate(options))
}

/// The recipes to play before seeds and policies are applied: the one named by
/// `--recipe`, or one per stage and starting weapon.
fn recipes(options: &Options, content: &GameContent) -> Result<Vec<RunConfiguration>, String> {
    if let Some(path) = &options.recipe {
        let text = storage::read_text(path)?;
        let recipe =
            ron::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
        return Ok(vec![recipe]);
    }
    let known = || content.stages.iter().map(|stage| stage.id.clone());
    let mut stages: Vec<String> = Vec::new();
    for stage in &options.stages {
        if stage == "all" {
            stages.extend(known());
        } else if known().any(|id| id == *stage) {
            stages.push(stage.clone());
        } else {
            let ids: Vec<_> = known().collect();
            return Err(format!(
                "unknown stage `{stage}`; choose from: {}",
                ids.join(", ")
            ));
        }
    }
    let default = RunConfiguration::default();
    if stages.is_empty() {
        stages.push(default.stage.clone());
    }
    stages.dedup();
    let weapons = if options.weapons.is_empty() {
        vec![default.weapons[0].weapon]
    } else {
        options.weapons.clone()
    };
    Ok(stages
        .iter()
        .flat_map(|stage| {
            weapons.iter().map(|weapon| RunConfiguration {
                stage: stage.clone(),
                weapons: vec![WeaponLoadout {
                    weapon: *weapon,
                    level: 1,
                    patterns: vec![],
                }],
                ..default.clone()
            })
        })
        .collect())
}

/// Every run the options ask for, grouped by recipe, pilot, policy, then seed.
pub fn scenarios(options: &Options, content: &GameContent) -> Result<Vec<Scenario>, String> {
    let policies = if options.policies.is_empty() {
        vec![UpgradePolicy::Solo]
    } else {
        options.policies.clone()
    };
    let pilots = if options.pilots.is_empty() {
        vec![MovementStyle::Bold]
    } else {
        options.pilots.clone()
    };
    let mut scenarios = Vec::new();
    for recipe in recipes(options, content)? {
        let first_seed = match (options.first_seed, &options.recipe) {
            (Some(seed), _) => seed,
            (None, Some(_)) => recipe.seed,
            (None, None) => 1,
        };
        for (movement, policy) in pilots
            .iter()
            .flat_map(|movement| policies.iter().map(move |policy| (*movement, *policy)))
        {
            for offset in 0..u64::from(options.seeds) {
                scenarios.push(Scenario {
                    recipe: RunConfiguration {
                        seed: first_seed.wrapping_add(offset),
                        ..recipe.clone()
                    },
                    movement,
                    policy,
                    max_seconds: options.minutes * 60.0,
                });
            }
        }
    }
    Ok(scenarios)
}

fn simulate(options: &Options) -> Result<bool, String> {
    let content = GameContent::load()?;
    let scenarios = scenarios(options, &content)?;
    let jobs = options
        .jobs
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, std::num::NonZero::get));
    let total = scenarios.len();
    eprintln!("Playing {total} runs, up to {jobs} at a time...");
    let finished = simulate_all(scenarios, jobs, |done, run| {
        eprintln!("[{done}/{total}] {}", report::headline(run));
    });
    println!("{}", report::summary(&finished));
    if let Some(path) = &options.csv {
        std::fs::write(path, report::csv(&finished))
            .map_err(|error| format!("{}: {error}", path.display()))?;
        eprintln!("Wrote {}", path.display());
    }
    Ok(finished.iter().all(|run| run.result.is_ok()))
}

/// Run the `simulate` command with the arguments that follow it.
pub fn run(args: impl IntoIterator<Item = String>) -> ExitCode {
    let outcome = match parse(args) {
        Ok(Command::Help) => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Ok(Command::Simulate(options)) => simulate(&options),
        Err(error) => Err(error),
    };
    match outcome {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => {
            eprintln!("Some runs failed; see the errors above.");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("simulate: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse a command line written as one string.
    fn options(line: &str) -> Result<Options, String> {
        match parse(line.split_whitespace().map(str::to_owned))? {
            Command::Simulate(options) => Ok(options),
            Command::Help => Err("help".into()),
        }
    }

    fn content() -> GameContent {
        GameContent::parse(crate::content::EMBEDDED_CONTENT).unwrap()
    }

    #[test]
    fn flags_parse_and_all_expands_weapons_pilots_and_policies() {
        let parsed = options(
            "--stage ash_walk --weapon knife_of_the_art --weapon magick_circle --pilot still \
             --policy greedy --seeds 3 --seed 40 --minutes 2.5 --jobs 2 --csv out.csv",
        )
        .unwrap();
        assert_eq!(parsed.stages, ["ash_walk"]);
        assert_eq!(
            parsed.weapons,
            [WeaponType::KnifeOfTheArt, WeaponType::MagickCircle]
        );
        assert_eq!(parsed.pilots, [MovementStyle::Still]);
        assert_eq!(parsed.policies, [UpgradePolicy::Greedy]);
        assert_eq!((parsed.seeds, parsed.first_seed), (3, Some(40)));
        assert_eq!((parsed.minutes, parsed.jobs), (2.5, Some(2)));
        assert_eq!(parsed.csv, Some(PathBuf::from("out.csv")));

        let all = options("--weapon all --pilot all --policy all").unwrap();
        assert_eq!(all.weapons.len(), WeaponType::iter().count());
        assert_eq!(all.pilots, MovementStyle::ALL);
        assert_eq!(all.policies, UpgradePolicy::ALL);
        assert_eq!(parse(["--help".to_owned()]), Ok(Command::Help));
    }

    #[test]
    fn mistakes_are_reported_instead_of_guessed_at() {
        for line in [
            "--weapon spoon",
            "--pilot sprinting",
            "--policy reckless",
            "--seeds 0",
            "--seeds many",
            "--minutes 0",
            "--jobs 0",
            "--stage",
            "--fast",
            "--recipe run.ron --weapon magick_circle",
        ] {
            assert!(options(line).is_err(), "{line}");
        }
        let unknown = options("--stage nowhere").unwrap();
        assert!(scenarios(&unknown, &content())
            .unwrap_err()
            .contains("courtyard"));
    }

    #[test]
    fn scenarios_cover_every_stage_weapon_pilot_policy_and_seed() {
        let content = content();
        let defaults = scenarios(&options("").unwrap(), &content).unwrap();
        assert_eq!(defaults.len(), 5);
        assert_eq!(defaults[0].recipe.stage, "courtyard");
        assert_eq!(defaults[0].movement, MovementStyle::Bold);
        assert_eq!(defaults[0].policy, UpgradePolicy::Solo);
        assert_eq!(defaults[0].max_seconds, 900.0);
        let seeds: Vec<_> = defaults.iter().map(|s| s.recipe.seed).collect();
        assert_eq!(seeds, [1, 2, 3, 4, 5]);

        let matrix = options(
            "--stage all --weapon magick_circle --weapon knife_of_the_art --pilot all \
             --policy all --seeds 2 --seed 10",
        )
        .unwrap();
        let runs = scenarios(&matrix, &content).unwrap();
        assert_eq!(runs.len(), content.stages.len() * 2 * 3 * 3 * 2);
        // Seeds vary fastest, so a group's runs are adjacent in the output.
        assert_eq!(runs[0].recipe.seed, 10);
        assert_eq!(runs[1].recipe.seed, 11);
        assert_eq!(runs[1].policy, runs[0].policy);
        assert!(runs
            .iter()
            .all(|run| run.recipe.weapons.len() == 1 && run.recipe.weapons[0].level == 1));
    }
}
