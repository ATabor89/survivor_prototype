//! Validated, authored game data shared by stages, enemies, waves, and loot.
use bevy::prelude::*;
use serde::Deserialize;
use std::{collections::HashSet, path::Path};

pub const EMBEDDED_CONTENT: &str = include_str!("../assets/data/stages.ron");

#[derive(Resource, Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameContent {
    pub enemies: Vec<EnemyDefinition>,
    pub stages: Vec<StageDefinition>,
}

/// Selected stage persists across runs; changing it takes effect on restart.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct ActiveStage(pub usize);

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnemyDefinition {
    pub id: String,
    pub name: String,
    pub health: i32,
    pub speed: f32,
    pub size: f32,
    pub color: [f32; 4],
    pub xp: u32,
    #[serde(default)]
    pub behavior: EnemyBehavior,
}

/// Authored attack timing; ordinary pursuit remains the backwards-compatible default.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum EnemyBehavior {
    #[default]
    Pursuer,
    Charger {
        trigger_range: f32,
        windup: f32,
        dash_speed: f32,
        dash_seconds: f32,
        recovery: f32,
    },
    Ranged {
        preferred_range: f32,
        windup: f32,
        cooldown: f32,
        projectile_speed: f32,
        projectile_lifetime: f32,
        damage: i32,
        #[serde(default = "one_projectile")]
        projectile_count: u8,
        #[serde(default)]
        spread_radians: f32,
    },
    Warden {
        charge: ChargeConfig,
        ranged: RangedConfig,
    },
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChargeConfig {
    pub trigger_range: f32,
    pub windup: f32,
    pub dash_speed: f32,
    pub dash_seconds: f32,
    pub recovery: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RangedConfig {
    pub preferred_range: f32,
    pub windup: f32,
    pub cooldown: f32,
    pub projectile_speed: f32,
    pub projectile_lifetime: f32,
    pub damage: i32,
    #[serde(default = "one_projectile")]
    pub projectile_count: u8,
    #[serde(default)]
    pub spread_radians: f32,
}

fn one_projectile() -> u8 {
    1
}

impl EnemyBehavior {
    pub fn charge(self) -> Option<ChargeConfig> {
        match self {
            Self::Charger {
                trigger_range,
                windup,
                dash_speed,
                dash_seconds,
                recovery,
            } => Some(ChargeConfig {
                trigger_range,
                windup,
                dash_speed,
                dash_seconds,
                recovery,
            }),
            Self::Warden { charge, .. } => Some(charge),
            _ => None,
        }
    }

    pub fn ranged(self) -> Option<RangedConfig> {
        match self {
            Self::Ranged {
                preferred_range,
                windup,
                cooldown,
                projectile_speed,
                projectile_lifetime,
                damage,
                projectile_count,
                spread_radians,
            } => Some(RangedConfig {
                preferred_range,
                windup,
                cooldown,
                projectile_speed,
                projectile_lifetime,
                damage,
                projectile_count,
                spread_radians,
            }),
            Self::Warden { ranged, .. } => Some(ranged),
            _ => None,
        }
    }

    fn validate(self, label: &str) -> Result<(), String> {
        if let Some(config) = self.charge() {
            ensure([config.trigger_range, config.windup, config.dash_speed, config.dash_seconds, config.recovery].into_iter().all(positive)
                && config.dash_speed <= 2000.0,
                &format!("{label}: charge ranges/times/speed must be positive and finite; dash speed must not exceed 2000"))?;
        }
        if let Some(config) = self.ranged() {
            ensure([config.preferred_range, config.windup, config.cooldown, config.projectile_speed, config.projectile_lifetime].into_iter().all(positive)
                && config.projectile_speed <= 2000.0 && config.damage > 0
                && (1..=12).contains(&config.projectile_count)
                && config.spread_radians.is_finite() && (0.0..=std::f32::consts::PI).contains(&config.spread_radians),
                &format!("{label}: ranged ranges/times/speed/damage must be positive and finite, speed <= 2000, projectile count 1..=12, spread 0..=PI"))?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum StageLayout {
    Arena,
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub enum StageWinCondition {
    #[default]
    TimedSurvival,
    DefeatBoss {
        enemy: String,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageDefinition {
    pub id: String,
    pub name: String,
    pub duration: f32,
    #[serde(default)]
    pub win_condition: StageWinCondition,
    pub half_size: [f32; 2],
    pub layout: StageLayout,
    pub schedule: Vec<WavePhase>,
    #[serde(default)]
    pub encounters: Vec<TimedEncounter>,
    #[serde(default)]
    pub obstacles: Vec<RectangleDefinition>,
    #[serde(default)]
    pub landmarks: Vec<LandmarkDefinition>,
    #[serde(default)]
    pub pickups: Vec<StaticPickupDefinition>,
    #[serde(default)]
    pub breakables: BreakableConfig,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WavePhase {
    pub start: f32,
    pub interval: f32,
    pub batch: usize,
    pub cap: usize,
    #[serde(default)]
    pub minimum: usize,
    pub health_scale: f32,
    pub enemies: Vec<EnemyWeight>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnemyWeight {
    pub id: String,
    pub weight: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimedEncounter {
    pub at: f32,
    pub enemy: String,
    pub count: usize,
    #[serde(default)]
    pub ring: Option<RingDefinition>,
    #[serde(default)]
    pub reward_chest: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RingDefinition {
    pub count: usize,
    pub enemy: String,
    pub radius: f32,
    pub pulse: f32,
    pub period: f32,
    pub duration: f32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RectangleDefinition {
    pub center: [f32; 2],
    pub half_size: [f32; 2],
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LandmarkDefinition {
    pub position: [f32; 2],
    pub name: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticPickupDefinition {
    pub position: [f32; 2],
    pub kind: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BreakableConfig {
    pub count: usize,
    pub health: i32,
    pub drop_chance: f32,
    pub respawn_seconds: f32,
    pub names: Vec<String>,
}

impl Default for BreakableConfig {
    fn default() -> Self {
        Self {
            count: 16,
            health: 10,
            drop_chance: 0.65,
            respawn_seconds: 8.0,
            names: vec!["Candle stand".into(), "Clay urn".into()],
        }
    }
}

impl Default for GameContent {
    fn default() -> Self {
        Self::load().unwrap_or_else(|error| panic!("Cannot load game content: {error}"))
    }
}

impl GameContent {
    /// Runtime edits to assets/data/stages.ron take effect on the next launch.
    /// An explicitly configured asset root is authoritative: mistakes there
    /// produce an actionable error rather than silently using different data.
    pub fn load() -> Result<Self, String> {
        if let Some(root) = std::env::var_os("BEVY_ASSET_ROOT") {
            return Self::load_from_path(Path::new(&root).join("assets/data/stages.ron"));
        }
        if let Ok(executable) = std::env::current_exe() {
            if let Some(directory) = executable.parent() {
                let path = directory.join("assets/data/stages.ron");
                if path.exists() {
                    return Self::load_from_path(path);
                }
            }
        }
        let source_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/data/stages.ron");
        if source_path.exists() {
            return Self::load_from_path(source_path);
        }
        warn!("assets/data/stages.ron was not found; using the bundled stage definitions");
        Self::parse(EMBEDDED_CONTENT)
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|error| {
            format!(
                "{}: {error}. Restore this asset or correct BEVY_ASSET_ROOT.",
                path.display()
            )
        })?;
        Self::parse(&text).map_err(|error| format!("{}: {error}", path.display()))
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let content: Self = ron::from_str(text).map_err(|error| error.to_string())?;
        content.validate()?;
        Ok(content)
    }

    pub fn stage(&self, active: &ActiveStage) -> &StageDefinition {
        // The selector uses existing indices; retain a safe initial stage when
        // a saved selection refers to content removed between launches.
        self.stages.get(active.0).unwrap_or(&self.stages[0])
    }

    pub fn enemy(&self, id: &str) -> Option<&EnemyDefinition> {
        self.enemies.iter().find(|enemy| enemy.id == id)
    }

    pub fn validate(&self) -> Result<(), String> {
        ensure(
            !self.enemies.is_empty(),
            "enemies must contain at least one definition",
        )?;
        ensure(
            !self.stages.is_empty(),
            "stages must contain at least one definition",
        )?;
        let mut enemy_ids = HashSet::new();
        for enemy in &self.enemies {
            let label = format!("enemy '{}'", enemy.id);
            ensure(
                !enemy.id.trim().is_empty() && enemy_ids.insert(enemy.id.as_str()),
                &format!("{label}: IDs must be nonempty and unique"),
            )?;
            ensure(
                !enemy.name.trim().is_empty(),
                &format!("{label}: name is empty"),
            )?;
            ensure(enemy.health > 0 && positive(enemy.size) && enemy.speed.is_finite() && enemy.speed >= 0.0, &format!("{label}: health and size must be positive; speed must be finite and nonnegative"))?;
            enemy.behavior.validate(&label)?;
            ensure(
                enemy
                    .color
                    .iter()
                    .all(|channel| channel.is_finite() && (0.0..=1.0).contains(channel)),
                &format!("{label}: color channels must be in 0..=1"),
            )?;
        }
        let mut stage_ids = HashSet::new();
        for stage in &self.stages {
            let label = format!("stage '{}'", stage.id);
            ensure(
                !stage.id.trim().is_empty() && stage_ids.insert(stage.id.as_str()),
                &format!("{label}: IDs must be nonempty and unique"),
            )?;
            ensure(
                !stage.name.trim().is_empty() && positive(stage.duration),
                &format!("{label}: name and positive duration are required"),
            )?;
            if let StageWinCondition::DefeatBoss { enemy } = &stage.win_condition {
                ensure(
                    enemy_ids.contains(enemy.as_str()),
                    &format!("{label}: victory requires unknown boss '{enemy}'"),
                )?;
            }
            ensure(
                stage
                    .half_size
                    .iter()
                    .all(|axis| positive(*axis) && *axis >= 256.0),
                &format!("{label}: each arena half-size must be at least 256"),
            )?;
            let [width, height] = stage.half_size;
            ensure(
                match stage.layout {
                    StageLayout::Arena => true,
                    StageLayout::Horizontal => width >= height * 2.0,
                    StageLayout::Vertical => height >= width * 2.0,
                },
                &format!("{label}: corridor long axis must be at least twice its short axis"),
            )?;
            ensure(
                stage
                    .schedule
                    .first()
                    .is_some_and(|phase| phase.start == 0.0),
                &format!("{label}: schedule must begin at 0 seconds"),
            )?;
            let mut previous = -1.0;
            for (index, phase) in stage.schedule.iter().enumerate() {
                let phase_label = format!("{label}, phase {index}");
                ensure(
                    phase.start.is_finite()
                        && phase.start > previous
                        && phase.start < stage.duration,
                    &format!("{phase_label}: start must increase and precede the stage duration"),
                )?;
                ensure(positive(phase.interval) && positive(phase.health_scale) && phase.batch > 0 && phase.batch <= phase.cap && phase.cap <= 1000 && phase.minimum <= phase.cap, &format!("{phase_label}: interval/health scale must be positive, 0 < batch <= cap <= 1000, minimum <= cap"))?;
                ensure(
                    !phase.enemies.is_empty(),
                    &format!("{phase_label}: enemy pool is empty"),
                )?;
                let mut total = 0u32;
                for weight in &phase.enemies {
                    ensure(
                        enemy_ids.contains(weight.id.as_str()) && weight.weight > 0,
                        &format!(
                            "{phase_label}: unknown enemy '{}' or zero weight",
                            weight.id
                        ),
                    )?;
                    total = total
                        .checked_add(weight.weight)
                        .ok_or_else(|| format!("{phase_label}: enemy weights overflow u32"))?;
                }
                previous = phase.start;
            }
            for (index, encounter) in stage.encounters.iter().enumerate() {
                let encounter_label = format!("{label}, encounter {index}");
                ensure(
                    encounter.at.is_finite()
                        && encounter.at >= 0.0
                        && encounter.at < stage.duration,
                    &format!("{encounter_label}: time must be within the stage"),
                )?;
                ensure(
                    enemy_ids.contains(encounter.enemy.as_str())
                        && encounter.count > 0
                        && encounter.count <= 256,
                    &format!("{encounter_label}: unknown enemy or count outside 1..=256"),
                )?;
                if let Some(ring) = &encounter.ring {
                    ensure(
                        enemy_ids.contains(ring.enemy.as_str()) && (3..=256).contains(&ring.count),
                        &format!("{encounter_label}: ring needs a known enemy and 3..=256 members"),
                    )?;
                    ensure(positive(ring.radius) && positive(ring.period) && positive(ring.duration) && ring.pulse.is_finite() && (0.0..=1.0).contains(&ring.pulse), &format!("{encounter_label}: radius/period/duration must be positive, pulse must be a 0..=1 fraction"))?;
                }
            }
            for (index, obstacle) in stage.obstacles.iter().enumerate() {
                ensure(
                    obstacle.half_size.iter().all(|value| positive(*value))
                        && obstacle.center.iter().all(|value| value.is_finite()),
                    &format!(
                        "{label}, obstacle {index}: finite center and positive half-size required"
                    ),
                )?;
                ensure(
                    (0..2).all(|axis| {
                        obstacle.center[axis].abs() + obstacle.half_size[axis]
                            <= stage.half_size[axis] - 24.0
                    }),
                    &format!("{label}, obstacle {index}: must leave clearance from the boundary"),
                )?;
                ensure(
                    (0..2)
                        .any(|axis| obstacle.center[axis].abs() > obstacle.half_size[axis] + 24.0),
                    &format!("{label}, obstacle {index}: covers the player's starting position"),
                )?;
            }
            for landmark in &stage.landmarks {
                ensure(
                    !landmark.name.trim().is_empty() && stage.contains(landmark.position),
                    &format!(
                        "{label}: landmark '{}' must have a name and be inside the arena",
                        landmark.name
                    ),
                )?;
            }
            for pickup in &stage.pickups {
                ensure(
                    matches!(pickup.kind.as_str(), "healing" | "gold" | "chest")
                        && stage.contains(pickup.position),
                    &format!(
                        "{label}: pickup '{}' must be healing/gold/chest and inside the arena",
                        pickup.kind
                    ),
                )?;
                ensure(
                    !stage.obstacles.iter().any(|obstacle| {
                        (0..2).all(|axis| {
                            (pickup.position[axis] - obstacle.center[axis]).abs()
                                <= obstacle.half_size[axis] + 16.0
                        })
                    }),
                    &format!(
                        "{label}: pickup at {:?} overlaps an obstacle",
                        pickup.position
                    ),
                )?;
            }
            let props = &stage.breakables;
            ensure(props.count <= 256 && props.health > 0 && positive(props.respawn_seconds) && props.drop_chance.is_finite() && (0.0..=1.0).contains(&props.drop_chance) && !props.names.is_empty() && props.names.iter().all(|name| !name.trim().is_empty()), &format!("{label}: invalid breakable count/health/drop chance/respawn/name configuration"))?;
        }
        Ok(())
    }
}

impl StageDefinition {
    fn contains(&self, point: [f32; 2]) -> bool {
        (0..2)
            .all(|axis| point[axis].is_finite() && point[axis].abs() < self.half_size[axis] - 24.0)
    }
}

fn positive(value: f32) -> bool {
    value.is_finite() && value > 0.0
}
fn ensure(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}
