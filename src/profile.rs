//! Progress that outlives a run: banked resources and expedition history.
//!
//! The profile is plain data. Expeditions add to it through
//! [`crate::expedition`], and it is written back to disk whenever it changes.
//! Headless harnesses keep it in memory; only the executable names a file.
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const SAVE_PATH: &str = "save.ron";
/// Version 2 records upgrade ranks and what is prepared for the next expedition.
pub const PROFILE_VERSION: u32 = 2;

/// The kinds of resource an expedition can bring home.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, strum::EnumIter, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    /// Coin found in breakables and treasure.
    Gold,
    /// Distilled from defeated enemies.
    Essence,
    /// Taken from bosses and completed expeditions.
    Seals,
}

impl ResourceKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Gold => "Gold",
            Self::Essence => "Essence",
            Self::Seals => "Seals",
        }
    }
}

/// An amount of every resource: a haul, a price, or the banked total.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Stockpile {
    pub gold: u32,
    pub essence: u32,
    pub seals: u32,
}

impl Stockpile {
    /// `amount` of one resource and nothing else.
    pub fn of(kind: ResourceKind, amount: u32) -> Self {
        let mut pile = Self::default();
        *pile.slot(kind) = amount;
        pile
    }

    /// The largest single amount held.
    pub fn max(&self) -> u32 {
        self.gold.max(self.essence).max(self.seals)
    }

    pub const fn get(&self, kind: ResourceKind) -> u32 {
        match kind {
            ResourceKind::Gold => self.gold,
            ResourceKind::Essence => self.essence,
            ResourceKind::Seals => self.seals,
        }
    }

    fn slot(&mut self, kind: ResourceKind) -> &mut u32 {
        match kind {
            ResourceKind::Gold => &mut self.gold,
            ResourceKind::Essence => &mut self.essence,
            ResourceKind::Seals => &mut self.seals,
        }
    }

    pub fn add(&mut self, other: &Self) {
        use strum::IntoEnumIterator;
        for kind in ResourceKind::iter() {
            let slot = self.slot(kind);
            *slot = slot.saturating_add(other.get(kind));
        }
    }

    pub fn covers(&self, cost: &Self) -> bool {
        use strum::IntoEnumIterator;
        ResourceKind::iter().all(|kind| self.get(kind) >= cost.get(kind))
    }

    /// Pay `cost` in full, or change nothing and report that it is unaffordable.
    pub fn spend(&mut self, cost: &Self) -> bool {
        use strum::IntoEnumIterator;
        if !self.covers(cost) {
            return false;
        }
        for kind in ResourceKind::iter() {
            *self.slot(kind) -= cost.get(kind);
        }
        true
    }
}

impl std::fmt::Display for Stockpile {
    /// Lists non-zero amounts, e.g. `Gold 12, Seals 1`; `nothing` when empty.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use strum::IntoEnumIterator;
        let parts: Vec<String> = ResourceKind::iter()
            .filter(|kind| self.get(*kind) > 0)
            .map(|kind| format!("{} {}", kind.name(), self.get(kind)))
            .collect();
        if parts.is_empty() {
            write!(f, "nothing")
        } else {
            write!(f, "{}", parts.join(", "))
        }
    }
}

#[derive(Resource, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub version: u32,
    #[serde(default)]
    pub stockpile: Stockpile,
    /// Estate expeditions that reached a result screen, won or lost.
    #[serde(default)]
    pub expeditions: u32,
    #[serde(default)]
    pub victories: u32,
    /// Built estate upgrades by id, with the highest rank built; see `estate.ron`.
    #[serde(default)]
    pub upgrades: BTreeMap<String, u32>,
    /// Provisions bought for the next estate expedition, consumed when it departs.
    #[serde(default)]
    pub provisions: Vec<String>,
    /// The ritual performed for the next estate expedition, consumed when it departs.
    #[serde(default)]
    pub ritual: Option<String>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            version: PROFILE_VERSION,
            stockpile: Stockpile::default(),
            expeditions: 0,
            victories: 0,
            upgrades: BTreeMap::new(),
            provisions: Vec::new(),
            ritual: None,
        }
    }
}

/// The first saved format: upgrades were one-time purchases listed by id.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileV1 {
    #[expect(dead_code, reason = "Checked by the version probe before parsing.")]
    version: u32,
    #[serde(default)]
    stockpile: Stockpile,
    #[serde(default)]
    expeditions: u32,
    #[serde(default)]
    victories: u32,
    #[serde(default)]
    upgrades: Vec<String>,
}

impl From<ProfileV1> for Profile {
    fn from(old: ProfileV1) -> Self {
        Self {
            stockpile: old.stockpile,
            expeditions: old.expeditions,
            victories: old.victories,
            upgrades: old.upgrades.into_iter().map(|id| (id, 1)).collect(),
            ..default()
        }
    }
}

/// Only the version, read first to decide how to parse the rest. Other
/// fields are ignored here and checked by the versioned parse.
#[derive(Deserialize)]
struct VersionProbe {
    version: u32,
}

impl Profile {
    /// The rank of `upgrade` that has been built; zero when it has not.
    pub fn rank(&self, upgrade: &str) -> u32 {
        self.upgrades.get(upgrade).copied().unwrap_or(0)
    }

    pub fn owns(&self, upgrade: &str) -> bool {
        self.rank(upgrade) > 0
    }

    /// Parse a save of the current version, upgrading older versions in memory.
    pub fn parse(text: &str) -> Result<Self, String> {
        let probe: VersionProbe = ron::from_str(text).map_err(|error| error.to_string())?;
        match probe.version {
            1 => ron::from_str::<ProfileV1>(text)
                .map(Self::from)
                .map_err(|error| error.to_string()),
            PROFILE_VERSION => ron::from_str(text).map_err(|error| error.to_string()),
            other => Err(format!(
                "Unsupported save version {other} (expected {PROFILE_VERSION} or older)."
            )),
        }
    }

    /// Load the profile at `path`, or start a new one if there is no file.
    ///
    /// A file that exists but cannot be used is copied to `<path>.bak` before a
    /// new profile replaces it, so a damaged or newer save is never destroyed.
    pub fn load_or_default(path: &Path) -> Self {
        if !path.exists() {
            return Self::default();
        }
        match crate::storage::read_text(path).and_then(|text| Self::parse(&text)) {
            Ok(profile) => profile,
            Err(error) => {
                let backup = backup_path(path);
                match std::fs::copy(path, &backup) {
                    Ok(_) => error!(
                        "Could not load {}: {error}. Kept a copy at {} and started a new profile.",
                        path.display(),
                        backup.display()
                    ),
                    Err(copy_error) => error!(
                        "Could not load {}: {error}. Backing it up also failed: {copy_error}.",
                        path.display()
                    ),
                }
                Self::default()
            }
        }
    }
}

fn backup_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".bak");
    PathBuf::from(name)
}

/// Where the profile is saved. `None` keeps it in memory, as tests do.
#[derive(Resource, Clone, Debug, Default)]
pub struct ProfileStore {
    pub path: Option<PathBuf>,
}

impl ProfileStore {
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self {
            path: Some(path.into()),
        }
    }
}

pub struct ProfilePlugin;

impl Plugin for ProfilePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Profile>()
            .init_resource::<ProfileStore>()
            .add_systems(
                Last,
                // The freshly loaded profile is already on disk.
                save_profile
                    .run_if(resource_changed::<Profile>.and_then(not(resource_added::<Profile>))),
            );
    }
}

fn save_profile(profile: Res<Profile>, store: Res<ProfileStore>) {
    let Some(path) = &store.path else {
        return;
    };
    if let Err(error) = crate::storage::write_ron(&*profile, path) {
        error!("Could not save progress: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pile(gold: u32, essence: u32, seals: u32) -> Stockpile {
        Stockpile {
            gold,
            essence,
            seals,
        }
    }

    fn scratch_file(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("survivors-profile-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        directory.join("save.ron")
    }

    #[test]
    fn spending_is_all_or_nothing() {
        let mut bank = pile(10, 5, 0);
        assert!(!bank.spend(&pile(4, 0, 1)));
        assert_eq!(bank, pile(10, 5, 0));
        assert!(bank.spend(&pile(4, 5, 0)));
        assert_eq!(bank, pile(6, 0, 0));
    }

    #[test]
    fn adding_saturates_instead_of_wrapping() {
        let mut bank = pile(u32::MAX - 1, 0, 0);
        bank.add(&pile(5, 2, 1));
        assert_eq!(bank, pile(u32::MAX, 2, 1));
    }

    #[test]
    fn stockpiles_describe_only_what_they_hold() {
        assert_eq!(pile(12, 0, 1).to_string(), "Gold 12, Seals 1");
        assert_eq!(Stockpile::default().to_string(), "nothing");
    }

    #[test]
    fn profile_survives_a_save_and_load() {
        let path = scratch_file("round-trip");
        let profile = Profile {
            stockpile: pile(7, 3, 1),
            expeditions: 4,
            victories: 2,
            upgrades: [("armory".to_owned(), 2)].into(),
            provisions: vec!["tincture".into()],
            ritual: Some("vigil".into()),
            ..default()
        };
        crate::storage::write_ron(&profile, &path).unwrap();
        assert_eq!(Profile::load_or_default(&path), profile);
    }

    #[test]
    fn a_missing_save_starts_a_new_profile_without_creating_files() {
        let path = scratch_file("missing");
        assert_eq!(Profile::load_or_default(&path), Profile::default());
        assert!(!path.exists());
        assert!(!backup_path(&path).exists());
    }

    #[test]
    fn an_unusable_save_is_backed_up_rather_than_lost() {
        for (name, text) in [
            ("corrupt", "(version: 2, stockpile: (gold: "),
            ("newer", "(version: 99, stockpile: (gold: 500))"),
            ("unknown-field", "(version: 2, mystery: 3)"),
            ("unknown-old-field", "(version: 1, mystery: 3)"),
        ] {
            let path = scratch_file(name);
            std::fs::write(&path, text).unwrap();
            assert_eq!(Profile::load_or_default(&path), Profile::default());
            assert_eq!(std::fs::read_to_string(backup_path(&path)).unwrap(), text);
        }
    }

    #[test]
    fn saves_without_newer_fields_still_load() {
        assert_eq!(Profile::parse("(version: 2)").unwrap(), Profile::default());
        assert_eq!(
            Profile::parse("(version: 2, stockpile: (seals: 2))")
                .unwrap()
                .stockpile,
            pile(0, 0, 2)
        );
    }

    #[test]
    fn first_version_saves_are_upgraded_with_their_upgrades_at_rank_one() {
        let profile = Profile::parse(
            "(version: 1, stockpile: (gold: 3), expeditions: 2, victories: 1, upgrades: [\"larder\", \"whetstone\"])",
        )
        .unwrap();
        assert_eq!(profile.version, PROFILE_VERSION);
        assert_eq!(profile.stockpile, pile(3, 0, 0));
        assert_eq!((profile.expeditions, profile.victories), (2, 1));
        assert_eq!(profile.rank("larder"), 1);
        assert_eq!(profile.rank("whetstone"), 1);
        assert_eq!(profile.rank("bell_frame"), 0);
        assert!(profile.provisions.is_empty() && profile.ritual.is_none());
        assert_eq!(Profile::parse("(version: 1)").unwrap(), Profile::default());
    }

    #[test]
    fn changes_are_written_but_a_fresh_load_is_not_rewritten() {
        let path = scratch_file("autosave");
        let mut app = App::new();
        app.add_plugins(ProfilePlugin)
            .insert_resource(ProfileStore::at(&path));
        app.update();
        app.update();
        assert!(!path.exists(), "an untouched profile must not be written");

        app.world_mut().resource_mut::<Profile>().stockpile.gold = 9;
        app.update();
        assert_eq!(Profile::load_or_default(&path).stockpile.gold, 9);
    }
}
