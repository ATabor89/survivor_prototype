// Basic type definitions
#[derive(Clone, Debug, PartialEq)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WeaponType {
    Sword,
    Axe,
    Spear,
    Bow,
    Magic,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EquipmentType {
    Armor,
    Ring,
    Amulet,
    Boots,
    Gloves,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StatType {
    Health,
    Speed,
    Attack,
    Defense,
    Luck,
}

// Display implementations for our enum types
impl std::fmt::Display for WeaponType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WeaponType::Sword => write!(f, "Sword"),
            WeaponType::Axe => write!(f, "Axe"),
            WeaponType::Spear => write!(f, "Spear"),
            WeaponType::Bow => write!(f, "Bow"),
            WeaponType::Magic => write!(f, "Magic"),
        }
    }
}

impl std::fmt::Display for EquipmentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EquipmentType::Armor => write!(f, "Armor"),
            EquipmentType::Ring => write!(f, "Ring"),
            EquipmentType::Amulet => write!(f, "Amulet"),
            EquipmentType::Boots => write!(f, "Boots"),
            EquipmentType::Gloves => write!(f, "Gloves"),
        }
    }
}

impl std::fmt::Display for StatType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StatType::Health => write!(f, "Health"),
            StatType::Speed => write!(f, "Speed"),
            StatType::Attack => write!(f, "Attack"),
            StatType::Defense => write!(f, "Defense"),
            StatType::Luck => write!(f, "Luck"),
        }
    }
}