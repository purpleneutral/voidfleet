use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::engine::ship::{Ship, ShipType};

/// Top-level game state — everything that persists between sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    // Resources
    pub scrap: u64,
    pub credits: u64,
    pub blueprints: u64,
    pub artifacts: u64,

    // Progression
    pub sector: u32,
    pub level: u32,
    pub xp: u64,
    pub xp_to_next: u64,

    // Fleet
    pub fleet: Vec<Ship>,

    // Tech levels
    pub tech_lasers: u8,
    pub tech_shields: u8,
    pub tech_engines: u8,
    pub tech_beams: u8,

    // Current phase
    pub phase: GamePhase,

    // Stats
    pub total_battles: u64,
    pub total_raids: u64,
    pub total_scrap: u64,
    pub enemies_destroyed: u64,

    // Timing
    pub phase_timer: f32, // seconds remaining in current phase
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GamePhase {
    Travel,
    Battle,
    Raid,
    Loot,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            scrap: 0,
            credits: 0,
            blueprints: 0,
            artifacts: 0,
            sector: 1,
            level: 1,
            xp: 0,
            xp_to_next: 100,
            fleet: vec![Ship::new(ShipType::Scout)],
            tech_lasers: 1,
            tech_shields: 0,
            tech_engines: 1,
            tech_beams: 0,
            phase: GamePhase::Travel,
            total_battles: 0,
            total_raids: 0,
            total_scrap: 0,
            enemies_destroyed: 0,
            phase_timer: 45.0,
        }
    }

    fn save_path() -> PathBuf {
        let home = dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let dir = home.join(".voidfleet");
        fs::create_dir_all(&dir).ok();
        dir.join("save.json")
    }

    pub fn save(&self) {
        let path = Self::save_path();
        if let Ok(json) = serde_json::to_string_pretty(self) {
            fs::write(path, json).ok();
        }
    }

    pub fn load() -> Self {
        let path = Self::save_path();
        if path.exists() {
            if let Ok(data) = fs::read_to_string(&path) {
                if let Ok(state) = serde_json::from_str(&data) {
                    return state;
                }
            }
        }
        Self::new()
    }

    pub fn fleet_total_hp(&self) -> u32 {
        self.fleet.iter().map(|s| s.current_hp).sum()
    }

    pub fn fleet_max_hp(&self) -> u32 {
        self.fleet.iter().map(|s| s.max_hp()).sum()
    }

    pub fn fleet_total_dps(&self) -> f32 {
        self.fleet.iter().map(|s| s.dps()).sum()
    }

    pub fn add_xp(&mut self, amount: u64) {
        self.xp += amount;
        while self.xp >= self.xp_to_next {
            self.xp -= self.xp_to_next;
            self.level += 1;
            self.xp_to_next = (self.xp_to_next as f64 * 1.3) as u64;
        }
    }
}
