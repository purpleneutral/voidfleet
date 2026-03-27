/// Procedural generation — sector-scaled enemy fleet generation with boss encounters.

/// Template for a generated enemy ship.
#[derive(Debug, Clone)]
pub struct EnemyTemplate {
    pub name: &'static str,
    pub hp: u32,
    pub damage: u32,
    pub speed: f32,
    pub sprite: &'static [&'static str],
    pub is_boss: bool,
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

// -- Fleet generation --------------------------------------------------------

/// Generate an enemy fleet for a given sector.
/// Scales difficulty with sector number; every 10th sector spawns a boss + escorts.
pub fn generate_enemy_fleet(sector: u32) -> Vec<EnemyTemplate> {
    let is_boss_sector = sector % 10 == 0 && sector > 0;

    let mut fleet = Vec::new();

    if is_boss_sector {
        fleet.push(generate_boss(sector));
        // Boss gets escort ships
        let escort_count = (sector / 10).min(4) as usize;
        for _ in 0..escort_count {
            fleet.push(generate_escort(sector));
        }
    } else {
        let enemies = generate_standard_fleet(sector);
        fleet.extend(enemies);
    }

    // Apply sector scaling to all enemies
    let scale = sector_scale(sector);
    for enemy in &mut fleet {
        enemy.hp = (enemy.hp as f32 * scale) as u32;
        enemy.damage = (enemy.damage as f32 * scale) as u32;
    }

    fleet
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
                        }
                    } else {
                        EnemyTemplate {
                            name: "Militia Gunship",
                            hp: 28,
                            damage: 10,
                            speed: 5.0,
                            sprite: SPRITE_MILITIA_GUNSHIP,
                            is_boss: false,
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
                    },
                    1 => EnemyTemplate {
                        name: "Militia Gunship",
                        hp: 28,
                        damage: 10,
                        speed: 5.0,
                        sprite: SPRITE_MILITIA_GUNSHIP,
                        is_boss: false,
                    },
                    _ => EnemyTemplate {
                        name: "Militia Fighter",
                        hp: 18,
                        damage: 6,
                        speed: 7.0,
                        sprite: SPRITE_MILITIA_FIGHTER,
                        is_boss: false,
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
                    },
                    1 => EnemyTemplate {
                        name: "Military Frigate",
                        hp: 70,
                        damage: 14,
                        speed: 4.5,
                        sprite: SPRITE_MILITARY_FRIGATE,
                        is_boss: false,
                    },
                    2 => EnemyTemplate {
                        name: "Elite Cruiser",
                        hp: 100,
                        damage: 22,
                        speed: 3.5,
                        sprite: SPRITE_ELITE_CRUISER,
                        is_boss: false,
                    },
                    _ => EnemyTemplate {
                        name: "Militia Gunship",
                        hp: 28,
                        damage: 10,
                        speed: 5.0,
                        sprite: SPRITE_MILITIA_GUNSHIP,
                        is_boss: false,
                    },
                })
                .collect()
        }
    }
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
        },
        2 => EnemyTemplate {
            name: "Militia Commander",
            hp: 18 * 5,
            damage: 25,
            speed: 3.5,
            sprite: SPRITE_BOSS,
            is_boss: true,
        },
        3 => EnemyTemplate {
            name: "Admiral Vex",
            hp: 70 * 5,
            damage: 45,
            speed: 3.0,
            sprite: SPRITE_BOSS_DREADNOUGHT,
            is_boss: true,
        },
        _ => EnemyTemplate {
            name: "Void Dreadnought",
            hp: 140 * 5,
            damage: 80,
            speed: 2.0,
            sprite: SPRITE_BOSS_DREADNOUGHT,
            is_boss: true,
        },
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
        }
    } else {
        EnemyTemplate {
            name: "Escort Fighter",
            hp: 15,
            damage: 5,
            speed: 8.0,
            sprite: SPRITE_MILITIA_FIGHTER,
            is_boss: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_early_sector_fleet_size() {
        let fleet = generate_enemy_fleet(3);
        assert!(!fleet.is_empty());
        assert!(fleet.len() <= 2);
        assert!(fleet.iter().all(|e| !e.is_boss));
    }

    #[test]
    fn test_boss_sector() {
        let fleet = generate_enemy_fleet(10);
        assert!(fleet.iter().any(|e| e.is_boss));
    }

    #[test]
    fn test_boss_has_escorts() {
        let fleet = generate_enemy_fleet(20);
        let bosses: Vec<_> = fleet.iter().filter(|e| e.is_boss).collect();
        let escorts: Vec<_> = fleet.iter().filter(|e| !e.is_boss).collect();
        assert_eq!(bosses.len(), 1);
        assert!(!escorts.is_empty());
    }

    #[test]
    fn test_scaling_increases_stats() {
        let early = generate_enemy_fleet(1);
        let late = generate_enemy_fleet(30);
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
