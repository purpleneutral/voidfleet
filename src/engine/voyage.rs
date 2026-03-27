/// Voyage system — escalating prestige with themed cycles.
///
/// Each voyage represents a full run through an expanding universe.
/// Completing a voyage grants permanent bonuses and resets progression.

use serde::{Deserialize, Serialize};

// ── Voyage bonuses ──────────────────────────────────────────────────────────

/// Accumulated permanent bonuses from completing voyages.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VoyageBonuses {
    pub damage_pct: f32,
    pub hull_hp_pct: f32,
    pub speed_pct: f32,
    pub crit_pct: f32,
}

impl VoyageBonuses {
    /// Calculate bonuses earned for completing a specific voyage.
    pub fn for_completion(_voyage: u32) -> Self {
        VoyageBonuses {
            damage_pct: VOYAGE_DAMAGE_BONUS * 100.0,  // stored as percentage
            hull_hp_pct: VOYAGE_HP_BONUS * 100.0,
            speed_pct: VOYAGE_SPEED_BONUS * 100.0,
            crit_pct: VOYAGE_CRIT_BONUS * 100.0,
        }
    }

    /// Add another set of bonuses onto this one.
    pub fn accumulate(&mut self, other: &VoyageBonuses) {
        self.damage_pct += other.damage_pct;
        self.hull_hp_pct += other.hull_hp_pct;
        self.speed_pct += other.speed_pct;
        self.crit_pct += other.crit_pct;
    }

    /// Format as a compact HUD string. Returns empty if no bonuses.
    pub fn hud_string(&self) -> String {
        if self.damage_pct == 0.0 && self.hull_hp_pct == 0.0 {
            return String::new();
        }
        format!(
            "+{:.0}% DMG, +{:.0}% HP",
            self.damage_pct, self.hull_hp_pct
        )
    }
}

/// Stats tracked per voyage for the cinematic screen.
#[derive(Debug, Clone, Default)]
pub struct VoyageStats {
    pub sectors_cleared: u32,
    pub battles_won: u64,
    pub enemies_destroyed: u64,
    pub ships_built: u64,
    pub crew_recruited: u64,
    pub equipment_found: u64,
    pub credits_earned: u64,
    pub time_played_secs: u64,
}

// ── Voyage definitions ──────────────────────────────────────────────────────

/// Metadata for a single voyage cycle.
#[derive(Debug, Clone)]
pub struct VoyageInfo {
    pub number: u32,
    pub name: &'static str,
    pub subtitle: &'static str,
    pub target_sector: u32,
    pub enemy_scale: f32,
    pub equipment_tier: u8,
}

/// The first 5 voyages have unique themes; beyond that, they scale infinitely.
const VOYAGES: [VoyageInfo; 5] = [
    VoyageInfo {
        number: 1,
        name: "The Frontier",
        subtitle: "Chart the unknown",
        target_sector: 100,
        enemy_scale: 1.0,
        equipment_tier: 1,
    },
    VoyageInfo {
        number: 2,
        name: "The Expanse",
        subtitle: "Push deeper into hostile space",
        target_sector: 200,
        enemy_scale: 1.3,
        equipment_tier: 2,
    },
    VoyageInfo {
        number: 3,
        name: "The Void",
        subtitle: "Where light fears to reach",
        target_sector: 300,
        enemy_scale: 1.6,
        equipment_tier: 3,
    },
    VoyageInfo {
        number: 4,
        name: "The Abyss",
        subtitle: "Ancient horrors stir",
        target_sector: 400,
        enemy_scale: 2.0,
        equipment_tier: 4,
    },
    VoyageInfo {
        number: 5,
        name: "The Infinite",
        subtitle: "Beyond all maps",
        target_sector: 500,
        enemy_scale: 2.5,
        equipment_tier: 5,
    },
];

impl VoyageInfo {
    /// Get voyage info for any voyage number (1-indexed).
    /// Voyages 1–5 are hand-crafted; 6+ scale infinitely from The Infinite.
    pub fn for_voyage(num: u32) -> Self {
        let num = num.max(1);
        if num <= 5 {
            VOYAGES[(num - 1) as usize].clone()
        } else {
            let extra = num - 5;
            let suffix = match extra {
                1 => "II",
                2 => "III",
                3 => "IV",
                4 => "V",
                5 => "VI",
                6 => "VII",
                7 => "VIII",
                8 => "IX",
                9 => "X",
                _ => "∞",
            };
            VoyageInfo {
                number: num,
                name: "The Infinite",
                subtitle: suffix, // Caller can format "{name} {subtitle}"
                target_sector: 500 + extra * 100,
                enemy_scale: 2.5 + extra as f32 * 0.5,
                equipment_tier: (5 + extra as u8).min(10),
            }
        }
    }

    /// Format the full display name (e.g., "The Infinite III").
    pub fn display_name(&self) -> String {
        if self.number <= 5 {
            self.name.to_string()
        } else {
            format!("{} {}", self.name, self.subtitle)
        }
    }
}

// ── Permanent bonuses ───────────────────────────────────────────────────────

/// Per-voyage permanent bonus increments.
pub const VOYAGE_DAMAGE_BONUS: f32 = 0.05;
pub const VOYAGE_HP_BONUS: f32 = 0.05;
pub const VOYAGE_SPEED_BONUS: f32 = 0.03;
pub const VOYAGE_CRIT_BONUS: f32 = 0.02;
/// Starting credits granted each new voyage.
pub const VOYAGE_STARTING_CREDITS: u64 = 100;

// ── Voyage boss sprites ─────────────────────────────────────────────────────

pub const SPRITE_PIRATE_WARLORD: &[&str] = &[
    "    ╔══════╗",
    "  ╔═╣██████╠═╗",
    "◄═══╣████████╠═══►",
    "  ╚═╣██████╠═╝",
    "    ╚══════╝",
];

pub const SPRITE_FLEET_ADMIRAL: &[&str] = &[
    "     ╔════════╗",
    "   ╔═╣████████╠═╗",
    " ◄═══╣██████████╠═══►",
    "   ╚═╣████████╠═╝",
    "     ╚════════╝",
];

pub const SPRITE_VOID_LEVIATHAN: &[&str] = &[
    "      ╔══════════════╗",
    "    ╔═╣██████████████╠═══╗",
    "  ◄═══╣████████████████████╠═══►",
    "    ╚═╣██████████████╠═══╝",
    "      ╚══════════════╝",
];

pub const SPRITE_ELDRITCH_TITAN: &[&str] = &[
    "        ╔═══╗",
    "      ╔═╣███╠═╗",
    "    ╔═╣███████╠═╗",
    "  ◄═══╣█████████╠═══►",
    "    ╚═╣███████╠═╝",
    "      ╚═╣███╠═╝",
    "        ╚═══╝",
];

// Voyage 2+ enemy types
pub const SPRITE_VOID_WRAITH: &[&str] = &["◄~≈~≈"];
pub const SPRITE_SWARM_DRONE: &[&str] = &["◄·"];
pub const SPRITE_ELITE_VARIANT: &[&str] = &["╔══╗", "◄╣██╠═", "╚══╝"];

// ── Voyage boss generation ──────────────────────────────────────────────────

use crate::engine::factions::Faction;
use crate::engine::procedural::EnemyTemplate;

/// Generate the voyage boss fleet for the end of a voyage.
pub fn generate_voyage_boss(voyage: u32) -> Vec<EnemyTemplate> {
    let voyage_info = VoyageInfo::for_voyage(voyage);
    let scale = voyage_info.enemy_scale;

    match voyage {
        1 => {
            // Pirate Warlord — large ship + 4 escorts
            let mut fleet = vec![EnemyTemplate {
                name: "Pirate Warlord",
                hp: (250.0 * scale) as u32,
                damage: (30.0 * scale) as u32,
                speed: 3.5,
                sprite: SPRITE_PIRATE_WARLORD,
                is_boss: true,
                faction: Faction::Independent,
            }];
            for _ in 0..4 {
                fleet.push(EnemyTemplate {
                    name: "Warlord Escort",
                    hp: (40.0 * scale) as u32,
                    damage: (10.0 * scale) as u32,
                    speed: 7.0,
                    sprite: &["◄═╗", " ═╝"],
                    is_boss: false,
                    faction: Faction::Independent,
                });
            }
            fleet
        }
        2 => {
            // Fleet Admiral — 2 capital ships + fighter screen
            let mut fleet = vec![
                EnemyTemplate {
                    name: "Fleet Admiral",
                    hp: (500.0 * scale) as u32,
                    damage: (50.0 * scale) as u32,
                    speed: 2.5,
                    sprite: SPRITE_FLEET_ADMIRAL,
                    is_boss: true,
                    faction: Faction::Independent,
                },
                EnemyTemplate {
                    name: "Admiral's Flagship",
                    hp: (350.0 * scale) as u32,
                    damage: (40.0 * scale) as u32,
                    speed: 3.0,
                    sprite: SPRITE_PIRATE_WARLORD,
                    is_boss: false,
                    faction: Faction::Independent,
                },
            ];
            for _ in 0..4 {
                fleet.push(EnemyTemplate {
                    name: "Navy Fighter",
                    hp: (30.0 * scale) as u32,
                    damage: (12.0 * scale) as u32,
                    speed: 8.0,
                    sprite: &["◄╔═"],
                    is_boss: false,
                    faction: Faction::Independent,
                });
            }
            fleet
        }
        3 => {
            // Void Leviathan — massive sprite, high HP
            let mut fleet = vec![EnemyTemplate {
                name: "Void Leviathan",
                hp: (1000.0 * scale) as u32,
                damage: (80.0 * scale) as u32,
                speed: 1.5,
                sprite: SPRITE_VOID_LEVIATHAN,
                is_boss: true,
                faction: Faction::Independent,
            }];
            // Spawns void wraith escorts
            for _ in 0..6 {
                fleet.push(EnemyTemplate {
                    name: "Void Wraith",
                    hp: (50.0 * scale) as u32,
                    damage: (15.0 * scale) as u32,
                    speed: 9.0,
                    sprite: SPRITE_VOID_WRAITH,
                    is_boss: false,
                    faction: Faction::Independent,
                });
            }
            fleet
        }
        4 => {
            // Eldritch Titan — 7-line sprite, summons minions
            let mut fleet = vec![EnemyTemplate {
                name: "Eldritch Titan",
                hp: (2000.0 * scale) as u32,
                damage: (120.0 * scale) as u32,
                speed: 1.0,
                sprite: SPRITE_ELDRITCH_TITAN,
                is_boss: true,
                faction: Faction::Independent,
            }];
            for _ in 0..8 {
                fleet.push(EnemyTemplate {
                    name: "Swarm Drone",
                    hp: (15.0 * scale) as u32,
                    damage: (8.0 * scale) as u32,
                    speed: 12.0,
                    sprite: SPRITE_SWARM_DRONE,
                    is_boss: false,
                    faction: Faction::Independent,
                });
            }
            fleet
        }
        _ => {
            // Voyage 5+: scaled Eldritch Titan + more minions
            let extra = (voyage - 4) as f32;
            let mut fleet = vec![EnemyTemplate {
                name: "Eldritch Titan",
                hp: (2000.0 * scale + extra * 500.0) as u32,
                damage: (120.0 * scale + extra * 30.0) as u32,
                speed: 1.0,
                sprite: SPRITE_ELDRITCH_TITAN,
                is_boss: true,
                faction: Faction::Independent,
            }];
            let minion_count = 8 + (voyage - 4).min(8) as usize;
            for _ in 0..minion_count {
                fleet.push(EnemyTemplate {
                    name: "Void Wraith",
                    hp: (50.0 * scale) as u32,
                    damage: (15.0 * scale) as u32,
                    speed: 9.0,
                    sprite: SPRITE_VOID_WRAITH,
                    is_boss: false,
                    faction: Faction::Independent,
                });
            }
            fleet
        }
    }
}

/// Check if a given sector is the voyage boss sector for the current voyage.
pub fn is_voyage_boss_sector(sector: u32, voyage: u32) -> bool {
    let info = VoyageInfo::for_voyage(voyage);
    sector == info.target_sector
}

/// Get the target sector for a given voyage.
pub fn voyage_target_sector(voyage: u32) -> u32 {
    VoyageInfo::for_voyage(voyage).target_sector
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voyage_info_fixed() {
        let v1 = VoyageInfo::for_voyage(1);
        assert_eq!(v1.number, 1);
        assert_eq!(v1.name, "The Frontier");
        assert_eq!(v1.target_sector, 100);
        assert!((v1.enemy_scale - 1.0).abs() < f32::EPSILON);

        let v5 = VoyageInfo::for_voyage(5);
        assert_eq!(v5.number, 5);
        assert_eq!(v5.name, "The Infinite");
        assert_eq!(v5.target_sector, 500);
        assert!((v5.enemy_scale - 2.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_voyage_info_infinite_scaling() {
        let v6 = VoyageInfo::for_voyage(6);
        assert_eq!(v6.number, 6);
        assert_eq!(v6.target_sector, 600);
        assert!((v6.enemy_scale - 3.0).abs() < f32::EPSILON);
        assert_eq!(v6.display_name(), "The Infinite II");

        let v8 = VoyageInfo::for_voyage(8);
        assert_eq!(v8.target_sector, 800);
        assert!((v8.enemy_scale - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_voyage_info_zero_clamps_to_one() {
        let v = VoyageInfo::for_voyage(0);
        assert_eq!(v.number, 1);
        assert_eq!(v.name, "The Frontier");
    }

    #[test]
    fn test_is_voyage_boss_sector() {
        assert!(is_voyage_boss_sector(100, 1));
        assert!(!is_voyage_boss_sector(99, 1));
        assert!(is_voyage_boss_sector(200, 2));
        assert!(is_voyage_boss_sector(600, 6));
    }

    #[test]
    fn test_voyage_boss_generation() {
        for v in 1..=6 {
            let fleet = generate_voyage_boss(v);
            assert!(!fleet.is_empty(), "Voyage {} boss fleet should not be empty", v);
            assert!(
                fleet.iter().any(|e| e.is_boss),
                "Voyage {} boss fleet should have a boss", v
            );
            // Boss should have significant HP
            let boss = fleet.iter().find(|e| e.is_boss).unwrap();
            assert!(boss.hp >= 200, "Voyage {} boss HP should be >= 200, got {}", v, boss.hp);
        }
    }

    #[test]
    fn test_voyage_boss_scaling() {
        let v1_boss = generate_voyage_boss(1);
        let v4_boss = generate_voyage_boss(4);
        let v1_hp: u32 = v1_boss.iter().filter(|e| e.is_boss).map(|e| e.hp).sum();
        let v4_hp: u32 = v4_boss.iter().filter(|e| e.is_boss).map(|e| e.hp).sum();
        assert!(v4_hp > v1_hp, "Later voyage bosses should have more HP");
    }

    #[test]
    fn test_voyage_target_sector() {
        assert_eq!(voyage_target_sector(1), 100);
        assert_eq!(voyage_target_sector(5), 500);
        assert_eq!(voyage_target_sector(7), 700);
    }

    #[test]
    fn test_equipment_tier_caps() {
        let v10 = VoyageInfo::for_voyage(10);
        assert!(v10.equipment_tier <= 10);
        let v20 = VoyageInfo::for_voyage(20);
        assert!(v20.equipment_tier <= 10);
    }
}
