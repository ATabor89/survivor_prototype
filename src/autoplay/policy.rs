//! How the pilot plays: where it walks and how it spends level-ups.
use crate::upgrade::UpgradeType;
use crate::weapons::WeaponType;
use rand::{Rng, RngExt};

/// A rule for choosing among level-up offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpgradePolicy {
    /// Invest only in the starting weapons, then passives. Measures how far a
    /// weapon carries a run on its own.
    Solo,
    /// Fill every weapon slot first, then upgrade weapons, then passives.
    Greedy,
    /// Pick uniformly at random.
    Random,
}

impl UpgradePolicy {
    pub const ALL: [Self; 3] = [Self::Solo, Self::Greedy, Self::Random];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Solo => "solo",
            Self::Greedy => "greedy",
            Self::Random => "random",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|policy| policy.name() == name)
    }

    /// Lower ranks are preferred. `focus` is the run's starting weapons.
    fn rank(self, offer: &UpgradeType, focus: &[WeaponType]) -> u8 {
        match (self, offer) {
            (Self::Random, _) => 0,
            (Self::Solo, UpgradeType::Weapon(weapon, _)) if focus.contains(weapon) => 0,
            (Self::Solo, UpgradeType::Passive(..)) => 1,
            (Self::Solo, UpgradeType::Generic(_)) => 2,
            (Self::Solo, UpgradeType::Weapon(..)) => 3,
            (Self::Solo, UpgradeType::AcquireWeapon(_)) => 4,
            (Self::Greedy, UpgradeType::AcquireWeapon(_)) => 0,
            (Self::Greedy, UpgradeType::Weapon(..)) => 1,
            (Self::Greedy, UpgradeType::Passive(..)) => 2,
            (Self::Greedy, UpgradeType::Generic(_)) => 3,
        }
    }

    /// The index of the offer to take, or `None` when there are no offers.
    /// Equally ranked offers are chosen between at random.
    pub fn choose(
        self,
        offers: &[UpgradeType],
        focus: &[WeaponType],
        rng: &mut impl Rng,
    ) -> Option<usize> {
        let best = offers.iter().map(|offer| self.rank(offer, focus)).min()?;
        let candidates: Vec<usize> = offers
            .iter()
            .enumerate()
            .filter(|(_, offer)| self.rank(offer, focus) == best)
            .map(|(index, _)| index)
            .collect();
        Some(candidates[rng.random_range(0..candidates.len())])
    }
}

/// How much room the pilot keeps between itself and danger.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Caution {
    /// Threats closer than this push the pilot away.
    pub threat_radius: f32,
    /// How strongly the nearest pickup pulls, against a threat's push of 1.
    pub lure_weight: f32,
}

/// A rule for moving around the stage.
///
/// The styles bracket how a person plays. A careful pilot is rarely touched,
/// so survival says little about a build; a pilot that stands still shows
/// whether the build alone can hold the stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MovementStyle {
    /// Keep well clear of everything and circle the crowd.
    Evasive,
    /// Chase pickups and only dodge what is about to connect.
    Bold,
    /// Never move.
    Still,
}

impl MovementStyle {
    pub const ALL: [Self; 3] = [Self::Evasive, Self::Bold, Self::Still];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Evasive => "evasive",
            Self::Bold => "bold",
            Self::Still => "still",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|style| style.name() == name)
    }

    /// `None` for a pilot that does not steer at all.
    pub const fn caution(self) -> Option<Caution> {
        match self {
            Self::Evasive => Some(Caution {
                threat_radius: 240.0,
                lure_weight: 0.6,
            }),
            Self::Bold => Some(Caution {
                threat_radius: 90.0,
                lure_weight: 1.2,
            }),
            Self::Still => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::PassiveType;
    use crate::upgrade::GenericUpgrade;
    use crate::weapons::weapon_upgrade::WeaponUpgradeSpec;
    use rand::{rngs::StdRng, SeedableRng};

    fn offers() -> Vec<UpgradeType> {
        vec![
            UpgradeType::Generic(GenericUpgrade::HealthPickup(20)),
            UpgradeType::AcquireWeapon(WeaponType::KnifeOfTheArt),
            UpgradeType::Passive(PassiveType::MoveSpeed, 1),
            UpgradeType::Weapon(
                WeaponType::MagickCircle,
                WeaponUpgradeSpec { changes: vec![] },
            ),
        ]
    }

    #[test]
    fn solo_invests_in_the_starting_weapon_and_never_prefers_new_ones() {
        let mut rng = StdRng::seed_from_u64(1);
        let focus = [WeaponType::MagickCircle];
        let all = offers();
        assert_eq!(UpgradePolicy::Solo.choose(&all, &focus, &mut rng), Some(3));
        // Without its own upgrade on offer it takes the passive, then healing.
        assert_eq!(
            UpgradePolicy::Solo.choose(&all[..3], &focus, &mut rng),
            Some(2)
        );
        assert_eq!(
            UpgradePolicy::Solo.choose(&all[..2], &focus, &mut rng),
            Some(0)
        );
    }

    #[test]
    fn greedy_fills_weapon_slots_before_upgrading() {
        let mut rng = StdRng::seed_from_u64(1);
        let all = offers();
        assert_eq!(UpgradePolicy::Greedy.choose(&all, &[], &mut rng), Some(1));
        assert_eq!(
            UpgradePolicy::Greedy.choose(&[all[0].clone(), all[2].clone()], &[], &mut rng),
            Some(1)
        );
    }

    #[test]
    fn random_reaches_every_offer_and_empty_menus_choose_nothing() {
        let mut rng = StdRng::seed_from_u64(7);
        let all = offers();
        let mut seen = [false; 4];
        for _ in 0..200 {
            seen[UpgradePolicy::Random.choose(&all, &[], &mut rng).unwrap()] = true;
        }
        assert_eq!(seen, [true; 4]);
        for policy in UpgradePolicy::ALL {
            assert_eq!(policy.choose(&[], &[], &mut rng), None);
            assert_eq!(UpgradePolicy::parse(policy.name()), Some(policy));
        }
        assert_eq!(UpgradePolicy::parse("reckless"), None);
        for style in MovementStyle::ALL {
            assert_eq!(MovementStyle::parse(style.name()), Some(style));
        }
        assert_eq!(MovementStyle::parse("sprinting"), None);
        assert_eq!(MovementStyle::Still.caution(), None);
    }
}
