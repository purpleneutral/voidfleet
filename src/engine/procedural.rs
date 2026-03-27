//! Procedural generation — sector-scaled enemy fleet generation with boss encounters.
//! Includes adaptive difficulty (rubber banding) and fleet composition variety.

use crate::state::GameState;

use crate::engine::factions::Faction;
use crate::engine::voyage::VoyageInfo;

/// Template for a generated enemy ship.
#[derive(Debug, Clone)]
pub struct EnemyTemplate {
    pub name: &'static str,
    pub hp: u32,
    pub damage: u32,
    pub speed: f32,
    pub sprite: &'static [&'static str],
    pub is_boss: bool,
    #[allow(dead_code)] // Faction stamped on enemies for future faction-aware combat
    pub faction: Faction,
}

// -- Enemy sprite constants --------------------------------------------------

const SPRITE_PIRATE_SCOUT: &[&str] = &["◄="];
const SPRITE_MILITIA_FIGHTER: &[&str] = &["◄╔═"];
const SPRITE_MILITIA_GUNSHIP: &[&str] = &["◄══╗", " ══╝"];
const SPRITE_MILITARY_FRIGATE: &[&str] = &["╔═══╗", "◄╣███║", "╚═══╝"];
const SPRITE_MILITARY_DESTROYER: &[&str] = &["  ╔════╗", "◄═╣████║", "  ╚════╝"];
const SPRITE_ELITE_CRUISER: &[&str] = &["  ╔═════╗", "◄═╣█████╠═", "  ╚═════╝"];

const SPRITE_BOSS: &[&str] = &[
    "   ╔══════╗",
    "◄══╣██████╠══",
    "   ╚══════╝",
];

const SPRITE_BOSS_DREADNOUGHT: &[&str] = &[
    "    ╔════════╗",
    "  ╔═╣████████╠═╗",
    "◄═╣██████████████╠═",
    "  ╚═╣████████╠═╝",
    "    ╚════════╝",
];

// -- Adaptive difficulty -----------------------------------------------------

/// Calculate a difficulty modifier based on player state (rubber banding).
/// Returns a multiplier around 1.0:
/// - Over-leveled / strong fleet → >1.0 (enemies tougher)
/// - Under-leveled / weak fleet / died recently → <1.0 (enemies easier)
pub fn difficulty_modifier(state: &GameState) -> f32 {
    let mut modifier = 1.0_f32;

    // Level vs sector expectation
    let expected_level = state.sector / 3 + 1;
    if state.level > expected_level + 5 {
        // Over-leveled: enemies get tougher (up to 1.3x)
        let excess = (state.level - expected_level - 5) as f32;
        modifier += (excess * 0.05).min(0.3);
    } else if state.level + 3 < expected_level {
        // Under-leveled: enemies get easier (down to 0.7x)
        let deficit = (expected_level - state.level - 3) as f32;
        modifier -= (deficit * 0.05).min(0.3);
    }

    // Fleet strength relative to sector
    let expected_dps = 5.0 + state.sector as f32 * 2.5;
    let actual_dps = state.fleet_total_dps();
    if actual_dps > expected_dps * 1.5 {
        modifier += 0.15; // Strong fleet → slightly harder
    } else if actual_dps < expected_dps * 0.5 {
        modifier -= 0.15; // Weak fleet → slightly easier
    }

    // Recent death mercy: if died and haven't recovered past death sector
    if state.deaths > 0 && state.sector <= 10 {
        modifier -= 0.2;
    }

    // Clamp to sane range
    modifier.clamp(0.5, 1.5)
}

/// Post-death easing: returns an additional multiplier if the player just died.
/// Call with `sectors_since_death` — if within 2 sectors of a death, reduce difficulty.
pub fn post_death_modifier(state: &GameState) -> f32 {
    // If player's current sector < highest sector, they died and are catching up
    if state.deaths > 0 && state.sector < state.highest_sector {
        let sectors_behind = state.highest_sector - state.sector;
        if sectors_behind <= 2 {
            return 0.7; // 30% easier for next 2 encounters after death
        }
    }
    1.0
}

// -- Fleet composition variety -----------------------------------------------

/// Variety enum for non-boss fleet compositions.
#[derive(Debug, Clone, Copy)]
enum FleetComposition {
    Standard,     // Mixed fleet (original behavior)
    FighterSwarm, // All small fast ships
    HeavyEscort,  // 1 heavy + small escorts
}

/// Pick a fleet composition for variety.
fn pick_composition(sector: u32) -> FleetComposition {
    let roll = pseudo_random(sector, 99, 10);
    match roll {
        0..=5 => FleetComposition::Standard,
        6..=7 => FleetComposition::FighterSwarm,
        _ => FleetComposition::HeavyEscort,
    }
}

// -- Fleet generation --------------------------------------------------------

/// Generate an enemy fleet scaled for a specific voyage.
/// Voyage affects enemy HP/damage via `enemy_scale` and fleet composition
/// at higher voyages (elite variants, void wraiths, swarm drones).
pub fn generate_enemy_fleet_for_voyage(sector: u32, modifier: f32, voyage: u32) -> Vec<EnemyTemplate> {
    let is_boss_sector = sector.is_multiple_of(10) && sector > 0;
    let voyage_info = VoyageInfo::for_voyage(voyage);

    let mut fleet = Vec::new();

    if is_boss_sector {
        fleet.push(generate_boss(sector));
        // Boss gets escort ships — composition varies
        let escort_count = (sector / 10).min(4) as usize;
        // Every other boss sector: add an extra mixed escort for variety
        let extra = if sector.is_multiple_of(20) { 1 } else { 0 };
        for i in 0..(escort_count + extra) {
            if i == escort_count {
                // Extra escort is always a heavier type
                fleet.push(generate_heavy_escort(sector));
            } else {
                fleet.push(generate_escort(sector));
            }
        }
    } else {
        let composition = pick_composition(sector);
        let mut enemies = match composition {
            FleetComposition::Standard => generate_standard_fleet(sector),
            FleetComposition::FighterSwarm => generate_fighter_swarm(sector),
            FleetComposition::HeavyEscort => generate_heavy_escort_fleet(sector),
        };

        // Voyage 2+: mix in voyage-specific enemy types
        if voyage >= 2 {
            inject_voyage_enemies(&mut enemies, sector, voyage);
        }

        fleet.extend(enemies);
    }

    // Apply sector scaling + adaptive difficulty + voyage scaling to all enemies
    let scale = sector_scale(sector) * modifier * voyage_info.enemy_scale;
    for enemy in &mut fleet {
        enemy.hp = ((enemy.hp as f32 * scale) as u32).max(1);
        enemy.damage = ((enemy.damage as f32 * scale) as u32).max(1);
    }

    fleet
}

/// Inject voyage-specific enemy types into an existing fleet based on voyage number.
fn inject_voyage_enemies(fleet: &mut Vec<EnemyTemplate>, sector: u32, voyage: u32) {
    let roll = pseudo_random(sector, 200, 10);

    if voyage >= 2 && roll < 3 {
        // Elite variant: tougher stats
        fleet.push(EnemyTemplate {
            name: "Elite Variant",
            hp: 60,
            damage: 18,
            speed: 5.5,
            sprite: crate::engine::voyage::SPRITE_ELITE_VARIANT,
            is_boss: false,
            faction: Faction::Independent,
        });
    }

    if voyage >= 3 && roll < 5 {
        // Void Wraith: fast, evasive
        fleet.push(EnemyTemplate {
            name: "Void Wraith",
            hp: 35,
            damage: 12,
            speed: 10.0,
            sprite: crate::engine::voyage::SPRITE_VOID_WRAITH,
            is_boss: false,
            faction: Faction::Independent,
        });
    }

    if voyage >= 4 && roll < 4 {
        // Swarm drones: tiny, many
        let count = 2 + pseudo_random(sector, 210, 3) as usize;
        for _ in 0..count {
            fleet.push(EnemyTemplate {
                name: "Swarm Drone",
                hp: 10,
                damage: 5,
                speed: 12.0,
                sprite: crate::engine::voyage::SPRITE_SWARM_DRONE,
                is_boss: false,
                faction: Faction::Independent,
            });
        }
    }
}



/// Sector difficulty multiplier — gradual power curve.
fn sector_scale(sector: u32) -> f32 {
    1.0 + (sector as f32 - 1.0) * 0.08
}

/// Deterministic-ish "random" from sector number to avoid pulling in rand.
/// Returns 0..modulus based on a simple hash of sector + seed.
fn pseudo_random(sector: u32, seed: u32, modulus: u32) -> u32 {
    let mut h = sector.wrapping_mul(2654435761).wrapping_add(seed.wrapping_mul(40503));
    h ^= h >> 16;
    h.wrapping_mul(0x45d9f3b) % modulus
}

fn generate_standard_fleet(sector: u32) -> Vec<EnemyTemplate> {
    match sector {
        1..=5 => {
            let count = 1 + pseudo_random(sector, 1, 2) as usize; // 1-2
            (0..count)
                .map(|_| EnemyTemplate {
                    name: "Pirate Scout",
                    hp: 8,
                    damage: 2,
                    speed: 9.0,
                    sprite: SPRITE_PIRATE_SCOUT,
                    is_boss: false,
                    faction: Faction::Independent,
                })
                .collect()
        }
        6..=15 => {
            let count = 2 + pseudo_random(sector, 2, 3) as usize; // 2-4
            (0..count)
                .map(|i| {
                    if i % 2 == 0 {
                        EnemyTemplate {
                            name: "Militia Fighter",
                            hp: 18,
                            damage: 6,
                            speed: 7.0,
                            sprite: SPRITE_MILITIA_FIGHTER,
                            is_boss: false,
                    faction: Faction::Independent,
                        }
                    } else {
                        EnemyTemplate {
                            name: "Militia Gunship",
                            hp: 28,
                            damage: 10,
                            speed: 5.0,
                            sprite: SPRITE_MILITIA_GUNSHIP,
                            is_boss: false,
                    faction: Faction::Independent,
                        }
                    }
                })
                .collect()
        }
        16..=25 => {
            let count = 3 + pseudo_random(sector, 3, 4) as usize; // 3-6
            (0..count)
                .map(|i| match i % 3 {
                    0 => EnemyTemplate {
                        name: "Military Frigate",
                        hp: 70,
                        damage: 14,
                        speed: 4.5,
                        sprite: SPRITE_MILITARY_FRIGATE,
                        is_boss: false,
                    faction: Faction::Independent,
                    },
                    1 => EnemyTemplate {
                        name: "Militia Gunship",
                        hp: 28,
                        damage: 10,
                        speed: 5.0,
                        sprite: SPRITE_MILITIA_GUNSHIP,
                        is_boss: false,
                    faction: Faction::Independent,
                    },
                    _ => EnemyTemplate {
                        name: "Militia Fighter",
                        hp: 18,
                        damage: 6,
                        speed: 7.0,
                        sprite: SPRITE_MILITIA_FIGHTER,
                        is_boss: false,
                    faction: Faction::Independent,
                    },
                })
                .collect()
        }
        _ => {
            // 26+
            let count = 4 + pseudo_random(sector, 4, 5) as usize; // 4-8
            (0..count)
                .map(|i| match i % 4 {
                    0 => EnemyTemplate {
                        name: "Military Destroyer",
                        hp: 140,
                        damage: 30,
                        speed: 3.0,
                        sprite: SPRITE_MILITARY_DESTROYER,
                        is_boss: false,
                    faction: Faction::Independent,
                    },
                    1 => EnemyTemplate {
                        name: "Military Frigate",
                        hp: 70,
                        damage: 14,
                        speed: 4.5,
                        sprite: SPRITE_MILITARY_FRIGATE,
                        is_boss: false,
                    faction: Faction::Independent,
                    },
                    2 => EnemyTemplate {
                        name: "Elite Cruiser",
                        hp: 100,
                        damage: 22,
                        speed: 3.5,
                        sprite: SPRITE_ELITE_CRUISER,
                        is_boss: false,
                    faction: Faction::Independent,
                    },
                    _ => EnemyTemplate {
                        name: "Militia Gunship",
                        hp: 28,
                        damage: 10,
                        speed: 5.0,
                        sprite: SPRITE_MILITIA_GUNSHIP,
                        is_boss: false,
                    faction: Faction::Independent,
                    },
                })
                .collect()
        }
    }
}

/// All-fighter swarm: many small fast ships.
fn generate_fighter_swarm(sector: u32) -> Vec<EnemyTemplate> {
    let count = match sector {
        1..=5 => 2 + pseudo_random(sector, 10, 2) as usize,
        6..=15 => 3 + pseudo_random(sector, 10, 3) as usize,
        16..=25 => 5 + pseudo_random(sector, 10, 4) as usize,
        _ => 6 + pseudo_random(sector, 10, 5) as usize,
    };

    let (name, hp, damage, speed, sprite) = if sector >= 16 {
        ("Militia Fighter", 18, 6, 7.0, SPRITE_MILITIA_FIGHTER as &[&str])
    } else {
        ("Pirate Scout", 8, 2, 9.0, SPRITE_PIRATE_SCOUT as &[&str])
    };

    (0..count)
        .map(|_| EnemyTemplate {
            name,
            hp,
            damage,
            speed,
            sprite,
            is_boss: false,
                    faction: Faction::Independent,
        })
        .collect()
}

/// Heavy escort fleet: 1 big ship + small escorts.
fn generate_heavy_escort_fleet(sector: u32) -> Vec<EnemyTemplate> {
    let mut fleet = Vec::new();

    // The heavy
    let heavy = if sector >= 26 {
        EnemyTemplate {
            name: "Military Destroyer",
            hp: 140,
            damage: 30,
            speed: 3.0,
            sprite: SPRITE_MILITARY_DESTROYER,
            is_boss: false,
                    faction: Faction::Independent,
        }
    } else if sector >= 16 {
        EnemyTemplate {
            name: "Military Frigate",
            hp: 70,
            damage: 14,
            speed: 4.5,
            sprite: SPRITE_MILITARY_FRIGATE,
            is_boss: false,
                    faction: Faction::Independent,
        }
    } else {
        EnemyTemplate {
            name: "Militia Gunship",
            hp: 28,
            damage: 10,
            speed: 5.0,
            sprite: SPRITE_MILITIA_GUNSHIP,
            is_boss: false,
                    faction: Faction::Independent,
        }
    };

    fleet.push(heavy);

    // Escorts: 1-3 small ships
    let escort_count = 1 + pseudo_random(sector, 20, 3) as usize;
    for _ in 0..escort_count {
        fleet.push(EnemyTemplate {
            name: "Pirate Scout",
            hp: 8,
            damage: 2,
            speed: 9.0,
            sprite: SPRITE_PIRATE_SCOUT,
            is_boss: false,
                    faction: Faction::Independent,
        });
    }

    fleet
}

fn generate_boss(sector: u32) -> EnemyTemplate {
    // Boss tier increases every 10 sectors
    let tier = sector / 10;
    match tier {
        1 => EnemyTemplate {
            name: "Pirate Warlord",
            hp: 8 * 5,  // 5x pirate scout base
            damage: 12,
            speed: 4.0,
            sprite: SPRITE_BOSS,
            is_boss: true,
                    faction: Faction::Independent,
        },
        2 => EnemyTemplate {
            name: "Militia Commander",
            hp: 18 * 5,
            damage: 25,
            speed: 3.5,
            sprite: SPRITE_BOSS,
            is_boss: true,
                    faction: Faction::Independent,
        },
        3 => EnemyTemplate {
            name: "Admiral Vex",
            hp: 70 * 5,
            damage: 45,
            speed: 3.0,
            sprite: SPRITE_BOSS_DREADNOUGHT,
            is_boss: true,
                    faction: Faction::Independent,
        },
        _ => EnemyTemplate {
            name: "Void Dreadnought",
            hp: 140 * 5,
            damage: 80,
            speed: 2.0,
            sprite: SPRITE_BOSS_DREADNOUGHT,
            is_boss: true,
                    faction: Faction::Independent,
        },
    }
}

fn generate_heavy_escort(sector: u32) -> EnemyTemplate {
    if sector >= 40 {
        EnemyTemplate {
            name: "Elite Cruiser",
            hp: 100,
            damage: 22,
            speed: 3.5,
            sprite: SPRITE_ELITE_CRUISER,
            is_boss: false,
                    faction: Faction::Independent,
        }
    } else {
        EnemyTemplate {
            name: "Military Frigate",
            hp: 70,
            damage: 14,
            speed: 4.5,
            sprite: SPRITE_MILITARY_FRIGATE,
            is_boss: false,
                    faction: Faction::Independent,
        }
    }
}

fn generate_escort(sector: u32) -> EnemyTemplate {
    if sector >= 30 {
        EnemyTemplate {
            name: "Elite Escort",
            hp: 60,
            damage: 18,
            speed: 5.0,
            sprite: SPRITE_MILITARY_FRIGATE,
            is_boss: false,
                    faction: Faction::Independent,
        }
    } else {
        EnemyTemplate {
            name: "Escort Fighter",
            hp: 15,
            damage: 5,
            speed: 8.0,
            sprite: SPRITE_MILITIA_FIGHTER,
            is_boss: false,
                    faction: Faction::Independent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_early_sector_fleet_size() {
        let fleet = generate_enemy_fleet_for_voyage(3, 1.0, 1);
        assert!(!fleet.is_empty());
        assert!(fleet.len() <= 2);
        assert!(fleet.iter().all(|e| !e.is_boss));
    }

    #[test]
    fn test_boss_sector() {
        let fleet = generate_enemy_fleet_for_voyage(10, 1.0, 1);
        assert!(fleet.iter().any(|e| e.is_boss));
    }

    #[test]
    fn test_boss_has_escorts() {
        let fleet = generate_enemy_fleet_for_voyage(20, 1.0, 1);
        let bosses: Vec<_> = fleet.iter().filter(|e| e.is_boss).collect();
        let escorts: Vec<_> = fleet.iter().filter(|e| !e.is_boss).collect();
        assert_eq!(bosses.len(), 1);
        assert!(!escorts.is_empty());
    }

    #[test]
    fn test_scaling_increases_stats() {
        let early = generate_enemy_fleet_for_voyage(1, 1.0, 1);
        let late = generate_enemy_fleet_for_voyage(30, 1.0, 1);
        // Late-game non-boss enemies should have more HP than early pirates
        let early_max_hp = early.iter().map(|e| e.hp).max().unwrap();
        let late_max_hp = late.iter().filter(|e| !e.is_boss).map(|e| e.hp).max().unwrap();
        assert!(late_max_hp > early_max_hp);
    }

    #[test]
    fn test_sector_scale() {
        assert!((sector_scale(1) - 1.0).abs() < f32::EPSILON);
        assert!(sector_scale(10) > 1.5);
    }
}
