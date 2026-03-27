//! Mission / Contract system — procedurally generated missions from factions.
//!
//! Missions are offered at stations based on the dominant faction in the current sector.
//! Players accept missions, track progress across sector transitions, and receive
//! rewards (credits, reputation, equipment) on completion.

use serde::{Deserialize, Serialize};

use crate::engine::factions::Faction;

// ── Mission Types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mission {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub mission_type: MissionType,
    pub faction: Faction,        // issuing faction
    pub reward_credits: u64,
    pub reward_rep: i32,
    pub reward_equipment: bool,  // chance of equipment on completion
    pub target_sector: u32,      // sector where mission completes (or sectors to travel)
    pub sectors_remaining: u32,  // countdown for delivery/escort missions
    pub status: MissionStatus,
    pub difficulty: u8,          // 1-5 stars
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MissionType {
    BountyHunt,     // destroy a specific enemy in target sector
    Delivery,       // carry goods for N sectors
    Escort,         // protect NPC convoy for N sectors (no fleet deaths)
    Exploration,    // reach a specific sector
    Sabotage,       // raid a faction's planet (hurts rep with them)
    Rescue,         // reach sector, gain crew member
    TradeRun,       // buy goods in sector A, sell in sector B
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MissionStatus {
    Available,
    Active,
    Completed,
    Failed,
    Expired,
}

impl Mission {
    /// Whether the mission is complete.
    pub fn is_complete(&self) -> bool {
        self.status == MissionStatus::Completed
    }

    /// Progress percentage (0.0 to 1.0) for display purposes.
    /// For countdown missions (Delivery/Escort), based on sectors_remaining vs original distance.
    /// For target-sector missions, based on current proximity.
    pub fn progress_pct(&self, current_sector: u32) -> f32 {
        match self.mission_type {
            MissionType::Delivery | MissionType::Escort | MissionType::TradeRun => {
                if self.status == MissionStatus::Completed {
                    return 1.0;
                }
                let total = self.target_sector.saturating_sub(
                    self.target_sector.saturating_sub(self.sectors_remaining + 1),
                );
                if total == 0 {
                    return 1.0;
                }
                let done = total.saturating_sub(self.sectors_remaining);
                (done as f32 / total as f32).clamp(0.0, 1.0)
            }
            _ => {
                if self.status == MissionStatus::Completed {
                    return 1.0;
                }
                if current_sector >= self.target_sector {
                    return 1.0;
                }
                // Approximate: assume mission was accepted ~5 sectors before target
                let estimated_start = self.target_sector.saturating_sub(10);
                let range = self.target_sector.saturating_sub(estimated_start);
                if range == 0 {
                    return 0.0;
                }
                let progress = current_sector.saturating_sub(estimated_start);
                (progress as f32 / range as f32).clamp(0.0, 0.99)
            }
        }
    }
}

impl MissionType {
    /// Icon for display.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::BountyHunt => "⚔",
            Self::Delivery => "📦",
            Self::Escort => "🛡",
            Self::Exploration => "🔭",
            Self::Sabotage => "💣",
            Self::Rescue => "🚑",
            Self::TradeRun => "💰",
        }
    }

    /// Short label for display.
    pub fn label(&self) -> &'static str {
        match self {
            Self::BountyHunt => "Bounty",
            Self::Delivery => "Delivery",
            Self::Escort => "Escort",
            Self::Exploration => "Exploration",
            Self::Sabotage => "Sabotage",
            Self::Rescue => "Rescue",
            Self::TradeRun => "Trade Run",
        }
    }
}

/// Result of checking mission progress after a sector transition.
#[derive(Debug, Clone)]
pub struct MissionUpdate {
    pub mission_id: u64,
    pub title: String,
    pub update_type: MissionUpdateType,
}

#[derive(Debug, Clone)]
pub enum MissionUpdateType {
    Completed { reward_credits: u64, reward_rep: i32, reward_equipment: bool },
    Failed { reason: String },
    Progress { sectors_remaining: u32 },
}

// ── Name Generation ─────────────────────────────────────────────────────────

const BOUNTY_NAMES: &[&str] = &[
    "Vex", "Karn", "Zul", "Drax", "Mor", "Skev", "Thar", "Rynn",
    "Grix", "Ashk", "Vorn", "Jak", "Bael", "Crix", "Nyx",
];

const BOUNTY_TITLES: &[&str] = &[
    "Pirate Lord", "Rogue Commander", "War Criminal", "Smuggler King",
    "Void Raider", "Shadow Captain", "Scourge", "Marauder",
    "Corsair", "Outlaw Baron",
];

const NEBULA_NAMES: &[&str] = &[
    "Crimson", "Shattered", "Obsidian", "Frozen", "Silent",
    "Burning", "Abyssal", "Phantom", "Crystal", "Hollow",
    "Drifting", "Iron", "Void", "Tempest", "Ashen",
];

const RESCUE_NAMES: &[&str] = &[
    "Dr. Yael", "Cpt. Mercer", "Eng. Tanaka", "Pvt. Orin", "Lt. Voss",
    "Cmdr. Reyes", "Prof. Amir", "Sgt. Kane", "Nav. Lira", "Tech. Brin",
];

/// Deterministic pseudo-random: returns 0..modulus based on hash of inputs.
fn pseudo_random(a: u32, b: u32, modulus: u32) -> u32 {
    if modulus == 0 {
        return 0;
    }
    let mut h = a.wrapping_mul(2654435761).wrapping_add(b.wrapping_mul(40503));
    h ^= h >> 16;
    h.wrapping_mul(0x45d9f3b) % modulus
}

fn generate_bounty_title(sector: u32, mission_seed: u32) -> (String, String) {
    let name_idx = pseudo_random(sector, mission_seed, BOUNTY_NAMES.len() as u32) as usize;
    let title_idx = pseudo_random(sector, mission_seed.wrapping_add(7), BOUNTY_TITLES.len() as u32) as usize;
    let name = BOUNTY_NAMES[name_idx];
    let title = BOUNTY_TITLES[title_idx];
    (
        format!("Eliminate {} the {}", name, title),
        format!("Hunt down the notorious {} known as '{}' operating near the target sector.", title.to_lowercase(), name),
    )
}

fn generate_delivery_title(target_sector: u32) -> (String, String) {
    (
        format!("Supply Run to Sector {}", target_sector),
        format!("Deliver critical supplies across {} sectors. Payment on arrival.", target_sector),
    )
}

fn generate_escort_title(faction: &Faction) -> (String, String) {
    (
        format!("Protect {} Convoy", faction.name()),
        format!("Escort a {} trade convoy safely. Bonus if no ships are lost.", faction.name()),
    )
}

fn generate_exploration_title(sector: u32, mission_seed: u32) -> (String, String) {
    let idx = pseudo_random(sector, mission_seed, NEBULA_NAMES.len() as u32) as usize;
    let nebula = NEBULA_NAMES[idx];
    (
        format!("Chart the {} Nebula", nebula),
        format!("Explore uncharted space and reach the {} Nebula region. High risk, high reward.", nebula),
    )
}

fn generate_sabotage_title(target_faction: &Faction) -> (String, String) {
    (
        format!("Strike {} Outpost", target_faction.name()),
        format!(
            "Raid a {} outpost. Warning: this will damage your reputation with {}.",
            target_faction.name(),
            target_faction.name(),
        ),
    )
}

fn generate_rescue_title(sector: u32, target_sector: u32, mission_seed: u32) -> (String, String) {
    let idx = pseudo_random(sector, mission_seed.wrapping_add(13), RESCUE_NAMES.len() as u32) as usize;
    let name = RESCUE_NAMES[idx];
    (
        format!("Rescue {} from Sector {}", name, target_sector),
        format!("{} is stranded in a hostile sector. Reach them and bring them aboard.", name),
    )
}

fn generate_traderun_title(target_sector: u32) -> (String, String) {
    (
        format!("Trade Run to Sector {}", target_sector),
        "Purchase specific goods and deliver them to the target sector for profit.".to_string(),
    )
}

// ── Mission Generation ──────────────────────────────────────────────────────

/// Generate N missions appropriate for the current sector and faction.
/// Uses deterministic seeding based on sector + faction + index for reproducibility
/// within a given sector visit, but `next_id` ensures unique IDs across game state.
pub fn generate_missions(sector: u32, faction: &Faction, count: usize, next_id: &mut u64) -> Vec<Mission> {
    let mut missions = Vec::with_capacity(count);

    for i in 0..count {
        let seed = sector.wrapping_mul(31).wrapping_add(i as u32).wrapping_add(*faction as u32 * 97);
        let mission_type = match pseudo_random(sector, seed, 7) {
            0 => MissionType::BountyHunt,
            1 => MissionType::Delivery,
            2 => MissionType::Escort,
            3 => MissionType::Exploration,
            4 => MissionType::Sabotage,
            5 => MissionType::Rescue,
            _ => MissionType::TradeRun,
        };

        let difficulty = (1 + pseudo_random(sector, seed.wrapping_add(3), 5)) as u8; // 1-5
        let base_reward = 50 + (sector as u64) * 20 + (difficulty as u64) * 30;

        let (target_sector, sectors_remaining, title, description) = match mission_type {
            MissionType::BountyHunt => {
                let target = sector + 2 + pseudo_random(sector, seed.wrapping_add(1), 4); // +2..+5
                let (t, d) = generate_bounty_title(sector, seed);
                (target, 0, t, d)
            }
            MissionType::Delivery => {
                let distance = 5 + pseudo_random(sector, seed.wrapping_add(1), 6); // 5-10
                let target = sector + distance;
                let (t, d) = generate_delivery_title(target);
                (target, distance, t, d)
            }
            MissionType::Escort => {
                let distance = 3 + pseudo_random(sector, seed.wrapping_add(1), 5); // 3-7
                let target = sector + distance;
                let (t, d) = generate_escort_title(faction);
                (target, distance, t, d)
            }
            MissionType::Exploration => {
                let target = sector + 8 + pseudo_random(sector, seed.wrapping_add(1), 8); // +8..+15
                let (t, d) = generate_exploration_title(sector, seed);
                (target, 0, t, d)
            }
            MissionType::Sabotage => {
                // Pick a rival faction as the target
                let rivals = faction.rivals();
                let target = sector + 3 + pseudo_random(sector, seed.wrapping_add(1), 5); // +3..+7
                let target_faction = if rivals.is_empty() {
                    // If no rivals (e.g. AlienCollective), pick a random faction
                    let idx = pseudo_random(sector, seed.wrapping_add(5), Faction::TRACKABLE.len() as u32) as usize;
                    &Faction::TRACKABLE[idx]
                } else {
                    let idx = pseudo_random(sector, seed.wrapping_add(5), rivals.len() as u32) as usize;
                    &rivals[idx]
                };
                let (t, d) = generate_sabotage_title(target_faction);
                (target, 0, t, d)
            }
            MissionType::Rescue => {
                let target = sector + 4 + pseudo_random(sector, seed.wrapping_add(1), 6); // +4..+9
                let (t, d) = generate_rescue_title(sector, target, seed);
                (target, 0, t, d)
            }
            MissionType::TradeRun => {
                let target = sector + 3 + pseudo_random(sector, seed.wrapping_add(1), 5); // +3..+7
                let (t, d) = generate_traderun_title(target);
                (target, 0, t, d)
            }
        };

        let reward_credits = match mission_type {
            MissionType::Exploration => base_reward * 2,     // big reward for long trips
            MissionType::Sabotage => base_reward * 3 / 2,    // risky pays more
            MissionType::Escort => base_reward * 3 / 2,      // convoy protection premium
            MissionType::BountyHunt => base_reward * 5 / 4,  // bounties pay above base
            _ => base_reward,
        };

        let reward_rep = match mission_type {
            MissionType::Sabotage => 15 + difficulty as i32 * 3, // big rep with issuer
            MissionType::Exploration => 10 + difficulty as i32 * 2,
            _ => 5 + difficulty as i32 * 2,
        };

        // Equipment chance scales with difficulty
        let reward_equipment = difficulty >= 3 && pseudo_random(sector, seed.wrapping_add(11), 100) < 40;

        let id = *next_id;
        *next_id += 1;

        missions.push(Mission {
            id,
            title,
            description,
            mission_type,
            faction: *faction,
            reward_credits,
            reward_rep,
            reward_equipment,
            target_sector,
            sectors_remaining,
            status: MissionStatus::Available,
            difficulty,
        });
    }

    missions
}

// ── Mission Progress Tracking ───────────────────────────────────────────────

/// Maximum sectors past target before a mission expires.
const EXPIRY_BUFFER: u32 = 20;

/// Check progress of active missions after a sector transition.
/// Returns updates for any missions that changed state.
pub fn check_mission_progress(
    missions: &mut [Mission],
    current_sector: u32,
    _battle_won_in_sector: bool,
    boss_killed_in_sector: bool,
    raid_completed_in_sector: bool,
    fleet_ship_lost: bool,
) -> Vec<MissionUpdate> {
    let mut updates = Vec::new();

    for mission in missions.iter_mut() {
        if mission.status != MissionStatus::Active {
            continue;
        }

        match mission.mission_type {
            MissionType::BountyHunt => {
                // Complete if boss killed in target sector
                if current_sector == mission.target_sector && boss_killed_in_sector {
                    mission.status = MissionStatus::Completed;
                    updates.push(MissionUpdate {
                        mission_id: mission.id,
                        title: mission.title.clone(),
                        update_type: MissionUpdateType::Completed {
                            reward_credits: mission.reward_credits,
                            reward_rep: mission.reward_rep,
                            reward_equipment: mission.reward_equipment,
                        },
                    });
                } else if current_sector > mission.target_sector + EXPIRY_BUFFER {
                    mission.status = MissionStatus::Expired;
                    updates.push(MissionUpdate {
                        mission_id: mission.id,
                        title: mission.title.clone(),
                        update_type: MissionUpdateType::Failed {
                            reason: "Target moved on — bounty expired.".to_string(),
                        },
                    });
                }
            }

            MissionType::Delivery | MissionType::TradeRun => {
                if mission.sectors_remaining > 0 {
                    mission.sectors_remaining -= 1;
                    if mission.sectors_remaining == 0 {
                        mission.status = MissionStatus::Completed;
                        updates.push(MissionUpdate {
                            mission_id: mission.id,
                            title: mission.title.clone(),
                            update_type: MissionUpdateType::Completed {
                                reward_credits: mission.reward_credits,
                                reward_rep: mission.reward_rep,
                                reward_equipment: mission.reward_equipment,
                            },
                        });
                    } else {
                        updates.push(MissionUpdate {
                            mission_id: mission.id,
                            title: mission.title.clone(),
                            update_type: MissionUpdateType::Progress {
                                sectors_remaining: mission.sectors_remaining,
                            },
                        });
                    }
                }
                // Expire if too far past target
                if mission.status == MissionStatus::Active
                    && current_sector > mission.target_sector + EXPIRY_BUFFER
                {
                    mission.status = MissionStatus::Expired;
                    updates.push(MissionUpdate {
                        mission_id: mission.id,
                        title: mission.title.clone(),
                        update_type: MissionUpdateType::Failed {
                            reason: "Delivery window expired.".to_string(),
                        },
                    });
                }
            }

            MissionType::Escort => {
                // Fail if fleet ship lost during escort
                if fleet_ship_lost {
                    mission.status = MissionStatus::Failed;
                    updates.push(MissionUpdate {
                        mission_id: mission.id,
                        title: mission.title.clone(),
                        update_type: MissionUpdateType::Failed {
                            reason: "Convoy ship destroyed — escort failed.".to_string(),
                        },
                    });
                    continue;
                }

                if mission.sectors_remaining > 0 {
                    mission.sectors_remaining -= 1;
                    if mission.sectors_remaining == 0 {
                        mission.status = MissionStatus::Completed;
                        updates.push(MissionUpdate {
                            mission_id: mission.id,
                            title: mission.title.clone(),
                            update_type: MissionUpdateType::Completed {
                                reward_credits: mission.reward_credits,
                                reward_rep: mission.reward_rep,
                                reward_equipment: mission.reward_equipment,
                            },
                        });
                    } else {
                        updates.push(MissionUpdate {
                            mission_id: mission.id,
                            title: mission.title.clone(),
                            update_type: MissionUpdateType::Progress {
                                sectors_remaining: mission.sectors_remaining,
                            },
                        });
                    }
                }
            }

            MissionType::Exploration => {
                if current_sector >= mission.target_sector {
                    mission.status = MissionStatus::Completed;
                    updates.push(MissionUpdate {
                        mission_id: mission.id,
                        title: mission.title.clone(),
                        update_type: MissionUpdateType::Completed {
                            reward_credits: mission.reward_credits,
                            reward_rep: mission.reward_rep,
                            reward_equipment: mission.reward_equipment,
                        },
                    });
                }
            }

            MissionType::Sabotage => {
                if current_sector == mission.target_sector && raid_completed_in_sector {
                    mission.status = MissionStatus::Completed;
                    updates.push(MissionUpdate {
                        mission_id: mission.id,
                        title: mission.title.clone(),
                        update_type: MissionUpdateType::Completed {
                            reward_credits: mission.reward_credits,
                            reward_rep: mission.reward_rep,
                            reward_equipment: mission.reward_equipment,
                        },
                    });
                } else if current_sector > mission.target_sector + EXPIRY_BUFFER {
                    mission.status = MissionStatus::Expired;
                    updates.push(MissionUpdate {
                        mission_id: mission.id,
                        title: mission.title.clone(),
                        update_type: MissionUpdateType::Failed {
                            reason: "Outpost security tightened — mission expired.".to_string(),
                        },
                    });
                }
            }

            MissionType::Rescue => {
                if current_sector >= mission.target_sector {
                    mission.status = MissionStatus::Completed;
                    updates.push(MissionUpdate {
                        mission_id: mission.id,
                        title: mission.title.clone(),
                        update_type: MissionUpdateType::Completed {
                            reward_credits: mission.reward_credits,
                            reward_rep: mission.reward_rep,
                            reward_equipment: mission.reward_equipment,
                        },
                    });
                }
            }
        }
    }

    updates
}

/// Mark all missions that have exceeded the expiry buffer as expired.
pub fn fail_expired_missions(missions: &mut [Mission], current_sector: u32) -> Vec<MissionUpdate> {
    let mut updates = Vec::new();

    for mission in missions.iter_mut() {
        if mission.status != MissionStatus::Active {
            continue;
        }

        if current_sector > mission.target_sector + EXPIRY_BUFFER {
            mission.status = MissionStatus::Expired;
            updates.push(MissionUpdate {
                mission_id: mission.id,
                title: mission.title.clone(),
                update_type: MissionUpdateType::Failed {
                    reason: "Mission expired — too far past target.".to_string(),
                },
            });
        }
    }

    updates
}

/// Accept a mission: move it from available to active.
/// Returns `true` if the mission was found and accepted.
pub fn accept_mission(
    available: &mut Vec<Mission>,
    active: &mut Vec<Mission>,
    mission_id: u64,
) -> bool {
    if let Some(pos) = available.iter().position(|m| m.id == mission_id && m.status == MissionStatus::Available) {
        let mut mission = available.remove(pos);
        mission.status = MissionStatus::Active;
        active.push(mission);
        true
    } else {
        false
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Generation ─────────────────────────────────────────────────

    #[test]
    fn generate_missions_returns_correct_count() {
        let mut next_id = 1;
        let missions = generate_missions(10, &Faction::TradeGuild, 5, &mut next_id);
        assert_eq!(missions.len(), 5);
        assert_eq!(next_id, 6); // IDs 1-5 used
    }

    #[test]
    fn generated_missions_have_unique_ids() {
        let mut next_id = 1;
        let missions = generate_missions(10, &Faction::PirateClan, 10, &mut next_id);
        let ids: Vec<u64> = missions.iter().map(|m| m.id).collect();
        let unique: std::collections::HashSet<u64> = ids.iter().copied().collect();
        assert_eq!(ids.len(), unique.len());
    }

    #[test]
    fn generated_missions_all_available() {
        let mut next_id = 1;
        let missions = generate_missions(5, &Faction::MilitaryCorp, 3, &mut next_id);
        for m in &missions {
            assert_eq!(m.status, MissionStatus::Available);
        }
    }

    #[test]
    fn generated_missions_have_valid_difficulty() {
        let mut next_id = 1;
        let missions = generate_missions(20, &Faction::RebelAlliance, 10, &mut next_id);
        for m in &missions {
            assert!(m.difficulty >= 1 && m.difficulty <= 5, "difficulty {} out of range", m.difficulty);
        }
    }

    #[test]
    fn generated_missions_target_sector_ahead() {
        let mut next_id = 1;
        let sector = 10;
        let missions = generate_missions(sector, &Faction::TradeGuild, 10, &mut next_id);
        for m in &missions {
            assert!(
                m.target_sector > sector,
                "mission '{}' target {} should be > sector {}",
                m.title, m.target_sector, sector
            );
        }
    }

    #[test]
    fn generated_missions_have_nonempty_titles() {
        let mut next_id = 1;
        let missions = generate_missions(1, &Faction::AlienCollective, 7, &mut next_id);
        for m in &missions {
            assert!(!m.title.is_empty(), "mission ID {} has empty title", m.id);
            assert!(!m.description.is_empty(), "mission ID {} has empty description", m.id);
        }
    }

    #[test]
    fn generated_missions_reward_credits_positive() {
        let mut next_id = 1;
        let missions = generate_missions(15, &Faction::PirateClan, 10, &mut next_id);
        for m in &missions {
            assert!(m.reward_credits > 0, "mission '{}' has 0 credits", m.title);
            assert!(m.reward_rep > 0, "mission '{}' has 0 rep", m.title);
        }
    }

    #[test]
    fn generate_missions_deterministic_for_same_inputs() {
        let mut id1 = 1;
        let mut id2 = 1;
        let m1 = generate_missions(10, &Faction::TradeGuild, 5, &mut id1);
        let m2 = generate_missions(10, &Faction::TradeGuild, 5, &mut id2);
        for (a, b) in m1.iter().zip(m2.iter()) {
            assert_eq!(a.title, b.title);
            assert_eq!(a.mission_type, b.mission_type);
            assert_eq!(a.reward_credits, b.reward_credits);
        }
    }

    // ── Accept ─────────────────────────────────────────────────────

    #[test]
    fn accept_mission_moves_to_active() {
        let mut next_id = 1;
        let mut available = generate_missions(10, &Faction::TradeGuild, 3, &mut next_id);
        let mut active = Vec::new();
        let id = available[1].id;

        assert!(accept_mission(&mut available, &mut active, id));
        assert_eq!(available.len(), 2);
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, id);
        assert_eq!(active[0].status, MissionStatus::Active);
    }

    #[test]
    fn accept_nonexistent_mission_returns_false() {
        let mut available = Vec::new();
        let mut active = Vec::new();
        assert!(!accept_mission(&mut available, &mut active, 999));
    }

    #[test]
    fn accept_already_active_mission_returns_false() {
        let mut next_id = 1;
        let mut available = generate_missions(10, &Faction::TradeGuild, 3, &mut next_id);
        let mut active = Vec::new();
        let id = available[0].id;

        assert!(accept_mission(&mut available, &mut active, id));
        // Try to accept again — it's no longer in available
        assert!(!accept_mission(&mut available, &mut active, id));
    }

    // ── Delivery progress ──────────────────────────────────────────

    #[test]
    fn delivery_mission_counts_down() {
        let mut missions = vec![Mission {
            id: 1,
            title: "Supply Run to Sector 15".into(),
            description: "Deliver supplies.".into(),
            mission_type: MissionType::Delivery,
            faction: Faction::TradeGuild,
            reward_credits: 200,
            reward_rep: 10,
            reward_equipment: false,
            target_sector: 15,
            sectors_remaining: 3,
            status: MissionStatus::Active,
            difficulty: 2,
        }];

        // Tick 1: 3→2
        let updates = check_mission_progress(&mut missions, 11, false, false, false, false);
        assert_eq!(updates.len(), 1);
        assert!(matches!(updates[0].update_type, MissionUpdateType::Progress { sectors_remaining: 2 }));

        // Tick 2: 2→1
        let updates = check_mission_progress(&mut missions, 12, false, false, false, false);
        assert!(matches!(updates[0].update_type, MissionUpdateType::Progress { sectors_remaining: 1 }));

        // Tick 3: 1→0 = complete
        let updates = check_mission_progress(&mut missions, 13, false, false, false, false);
        assert!(matches!(updates[0].update_type, MissionUpdateType::Completed { .. }));
        assert_eq!(missions[0].status, MissionStatus::Completed);
    }

    // ── Escort failure ─────────────────────────────────────────────

    #[test]
    fn escort_mission_fails_on_ship_loss() {
        let mut missions = vec![Mission {
            id: 2,
            title: "Protect Trade Guild Convoy".into(),
            description: "Escort convoy.".into(),
            mission_type: MissionType::Escort,
            faction: Faction::TradeGuild,
            reward_credits: 300,
            reward_rep: 15,
            reward_equipment: false,
            target_sector: 20,
            sectors_remaining: 5,
            status: MissionStatus::Active,
            difficulty: 3,
        }];

        let updates = check_mission_progress(&mut missions, 16, false, false, false, true);
        assert_eq!(updates.len(), 1);
        assert!(matches!(updates[0].update_type, MissionUpdateType::Failed { .. }));
        assert_eq!(missions[0].status, MissionStatus::Failed);
    }

    #[test]
    fn escort_mission_completes_with_no_losses() {
        let mut missions = vec![Mission {
            id: 3,
            title: "Protect Military Corp Convoy".into(),
            description: "Escort convoy.".into(),
            mission_type: MissionType::Escort,
            faction: Faction::MilitaryCorp,
            reward_credits: 350,
            reward_rep: 15,
            reward_equipment: true,
            target_sector: 13,
            sectors_remaining: 1,
            status: MissionStatus::Active,
            difficulty: 4,
        }];

        let updates = check_mission_progress(&mut missions, 13, false, false, false, false);
        assert!(matches!(updates[0].update_type, MissionUpdateType::Completed { .. }));
    }

    // ── Exploration ────────────────────────────────────────────────

    #[test]
    fn exploration_completes_on_reaching_target() {
        let mut missions = vec![Mission {
            id: 4,
            title: "Chart the Crimson Nebula".into(),
            description: "Explore.".into(),
            mission_type: MissionType::Exploration,
            faction: Faction::RebelAlliance,
            reward_credits: 500,
            reward_rep: 20,
            reward_equipment: true,
            target_sector: 25,
            sectors_remaining: 0,
            status: MissionStatus::Active,
            difficulty: 4,
        }];

        // Not there yet
        let updates = check_mission_progress(&mut missions, 20, false, false, false, false);
        assert!(updates.is_empty());

        // Reached target
        let updates = check_mission_progress(&mut missions, 25, false, false, false, false);
        assert!(matches!(updates[0].update_type, MissionUpdateType::Completed { .. }));
    }

    #[test]
    fn exploration_completes_past_target() {
        let mut missions = vec![Mission {
            id: 5,
            title: "Chart the Obsidian Nebula".into(),
            description: "Explore.".into(),
            mission_type: MissionType::Exploration,
            faction: Faction::AlienCollective,
            reward_credits: 600,
            reward_rep: 25,
            reward_equipment: false,
            target_sector: 30,
            sectors_remaining: 0,
            status: MissionStatus::Active,
            difficulty: 5,
        }];

        let updates = check_mission_progress(&mut missions, 35, false, false, false, false);
        assert!(matches!(updates[0].update_type, MissionUpdateType::Completed { .. }));
    }

    // ── Bounty Hunt ────────────────────────────────────────────────

    #[test]
    fn bounty_hunt_requires_boss_kill_in_target_sector() {
        let mut missions = vec![Mission {
            id: 6,
            title: "Eliminate Vex the Pirate Lord".into(),
            description: "Hunt bounty.".into(),
            mission_type: MissionType::BountyHunt,
            faction: Faction::MilitaryCorp,
            reward_credits: 400,
            reward_rep: 15,
            reward_equipment: false,
            target_sector: 12,
            sectors_remaining: 0,
            status: MissionStatus::Active,
            difficulty: 3,
        }];

        // Battle won but no boss kill in target
        let updates = check_mission_progress(&mut missions, 12, true, false, false, false);
        assert!(updates.is_empty());

        // Boss killed but wrong sector
        let updates = check_mission_progress(&mut missions, 11, false, true, false, false);
        assert!(updates.is_empty());

        // Boss killed in correct sector
        let updates = check_mission_progress(&mut missions, 12, false, true, false, false);
        assert!(matches!(updates[0].update_type, MissionUpdateType::Completed { .. }));
    }

    // ── Sabotage ───────────────────────────────────────────────────

    #[test]
    fn sabotage_requires_raid_in_target_sector() {
        let mut missions = vec![Mission {
            id: 7,
            title: "Strike Pirate Clans Outpost".into(),
            description: "Raid outpost.".into(),
            mission_type: MissionType::Sabotage,
            faction: Faction::TradeGuild,
            reward_credits: 450,
            reward_rep: 20,
            reward_equipment: false,
            target_sector: 18,
            sectors_remaining: 0,
            status: MissionStatus::Active,
            difficulty: 4,
        }];

        // Raid in wrong sector
        let updates = check_mission_progress(&mut missions, 17, false, false, true, false);
        assert!(updates.is_empty());

        // Raid in correct sector
        let updates = check_mission_progress(&mut missions, 18, false, false, true, false);
        assert!(matches!(updates[0].update_type, MissionUpdateType::Completed { .. }));
    }

    // ── Rescue ─────────────────────────────────────────────────────

    #[test]
    fn rescue_completes_on_reaching_sector() {
        let mut missions = vec![Mission {
            id: 8,
            title: "Rescue Dr. Yael from Sector 20".into(),
            description: "Rescue stranded crew.".into(),
            mission_type: MissionType::Rescue,
            faction: Faction::RebelAlliance,
            reward_credits: 250,
            reward_rep: 15,
            reward_equipment: false,
            target_sector: 20,
            sectors_remaining: 0,
            status: MissionStatus::Active,
            difficulty: 3,
        }];

        let updates = check_mission_progress(&mut missions, 20, false, false, false, false);
        assert!(matches!(updates[0].update_type, MissionUpdateType::Completed { .. }));
    }

    // ── Expiration ─────────────────────────────────────────────────

    #[test]
    fn bounty_expires_after_buffer() {
        let mut missions = vec![Mission {
            id: 9,
            title: "Eliminate Karn the Scourge".into(),
            description: "Hunt.".into(),
            mission_type: MissionType::BountyHunt,
            faction: Faction::PirateClan,
            reward_credits: 300,
            reward_rep: 10,
            reward_equipment: false,
            target_sector: 10,
            sectors_remaining: 0,
            status: MissionStatus::Active,
            difficulty: 2,
        }];

        // Still within buffer
        let updates = check_mission_progress(&mut missions, 30, false, false, false, false);
        assert!(updates.is_empty());

        // Past buffer (10 + 20 = 30, so 31 triggers expiry)
        let updates = check_mission_progress(&mut missions, 31, false, false, false, false);
        assert!(matches!(updates[0].update_type, MissionUpdateType::Failed { .. }));
        assert_eq!(missions[0].status, MissionStatus::Expired);
    }

    #[test]
    fn fail_expired_missions_catches_all() {
        let mut missions = vec![
            Mission {
                id: 10, title: "A".into(), description: "".into(),
                mission_type: MissionType::BountyHunt, faction: Faction::PirateClan,
                reward_credits: 100, reward_rep: 5, reward_equipment: false,
                target_sector: 5, sectors_remaining: 0,
                status: MissionStatus::Active, difficulty: 1,
            },
            Mission {
                id: 11, title: "B".into(), description: "".into(),
                mission_type: MissionType::Exploration, faction: Faction::TradeGuild,
                reward_credits: 200, reward_rep: 10, reward_equipment: false,
                target_sector: 50, sectors_remaining: 0,
                status: MissionStatus::Active, difficulty: 3,
            },
        ];

        let updates = fail_expired_missions(&mut missions, 30);
        // Mission A (target 5) should expire at sector 26+, so at 30 it's expired
        // Mission B (target 50) should NOT expire at sector 30
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].mission_id, 10);
    }

    // ── Completed/failed missions are not re-processed ─────────────

    #[test]
    fn completed_missions_not_re_processed() {
        let mut missions = vec![Mission {
            id: 12, title: "Done".into(), description: "".into(),
            mission_type: MissionType::Exploration, faction: Faction::TradeGuild,
            reward_credits: 100, reward_rep: 5, reward_equipment: false,
            target_sector: 10, sectors_remaining: 0,
            status: MissionStatus::Completed, difficulty: 1,
        }];

        let updates = check_mission_progress(&mut missions, 10, false, false, false, false);
        assert!(updates.is_empty());
    }

    #[test]
    fn failed_missions_not_re_processed() {
        let mut missions = vec![Mission {
            id: 13, title: "Failed".into(), description: "".into(),
            mission_type: MissionType::Escort, faction: Faction::MilitaryCorp,
            reward_credits: 100, reward_rep: 5, reward_equipment: false,
            target_sector: 15, sectors_remaining: 3,
            status: MissionStatus::Failed, difficulty: 2,
        }];

        let updates = check_mission_progress(&mut missions, 13, false, false, false, true);
        assert!(updates.is_empty());
    }

    // ── Pseudo-random ──────────────────────────────────────────────

    #[test]
    fn pseudo_random_deterministic() {
        assert_eq!(pseudo_random(10, 42, 100), pseudo_random(10, 42, 100));
    }

    #[test]
    fn pseudo_random_zero_modulus() {
        assert_eq!(pseudo_random(5, 5, 0), 0);
    }

    #[test]
    fn pseudo_random_within_range() {
        for a in 0..50 {
            for b in 0..10 {
                let result = pseudo_random(a, b, 7);
                assert!(result < 7);
            }
        }
    }
}
