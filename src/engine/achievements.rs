use crate::engine::ship::ShipType;
use crate::state::GameState;

pub struct Achievement {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub icon: char,
    condition: fn(&GameState) -> bool,
}

pub const ACHIEVEMENTS: &[Achievement] = &[
    Achievement {
        id: "first_blood",
        name: "First Blood",
        description: "Win your first battle",
        icon: '⚔',
        condition: |s| s.total_battles >= 1,
    },
    Achievement {
        id: "fleet_of_5",
        name: "Squadron",
        description: "Build a fleet of 5 ships",
        icon: '🚀',
        condition: |s| s.fleet.len() >= 5,
    },
    Achievement {
        id: "fleet_of_10",
        name: "Armada",
        description: "Build a fleet of 10 ships",
        icon: '⚓',
        condition: |s| s.fleet.len() >= 10,
    },
    Achievement {
        id: "sector_10",
        name: "Explorer",
        description: "Reach sector 10",
        icon: '🌟',
        condition: |s| s.highest_sector >= 10,
    },
    Achievement {
        id: "sector_25",
        name: "Voyager",
        description: "Reach sector 25",
        icon: '✦',
        condition: |s| s.highest_sector >= 25,
    },
    Achievement {
        id: "sector_50",
        name: "Admiral",
        description: "Reach sector 50",
        icon: '◆',
        condition: |s| s.highest_sector >= 50,
    },
    Achievement {
        id: "first_boss",
        name: "Boss Slayer",
        description: "Defeat your first boss",
        icon: '👑',
        // Bosses spawn at sector 10, 20, 30... — must have passed sector 10
        condition: |s| s.highest_sector >= 11,
    },
    Achievement {
        id: "rich",
        name: "Tycoon",
        description: "Accumulate 10,000 credits",
        icon: '💰',
        condition: |s| s.credits >= 10_000,
    },
    Achievement {
        id: "hoarder",
        name: "Hoarder",
        description: "Accumulate 50,000 scrap",
        icon: '◇',
        condition: |s| s.scrap >= 50_000,
    },
    Achievement {
        id: "first_death",
        name: "Learning Experience",
        description: "Lose your fleet for the first time",
        icon: '💀',
        condition: |s| s.deaths >= 1,
    },
    Achievement {
        id: "capital_ship",
        name: "Flagship",
        description: "Build a Capital Ship",
        icon: '⊕',
        condition: |s| s.fleet.iter().any(|ship| ship.ship_type == ShipType::Capital),
    },
    Achievement {
        id: "level_10",
        name: "Seasoned",
        description: "Reach level 10",
        icon: '★',
        condition: |s| s.level >= 10,
    },
    Achievement {
        id: "level_25",
        name: "Veteran",
        description: "Reach level 25",
        icon: '★',
        condition: |s| s.level >= 25,
    },
];

/// Check all achievements against current state. Returns newly unlocked ones.
pub fn check_achievements(state: &GameState) -> Vec<&'static Achievement> {
    ACHIEVEMENTS
        .iter()
        .filter(|a| {
            !state.achievements_unlocked.contains(&a.id.to_string()) && (a.condition)(state)
        })
        .collect()
}
