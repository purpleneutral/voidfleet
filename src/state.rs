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
    pub deaths: u64,
    pub highest_sector: u32,
    pub time_played_secs: u64,
    pub achievements_unlocked: Vec<String>,

    // Timing
    pub phase_timer: f32, // seconds remaining in current phase

    // Transient (not saved)
    #[serde(skip)]
    pub pending_popups: Vec<String>,
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
            deaths: 0,
            highest_sector: 1,
            time_played_secs: 0,
            achievements_unlocked: Vec::new(),
            phase_timer: 45.0,
            pending_popups: Vec::new(),
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

    /// Called when fleet is destroyed in battle. Returns penalty description.
    pub fn handle_fleet_death(&mut self) -> String {
        self.deaths += 1;

        // Lose 30% of scrap
        let scrap_lost = self.scrap * 30 / 100;
        self.scrap -= scrap_lost;

        // Lose 20% of credits
        let credits_lost = self.credits * 20 / 100;
        self.credits -= credits_lost;

        // Set sector back by 3 (minimum 1)
        let sectors_lost = 3.min(self.sector - 1);
        self.sector = self.sector.saturating_sub(sectors_lost).max(1);

        // Respawn fleet at full HP (don't lose ships — that's too punishing)
        for ship in &mut self.fleet {
            ship.heal_full();
        }

        format!(
            "Fleet destroyed! Lost {} scrap, {} credits. Pushed back {} sectors.",
            scrap_lost, credits_lost, sectors_lost
        )
    }

    /// Update highest_sector tracker when entering a new sector.
    pub fn update_highest_sector(&mut self) {
        if self.sector > self.highest_sector {
            self.highest_sector = self.sector;
        }
    }

    /// Check for new achievements and queue popup messages.
    pub fn check_achievements(&mut self) {
        let newly_unlocked = crate::engine::achievements::check_achievements(self);
        for achievement in newly_unlocked {
            self.achievements_unlocked.push(achievement.id.to_string());
            self.pending_popups.push(format!(
                "{} Achievement Unlocked: {} — {}",
                achievement.icon, achievement.name, achievement.description
            ));
        }
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
