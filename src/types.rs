// Basic type definitions
#[derive(Clone, Debug, PartialEq)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum EquipmentType {
    Armor,
    Ring,
    Amulet,
    Boots,
    Gloves,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum StatType {
    Health,
    Speed,
    Attack,
    Defense,
    Luck,
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
