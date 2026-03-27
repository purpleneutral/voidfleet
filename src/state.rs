use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use std::collections::HashMap;

use crate::engine::crew::{CrewBond, CrewMember};
use crate::engine::equipment::{Equipment, SetBonus, SET_BONUSES};
use crate::engine::factions::{FactionMission, FactionReputation};
use crate::engine::missions::Mission;
use crate::engine::ship::{Ship, ShipType};
use crate::engine::trade::{TradeGood, TradeRecord};
use crate::engine::voyage::{VoyageBonuses, VOYAGE_DAMAGE_BONUS, VOYAGE_HP_BONUS, VOYAGE_SPEED_BONUS, VOYAGE_CRIT_BONUS, VOYAGE_STARTING_CREDITS};

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

    // Prestige (legacy — kept for save compat, no longer used)
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

    // Voyage system
    #[serde(default = "default_voyage")]
    pub voyage: u32,
    #[serde(default)]
    pub voyage_permanent_dmg: f32,
    #[serde(default)]
    pub voyage_permanent_hp: f32,
    #[serde(default)]
    pub voyage_permanent_speed: f32,
    #[serde(default)]
    pub voyage_permanent_crit: f32,
    #[serde(default)]
    pub voyages_completed: u32,
    #[serde(default)]
    pub highest_sector_ever: u32,
    #[serde(default)]
    pub voyage_bonuses: VoyageBonuses,
    #[serde(default)]
    pub voyage_ships_built: u64,
    #[serde(default)]
    pub voyage_equipment_found: u64,
    #[serde(default)]
    pub voyage_crew_recruited: u64,
    #[serde(default)]
    pub voyage_credits_earned: u64,

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

    // Crew
    #[serde(default)]
    pub crew_roster: Vec<CrewMember>,
    #[serde(default = "default_crew_capacity")]
    pub crew_capacity: usize,
    #[serde(default)]
    pub next_crew_id: u64,
    #[serde(default)]
    pub crew_bonds: Vec<CrewBond>,

    // Factions
    #[serde(default)]
    pub faction_reputation: FactionReputation,
    #[serde(default)]
    pub pending_faction_mission: Option<FactionMission>,

    // Trade / Cargo
    #[serde(default)]
    pub cargo: HashMap<String, u32>,
    #[serde(default = "default_cargo_capacity")]
    pub cargo_capacity: u32,
    #[serde(default)]
    pub trade_history: Vec<TradeRecord>,

    // Missions
    #[serde(default)]
    pub active_missions: Vec<Mission>,
    #[serde(default)]
    pub completed_missions: u64,
    #[serde(default)]
    pub failed_missions: u64,
    #[serde(default)]
    pub available_missions: Vec<Mission>,
    #[serde(default = "default_next_mission_id")]
    pub next_mission_id: u64,

    // Transient (not saved)
    // Legacy field — kept for save compatibility, no longer actively used.
    #[serde(default)]
    #[allow(dead_code)]
    pub pending_popups: Vec<String>,
    #[serde(skip)]
    pub pending_loot: Vec<Equipment>,
    /// Set to true when a voyage boss is defeated (before complete_voyage is called).
    /// Main loop uses this to trigger the voyage cinematic screen.
    #[serde(skip)]
    pub voyage_boss_defeated: bool,
}

fn default_voyage() -> u32 { 1 }
fn default_cargo_capacity() -> u32 { 20 }
fn default_next_mission_id() -> u64 { 1 }
fn default_inventory_capacity() -> usize { 20 }
fn default_next_item_id() -> u64 { 1 }
fn default_crew_capacity() -> usize { 5 }

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
            voyage: 1,
            voyage_permanent_dmg: 0.0,
            voyage_permanent_hp: 0.0,
            voyage_permanent_speed: 0.0,
            voyage_permanent_crit: 0.0,
            voyages_completed: 0,
            highest_sector_ever: 0,
            voyage_bonuses: VoyageBonuses::default(),
            voyage_ships_built: 0,
            voyage_equipment_found: 0,
            voyage_crew_recruited: 0,
            voyage_credits_earned: 0,
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
            crew_roster: Vec::new(),
            crew_capacity: 5,
            next_crew_id: 1,
            crew_bonds: Vec::new(),
            faction_reputation: FactionReputation::default(),
            pending_faction_mission: None,
            cargo: HashMap::new(),
            cargo_capacity: 20,
            trade_history: Vec::new(),
            active_missions: Vec::new(),
            completed_missions: 0,
            failed_missions: 0,
            available_missions: Vec::new(),
            next_mission_id: 1,
            pending_popups: Vec::new(),
            pending_loot: Vec::new(),
            voyage_boss_defeated: false,
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
        if path.exists()
            && let Ok(data) = fs::read_to_string(&path) {
                // Reject suspiciously large save files (> 1MB)
                if data.len() > 1_000_000 {
                    return Self::new();
                }
                if let Ok(state) = serde_json::from_str::<Self>(&data) {
                    return state;
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
    #[allow(dead_code)] // For future equipment set bonus display
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

    /// Complete the current voyage: accumulate permanent bonuses and reset progression.
    /// Returns the bonuses earned from this voyage.
    pub fn complete_voyage(&mut self) -> VoyageBonuses {
        let earned = VoyageBonuses::for_completion(self.voyage);

        // Record stats
        self.voyages_completed += 1;
        if self.sector > self.highest_sector_ever {
            self.highest_sector_ever = self.sector;
        }

        // Track lifetime stats
        self.lifetime_sectors += self.sector as u64;
        self.lifetime_credits += self.credits;

        // Apply permanent bonuses
        self.voyage_permanent_dmg += VOYAGE_DAMAGE_BONUS;
        self.voyage_permanent_hp += VOYAGE_HP_BONUS;
        self.voyage_permanent_speed += VOYAGE_SPEED_BONUS;
        self.voyage_permanent_crit += VOYAGE_CRIT_BONUS;
        self.voyage_bonuses.accumulate(&earned);

        // Advance voyage
        self.voyage += 1;

        // Reset progression (keep permanents, pip, achievements, stats)
        self.sector = 1;
        self.level = 1;
        self.xp = 0;
        self.xp_to_next = 100;
        self.scrap = 0;
        self.credits = VOYAGE_STARTING_CREDITS;
        self.blueprints = 0;
        self.fleet = vec![Ship::new(ShipType::Scout)];
        self.inventory.clear();
        self.crew_roster.clear();
        self.crew_bonds.clear();
        self.next_crew_id = 1;
        self.active_missions.clear();
        self.available_missions.clear();
        self.next_mission_id = 1;
        self.cargo.clear();
        self.cargo_capacity = 20;
        self.trade_history.clear();
        self.tech_lasers = 1;
        self.tech_shields = 0;
        self.tech_engines = 1;
        self.tech_beams = 0;
        self.current_route_modifier = 1.0;
        self.phase = GamePhase::Travel;
        self.phase_timer = 45.0;
        self.faction_reputation = FactionReputation::default();
        self.pending_faction_mission = None;

        // Reset per-voyage stat trackers
        self.voyage_ships_built = 0;
        self.voyage_equipment_found = 0;
        self.voyage_crew_recruited = 0;
        self.voyage_credits_earned = 0;

        // Legacy prestige fields — zeroed to prevent double-stacking with voyage bonuses
        self.prestige_bonus_xp = 0.0;
        self.prestige_bonus_credits = 0.0;
        self.prestige_bonus_scrap = 0.0;

        // Keep: pip stats, achievements, deaths, highest_sector, time_played,
        //       enemies_destroyed, total_battles, voyage_permanent_* bonuses,
        //       voyages_completed, highest_sector_ever

        earned
    }

    /// Legacy prestige — replaced by voyage system. Kept for save compat.
    /// Now redirects to complete_voyage if voyage target is reached.
    #[allow(dead_code)] // Legacy prestige, voyage system is primary
    pub fn prestige(&mut self) -> bool {
        let target = crate::engine::voyage::voyage_target_sector(self.voyage);
        if self.sector < target {
            return false;
        }
        self.complete_voyage();
        true
    }

    /// Check if the current sector is the voyage boss sector.
    pub fn is_voyage_boss_sector(&self) -> bool {
        crate::engine::voyage::is_voyage_boss_sector(self.sector, self.voyage)
    }

    /// Get the target sector for the current voyage.
    #[allow(dead_code)] // Convenience wrapper for future use
    pub fn voyage_target_sector(&self) -> u32 {
        crate::engine::voyage::voyage_target_sector(self.voyage)
    }

    // ── Crew management methods ────────────────────────────────────

    /// Add a crew member to the roster. Returns false if at capacity.
    pub fn add_crew(&mut self, mut crew: CrewMember) -> bool {
        if self.crew_roster.len() >= self.crew_capacity {
            return false;
        }
        crew.id = self.next_crew_id;
        self.next_crew_id += 1;
        self.crew_roster.push(crew);
        true
    }

    /// Assign a crew member to a ship. Unassigns them from any previous ship first.
    /// Returns false if crew_id or ship_index is invalid, or ship already has crew.
    pub fn assign_crew(&mut self, crew_id: u64, ship_index: usize) -> bool {
        if ship_index >= self.fleet.len() {
            return false;
        }
        // Check ship doesn't already have a different crew assigned
        if let Some(existing_id) = self.fleet[ship_index].crew_id
            && existing_id != crew_id {
                return false;
            }
        // Find crew member
        let crew_idx = match self.crew_roster.iter().position(|c| c.id == crew_id) {
            Some(idx) => idx,
            None => return false,
        };
        // Unassign from previous ship if any
        if let Some(old_ship) = self.crew_roster[crew_idx].assigned_ship
            && old_ship < self.fleet.len() {
                self.fleet[old_ship].crew_id = None;
            }
        // Assign
        self.crew_roster[crew_idx].assigned_ship = Some(ship_index);
        self.fleet[ship_index].crew_id = Some(crew_id);
        true
    }

    /// Unassign a crew member from their ship. Returns false if crew_id not found.
    pub fn unassign_crew(&mut self, crew_id: u64) -> bool {
        let crew_idx = match self.crew_roster.iter().position(|c| c.id == crew_id) {
            Some(idx) => idx,
            None => return false,
        };
        if let Some(ship_idx) = self.crew_roster[crew_idx].assigned_ship
            && ship_idx < self.fleet.len() {
                self.fleet[ship_idx].crew_id = None;
            }
        self.crew_roster[crew_idx].assigned_ship = None;
        true
    }

    /// Get the crew member assigned to a specific ship.
    pub fn get_ship_crew(&self, ship_index: usize) -> Option<&CrewMember> {
        let crew_id = self.fleet.get(ship_index)?.crew_id?;
        self.crew_roster.iter().find(|c| c.id == crew_id)
    }

    /// Dismiss a crew member from the roster. Unassigns them first.
    pub fn dismiss_crew(&mut self, crew_id: u64) -> bool {
        let crew_idx = match self.crew_roster.iter().position(|c| c.id == crew_id) {
            Some(idx) => idx,
            None => return false,
        };
        // Unassign from ship first
        if let Some(ship_idx) = self.crew_roster[crew_idx].assigned_ship
            && ship_idx < self.fleet.len() {
                self.fleet[ship_idx].crew_id = None;
            }
        self.crew_roster.remove(crew_idx);
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

    // ── Faction methods ────────────────────────────────────────────

    /// Get player reputation with a faction.
    pub fn get_reputation(&self, faction: crate::engine::factions::Faction) -> i32 {
        self.faction_reputation.get(faction)
    }

    /// Change reputation with a faction (includes rival penalty cascade).
    /// Returns vec of (faction_key, old_rep, new_rep) for all changes.
    pub fn change_reputation(
        &mut self,
        faction: crate::engine::factions::Faction,
        amount: i32,
    ) -> Vec<(String, i32, i32)> {
        self.faction_reputation.change(faction, amount)
    }

    /// Get the dominant faction for a given sector.
    pub fn sector_faction(&self, sector: u32) -> crate::engine::factions::Faction {
        crate::engine::factions::sector_faction(sector)
    }

    /// Check if a faction is hostile to the player.
    #[allow(dead_code)] // For future faction-aware UI
    pub fn is_hostile(&self, faction: crate::engine::factions::Faction) -> bool {
        self.faction_reputation.is_hostile(faction)
    }

    /// Get price modifier for trading with a faction.
    /// 0.7 (allied) to 1.5 (hostile).
    pub fn price_modifier(&self, faction: crate::engine::factions::Faction) -> f32 {
        self.faction_reputation.price_modifier(faction)
    }

    // ── Mission methods ──────────────────────────────────────────

    /// Accept a mission from the available list, moving it to active.
    pub fn accept_mission(&mut self, mission_id: u64) -> bool {
        crate::engine::missions::accept_mission(
            &mut self.available_missions,
            &mut self.active_missions,
            mission_id,
        )
    }

    /// Check mission progress after a sector transition.
    pub fn check_mission_progress(
        &mut self,
        sector: u32,
        battle_won: bool,
        boss_killed: bool,
        raid_completed: bool,
        fleet_ship_lost: bool,
    ) -> Vec<crate::engine::missions::MissionUpdate> {
        let updates = crate::engine::missions::check_mission_progress(
            &mut self.active_missions,
            sector,
            battle_won,
            boss_killed,
            raid_completed,
            fleet_ship_lost,
        );

        // Update counters
        for update in &updates {
            match &update.update_type {
                crate::engine::missions::MissionUpdateType::Completed { .. } => {
                    self.completed_missions += 1;
                }
                crate::engine::missions::MissionUpdateType::Failed { .. } => {
                    self.failed_missions += 1;
                }
                _ => {}
            }
        }

        updates
    }

    /// Expire missions that are too far past their target.
    pub fn fail_expired_missions(&mut self) {
        let updates = crate::engine::missions::fail_expired_missions(
            &mut self.active_missions,
            self.sector,
        );
        self.failed_missions += updates.iter().filter(|u| {
            matches!(u.update_type, crate::engine::missions::MissionUpdateType::Failed { .. })
        }).count() as u64;
    }

    /// Refresh available missions for the current sector.
    pub fn refresh_available_missions(&mut self, sector: u32) {
        let faction = crate::engine::factions::sector_faction(sector);
        let count = 3 + (sector as usize / 10).min(2); // 3-5 missions available
        self.available_missions = crate::engine::missions::generate_missions(
            sector,
            &faction,
            count,
            &mut self.next_mission_id,
        );
    }

    // ── Cargo / Trade methods ──────────────────────────────────────

    /// Total number of items currently in cargo.
    pub fn cargo_total(&self) -> u32 {
        self.cargo.values().sum()
    }

    /// Remaining cargo space.
    pub fn cargo_space_remaining(&self) -> u32 {
        self.cargo_capacity.saturating_sub(self.cargo_total())
    }

    /// Buy goods from a market. Deducts credits and adds to cargo.
    /// Returns `true` on success, `false` if insufficient credits or cargo space.
    pub fn buy_goods(&mut self, good: TradeGood, quantity: u32, price_per_unit: u64) -> bool {
        if quantity == 0 {
            return false;
        }
        let total_cost = price_per_unit * quantity as u64;
        if self.credits < total_cost {
            return false;
        }
        if quantity > self.cargo_space_remaining() {
            return false;
        }

        self.credits -= total_cost;
        *self.cargo.entry(good.key().to_string()).or_insert(0) += quantity;

        // Record trade history (keep last 20)
        self.trade_history.push(TradeRecord {
            good,
            quantity,
            price_per_unit,
            was_buy: true,
            sector: self.sector,
        });
        if self.trade_history.len() > 20 {
            self.trade_history.remove(0);
        }

        true
    }

    /// Sell goods from cargo. Returns revenue earned, or 0 if insufficient goods.
    pub fn sell_goods(&mut self, good: TradeGood, quantity: u32, price_per_unit: u64) -> u64 {
        if quantity == 0 {
            return 0;
        }
        let held = self.cargo.get(good.key()).copied().unwrap_or(0);
        if held < quantity {
            return 0;
        }

        let revenue = price_per_unit * quantity as u64;
        let entry = self.cargo.get_mut(good.key()).expect("checked above");
        *entry -= quantity;
        if *entry == 0 {
            self.cargo.remove(good.key());
        }
        self.credits += revenue;

        // Record trade history (keep last 20)
        self.trade_history.push(TradeRecord {
            good,
            quantity,
            price_per_unit,
            was_buy: false,
            sector: self.sector,
        });
        if self.trade_history.len() > 20 {
            self.trade_history.remove(0);
        }

        revenue
    }

    /// Calculate profit/loss for a good based on trade history.
    /// Compares latest sell price per unit to the average buy price per unit.
    #[allow(dead_code)] // For future trade profit display
    pub fn trade_profit(&self, good: TradeGood) -> Option<i64> {
        let buys: Vec<&TradeRecord> = self
            .trade_history
            .iter()
            .filter(|r| r.good == good && r.was_buy)
            .collect();
        let sells: Vec<&TradeRecord> = self
            .trade_history
            .iter()
            .filter(|r| r.good == good && !r.was_buy)
            .collect();

        if buys.is_empty() || sells.is_empty() {
            return None;
        }

        let total_buy_cost: u64 = buys.iter().map(|r| r.total_cost()).sum();
        let total_buy_qty: u64 = buys.iter().map(|r| r.quantity as u64).sum();
        let avg_buy = total_buy_cost / total_buy_qty.max(1);

        let last_sell = sells.last().expect("checked above");
        Some(last_sell.price_per_unit as i64 - avg_buy as i64)
    }

    /// Apply a contraband fine: deduct credits and confiscate goods.
    pub fn apply_contraband_fine(&mut self, good: TradeGood, fine: u64) {
        // Confiscate all of the detected good
        self.cargo.remove(good.key());
        // Deduct fine (can't go below 0)
        self.credits = self.credits.saturating_sub(fine);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state_defaults() {
        let state = GameState::new();
        assert_eq!(state.voyage, 1);
        assert!((state.voyage_permanent_dmg - 0.0).abs() < f32::EPSILON);
        assert!((state.voyage_permanent_hp - 0.0).abs() < f32::EPSILON);
        assert!((state.voyage_permanent_speed - 0.0).abs() < f32::EPSILON);
        assert!((state.voyage_permanent_crit - 0.0).abs() < f32::EPSILON);
        assert_eq!(state.voyages_completed, 0);
        assert_eq!(state.highest_sector_ever, 0);
    }

    #[test]
    fn test_complete_voyage_increments() {
        let mut state = GameState::new();
        state.sector = 100;
        state.credits = 5000;
        state.scrap = 3000;
        state.level = 20;

        let earned = state.complete_voyage();
        assert!(earned.damage_pct > 0.0);
        assert_eq!(state.voyage, 2);
        assert_eq!(state.voyages_completed, 1);
        assert!((state.voyage_permanent_dmg - 0.05).abs() < f32::EPSILON);
        assert!((state.voyage_permanent_hp - 0.05).abs() < f32::EPSILON);
        assert!((state.voyage_permanent_speed - 0.03).abs() < f32::EPSILON);
        assert!((state.voyage_permanent_crit - 0.02).abs() < f32::EPSILON);
    }

    #[test]
    fn test_complete_voyage_resets_progression() {
        let mut state = GameState::new();
        state.sector = 100;
        state.credits = 5000;
        state.scrap = 3000;
        state.level = 20;
        state.xp = 500;
        state.tech_lasers = 5;
        state.tech_shields = 3;
        state.fleet.push(Ship::new(ShipType::Fighter));

        state.complete_voyage();

        assert_eq!(state.sector, 1);
        assert_eq!(state.level, 1);
        assert_eq!(state.xp, 0);
        assert_eq!(state.scrap, 0);
        assert_eq!(state.credits, VOYAGE_STARTING_CREDITS);
        assert_eq!(state.tech_lasers, 1);
        assert_eq!(state.tech_shields, 0);
        assert_eq!(state.fleet.len(), 1);
        assert!(state.inventory.is_empty());
    }

    #[test]
    fn test_complete_voyage_keeps_pip() {
        let mut state = GameState::new();
        state.sector = 100;
        state.pip_level = 5;
        state.pip_xp = 100;
        state.pip_bond = 50;
        state.pip_appearance = 3;

        state.complete_voyage();

        assert_eq!(state.pip_level, 5);
        assert_eq!(state.pip_xp, 100);
        assert_eq!(state.pip_bond, 50);
        assert_eq!(state.pip_appearance, 3);
    }

    #[test]
    fn test_complete_voyage_keeps_stats() {
        let mut state = GameState::new();
        state.sector = 100;
        state.total_battles = 50;
        state.enemies_destroyed = 200;
        state.deaths = 3;
        state.time_played_secs = 3600;
        state.achievements_unlocked.push("first_blood".to_string());

        state.complete_voyage();

        assert_eq!(state.total_battles, 50);
        assert_eq!(state.enemies_destroyed, 200);
        assert_eq!(state.deaths, 3);
        assert_eq!(state.time_played_secs, 3600);
        assert_eq!(state.achievements_unlocked.len(), 1);
    }

    #[test]
    fn test_complete_voyage_tracks_highest_sector() {
        let mut state = GameState::new();
        state.sector = 150;

        state.complete_voyage();

        assert_eq!(state.highest_sector_ever, 150);
    }

    #[test]
    fn test_complete_multiple_voyages() {
        let mut state = GameState::new();

        // Complete voyage 1
        state.sector = 100;
        state.complete_voyage();
        assert_eq!(state.voyage, 2);
        assert_eq!(state.voyages_completed, 1);

        // Complete voyage 2
        state.sector = 200;
        state.complete_voyage();
        assert_eq!(state.voyage, 3);
        assert_eq!(state.voyages_completed, 2);
        assert!((state.voyage_permanent_dmg - 0.10).abs() < f32::EPSILON);
        assert!((state.voyage_permanent_hp - 0.10).abs() < f32::EPSILON);
    }

    #[test]
    fn test_is_voyage_boss_sector() {
        let mut state = GameState::new();
        state.sector = 100;
        assert!(state.is_voyage_boss_sector());

        state.sector = 99;
        assert!(!state.is_voyage_boss_sector());

        state.voyage = 3;
        state.sector = 300;
        assert!(state.is_voyage_boss_sector());
    }

    #[test]
    fn test_voyage_target_sector() {
        let state = GameState::new();
        assert_eq!(state.voyage_target_sector(), 100);
    }
}
