// The palette is retained for future loot tiers; current upgrades use Common.
#[expect(
    dead_code,
    reason = "Only Common upgrades are currently offered; other tiers define the menu palette."
)]
#[derive(Clone, Debug, PartialEq)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}
