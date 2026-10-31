//! Who a kid is: what they're called, the animal and colour on their card,
//! their grade, and the day's allowance.

use serde::{Deserialize, Serialize};

/// The animal on a kid's card (drawn by `sekka-kid-kit`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Animal {
    Cat,
    Fox,
    Rabbit,
    Bear,
    Frog,
    Panda,
    Owl,
    Tanuki,
}

impl Animal {
    pub const ALL: [Animal; 8] = [
        Animal::Cat,
        Animal::Fox,
        Animal::Rabbit,
        Animal::Bear,
        Animal::Frog,
        Animal::Panda,
        Animal::Owl,
        Animal::Tanuki,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Animal::Cat => "Cat",
            Animal::Fox => "Fox",
            Animal::Rabbit => "Rabbit",
            Animal::Bear => "Bear",
            Animal::Frog => "Frog",
            Animal::Panda => "Panda",
            Animal::Owl => "Owl",
            Animal::Tanuki => "Tanuki",
        }
    }
}

/// A kid's colour: their card, their marker on a map.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KidColor {
    Coral,
    Amber,
    Leaf,
    Sky,
    Indigo,
    Plum,
}

impl KidColor {
    pub const ALL: [KidColor; 6] =
        [KidColor::Coral, KidColor::Amber, KidColor::Leaf, KidColor::Sky, KidColor::Indigo, KidColor::Plum];

    pub fn name(self) -> &'static str {
        match self {
            KidColor::Coral => "Coral",
            KidColor::Amber => "Amber",
            KidColor::Leaf => "Leaf",
            KidColor::Sky => "Sky",
            KidColor::Indigo => "Indigo",
            KidColor::Plum => "Plum",
        }
    }

    /// As `0xRRGGBB`.
    pub fn rgb(self) -> u32 {
        match self {
            KidColor::Coral => 0xf2735e,
            KidColor::Amber => 0xf5a524,
            KidColor::Leaf => 0x4fb06d,
            KidColor::Sky => 0x45a5e8,
            KidColor::Indigo => 0x5b63d6,
            KidColor::Plum => 0xb05cc4,
        }
    }
}

/// Where a kid is at school, which picks where apps start them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Grade {
    Preschool,
    Kindergarten,
    First,
    Second,
    Third,
    Fourth,
    Fifth,
}

impl Grade {
    pub const ALL: [Grade; 7] =
        [Grade::Preschool, Grade::Kindergarten, Grade::First, Grade::Second, Grade::Third, Grade::Fourth, Grade::Fifth];

    pub fn name(self) -> &'static str {
        match self {
            Grade::Preschool => "Preschool",
            Grade::Kindergarten => "Kindergarten",
            Grade::First => "1st grade",
            Grade::Second => "2nd grade",
            Grade::Third => "3rd grade",
            Grade::Fourth => "4th grade",
            Grade::Fifth => "5th grade",
        }
    }

    /// One word, for a chip.
    pub fn short(self) -> &'static str {
        match self {
            Grade::Preschool => "Pre-K",
            Grade::Kindergarten => "K",
            Grade::First => "1st",
            Grade::Second => "2nd",
            Grade::Third => "3rd",
            Grade::Fourth => "4th",
            Grade::Fifth => "5th",
        }
    }
}

/// A kid.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Profile {
    /// Stable, never shown: the folder of their log.
    pub id: String,
    pub name: String,
    /// Their name spelt how it sounds ("Ree-ah"), for the apps' voices,
    /// when it isn't said the way it's written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sounds_like: Option<String>,
    pub animal: Animal,
    pub color: KidColor,
    pub grade: Grade,
    /// Minutes a day across the catalogue a grown-up allows; none, no limit.
    #[serde(default)]
    pub daily_minutes: Option<u32>,
    /// When the profile was made, in Unix milliseconds.
    pub created: i64,
}

/// What a profile is made from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewProfile {
    pub name: String,
    pub animal: Animal,
    pub color: KidColor,
    pub grade: Grade,
}

impl NewProfile {
    /// The name as it will be kept: trimmed, at most 24 characters.
    pub fn clean_name(name: &str) -> String {
        name.trim().chars().take(24).collect()
    }

    /// How the name sounds, as it will be kept: trimmed, at most 40
    /// characters, and none if that leaves nothing.
    pub fn clean_sound(sound: Option<&str>) -> Option<String> {
        sound.map(|s| s.trim().chars().take(40).collect::<String>()).filter(|s| !s.is_empty())
    }
}
