use std::collections::VecDeque;

use crate::state::GameState;

// ── Event Types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum GameEvent {
    // Combat
    BattleStarted { sector: u32, enemy_count: u32 },
    BattleWon { sector: u32, enemies_killed: u32, fleet_hp_pct: f32, was_boss: bool },
    BattleLost { sector: u32, penalty_description: String },
    EnemyKilled { sector: u32, enemy_tier: u32, is_boss: bool },
    ShipDamaged { ship_index: usize, damage: u32, hp_remaining: u32 },
    CriticalHit { ship_index: usize, damage: u32 },

    // Progression
    SectorCleared { sector: u32 },
    LevelUp { new_level: u32 },
    AchievementUnlocked { id: String, name: String, icon: char },
    PrestigeCompleted { level: u32 },

    // Economy
    ScrapGained { amount: u64, source: String },
    CreditsGained { amount: u64, source: String },
    EquipmentDropped { rarity: String, name: String },
    ItemSalvaged { scrap_value: u64 },

    // Fleet
    ShipBuilt { ship_type: String },
    ShipUpgraded { ship_index: usize, new_level: u8 },
    EquipmentChanged { ship_index: usize, slot: String },

    // Companion
    PipFed,
    PipPetted,
    PipLevelUp { new_level: u8 },

    // Travel
    EventEncountered { event_type: String },
    EventResolved { event_type: String, outcome: String },
    RouteChosen { route_name: String, modifier: f32 },

    // Raid
    RaidStarted { planet_type: String },
    RaidCompleted { scrap: u64, credits: u64 },

    // Crew relationships
    CrewAbilityTriggered { crew_name: String, ability_name: String, icon: char },
    CrewBondFormed { crew_a: String, crew_b: String, bond_type: String },
    CrewGrief { survivor: String, fallen: String },
    CrewVengeance { crew_name: String },
    CrewDeserted { crew_name: String },
    CrewMoraleChange { crew_id: u64, amount: i8 },
    CrewBondProgress { crew_a_id: u64, crew_b_id: u64, amount: u32 },
}

// ── Event Bus ───────────────────────────────────────────────────────────────

pub struct EventBus {
    queue: Vec<GameEvent>,
    history: VecDeque<GameEvent>, // last N events for debug/display
    history_cap: usize,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            queue: Vec::with_capacity(32),
            history: VecDeque::with_capacity(50),
            history_cap: 50,
        }
    }

    /// Emit an event — queued for processing at end of tick.
    pub fn emit(&mut self, event: GameEvent) {
        self.queue.push(event);
    }

    /// Drain all queued events for processing.
    pub fn drain(&mut self) -> Vec<GameEvent> {
        let events = std::mem::take(&mut self.queue);
        for e in &events {
            self.history.push_back(e.clone());
            if self.history.len() > self.history_cap {
                self.history.pop_front();
            }
        }
        events
    }

    /// Get recent event history (for debug/stats display).
    pub fn recent(&self) -> &VecDeque<GameEvent> {
        &self.history
    }

    pub fn has_pending(&self) -> bool {
        !self.queue.is_empty()
    }
}

// ── Event Processor ─────────────────────────────────────────────────────────

/// Process events and update all relevant systems in ONE place.
/// This is the central hub for cross-system effects.
///
/// `bridge` is passed so Pip can react to game events (battle wins/losses,
/// achievements, etc.) without scenes needing to know about the bridge.
pub fn process_events(
    events: &[GameEvent],
    state: &mut GameState,
    bridge: &mut crate::scenes::bridge::BridgeScene,
    popup_text: &mut Option<String>,
    popup_timer: &mut u8,
) {
    for event in events {
        match event {
            GameEvent::EnemyKilled { enemy_tier, is_boss, .. } => {
                state.enemies_destroyed += 1;

                // Crew: all assigned crew get bonus XP per kill
                // (Can't attribute kills to specific ships without battle refactor)
                let bonus_xp = 3 + (*enemy_tier as u64) * 2;
                for crew in &mut state.crew_roster {
                    if crew.assigned_ship.is_some() {
                        crew.kills += 1;
                        crew.add_xp(bonus_xp);
                    }
                }
                let _ = is_boss;
            }
            GameEvent::CriticalHit { damage, .. } => {
                let _ = damage; // logged to history for stats display
            }
            GameEvent::BattleWon { sector, enemies_killed, was_boss, .. } => {
                state.total_battles += 1;
                // Note: individual EnemyKilled events handle enemies_destroyed
                let _ = (enemies_killed, was_boss);
                bridge.notify_battle_win(state);

                // Crew: all assigned crew gain XP and morale on victory
                let xp_gain = 10 + (*sector as u64) * 2;
                let mut leveled_names: Vec<String> = Vec::new();
                for crew in &mut state.crew_roster {
                    if crew.assigned_ship.is_some() {
                        crew.battles_survived += 1;
                        crew.morale = crew.morale.saturating_add(5).min(100);
                        if crew.add_xp(xp_gain) {
                            leveled_names.push(format!("{} (Lv.{})", crew.name, crew.level));
                        }
                    }
                }
                for name in leveled_names {
                    if popup_text.is_some() {
                        // Don't overwrite existing popup
                    } else {
                        *popup_text = Some(format!("\u{2b50} {} leveled up!", name));
                        *popup_timer = 50;
                    }
                }
            }
            GameEvent::BattleLost { sector, penalty_description } => {
                state.total_battles += 1;
                // Note: handle_fleet_death increments state.deaths internally
                *popup_text = Some(format!("\u{1f480} {}", penalty_description));
                *popup_timer = 80;
                bridge.notify_battle_loss(state);

                // Crew: morale penalty for all assigned crew
                // Check for crew deaths on destroyed ships (ships with 0 HP)
                let dead_crew_names: Vec<String> = Vec::new();
                for crew in &mut state.crew_roster {
                    if crew.assigned_ship.is_some() {
                        crew.morale = crew.morale.saturating_sub(15);
                    }
                }
                // Note: handle_fleet_death already heals all ships back, so we can't
                // check HP=0 here. In a fleet wipe, crew survive but with morale hit.
                // Future: track which ships were destroyed before heal for crew death.
                let _ = (sector, &dead_crew_names);
            }
            GameEvent::LevelUp { new_level } => {
                *popup_text = Some(format!("⭐ LEVEL UP! — Level {}", new_level));
                *popup_timer = 60;
            }
            GameEvent::SectorCleared { sector } => {
                let _ = sector; // logged to history, achievements check sector via state
            }
            GameEvent::AchievementUnlocked { icon, name, .. } => {
                *popup_text = Some(format!("{} Achievement: {}", icon, name));
                *popup_timer = 60;
                bridge.notify_achievement(state);
            }
            GameEvent::EquipmentDropped { rarity, name } => {
                if rarity == "Legendary" || rarity == "Epic" {
                    *popup_text = Some(format!("✦ {} drop: {}!", rarity, name));
                    *popup_timer = 50;
                }
            }
            GameEvent::ScrapGained { amount, source } => {
                let _ = (amount, source); // logged to history for stats
            }
            GameEvent::EventResolved { event_type, outcome } => {
                let _ = (event_type, outcome); // logged to history for stats
            }
            GameEvent::PipLevelUp { new_level } => {
                *popup_text = Some(format!("🤖 Pip leveled up to {}!", new_level));
                *popup_timer = 50;
            }
            GameEvent::PrestigeCompleted { level } => {
                *popup_text = Some(format!("★ PRESTIGE {} ★", level));
                *popup_timer = 80;
            }
            GameEvent::RaidCompleted { scrap, credits } => {
                state.total_raids += 1;
                let _ = (scrap, credits); // amounts already applied in raid scene
            }
            // Crew relationship events
            GameEvent::CrewBondFormed { crew_a, crew_b, bond_type } => {
                *popup_text = Some(format!("⚔ {} & {}: {}!", crew_a, crew_b, bond_type));
                *popup_timer = 60;
            }
            GameEvent::CrewGrief { survivor, fallen } => {
                *popup_text = Some(format!("💔 {} grieves for {}...", survivor, fallen));
                *popup_timer = 70;
            }
            GameEvent::CrewVengeance { crew_name } => {
                *popup_text = Some(format!("🔥 {} burns with vengeance!", crew_name));
                *popup_timer = 60;
            }
            GameEvent::CrewDeserted { crew_name } => {
                *popup_text = Some(format!("🚪 {} has deserted the crew!", crew_name));
                *popup_timer = 80;
            }
            GameEvent::CrewMoraleChange { crew_id, amount } => {
                if let Some(crew) = state.crew_roster.iter_mut().find(|c| c.id == *crew_id) {
                    if *amount >= 0 {
                        crew.morale = crew.morale.saturating_add(*amount as u8).min(100);
                    } else {
                        crew.morale = crew.morale.saturating_sub(amount.unsigned_abs());
                    }
                }
            }
            GameEvent::CrewBondProgress { crew_a_id, crew_b_id, amount } => {
                if let Some(idx) = crate::engine::crew::find_bond(&state.crew_bonds, *crew_a_id, *crew_b_id) {
                    state.crew_bonds[idx].battles_together += amount;
                }
            }
            // Other events are logged to history but don't trigger cross-system effects yet.
            _ => {}
        }
    }
}
