/// Trade economy system — trade goods, sector markets, cargo management, and contraband risk.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::engine::factions::Faction;

// ── Trade Goods ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TradeGood {
    Ore,
    Food,
    Tech,
    Weapons,
    Luxuries,
    MedSupplies,
    Contraband,
    Artifacts,
}

impl TradeGood {
    pub const ALL: [TradeGood; 8] = [
        Self::Ore,
        Self::Food,
        Self::Tech,
        Self::Weapons,
        Self::Luxuries,
        Self::MedSupplies,
        Self::Contraband,
        Self::Artifacts,
    ];

    pub fn base_price(&self) -> u64 {
        match self {
            Self::Ore => 20,
            Self::Food => 30,
            Self::Tech => 80,
            Self::Weapons => 120,
            Self::Luxuries => 150,
            Self::MedSupplies => 60,
            Self::Contraband => 200,
            Self::Artifacts => 500,
        }
    }

    pub fn icon(&self) -> char {
        match self {
            Self::Ore => '▣',
            Self::Food => '🌾',
            Self::Tech => '⚙',
            Self::Weapons => '⚔',
            Self::Luxuries => '💎',
            Self::MedSupplies => '✚',
            Self::Contraband => '☠',
            Self::Artifacts => '◈',
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Ore => "Ore",
            Self::Food => "Food",
            Self::Tech => "Tech",
            Self::Weapons => "Weapons",
            Self::Luxuries => "Luxuries",
            Self::MedSupplies => "Med Supplies",
            Self::Contraband => "Contraband",
            Self::Artifacts => "Artifacts",
        }
    }

    pub fn is_illegal(&self) -> bool {
        matches!(self, Self::Contraband | Self::Weapons)
    }

    /// Parse from string key (used for serde-compatible cargo storage).
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "Ore" => Some(Self::Ore),
            "Food" => Some(Self::Food),
            "Tech" => Some(Self::Tech),
            "Weapons" => Some(Self::Weapons),
            "Luxuries" => Some(Self::Luxuries),
            "MedSupplies" => Some(Self::MedSupplies),
            "Contraband" => Some(Self::Contraband),
            "Artifacts" => Some(Self::Artifacts),
            _ => None,
        }
    }

    /// String key for HashMap storage.
    pub fn key(&self) -> &'static str {
        match self {
            Self::Ore => "Ore",
            Self::Food => "Food",
            Self::Tech => "Tech",
            Self::Weapons => "Weapons",
            Self::Luxuries => "Luxuries",
            Self::MedSupplies => "MedSupplies",
            Self::Contraband => "Contraband",
            Self::Artifacts => "Artifacts",
        }
    }
}

// ── Supply & Demand ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Supply {
    Surplus,
    Normal,
    Scarce,
}

impl Supply {
    pub fn modifier(&self) -> f64 {
        match self {
            Self::Surplus => 0.7,
            Self::Normal => 1.0,
            Self::Scarce => 1.4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Demand {
    High,
    Normal,
    Low,
}

impl Demand {
    pub fn modifier(&self) -> f64 {
        match self {
            Self::High => 1.3,
            Self::Normal => 1.0,
            Self::Low => 0.75,
        }
    }
}

// ── Market Price ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketPrice {
    pub buy_price: u64,
    pub sell_price: u64,
    pub supply: Supply,
    pub demand: Demand,
}

// ── Sector Market ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorMarket {
    pub prices: HashMap<TradeGood, MarketPrice>,
}

impl SectorMarket {
    /// Get the buy price for a good (what the player pays). Returns None if not traded here.
    pub fn buy_price(&self, good: TradeGood) -> Option<u64> {
        self.prices.get(&good).map(|p| p.buy_price)
    }

    /// Get the sell price for a good (what the player receives). Returns None if not traded here.
    pub fn sell_price(&self, good: TradeGood) -> Option<u64> {
        self.prices.get(&good).map(|p| p.sell_price)
    }

    /// Check if a good is available for purchase in this market.
    pub fn is_available(&self, good: TradeGood) -> bool {
        self.prices.contains_key(&good)
    }
}

// ── Trade Record ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub good: TradeGood,
    pub quantity: u32,
    pub price_per_unit: u64,
    pub was_buy: bool,
    pub sector: u32,
}

impl TradeRecord {
    pub fn total_cost(&self) -> u64 {
        self.price_per_unit * self.quantity as u64
    }
}

// ── Market Generation ───────────────────────────────────────────────────────

/// Deterministic pseudo-random float in [0, 1) from a seed.
fn pseudo_rand(seed: u32) -> f64 {
    // Simple hash-based PRNG — deterministic for same sector+good
    let mut h = seed;
    h ^= h >> 13;
    h = h.wrapping_mul(0x45d9f3b);
    h ^= h >> 16;
    h = h.wrapping_mul(0x45d9f3b);
    h ^= h >> 16;
    (h % 10000) as f64 / 10000.0
}

/// Get faction-specific supply/demand profile for a trade good.
fn faction_supply_demand(faction: &Faction, good: TradeGood) -> (Supply, Demand) {
    match faction {
        Faction::PirateClan => match good {
            TradeGood::Weapons => (Supply::Surplus, Demand::Low),
            TradeGood::Contraband => (Supply::Surplus, Demand::Normal),
            TradeGood::Food => (Supply::Scarce, Demand::High),
            TradeGood::MedSupplies => (Supply::Scarce, Demand::High),
            TradeGood::Ore => (Supply::Surplus, Demand::Low),
            TradeGood::Tech => (Supply::Scarce, Demand::Normal),
            TradeGood::Luxuries => (Supply::Normal, Demand::High),
            TradeGood::Artifacts => (Supply::Scarce, Demand::Normal),
        },
        Faction::TradeGuild => match good {
            TradeGood::Luxuries => (Supply::Normal, Demand::High),
            TradeGood::Tech => (Supply::Normal, Demand::Normal),
            TradeGood::Food => (Supply::Normal, Demand::Normal),
            TradeGood::Ore => (Supply::Normal, Demand::Normal),
            TradeGood::Weapons => (Supply::Normal, Demand::Low),
            TradeGood::MedSupplies => (Supply::Normal, Demand::Normal),
            TradeGood::Contraband => (Supply::Scarce, Demand::Low),
            TradeGood::Artifacts => (Supply::Scarce, Demand::High),
        },
        Faction::MilitaryCorp => match good {
            TradeGood::Tech => (Supply::Surplus, Demand::Normal),
            TradeGood::Weapons => (Supply::Surplus, Demand::Low),
            TradeGood::MedSupplies => (Supply::Normal, Demand::Normal),
            TradeGood::Food => (Supply::Normal, Demand::Normal),
            TradeGood::Ore => (Supply::Normal, Demand::Low),
            TradeGood::Luxuries => (Supply::Scarce, Demand::Normal),
            TradeGood::Contraband => (Supply::Scarce, Demand::Low), // banned, won't appear
            TradeGood::Artifacts => (Supply::Scarce, Demand::High),
        },
        Faction::AlienCollective => match good {
            TradeGood::Artifacts => (Supply::Surplus, Demand::Low),
            TradeGood::Tech => (Supply::Normal, Demand::High),
            TradeGood::Ore => (Supply::Scarce, Demand::High),
            TradeGood::Food => (Supply::Scarce, Demand::High),
            TradeGood::Weapons => (Supply::Scarce, Demand::Normal),
            TradeGood::Luxuries => (Supply::Normal, Demand::Normal),
            TradeGood::MedSupplies => (Supply::Scarce, Demand::Normal),
            TradeGood::Contraband => (Supply::Scarce, Demand::Low),
        },
        Faction::RebelAlliance => match good {
            TradeGood::MedSupplies => (Supply::Surplus, Demand::Normal),
            TradeGood::Tech => (Supply::Scarce, Demand::High),
            TradeGood::Weapons => (Supply::Normal, Demand::High),
            TradeGood::Food => (Supply::Normal, Demand::Normal),
            TradeGood::Ore => (Supply::Normal, Demand::Normal),
            TradeGood::Luxuries => (Supply::Scarce, Demand::Low),
            TradeGood::Contraband => (Supply::Normal, Demand::Normal),
            TradeGood::Artifacts => (Supply::Scarce, Demand::High),
        },
        Faction::Independent => {
            // Balanced — everything normal
            (Supply::Normal, Demand::Normal)
        }
    }
}

/// Faction modifier on price — some factions tax more or less for certain goods.
fn faction_price_modifier(faction: &Faction, good: TradeGood) -> f64 {
    match faction {
        Faction::PirateClan => match good {
            TradeGood::Weapons | TradeGood::Contraband => 0.75,
            TradeGood::Food | TradeGood::MedSupplies => 1.3,
            _ => 1.0,
        },
        Faction::TradeGuild => match good {
            TradeGood::Luxuries => 1.15,
            _ => 0.95, // Trade Guild has slightly better prices overall
        },
        Faction::MilitaryCorp => match good {
            TradeGood::Tech | TradeGood::Weapons => 0.8,
            TradeGood::Contraband => 2.0, // huge fine if caught
            _ => 1.05,
        },
        Faction::AlienCollective => match good {
            TradeGood::Artifacts => 0.6,
            _ => 1.25, // everything else expensive
        },
        Faction::RebelAlliance => match good {
            TradeGood::MedSupplies => 0.7,
            TradeGood::Tech => 1.3,
            _ => 1.0,
        },
        Faction::Independent => 1.0,
    }
}

/// Generate a market for a specific sector and its dominant faction.
/// Prices are deterministic for a given sector (reproducible from sector seed).
pub fn generate_market(sector: u32, faction: &Faction) -> SectorMarket {
    let mut prices = HashMap::new();

    for good in TradeGood::ALL {
        // Military sectors don't sell contraband at all
        if *faction == Faction::MilitaryCorp && good == TradeGood::Contraband {
            continue;
        }

        let (supply, demand) = faction_supply_demand(faction, good);
        let faction_mod = faction_price_modifier(faction, good);

        // Deterministic random per sector+good combination
        let good_seed = sector
            .wrapping_mul(2654435761)
            .wrapping_add(good.base_price() as u32 * 7919);
        let rand_factor = 0.85 + pseudo_rand(good_seed) * 0.30; // [0.85, 1.15]

        let raw_price =
            good.base_price() as f64 * supply.modifier() * demand.modifier() * faction_mod * rand_factor;

        let buy_price = (raw_price).max(1.0) as u64;
        // Sell price is 80-90% of buy price (market spread)
        let spread_seed = good_seed.wrapping_add(12345);
        let spread = 0.80 + pseudo_rand(spread_seed) * 0.10; // [0.80, 0.90]
        let sell_price = ((raw_price * spread).max(1.0) as u64).min(buy_price.saturating_sub(1)).max(1);

        prices.insert(good, MarketPrice {
            buy_price,
            sell_price,
            supply,
            demand,
        });
    }

    SectorMarket { prices }
}

// ── Contraband Detection ────────────────────────────────────────────────────

/// Result of a contraband scan when entering a sector.
#[derive(Debug, Clone)]
pub struct ContrabandResult {
    pub detected: bool,
    pub good: TradeGood,
    pub quantity: u32,
    pub fine: u64,
}

/// Check for contraband when entering a sector controlled by a law-enforcing faction.
/// Returns a list of detected contraband items.
/// Detection chance: 30% per illegal good type carried.
pub fn check_contraband(
    sector: u32,
    faction: &Faction,
    cargo: &HashMap<String, u32>,
    encounter_seed: u32,
) -> Vec<ContrabandResult> {
    // Only Military and Trade Guild enforce contraband laws
    let enforces = matches!(faction, Faction::MilitaryCorp | Faction::TradeGuild);
    if !enforces {
        return Vec::new();
    }

    let mut results = Vec::new();

    for good in TradeGood::ALL {
        if !good.is_illegal() {
            continue;
        }

        let quantity = cargo.get(good.key()).copied().unwrap_or(0);
        if quantity == 0 {
            continue;
        }

        // 30% detection chance, deterministic from seed
        let detect_seed = sector
            .wrapping_mul(0x9e3779b9)
            .wrapping_add(encounter_seed)
            .wrapping_add(good.base_price() as u32 * 31);
        let roll = pseudo_rand(detect_seed);

        if roll < 0.30 {
            // Fine = 50% of goods value at base price
            let fine = good.base_price() * quantity as u64 / 2;
            results.push(ContrabandResult {
                detected: true,
                good,
                quantity,
                fine,
            });
        }
    }

    results
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── TradeGood basics ───────────────────────────────────────────

    #[test]
    fn all_goods_have_nonzero_base_price() {
        for good in TradeGood::ALL {
            assert!(good.base_price() > 0, "{:?} has zero base price", good);
        }
    }

    #[test]
    fn all_goods_have_name() {
        for good in TradeGood::ALL {
            assert!(!good.name().is_empty(), "{:?} has empty name", good);
        }
    }

    #[test]
    fn trade_good_key_roundtrip() {
        for good in TradeGood::ALL {
            let key = good.key();
            let parsed = TradeGood::from_key(key);
            assert_eq!(parsed, Some(good), "roundtrip failed for {:?}", good);
        }
    }

    #[test]
    fn from_key_invalid() {
        assert_eq!(TradeGood::from_key("NotAGood"), None);
        assert_eq!(TradeGood::from_key(""), None);
    }

    #[test]
    fn illegal_goods_identified() {
        assert!(TradeGood::Contraband.is_illegal());
        assert!(TradeGood::Weapons.is_illegal());
        assert!(!TradeGood::Food.is_illegal());
        assert!(!TradeGood::Ore.is_illegal());
        assert!(!TradeGood::Tech.is_illegal());
        assert!(!TradeGood::MedSupplies.is_illegal());
    }

    // ── Supply/Demand modifiers ────────────────────────────────────

    #[test]
    fn supply_modifiers_correct() {
        assert!(Supply::Surplus.modifier() < 1.0);
        assert!((Supply::Normal.modifier() - 1.0).abs() < f64::EPSILON);
        assert!(Supply::Scarce.modifier() > 1.0);
    }

    #[test]
    fn demand_modifiers_correct() {
        assert!(Demand::High.modifier() > 1.0);
        assert!((Demand::Normal.modifier() - 1.0).abs() < f64::EPSILON);
        assert!(Demand::Low.modifier() < 1.0);
    }

    // ── Market generation ──────────────────────────────────────────

    #[test]
    fn market_generation_deterministic() {
        let m1 = generate_market(10, &Faction::TradeGuild);
        let m2 = generate_market(10, &Faction::TradeGuild);
        for good in TradeGood::ALL {
            assert_eq!(
                m1.buy_price(good),
                m2.buy_price(good),
                "non-deterministic price for {:?}",
                good
            );
        }
    }

    #[test]
    fn market_buy_price_always_above_sell_price() {
        for faction in Faction::ALL {
            for sector in [1, 5, 10, 20, 30, 45] {
                let market = generate_market(sector, &faction);
                for (good, price) in &market.prices {
                    assert!(
                        price.buy_price > price.sell_price,
                        "sector {} {:?} {:?}: buy {} <= sell {}",
                        sector, faction, good, price.buy_price, price.sell_price
                    );
                }
            }
        }
    }

    #[test]
    fn market_prices_nonzero() {
        for faction in Faction::ALL {
            let market = generate_market(10, &faction);
            for (good, price) in &market.prices {
                assert!(price.buy_price > 0, "{:?} buy=0 in {:?}", good, faction);
                assert!(price.sell_price > 0, "{:?} sell=0 in {:?}", good, faction);
            }
        }
    }

    #[test]
    fn military_sector_no_contraband() {
        let market = generate_market(25, &Faction::MilitaryCorp);
        assert!(
            !market.is_available(TradeGood::Contraband),
            "Military sectors should not sell contraband"
        );
    }

    #[test]
    fn pirate_weapons_cheaper_than_military_food() {
        let pirate = generate_market(5, &Faction::PirateClan);
        let pirate_weapons = pirate.buy_price(TradeGood::Weapons).unwrap();
        let pirate_food = pirate.buy_price(TradeGood::Food).unwrap();
        // Pirates: weapons surplus+cheap modifier, food scarce+expensive modifier
        assert!(
            pirate_weapons < pirate_food * 3,
            "pirate weapons ({}) should be relatively cheap vs food ({})",
            pirate_weapons, pirate_food
        );
    }

    #[test]
    fn alien_artifacts_cheap() {
        let alien = generate_market(40, &Faction::AlienCollective);
        let independent = generate_market(40, &Faction::Independent);
        let alien_price = alien.buy_price(TradeGood::Artifacts).unwrap();
        let indie_price = independent.buy_price(TradeGood::Artifacts).unwrap();
        assert!(
            alien_price < indie_price,
            "alien artifacts ({}) should be cheaper than independent ({})",
            alien_price, indie_price
        );
    }

    #[test]
    fn different_sectors_different_prices() {
        let m1 = generate_market(5, &Faction::TradeGuild);
        let m2 = generate_market(20, &Faction::TradeGuild);
        // At least some goods should have different prices due to random component
        let mut any_different = false;
        for good in TradeGood::ALL {
            if m1.buy_price(good) != m2.buy_price(good) {
                any_different = true;
                break;
            }
        }
        assert!(any_different, "different sectors should have different prices");
    }

    // ── Trade Record ───────────────────────────────────────────────

    #[test]
    fn trade_record_total_cost() {
        let record = TradeRecord {
            good: TradeGood::Ore,
            quantity: 5,
            price_per_unit: 20,
            was_buy: true,
            sector: 1,
        };
        assert_eq!(record.total_cost(), 100);
    }

    // ── Contraband detection ───────────────────────────────────────

    #[test]
    fn contraband_not_checked_in_pirate_sectors() {
        let mut cargo = HashMap::new();
        cargo.insert("Contraband".to_string(), 10);
        let results = check_contraband(5, &Faction::PirateClan, &cargo, 42);
        assert!(results.is_empty(), "pirates don't check contraband");
    }

    #[test]
    fn contraband_not_checked_in_rebel_sectors() {
        let mut cargo = HashMap::new();
        cargo.insert("Contraband".to_string(), 10);
        let results = check_contraband(10, &Faction::RebelAlliance, &cargo, 42);
        assert!(results.is_empty(), "rebels don't check contraband");
    }

    #[test]
    fn contraband_checked_in_military_sectors() {
        let mut cargo = HashMap::new();
        cargo.insert("Contraband".to_string(), 10);
        // Try many seeds — with 30% chance, should detect at least once in 100 tries
        let mut detected_any = false;
        for seed in 0..100 {
            let results = check_contraband(25, &Faction::MilitaryCorp, &cargo, seed);
            if !results.is_empty() {
                detected_any = true;
                // Verify fine calculation
                let r = &results[0];
                assert_eq!(r.good, TradeGood::Contraband);
                assert_eq!(r.quantity, 10);
                assert_eq!(r.fine, TradeGood::Contraband.base_price() * 10 / 2);
                break;
            }
        }
        assert!(detected_any, "military should detect contraband sometimes");
    }

    #[test]
    fn contraband_checked_in_trade_guild_sectors() {
        let mut cargo = HashMap::new();
        cargo.insert("Weapons".to_string(), 5);
        let mut detected_any = false;
        for seed in 0..100 {
            let results = check_contraband(15, &Faction::TradeGuild, &cargo, seed);
            if !results.is_empty() {
                detected_any = true;
                break;
            }
        }
        assert!(detected_any, "trade guild should detect illegal weapons sometimes");
    }

    #[test]
    fn no_contraband_no_detection() {
        let cargo = HashMap::new();
        let results = check_contraband(25, &Faction::MilitaryCorp, &cargo, 42);
        assert!(results.is_empty());
    }

    #[test]
    fn legal_goods_never_detected() {
        let mut cargo = HashMap::new();
        cargo.insert("Food".to_string(), 100);
        cargo.insert("Ore".to_string(), 100);
        cargo.insert("Tech".to_string(), 100);
        for seed in 0..100 {
            let results = check_contraband(25, &Faction::MilitaryCorp, &cargo, seed);
            assert!(results.is_empty(), "legal goods should never trigger detection");
        }
    }

    #[test]
    fn detection_rate_approximately_30_percent() {
        let mut cargo = HashMap::new();
        cargo.insert("Contraband".to_string(), 1);
        let total = 1000;
        let mut detected = 0u32;
        for seed in 0..total {
            let results = check_contraband(25, &Faction::MilitaryCorp, &cargo, seed);
            if !results.is_empty() {
                detected += 1;
            }
        }
        let rate = detected as f32 / total as f32;
        assert!(
            rate > 0.20 && rate < 0.45,
            "expected ~30% detection, got {:.1}%",
            rate * 100.0
        );
    }

    // ── Profit calculation ─────────────────────────────────────────

    #[test]
    fn trade_profit_buy_low_sell_high() {
        // Buy artifacts from aliens (cheap), sell to military (expensive)
        let alien_market = generate_market(40, &Faction::AlienCollective);
        let military_market = generate_market(25, &Faction::MilitaryCorp);

        let buy_cost = alien_market.buy_price(TradeGood::Artifacts).unwrap();
        let sell_revenue = military_market.sell_price(TradeGood::Artifacts).unwrap();

        // Alien artifacts should be cheap enough to profit from
        assert!(
            sell_revenue > buy_cost,
            "should profit from alien->military artifact trade: buy={}, sell={}",
            buy_cost, sell_revenue
        );
    }

    #[test]
    fn market_spread_reasonable() {
        // Buy and sell in same market should always lose money (spread)
        for faction in Faction::ALL {
            let market = generate_market(10, &faction);
            for (good, price) in &market.prices {
                let spread = price.buy_price as f64 - price.sell_price as f64;
                let spread_pct = spread / price.buy_price as f64;
                assert!(
                    spread_pct >= 0.09 && spread_pct <= 0.21,
                    "{:?} {:?} spread {:.1}% out of expected 10-20% range (buy={}, sell={})",
                    faction, good, spread_pct * 100.0, price.buy_price, price.sell_price
                );
            }
        }
    }
}
