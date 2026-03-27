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

    // Factions
    FactionRepChange { faction: String, amount: i32, reason: String },
    FactionEncounter { faction: String, hostile: bool },

    // Trade
    GoodsBought { good: String, quantity: u32, total_cost: u64 },
    GoodsSold { good: String, quantity: u32, revenue: u64, profit: i64 },
    ContrabandDetected { good: String, faction: String, fine: u64 },

    // Missions
    MissionAccepted { title: String, faction: String },
    MissionCompleted { title: String, reward_credits: u64, reward_rep: i32 },
    MissionFailed { title: String, reason: String },

    // Voyage
    VoyageCompleted { voyage: u32, next_name: String },
    VoyageBossSpawned { voyage: u32, boss_name: String },
}

// ── Event → Log Entry conversion ────────────────────────────────────────────

use ratatui::style::Color;
use crate::scenes::gamelog::LogEntry;

pub fn event_to_log_entries(event: &GameEvent, state: &GameState) -> Vec<LogEntry> {
    match event {
        GameEvent::BattleWon { sector, enemies_killed, was_boss, fleet_hp_pct, .. } => {
            let mut details = vec![format!("Destroyed {} ships", enemies_killed)];
            if *was_boss {
                details.push("Boss defeated!".into());
            }
            if *fleet_hp_pct < 0.3 {
                details.push("Close call — fleet nearly destroyed!".into());
            }
            vec![LogEntry {
                sector: *sector,
                icon: '⚔',
                title: if *was_boss { "Boss Battle Won!".into() } else { "Battle Won".into() },
                details,
                color: Color::Green,
            }]
        }
        GameEvent::BattleLost { sector, penalty_description } => {
            vec![LogEntry {
                sector: *sector,
                icon: '💀',
                title: "Fleet Destroyed".into(),
                details: vec![penalty_description.clone()],
                color: Color::Red,
            }]
        }
        GameEvent::BattleStarted { sector, enemy_count } => {
            vec![LogEntry {
                sector: *sector,
                icon: '⚠',
                title: format!("Battle! {} hostiles", enemy_count),
                details: vec![],
                color: Color::Yellow,
            }]
        }
        GameEvent::SectorCleared { sector } => {
            vec![LogEntry {
                sector: *sector,
                icon: '✓',
                title: format!("Sector {} cleared", sector),
                details: vec![],
                color: Color::Cyan,
            }]
        }
        GameEvent::LevelUp { new_level } => {
            vec![LogEntry {
                sector: state.sector,
                icon: '⭐',
                title: format!("Level Up! Now Lv.{}", new_level),
                details: vec![],
                color: Color::Yellow,
            }]
        }
        GameEvent::AchievementUnlocked { icon, name, .. } => {
            vec![LogEntry {
                sector: state.sector,
                icon: *icon,
                title: format!("Achievement: {}", name),
                details: vec![],
                color: Color::Yellow,
            }]
        }
        GameEvent::PrestigeCompleted { level } => {
            vec![LogEntry {
                sector: state.sector,
                icon: '★',
                title: format!("PRESTIGE {} completed!", level),
                details: vec!["All progression reset. Permanent bonuses gained.".into()],
                color: Color::Magenta,
            }]
        }
        GameEvent::EquipmentDropped { rarity, name } => {
            let icon = match rarity.as_str() {
                "Legendary" => '★',
                "Epic" => '◆',
                "Rare" => '●',
                "Uncommon" => '○',
                _ => '·',
            };
            vec![LogEntry {
                sector: state.sector,
                icon,
                title: format!("{} {}", rarity, name),
                details: vec![],
                color: match rarity.as_str() {
                    "Legendary" => Color::Yellow,
                    "Epic" => Color::Magenta,
                    "Rare" => Color::Blue,
                    "Uncommon" => Color::Green,
                    _ => Color::Gray,
                },
            }]
        }
        GameEvent::ScrapGained { amount, source } => {
            vec![LogEntry {
                sector: state.sector,
                icon: '◇',
                title: format!("+{} scrap", amount),
                details: vec![source.clone()],
                color: Color::Gray,
            }]
        }
        GameEvent::CreditsGained { amount, source } => {
            vec![LogEntry {
                sector: state.sector,
                icon: '₿',
                title: format!("+{} credits", amount),
                details: vec![source.clone()],
                color: Color::Yellow,
            }]
        }
        GameEvent::CrewAbilityTriggered { crew_name, ability_name, .. } => {
            vec![LogEntry {
                sector: state.sector,
                icon: '⚡',
                title: format!("{}: {}!", crew_name, ability_name),
                details: vec![],
                color: Color::Cyan,
            }]
        }
        GameEvent::CrewBondFormed { crew_a, crew_b, bond_type } => {
            vec![LogEntry {
                sector: state.sector,
                icon: '♦',
                title: format!("{} & {}", crew_a, crew_b),
                details: vec![format!("Bond formed: {}", bond_type)],
                color: Color::Magenta,
            }]
        }
        GameEvent::CrewDeserted { crew_name } => {
            vec![LogEntry {
                sector: state.sector,
                icon: '🚪',
                title: format!("{} deserted!", crew_name),
                details: vec![],
                color: Color::Red,
            }]
        }
        GameEvent::FactionRepChange { faction, amount, reason } => {
            let icon = if *amount > 0 { '↑' } else { '↓' };
            vec![LogEntry {
                sector: state.sector,
                icon,
                title: format!("{} {}{}", faction, if *amount > 0 { "+" } else { "" }, amount),
                details: vec![reason.clone()],
                color: if *amount > 0 { Color::Green } else { Color::Red },
            }]
        }
        GameEvent::FactionEncounter { faction, hostile } => {
            vec![LogEntry {
                sector: state.sector,
                icon: if *hostile { '⚠' } else { '🏳' },
                title: format!("{} {} encounter", faction, if *hostile { "hostile" } else { "friendly" }),
                details: vec![],
                color: if *hostile { Color::Red } else { Color::Green },
            }]
        }
        GameEvent::RaidCompleted { scrap, credits } => {
            vec![LogEntry {
                sector: state.sector,
                icon: '🌍',
                title: "Raid Completed".into(),
                details: vec![format!("+{} scrap, +{} credits", scrap, credits)],
                color: Color::Green,
            }]
        }
        GameEvent::EventEncountered { event_type } => {
            vec![LogEntry {
                sector: state.sector,
                icon: '🚀',
                title: format!("Event: {}", event_type),
                details: vec![],
                color: Color::Cyan,
            }]
        }
        GameEvent::EventResolved { event_type, outcome } => {
            vec![LogEntry {
                sector: state.sector,
                icon: '✦',
                title: format!("{} resolved", event_type),
                details: vec![outcome.clone()],
                color: Color::Cyan,
            }]
        }
        GameEvent::MissionAccepted { title, faction } => {
            vec![LogEntry {
                sector: state.sector,
                icon: '📋',
                title: format!("Mission: {}", title),
                details: vec![format!("From: {}", faction)],
                color: Color::Cyan,
            }]
        }
        GameEvent::MissionCompleted { title, reward_credits, .. } => {
            vec![LogEntry {
                sector: state.sector,
                icon: '✅',
                title: format!("Mission Complete: {}", title),
                details: vec![format!("Reward: {}₿", reward_credits)],
                color: Color::Green,
            }]
        }
        GameEvent::MissionFailed { title, reason } => {
            vec![LogEntry {
                sector: state.sector,
                icon: '❌',
                title: format!("Mission Failed: {}", title),
                details: vec![reason.clone()],
                color: Color::Red,
            }]
        }
        GameEvent::GoodsSold { good, quantity, revenue, profit } => {
            let mut details = vec![format!("{} × {} for {} cr", quantity, good, revenue)];
            if *profit > 0 {
                details.push(format!("+{} profit/unit", profit));
            }
            vec![LogEntry {
                sector: state.sector,
                icon: '💰',
                title: "Trade: Sold goods".into(),
                details,
                color: Color::Yellow,
            }]
        }
        GameEvent::ContrabandDetected { good, faction, fine } => {
            vec![LogEntry {
                sector: state.sector,
                icon: '⚠',
                title: format!("{} detected contraband!", faction),
                details: vec![format!("{} confiscated, fined {} cr", good, fine)],
                color: Color::Red,
            }]
        }
        GameEvent::ShipBuilt { ship_type } => {
            vec![LogEntry {
                sector: state.sector,
                icon: '🔧',
                title: format!("Ship Built: {}", ship_type),
                details: vec![],
                color: Color::Green,
            }]
        }
        GameEvent::PipLevelUp { new_level } => {
            vec![LogEntry {
                sector: state.sector,
                icon: '🤖',
                title: format!("Pip leveled up to {}!", new_level),
                details: vec![],
                color: Color::Magenta,
            }]
        }
        GameEvent::VoyageCompleted { voyage, next_name } => {
            vec![LogEntry {
                sector: state.sector,
                icon: '◈',
                title: format!("VOYAGE {} COMPLETE!", voyage),
                details: vec![
                    format!("Beginning {}", next_name),
                    "Permanent bonuses gained!".into(),
                ],
                color: Color::Magenta,
            }]
        }
        GameEvent::VoyageBossSpawned { voyage, boss_name } => {
            vec![LogEntry {
                sector: state.sector,
                icon: '☠',
                title: format!("VOYAGE {} BOSS: {}", voyage, boss_name),
                details: vec!["Defeat the boss to complete this voyage!".into()],
                color: Color::Red,
            }]
        }
        // Events that are too granular for the log (per-tick combat events, etc.)
        _ => vec![],
    }
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

// ── Pip Commentary ──────────────────────────────────────────────────────────

/// Pip comments on game events with personality.
/// Returns a speech bubble text to display in the HUD area.
pub fn pip_commentary(event: &GameEvent, state: &GameState) -> Option<String> {
    match event {
        GameEvent::BattleStarted { enemy_count, .. } => {
            match *enemy_count {
                1 => Some("Just one? Easy!".into()),
                2..=3 => Some("Let's get 'em!".into()),
                4..=6 => Some("That's a lot of ships...".into()),
                _ => Some("Uh oh...".into()),
            }
        }
        GameEvent::BattleWon { was_boss, fleet_hp_pct, .. } => {
            if *was_boss {
                Some("WE DID IT! Boss down!!".into())
            } else if *fleet_hp_pct > 0.9 {
                Some("Flawless! \u{2665}".into())
            } else if *fleet_hp_pct < 0.3 {
                Some("That was too close...".into())
            } else {
                let options = ["Victory!", "Nice work!", "Another one down!", "Woohoo!"];
                Some(options[state.total_battles as usize % options.len()].into())
            }
        }
        GameEvent::BattleLost { .. } => {
            let options = ["We'll get them next time...", "Ouch...", "That hurt...", "*hides*"];
            Some(options[state.deaths as usize % options.len()].into())
        }
        GameEvent::CriticalHit { damage, .. } => {
            if *damage > 50 {
                Some("BOOM! Critical!!".into())
            } else {
                Some("Nice shot!".into())
            }
        }
        GameEvent::EquipmentDropped { rarity, .. } => {
            match rarity.as_str() {
                "Legendary" => Some("\u{2726} LEGENDARY?! No way!!".into()),
                "Epic" => Some("Ooh, shiny purple!".into()),
                "Rare" => Some("That's a keeper!".into()),
                _ => None,
            }
        }
        GameEvent::LevelUp { new_level } => {
            Some(format!("Level {}! We're getting stronger!", new_level))
        }
        GameEvent::CrewAbilityTriggered { crew_name, ability_name, .. } => {
            let options = [
                format!("Go {}!", crew_name),
                format!("{}! Nice!", ability_name),
                format!("{} is amazing!", crew_name),
            ];
            Some(options[state.sector as usize % options.len()].clone())
        }
        GameEvent::SectorCleared { sector } => {
            if sector % 10 == 0 {
                Some(format!("Sector {}! Milestone!", sector))
            } else if sector % 5 == 0 {
                Some("Good progress!".into())
            } else {
                None
            }
        }
        GameEvent::FactionRepChange { faction, amount, .. } => {
            if *amount <= -20 {
                Some(format!("The {} won't like that...", faction))
            } else if *amount >= 20 {
                Some(format!("The {} are pleased!", faction))
            } else {
                None
            }
        }
        GameEvent::MissionCompleted { .. } => {
            Some("Mission complete! Ka-ching!".into())
        }
        GameEvent::CrewBondFormed { crew_a, crew_b, bond_type } => {
            Some(format!("{} and {} -- {}!", crew_a, crew_b, bond_type))
        }
        GameEvent::CrewGrief { survivor, fallen } => {
            Some(format!("Poor {}... {} is gone.", survivor, fallen))
        }
        GameEvent::PrestigeCompleted { level } => {
            Some(format!("Prestige {}! A fresh start!", level))
        }
        GameEvent::VoyageCompleted { voyage, next_name } => {
            Some(format!("Voyage {} done! {} awaits!", voyage, next_name))
        }
        GameEvent::VoyageBossSpawned { boss_name, .. } => {
            Some(format!("{}?! This is it!!", boss_name))
        }
        GameEvent::ContrabandDetected { .. } => {
            Some("Busted! Run!!".into())
        }
        GameEvent::CrewDeserted { crew_name } => {
            Some(format!("{} left us...", crew_name))
        }
        _ => None,
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
                bridge.react_to_event(event, state);

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

                // Update crew bonds from shared combat experience
                use crate::engine::crew::update_crew_bonds;
                let bond_changes = update_crew_bonds(&state.crew_roster, &mut state.crew_bonds);
                for (crew_a, crew_b, bond_type) in bond_changes {
                    if popup_text.is_none() {
                        *popup_text = Some(format!("{} & {} — {}!", crew_a, crew_b, bond_type.description()));
                        *popup_timer = 50;
                    }
                }

                // Tick grief/vengeance for assigned crew
                use crate::engine::crew::tick_grief_vengeance;
                for crew in &mut state.crew_roster {
                    if crew.assigned_ship.is_some() {
                        tick_grief_vengeance(crew);
                    }
                }
            }
            GameEvent::BattleLost { sector, penalty_description } => {
                state.total_battles += 1;
                // Note: handle_fleet_death increments state.deaths internally
                *popup_text = Some(format!("\u{1f480} {}", penalty_description));
                *popup_timer = 80;
                bridge.react_to_event(event, state);

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
                bridge.react_to_event(event, state);
            }
            GameEvent::EquipmentDropped { rarity, name } => {
                if rarity == "Legendary" || rarity == "Epic" {
                    *popup_text = Some(format!("✦ {} drop: {}!", rarity, name));
                    *popup_timer = 50;
                }
                bridge.react_to_event(event, state);
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
                bridge.react_to_event(event, state);
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
                bridge.react_to_event(event, state);
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
            // Faction events — apply actual rep changes to state
            GameEvent::FactionRepChange { faction, amount, reason } => {
                if let Some(f) = crate::engine::factions::Faction::from_key(faction)
                    .or_else(|| {
                        // Also match by display name for convenience
                        crate::engine::factions::Faction::ALL.iter()
                            .find(|ff| ff.name() == faction.as_str())
                            .copied()
                    })
                {
                    state.change_reputation(f, *amount);
                }
                let abs_amount = amount.unsigned_abs();
                if abs_amount >= 10 {
                    let direction = if *amount > 0 { "+" } else { "" };
                    *popup_text = Some(format!(
                        "{}{} rep with {} ({})",
                        direction, amount, faction, reason
                    ));
                    *popup_timer = 50;
                }
            }
            GameEvent::FactionEncounter { faction, hostile } => {
                if *hostile {
                    *popup_text = Some(format!("⚠ Hostile {} fleet detected!", faction));
                    *popup_timer = 40;
                }
            }
            // Trade events
            GameEvent::GoodsBought { good, quantity, total_cost } => {
                *popup_text = Some(format!(
                    "🛒 Bought {} {} for {} cr",
                    quantity, good, total_cost
                ));
                *popup_timer = 40;
                // Trading gives small rep boost with sector faction
                let sector_faction = state.sector_faction(state.sector);
                if sector_faction != crate::engine::factions::Faction::Independent {
                    state.change_reputation(sector_faction, crate::engine::factions::ReputationChange::TRADE);
                }
            }
            GameEvent::GoodsSold { good, quantity, revenue, profit } => {
                let profit_color = if *profit > 0 { "+" } else { "" };
                *popup_text = Some(format!(
                    "💰 Sold {} {} for {} cr ({}{}cr/unit)",
                    quantity, good, revenue, profit_color, profit
                ));
                *popup_timer = 40;
            }
            GameEvent::ContrabandDetected { good, faction, fine } => {
                // Apply fine and confiscate goods
                if let Some(trade_good) = crate::engine::trade::TradeGood::from_key(good) {
                    state.apply_contraband_fine(trade_good, *fine);
                }
                // Reputation hit with enforcing faction
                if let Some(f) = crate::engine::factions::Faction::from_key(faction)
                    .or_else(|| {
                        crate::engine::factions::Faction::ALL.iter()
                            .find(|ff| ff.name() == faction.as_str())
                            .copied()
                    })
                {
                    state.change_reputation(f, -10);
                }
                *popup_text = Some(format!(
                    "⚠ {} detected {} contraband! Fined {} credits!",
                    faction, good, fine
                ));
                *popup_timer = 70;
            }
            // Mission events
            GameEvent::MissionAccepted { title, faction } => {
                *popup_text = Some(format!("📋 Mission accepted: {} ({})", title, faction));
                *popup_timer = 50;
            }
            GameEvent::MissionCompleted { title, reward_credits, reward_rep } => {
                // Apply mission rewards
                state.credits += reward_credits;
                // Rep is applied via the mission's faction — find it from active/completed missions
                // Note: the mission progress checker already incremented completed_missions counter
                *popup_text = Some(format!(
                    "✅ Mission complete: {} (+{} cr, +{} rep)",
                    title, reward_credits, reward_rep
                ));
                *popup_timer = 70;
            }
            GameEvent::MissionFailed { title, reason } => {
                *popup_text = Some(format!("❌ Mission failed: {} — {}", title, reason));
                *popup_timer = 60;
            }
            GameEvent::VoyageCompleted { voyage, next_name } => {
                *popup_text = Some(format!(
                    "◈ VOYAGE {} COMPLETE ◈ — {} awaits!",
                    voyage, next_name
                ));
                *popup_timer = 100;
                bridge.react_to_event(event, state);
            }
            GameEvent::VoyageBossSpawned { voyage, boss_name } => {
                *popup_text = Some(format!(
                    "☠ VOYAGE {} BOSS: {} ☠",
                    voyage, boss_name
                ));
                *popup_timer = 80;
                bridge.react_to_event(event, state);
            }
            // Other events are logged to history but don't trigger cross-system effects yet.
            _ => {}
        }
    }
}
