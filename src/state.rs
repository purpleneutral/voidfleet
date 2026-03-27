use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::engine::equipment::{Equipment, SetBonus, SET_BONUSES};
use crate::engine::ship::{Ship, ShipType};

fn default_route_modifier() -> f32 {
    1.0
}

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

    // Prestige
    #[serde(default)]
    pub prestige_level: u32,
    #[serde(default)]
    pub prestige_bonus_xp: f32,
    #[serde(default)]
    pub prestige_bonus_credits: f32,
    #[serde(default)]
    pub prestige_bonus_scrap: f32,
    #[serde(default)]
    pub lifetime_sectors: u64,
    #[serde(default)]
    pub lifetime_credits: u64,

    // Sector map route
    #[serde(default = "default_route_modifier")]
    pub current_route_modifier: f32,

    // Timing
    pub phase_timer: f32, // seconds remaining in current phase

    // Pip companion state
    #[serde(default = "default_pip_hunger")]
    pub pip_hunger: u8,
    #[serde(default = "default_pip_energy")]
    pub pip_energy: u8,
    #[serde(default = "default_pip_happiness")]
    pub pip_happiness: u8,
    #[serde(default)]
    pub pip_bond: u16,
    #[serde(default = "default_pip_level")]
    pub pip_level: u8,
    #[serde(default)]
    pub pip_xp: u64,
    #[serde(default)]
    pub pip_appearance: u8,

    // Inventory
    #[serde(default)]
    pub inventory: Vec<Equipment>,
    #[serde(default = "default_inventory_capacity")]
    pub inventory_capacity: usize,
    #[serde(default = "default_next_item_id")]
    pub next_item_id: u64,

    // Transient (not saved)
    #[serde(skip)]
    pub pending_popups: Vec<String>,
    #[serde(skip)]
    pub pending_loot: Vec<Equipment>,
}

fn default_inventory_capacity() -> usize { 20 }
fn default_next_item_id() -> u64 { 1 }

fn default_pip_hunger() -> u8 { 80 }
fn default_pip_energy() -> u8 { 80 }
fn default_pip_happiness() -> u8 { 70 }
fn default_pip_level() -> u8 { 1 }

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
            prestige_level: 0,
            prestige_bonus_xp: 0.0,
            prestige_bonus_credits: 0.0,
            prestige_bonus_scrap: 0.0,
            lifetime_sectors: 0,
            lifetime_credits: 0,
            current_route_modifier: 1.0,
            total_battles: 0,
            total_raids: 0,
            total_scrap: 0,
            enemies_destroyed: 0,
            deaths: 0,
            highest_sector: 1,
            time_played_secs: 0,
            achievements_unlocked: Vec::new(),
            phase_timer: 45.0,
            pip_hunger: 80,
            pip_energy: 80,
            pip_happiness: 70,
            pip_bond: 0,
            pip_level: 1,
            pip_xp: 0,
            pip_appearance: 0,
            inventory: Vec::new(),
            inventory_capacity: 20,
            next_item_id: 1,
            pending_popups: Vec::new(),
            pending_loot: Vec::new(),
        }
    }

    // ── Inventory methods ──────────────────────────────────────────

    /// Try to add equipment to inventory. Returns `Ok(())` on success,
    /// or `Err(salvage_value)` if inventory is full (item auto-salvaged).
    pub fn try_add_to_inventory(&mut self, mut item: Equipment) -> Result<(), u64> {
        if self.inventory.len() >= self.inventory_capacity {
            // Inventory full — auto-salvage
            let value = item.salvage_value();
            self.scrap += value;
            Err(value)
        } else {
            item.id = self.next_item_id;
            self.next_item_id += 1;
            self.inventory.push(item);
            Ok(())
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
                // Reject suspiciously large save files (> 1MB)
                if data.len() > 1_000_000 {
                    return Self::new();
                }
                if let Ok(state) = serde_json::from_str::<Self>(&data) {
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

    // ── Pip companion methods ──────────────────────────────────────────

    pub fn pip_xp_to_next(&self) -> u64 {
        50 * (self.pip_level as u64 + 1).pow(2)
    }

    pub fn add_pip_xp(&mut self, amount: u64) {
        self.pip_xp += amount;
        while self.pip_level < 10 && self.pip_xp >= self.pip_xp_to_next() {
            self.pip_xp -= self.pip_xp_to_next();
            self.pip_level = (self.pip_level + 1).min(10);
        }
    }

    /// Returns damage/loot multiplier from Pip happiness + level + appearance
    pub fn pip_combat_bonus(&self) -> f32 {
        let happiness_bonus = if self.pip_happiness > 80 { 0.05 } else { 0.0 };
        let level_bonus = self.pip_level as f32 * 0.01;
        let appearance_bonus = match self.pip_appearance {
            2 => 0.05,  // visor: +5% loot
            4 => 0.15,  // crown: +15%
            _ => 0.0,
        };
        1.0 + happiness_bonus + level_bonus + appearance_bonus
    }

    /// Returns travel speed multiplier (lower = faster)
    pub fn pip_travel_bonus(&self) -> f32 {
        match self.pip_appearance {
            3 => 0.90,  // wings: 10% faster
            4 => 0.85,  // crown: 15% faster
            _ => 1.0,
        }
    }

    /// Remove an item from inventory by its unique ID.
    pub fn remove_from_inventory(&mut self, item_id: u64) -> Option<Equipment> {
        if let Some(pos) = self.inventory.iter().position(|i| i.id == item_id) {
            Some(self.inventory.remove(pos))
        } else {
            None
        }
    }

    /// Salvage an item from inventory, returning scrap value.
    /// Value: Common=5, Uncommon=15, Rare=40, Epic=100, Legendary=250 * (1 + level/10)
    pub fn salvage_item(&mut self, item_id: u64) -> u64 {
        if let Some(item) = self.remove_from_inventory(item_id) {
            let value = item.salvage_value();
            self.scrap += value;
            self.total_scrap += value;
            value
        } else {
            0
        }
    }

    /// Check all ships' equipped items for matching set IDs and return active set bonuses.
    pub fn active_set_bonuses(&self) -> Vec<&'static SetBonus> {
        // Count set pieces across all ships
        let mut set_counts: std::collections::HashMap<&str, u8> = std::collections::HashMap::new();
        for ship in &self.fleet {
            for item in ship.equipped_items() {
                if let Some(ref set_id) = item.set_id {
                    *set_counts.entry(set_id.as_str()).or_insert(0) += 1;
                }
            }
        }

        SET_BONUSES.iter()
            .filter(|bonus| {
                set_counts
                    .get(bonus.set_id)
                    .copied()
                    .unwrap_or(0)
                    >= bonus.pieces_required
            })
            .collect()
    }

    /// Reset progression and gain permanent prestige bonuses.
    /// Requires sector 30+ to activate.
    pub fn prestige(&mut self) -> bool {
        if self.sector < 30 {
            return false;
        }

        self.prestige_level += 1;
        self.prestige_bonus_xp = self.prestige_level as f32 * 0.10;
        self.prestige_bonus_credits = self.prestige_level as f32 * 0.05;
        self.prestige_bonus_scrap = self.prestige_level as f32 * 0.05;

        // Track lifetime stats
        self.lifetime_sectors += self.sector as u64;
        self.lifetime_credits += self.credits;

        // Reset progression but keep prestige bonuses
        self.scrap = 0;
        self.credits = 0;
        self.blueprints = 0;
        self.sector = 1;
        self.level = 1;
        self.xp = 0;
        self.xp_to_next = 100;
        self.fleet = vec![Ship::new(ShipType::Scout)];
        self.tech_lasers = 1;
        self.tech_shields = 0;
        self.tech_engines = 1;
        self.tech_beams = 0;
        self.current_route_modifier = 1.0;
        self.phase = GamePhase::Travel;
        self.phase_timer = 45.0;
        self.inventory.clear();
        // Keep: achievements, deaths, highest_sector, prestige_level, totals, time_played, inventory_capacity
        true
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
