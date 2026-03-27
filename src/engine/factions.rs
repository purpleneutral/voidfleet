/// Faction system — faction definitions, reputation tracking, rivalries, and sector control.

use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Faction Enum ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Faction {
    TradeGuild,      // merchants, traders — neutral, economic
    PirateClan,      // raiders — hostile early, can ally
    MilitaryCorp,    // government military — powerful, orderly
    AlienCollective, // alien species — mysterious, advanced tech
    RebelAlliance,   // freedom fighters — scrappy, idealistic
    Independent,     // player + unaffiliated NPCs
}

/// Type alias for backwards compatibility with scenes that use `FactionId`.
pub type FactionId = Faction;

/// Alias for `sector_dominant_faction` used by scene code.
pub fn sector_faction(sector: u32) -> Faction {
    sector_dominant_faction(sector)
}

impl Faction {
    pub const ALL: [Faction; 6] = [
        Self::TradeGuild,
        Self::PirateClan,
        Self::MilitaryCorp,
        Self::AlienCollective,
        Self::RebelAlliance,
        Self::Independent,
    ];

    /// Alias for `PirateClan` used by some scene code.
    #[allow(non_upper_case_globals)]
    pub const Pirates: Faction = Faction::PirateClan;

    /// All factions that have reputation tracking (excludes Independent).
    pub const TRACKABLE: [Faction; 5] = [
        Self::TradeGuild,
        Self::PirateClan,
        Self::MilitaryCorp,
        Self::AlienCollective,
        Self::RebelAlliance,
    ];

    /// Get the static info for this faction.
    pub fn info(&self) -> &'static FactionInfo {
        match self {
            Faction::PirateClan => &FACTIONS[0],
            Faction::TradeGuild => &FACTIONS[1],
            Faction::MilitaryCorp => &FACTIONS[2],
            Faction::AlienCollective => &FACTIONS[3],
            Faction::RebelAlliance => &FACTIONS[4],
            Faction::Independent => &INDEPENDENT_INFO,
        }
    }

    /// Canonical string key used for serde-compatible reputation storage.
    pub fn key(&self) -> &'static str {
        match self {
            Faction::TradeGuild => "TradeGuild",
            Faction::PirateClan => "PirateClan",
            Faction::MilitaryCorp => "MilitaryCorp",
            Faction::AlienCollective => "AlienCollective",
            Faction::RebelAlliance => "RebelAlliance",
            Faction::Independent => "Independent",
        }
    }

    /// Parse a faction from its key string.
    pub fn from_key(key: &str) -> Option<Faction> {
        match key {
            "TradeGuild" => Some(Faction::TradeGuild),
            "PirateClan" => Some(Faction::PirateClan),
            "MilitaryCorp" => Some(Faction::MilitaryCorp),
            "AlienCollective" => Some(Faction::AlienCollective),
            "RebelAlliance" => Some(Faction::RebelAlliance),
            "Independent" => Some(Faction::Independent),
            _ => None,
        }
    }

    /// Human-readable name of this faction.
    pub fn name(&self) -> &'static str {
        self.info().name
    }

    /// Short code (2-3 chars) for this faction.
    pub fn code(&self) -> &'static str {
        self.info().short_name
    }

    /// Description of this faction.
    pub fn description(&self) -> &'static str {
        self.info().description
    }

    /// Icon character for this faction.
    pub fn icon(&self) -> &'static str {
        // Return as str for Span compatibility
        match self {
            Faction::PirateClan => "☠",
            Faction::TradeGuild => "₿",
            Faction::MilitaryCorp => "⚔",
            Faction::AlienCollective => "◈",
            Faction::RebelAlliance => "★",
            Faction::Independent => "◊",
        }
    }

    /// Get the rival factions.
    pub fn rivals(&self) -> &'static [Faction] {
        faction_rivals(*self)
    }

    /// Get the reputation tier for a given reputation value.
    pub fn reputation_tier(rep: i32) -> ReputationTier {
        match rep {
            r if r <= -75 => ReputationTier::Hostile,
            r if r <= -25 => ReputationTier::Unfriendly,
            r if r <= 25 => ReputationTier::Neutral,
            r if r <= 75 => ReputationTier::Friendly,
            _ => ReputationTier::Allied,
        }
    }
}

// ── Reputation Tier ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReputationTier {
    Hostile,     // attack on sight
    Unfriendly,  // may attack, bad prices
    Neutral,     // normal
    Friendly,    // good prices, help in battle
    Allied,      // best prices, reinforcements, unique gear
}

impl ReputationTier {
    /// Create a ReputationTier from a raw reputation value.
    pub fn from_rep(rep: i32) -> Self {
        Faction::reputation_tier(rep)
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Hostile => "Hostile",
            Self::Unfriendly => "Unfriendly",
            Self::Neutral => "Neutral",
            Self::Friendly => "Friendly",
            Self::Allied => "Allied",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Self::Hostile => Color::Red,
            Self::Unfriendly => Color::LightRed,
            Self::Neutral => Color::Gray,
            Self::Friendly => Color::LightGreen,
            Self::Allied => Color::Green,
        }
    }
}

// ── Faction Info ────────────────────────────────────────────────────────────

pub struct FactionInfo {
    pub faction: Faction,
    pub name: &'static str,
    pub short_name: &'static str,
    pub description: &'static str,
    pub color: Color,
    pub icon: char,
    pub base_hostility: i32,
    pub controls_sectors: (u32, u32),
}

pub const FACTIONS: &[FactionInfo] = &[
    FactionInfo {
        faction: Faction::PirateClan,
        name: "Pirate Clans",
        short_name: "PIR",
        description: "Raiders and scavengers. Hostile until you prove useful.",
        color: Color::Red,
        icon: '☠',
        base_hostility: -30,
        controls_sectors: (1, 15),
    },
    FactionInfo {
        faction: Faction::TradeGuild,
        name: "Trade Guild",
        short_name: "TRD",
        description: "Merchants and brokers. Neutral, driven by profit.",
        color: Color::Yellow,
        icon: '₿',
        base_hostility: 10,
        controls_sectors: (5, 25),
    },
    FactionInfo {
        faction: Faction::MilitaryCorp,
        name: "Military Corp",
        short_name: "MIL",
        description: "Government forces. Powerful and suspicious of outsiders.",
        color: Color::Blue,
        icon: '⚔',
        base_hostility: -10,
        controls_sectors: (15, 35),
    },
    FactionInfo {
        faction: Faction::AlienCollective,
        name: "Alien Collective",
        short_name: "ALN",
        description: "Ancient alien civilization. Enigmatic, advanced technology.",
        color: Color::Green,
        icon: '◈',
        base_hostility: -50,
        controls_sectors: (25, 50),
    },
    FactionInfo {
        faction: Faction::RebelAlliance,
        name: "Rebel Alliance",
        short_name: "RBL",
        description: "Freedom fighters opposing Military Corp. Idealistic, resourceful.",
        color: Color::Cyan,
        icon: '★',
        base_hostility: 20,
        controls_sectors: (10, 40),
    },
];

const INDEPENDENT_INFO: FactionInfo = FactionInfo {
    faction: Faction::Independent,
    name: "Independent",
    short_name: "IND",
    description: "Unaffiliated ships and stations.",
    color: Color::White,
    icon: '◊',
    base_hostility: 0,
    controls_sectors: (0, 0),
};

// ── Faction Reputation (serializable wrapper) ───────────────────────────────

/// Serializable reputation tracker. Uses String keys for JSON compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionReputation {
    pub standings: HashMap<String, i32>,
}

impl Default for FactionReputation {
    fn default() -> Self {
        let mut standings = HashMap::new();
        for info in FACTIONS {
            standings.insert(info.faction.key().to_string(), info.base_hostility);
        }
        Self { standings }
    }
}

impl FactionReputation {
    /// Get reputation with a faction. Returns 0 for Independent/unknown.
    pub fn get(&self, faction: Faction) -> i32 {
        self.standings
            .get(faction.key())
            .copied()
            .unwrap_or(0)
    }

    /// Get the reputation tier for a faction.
    pub fn tier(&self, faction: Faction) -> ReputationTier {
        Faction::reputation_tier(self.get(faction))
    }

    /// Change reputation with a faction, clamped to [-100, 100].
    /// Also applies rival penalty: rivals lose half the gained amount.
    /// Returns a vec of (faction_key, old_rep, new_rep) for all changes made.
    pub fn change(&mut self, faction: Faction, amount: i32) -> Vec<(String, i32, i32)> {
        let mut changes = Vec::new();

        if faction == Faction::Independent {
            return changes;
        }

        // Apply direct change
        let old = self.get(faction);
        let new = (old + amount).clamp(-100, 100);
        self.standings.insert(faction.key().to_string(), new);
        changes.push((faction.key().to_string(), old, new));

        // Apply rival penalty (only when gaining rep, i.e., amount > 0)
        if amount > 0 {
            let penalty = -((amount as f32 * ReputationChange::RIVAL_PENALTY_RATIO) as i32).max(1);
            for &rival in faction_rivals(faction) {
                let rival_old = self.get(rival);
                let rival_new = (rival_old + penalty).clamp(-100, 100);
                self.standings.insert(rival.key().to_string(), rival_new);
                changes.push((rival.key().to_string(), rival_old, rival_new));
            }
        }

        changes
    }

    /// Alias for `change` — used by scene code.
    pub fn modify(&mut self, faction: Faction, amount: i32) -> Vec<(String, i32, i32)> {
        self.change(faction, amount)
    }

    /// Check if a faction is hostile to the player.
    pub fn is_hostile(&self, faction: Faction) -> bool {
        matches!(self.tier(faction), ReputationTier::Hostile)
    }

    /// Price modifier based on reputation with a faction.
    /// Allied: 0.7 (30% discount), Hostile: 1.5 (50% markup).
    pub fn price_modifier(&self, faction: Faction) -> f32 {
        price_modifier(self.get(faction))
    }
}

// ── Faction Mission ─────────────────────────────────────────────────────────

/// A mission offered by a faction. Completing it grants reputation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionMission {
    pub faction: Faction,
    pub description: String,
    /// Sectors remaining to complete the mission (decremented each sector).
    pub sectors_remaining: u32,
    /// Credits reward on completion.
    pub reward_credits: u64,
    /// Reputation reward on completion.
    pub rep_reward: i32,
}

impl FactionMission {
    /// Tick one sector off the mission timer. Returns true if mission completed.
    pub fn tick_sector(&mut self) -> bool {
        if self.sectors_remaining > 0 {
            self.sectors_remaining -= 1;
        }
        self.sectors_remaining == 0
    }

    /// Check if the mission is complete (sectors exhausted).
    pub fn is_complete(&self) -> bool {
        self.sectors_remaining == 0
    }
}

// ── Faction Rivalries ───────────────────────────────────────────────────────

/// Get the rival factions for a given faction.
pub fn faction_rivals(f: Faction) -> &'static [Faction] {
    match f {
        Faction::PirateClan => &[Faction::MilitaryCorp, Faction::TradeGuild],
        Faction::MilitaryCorp => &[Faction::PirateClan, Faction::RebelAlliance],
        Faction::RebelAlliance => &[Faction::MilitaryCorp],
        Faction::TradeGuild => &[Faction::PirateClan],
        Faction::AlienCollective => &[], // neutral to all, hostile to aggressors
        Faction::Independent => &[],
    }
}

// ── Reputation Change Reasons ───────────────────────────────────────────────

/// Standard reputation change amounts for common actions.
pub struct ReputationChange;

impl ReputationChange {
    pub const KILL_SHIP: i32 = -15;
    pub const KILL_BOSS: i32 = -30;
    pub const COMPLETE_MISSION: i32 = 20;
    pub const TRADE: i32 = 5;
    pub const HELP_EVENT: i32 = 10;
    pub const RAID_PLANET: i32 = -25;
    pub const RIVAL_ALLIANCE_PENALTY: i32 = -10;

    /// Rival penalty ratio: when you gain rep with a faction, rivals lose this fraction.
    pub const RIVAL_PENALTY_RATIO: f32 = 0.5;
}

// ── Sector Faction Determination ────────────────────────────────────────────

/// Determine the dominant faction for a given sector.
/// Uses the faction whose `controls_sectors` range contains the sector.
/// If multiple factions overlap, picks the one whose range center is closest.
/// Falls back to Independent for sectors outside all ranges.
pub fn sector_dominant_faction(sector: u32) -> Faction {
    let mut best: Option<(Faction, u32)> = None;

    for info in FACTIONS {
        let (lo, hi) = info.controls_sectors;
        if sector >= lo && sector <= hi {
            let center = (lo + hi) / 2;
            let dist = sector.abs_diff(center);
            match best {
                None => best = Some((info.faction, dist)),
                Some((_, best_dist)) if dist < best_dist => {
                    best = Some((info.faction, dist));
                }
                _ => {}
            }
        }
    }

    best.map(|(f, _)| f).unwrap_or(Faction::Independent)
}

/// Determine the faction for an enemy encounter in a given sector.
/// 70% chance of the dominant faction, 30% chance of a random other faction.
/// Uses deterministic pseudo-random based on sector + seed.
pub fn encounter_faction(sector: u32, encounter_seed: u32) -> Faction {
    let dominant = sector_dominant_faction(sector);

    let mut h = sector.wrapping_mul(2654435761).wrapping_add(encounter_seed.wrapping_mul(40503));
    h ^= h >> 16;
    let roll = h.wrapping_mul(0x45d9f3b) % 100;

    if roll < 70 {
        dominant
    } else {
        let other_factions: Vec<Faction> = Faction::TRACKABLE
            .iter()
            .copied()
            .filter(|f| *f != dominant)
            .collect();
        if other_factions.is_empty() {
            return dominant;
        }
        let idx = (h.wrapping_mul(0x9e3779b9) % other_factions.len() as u32) as usize;
        other_factions[idx]
    }
}

/// Price modifier based on reputation with a faction.
/// Allied: 0.7 (30% discount), Hostile: 1.5 (50% markup).
pub fn price_modifier(reputation: i32) -> f32 {
    match Faction::reputation_tier(reputation) {
        ReputationTier::Hostile => 1.5,
        ReputationTier::Unfriendly => 1.25,
        ReputationTier::Neutral => 1.0,
        ReputationTier::Friendly => 0.85,
        ReputationTier::Allied => 0.7,
    }
}

/// Whether a faction is hostile to the player at the given reputation.
pub fn is_hostile(reputation: i32) -> bool {
    matches!(
        Faction::reputation_tier(reputation),
        ReputationTier::Hostile
    )
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Faction basics ─────────────────────────────────────────────

    #[test]
    fn all_factions_have_info() {
        for faction in Faction::ALL {
            let info = faction.info();
            assert_eq!(info.faction, faction);
            assert!(!info.name.is_empty());
            assert!(!info.short_name.is_empty());
            assert!(info.short_name.len() <= 3);
        }
    }

    #[test]
    fn faction_key_roundtrip() {
        for faction in Faction::ALL {
            let key = faction.key();
            let parsed = Faction::from_key(key);
            assert_eq!(parsed, Some(faction), "roundtrip failed for {:?}", faction);
        }
    }

    #[test]
    fn from_key_invalid() {
        assert_eq!(Faction::from_key("NotAFaction"), None);
        assert_eq!(Faction::from_key(""), None);
    }

    // ── Reputation tiers ───────────────────────────────────────────

    #[test]
    fn reputation_tier_boundaries() {
        assert_eq!(Faction::reputation_tier(-100), ReputationTier::Hostile);
        assert_eq!(Faction::reputation_tier(-75), ReputationTier::Hostile);
        assert_eq!(Faction::reputation_tier(-74), ReputationTier::Unfriendly);
        assert_eq!(Faction::reputation_tier(-25), ReputationTier::Unfriendly);
        assert_eq!(Faction::reputation_tier(-24), ReputationTier::Neutral);
        assert_eq!(Faction::reputation_tier(0), ReputationTier::Neutral);
        assert_eq!(Faction::reputation_tier(25), ReputationTier::Neutral);
        assert_eq!(Faction::reputation_tier(26), ReputationTier::Friendly);
        assert_eq!(Faction::reputation_tier(75), ReputationTier::Friendly);
        assert_eq!(Faction::reputation_tier(76), ReputationTier::Allied);
        assert_eq!(Faction::reputation_tier(100), ReputationTier::Allied);
    }

    #[test]
    fn reputation_tier_names() {
        assert_eq!(ReputationTier::Hostile.name(), "Hostile");
        assert_eq!(ReputationTier::Allied.name(), "Allied");
    }

    // ── FactionReputation ──────────────────────────────────────────

    #[test]
    fn default_reputation_has_all_trackable() {
        let rep = FactionReputation::default();
        for faction in Faction::TRACKABLE {
            assert!(
                rep.standings.contains_key(faction.key()),
                "missing {:?}",
                faction
            );
        }
        assert!(!rep.standings.contains_key("Independent"));
    }

    #[test]
    fn default_reputation_matches_base_hostility() {
        let rep = FactionReputation::default();
        for info in FACTIONS {
            let val = rep.get(info.faction);
            assert_eq!(
                val, info.base_hostility,
                "mismatch for {:?}",
                info.faction
            );
        }
    }

    #[test]
    fn change_reputation_clamps() {
        let mut rep = FactionReputation::default();
        // Slam to max
        rep.change(Faction::TradeGuild, 200);
        assert_eq!(rep.get(Faction::TradeGuild), 100);
        // Slam to min
        rep.change(Faction::TradeGuild, -300);
        assert_eq!(rep.get(Faction::TradeGuild), -100);
    }

    #[test]
    fn change_reputation_applies_rival_penalty_on_gain() {
        let mut rep = FactionReputation::default();
        let pirate_before = rep.get(Faction::PirateClan);
        // Gain rep with TradeGuild — pirates are rivals of trade guild
        rep.change(Faction::TradeGuild, 20);
        let pirate_after = rep.get(Faction::PirateClan);
        // Pirate should lose half of 20 = 10
        assert!(
            pirate_after < pirate_before,
            "pirate rep should decrease: {} -> {}",
            pirate_before,
            pirate_after
        );
    }

    #[test]
    fn change_reputation_no_rival_penalty_on_loss() {
        let mut rep = FactionReputation::default();
        let pirate_before = rep.get(Faction::PirateClan);
        // Lose rep with TradeGuild — should NOT affect pirates
        rep.change(Faction::TradeGuild, -20);
        let pirate_after = rep.get(Faction::PirateClan);
        assert_eq!(pirate_before, pirate_after);
    }

    #[test]
    fn change_reputation_independent_is_noop() {
        let mut rep = FactionReputation::default();
        let changes = rep.change(Faction::Independent, 50);
        assert!(changes.is_empty());
    }

    #[test]
    fn is_hostile_check() {
        let mut rep = FactionReputation::default();
        // AlienCollective starts at -50 (Unfriendly, not Hostile)
        assert!(!rep.is_hostile(Faction::AlienCollective));
        // Push to hostile
        rep.change(Faction::AlienCollective, -30);
        assert!(rep.is_hostile(Faction::AlienCollective));
    }

    #[test]
    fn price_modifier_by_tier() {
        let mut rep = FactionReputation::default();
        // TradeGuild starts at 10 (Neutral)
        let neutral_price = rep.price_modifier(Faction::TradeGuild);
        assert!((neutral_price - 1.0).abs() < f32::EPSILON);

        // Push to allied
        rep.change(Faction::TradeGuild, 80);
        let allied_price = rep.price_modifier(Faction::TradeGuild);
        assert!((allied_price - 0.7).abs() < f32::EPSILON);
    }

    // ── Rivalries ──────────────────────────────────────────────────

    #[test]
    fn pirate_rivals_include_military() {
        let rivals = faction_rivals(Faction::PirateClan);
        assert!(rivals.contains(&Faction::MilitaryCorp));
        assert!(rivals.contains(&Faction::TradeGuild));
    }

    #[test]
    fn military_rivals_include_pirates_and_rebels() {
        let rivals = faction_rivals(Faction::MilitaryCorp);
        assert!(rivals.contains(&Faction::PirateClan));
        assert!(rivals.contains(&Faction::RebelAlliance));
    }

    #[test]
    fn alien_has_no_rivals() {
        assert!(faction_rivals(Faction::AlienCollective).is_empty());
    }

    #[test]
    fn independent_has_no_rivals() {
        assert!(faction_rivals(Faction::Independent).is_empty());
    }

    #[test]
    fn rivalries_are_symmetric() {
        for faction in Faction::TRACKABLE {
            for &rival in faction_rivals(faction) {
                let reverse = faction_rivals(rival);
                assert!(
                    reverse.contains(&faction),
                    "{:?} lists {:?} as rival, but {:?} doesn't list {:?}",
                    faction, rival, rival, faction
                );
            }
        }
    }

    // ── Sector dominance ───────────────────────────────────────────

    #[test]
    fn sector_1_is_pirate_territory() {
        assert_eq!(sector_dominant_faction(1), Faction::PirateClan);
    }

    #[test]
    fn sector_5_overlap_resolved() {
        let faction = sector_dominant_faction(5);
        assert_eq!(faction, Faction::PirateClan);
    }

    #[test]
    fn high_sector_alien() {
        let faction = sector_dominant_faction(45);
        assert_eq!(faction, Faction::AlienCollective);
    }

    #[test]
    fn sector_beyond_all_ranges() {
        assert_eq!(sector_dominant_faction(100), Faction::Independent);
    }

    #[test]
    fn sector_0_is_independent() {
        assert_eq!(sector_dominant_faction(0), Faction::Independent);
    }

    // ── Encounter faction ──────────────────────────────────────────

    #[test]
    fn encounter_faction_deterministic() {
        let f1 = encounter_faction(10, 42);
        let f2 = encounter_faction(10, 42);
        assert_eq!(f1, f2);
    }

    #[test]
    fn encounter_faction_varies_with_seed() {
        let mut seen = std::collections::HashSet::new();
        for seed in 0..100 {
            seen.insert(encounter_faction(10, seed));
        }
        assert!(seen.len() > 1);
    }

    #[test]
    fn encounter_faction_mostly_dominant() {
        let dominant = sector_dominant_faction(10);
        let mut dominant_count = 0u32;
        let total = 1000;
        for seed in 0..total {
            if encounter_faction(10, seed) == dominant {
                dominant_count += 1;
            }
        }
        let ratio = dominant_count as f32 / total as f32;
        assert!(
            ratio > 0.55 && ratio < 0.85,
            "expected ~70% dominant, got {:.1}%",
            ratio * 100.0,
        );
    }

    // ── Price modifier ─────────────────────────────────────────────

    #[test]
    fn price_modifier_hostile_is_expensive() {
        assert!((price_modifier(-100) - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn price_modifier_allied_is_cheap() {
        assert!((price_modifier(100) - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn price_modifier_neutral_is_normal() {
        assert!((price_modifier(0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn price_modifier_monotonic() {
        let hostile = price_modifier(-100);
        let unfriendly = price_modifier(-50);
        let neutral = price_modifier(0);
        let friendly = price_modifier(50);
        let allied = price_modifier(100);
        assert!(hostile >= unfriendly);
        assert!(unfriendly >= neutral);
        assert!(neutral >= friendly);
        assert!(friendly >= allied);
    }

    // ── Hostility check ────────────────────────────────────────────

    #[test]
    fn is_hostile_at_minus_75() {
        assert!(is_hostile(-75));
        assert!(is_hostile(-100));
    }

    #[test]
    fn not_hostile_at_minus_74() {
        assert!(!is_hostile(-74));
        assert!(!is_hostile(0));
        assert!(!is_hostile(100));
    }

    // ── Faction info validation ────────────────────────────────────

    #[test]
    fn faction_controls_sectors_valid() {
        for info in FACTIONS {
            assert!(
                info.controls_sectors.0 <= info.controls_sectors.1,
                "{:?} has inverted sector range",
                info.faction
            );
        }
    }

    #[test]
    fn faction_base_hostility_in_range() {
        for info in FACTIONS {
            assert!(
                info.base_hostility >= -100 && info.base_hostility <= 100,
                "{:?} base_hostility {} out of range",
                info.faction,
                info.base_hostility
            );
        }
    }

    // ── Mission tests ──────────────────────────────────────────────

    #[test]
    fn mission_tick_sector_countdown() {
        let mut mission = FactionMission {
            faction: Faction::PirateClan,
            description: "Pirate contract".into(),
            sectors_remaining: 3,
            reward_credits: 100,
            rep_reward: 20,
        };
        assert!(!mission.is_complete());
        assert!(!mission.tick_sector()); // 2 remaining
        assert!(!mission.tick_sector()); // 1 remaining
        assert!(mission.tick_sector());  // 0 — complete!
        assert!(mission.is_complete());
    }

    #[test]
    fn mission_already_complete() {
        let mission = FactionMission {
            faction: Faction::MilitaryCorp,
            description: "Military contract".into(),
            sectors_remaining: 0,
            reward_credits: 200,
            rep_reward: 15,
        };
        assert!(mission.is_complete());
    }

    // ── Reputation change with rival cascading ─────────────────────

    #[test]
    fn gaining_military_rep_hurts_pirates_and_rebels() {
        let mut rep = FactionReputation::default();
        let pirate_before = rep.get(Faction::PirateClan);
        let rebel_before = rep.get(Faction::RebelAlliance);
        rep.change(Faction::MilitaryCorp, 20);
        assert!(rep.get(Faction::PirateClan) < pirate_before);
        assert!(rep.get(Faction::RebelAlliance) < rebel_before);
    }

    #[test]
    fn change_returns_all_affected_factions() {
        let mut rep = FactionReputation::default();
        let changes = rep.change(Faction::MilitaryCorp, 20);
        // Should include MilitaryCorp + 2 rivals (PirateClan, RebelAlliance)
        assert_eq!(changes.len(), 3);
        let keys: Vec<&str> = changes.iter().map(|(k, _, _)| k.as_str()).collect();
        assert!(keys.contains(&"MilitaryCorp"));
        assert!(keys.contains(&"PirateClan"));
        assert!(keys.contains(&"RebelAlliance"));
    }
}
