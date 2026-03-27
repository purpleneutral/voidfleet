/// Economy — cost calculations, unlock gates, dynamic loot, and catch-up mechanics.

use crate::engine::ship::ShipType;
use crate::state::GameState;

/// Cost to upgrade a tech tree from `current_level` to the next level.
/// Quadratic scaling: gets expensive fast.
pub fn tech_upgrade_cost(current_level: u8) -> u64 {
    50 * (current_level as u64 + 1).pow(2)
}

/// Cost to build a ship, factoring in fleet size (larger fleets pay more).
pub fn ship_build_cost(ship_type: ShipType, fleet_size: usize) -> u64 {
    let base = ship_type.cost();
    base + (base / 4 * fleet_size as u64)
}

/// Whether a ship type is unlocked at the given player level.
pub fn is_ship_unlocked(ship_type: ShipType, level: u32) -> bool {
    level >= ship_type.unlock_level()
}

// ── Dynamic loot system ─────────────────────────────────────

/// Reward from a battle, scaled by performance and progression.
#[derive(Debug, Clone)]
pub struct LootReward {
    pub scrap: u64,
    pub credits: u64,
    pub xp: u64,
    pub blueprint_chance: f32,
    pub artifact_chance: f32,
}

/// Calculate loot rewards based on game state and battle performance.
///
/// `battle_performance`: 0.0 (barely survived) to 1.0 (flawless victory).
/// Performance is typically calculated as: remaining_fleet_hp / max_fleet_hp.
pub fn calculate_loot(state: &GameState, battle_performance: f32) -> LootReward {
    let perf = battle_performance.clamp(0.0, 1.0);
    let sector = state.sector.max(1) as f32;

    // Base values scale with sector
    let base_scrap = 20.0 + sector * 8.0;
    let base_credits = 15.0 + sector * 5.0;
    let base_xp = 30.0 + sector * 10.0;

    // Performance multiplier: 0.5x to 1.5x (floor at 0.5 so you always get something)
    let perf_mult = 0.5 + perf;

    // Catch-up multiplier
    let catchup = catchup_multiplier(state);

    LootReward {
        scrap: (base_scrap * perf_mult * catchup) as u64,
        credits: (base_credits * perf_mult * catchup) as u64,
        xp: (base_xp * perf_mult * catchup) as u64,
        blueprint_chance: blueprint_drop_chance(state, perf),
        artifact_chance: artifact_drop_chance(state, perf),
    }
}

/// Blueprint drop chance: 5% base, +5% per performance tier, +2% if struggling.
fn blueprint_drop_chance(state: &GameState, performance: f32) -> f32 {
    let mut chance = 0.05 + performance * 0.05;
    if state.sector > state.highest_sector.saturating_sub(3) && state.deaths > 0 {
        chance += 0.02;
    }
    // Diminishing returns if player has many blueprints
    if state.blueprints > 10 {
        chance *= 0.7;
    }
    chance.clamp(0.01, 0.25)
}

/// Artifact drop chance: 2% base, higher with flawless performance.
fn artifact_drop_chance(state: &GameState, performance: f32) -> f32 {
    let mut chance = 0.02;
    if performance > 0.9 {
        chance += 0.05; // Flawless bonus
    }
    // Higher sectors = slightly better artifact chance
    chance += (state.sector as f32 * 0.001).min(0.05);
    chance.clamp(0.01, 0.15)
}

// ── Catch-up mechanics ──────────────────────────────────────

/// Overall catch-up loot multiplier.
/// Returns >1.0 if the player is struggling, 1.0 if on track.
fn catchup_multiplier(state: &GameState) -> f32 {
    let mut mult = 1.0_f32;

    // Died in last 3 sectors: 1.5x loot
    if state.deaths > 0 && state.sector <= state.highest_sector {
        let sectors_behind = state.highest_sector.saturating_sub(state.sector);
        if sectors_behind <= 3 {
            mult *= 1.5;
        }
    }

    // Fleet value low relative to sector (cheap fleet for the sector)
    let fleet_value: u64 = state.fleet.iter().map(|s| s.ship_type.cost()).sum();
    let expected_value = state.sector as u64 * 100;
    if fleet_value < expected_value / 2 {
        mult *= 1.25;
    }

    // Level deficit
    let expected_level = state.sector / 3 + 1;
    if state.level + 3 < expected_level {
        mult *= 1.2;
    }

    mult.min(2.5) // Cap at 2.5x to prevent runaway inflation
}

/// Whether to trigger a bonus "sale" event.
/// Returns true if the player hasn't had meaningful upgrades in a while.
pub fn should_trigger_sale(state: &GameState) -> bool {
    // Heuristic: if total tech level is very low relative to sector
    let total_tech = state.tech_lasers as u32
        + state.tech_shields as u32
        + state.tech_engines as u32
        + state.tech_beams as u32;
    let expected_tech = state.sector / 4; // ~1 upgrade per 4 sectors
    total_tech + 3 < expected_tech
}

/// Discount multiplier for sale events. 0.5 = 50% off.
pub fn sale_discount() -> f32 {
    0.5
}

/// Travel bonus scrap drops for struggling players.
/// Returns extra scrap per collectible pickup if the player is behind.
pub fn travel_bonus_scrap(state: &GameState) -> u64 {
    // Fleet value low relative to sector
    let fleet_value: u64 = state.fleet.iter().map(|s| s.ship_type.cost()).sum();
    let expected_value = state.sector as u64 * 100;

    if fleet_value < expected_value / 2 {
        // Bonus scales with how far behind they are
        let deficit_ratio = if expected_value > 0 {
            1.0 - (fleet_value as f32 / expected_value as f32)
        } else {
            0.0
        };
        (10.0 * deficit_ratio * (1.0 + state.sector as f32 / 20.0)) as u64
    } else {
        0
    }
}

// ── Cost balancing ──────────────────────────────────────────

/// Adjust an upgrade cost based on player progression.
/// Prevents both trivial progression (hoarding) and hard walls (struggling).
pub fn adjusted_upgrade_cost(base_cost: u64, state: &GameState) -> u64 {
    let mut cost = base_cost as f64;

    // Hoarding penalty: if sitting on lots of scrap, slight increase
    if state.scrap > 5000 {
        let excess = (state.scrap - 5000) as f64;
        // +1% per 1000 excess scrap, capped at +15%
        let penalty = (excess / 1000.0 * 0.01).min(0.15);
        cost *= 1.0 + penalty;
    }

    // Struggling discount: if low resources relative to sector
    let expected_scrap = state.sector as f64 * 50.0;
    if (state.scrap as f64) < expected_scrap * 0.3 {
        // Up to 20% discount when really struggling
        let deficit = 1.0 - (state.scrap as f64 / (expected_scrap * 0.3));
        let discount = (deficit * 0.2).min(0.2);
        cost *= 1.0 - discount;
    }

    // Never reduce below 50% of base cost
    let minimum = base_cost / 2;
    (cost as u64).max(minimum).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tech_upgrade_cost_scaling() {
        assert_eq!(tech_upgrade_cost(0), 50);
        assert_eq!(tech_upgrade_cost(1), 200);
        assert_eq!(tech_upgrade_cost(2), 450);
        assert_eq!(tech_upgrade_cost(5), 1800);
    }

    #[test]
    fn test_ship_build_cost_scales_with_fleet() {
        let solo = ship_build_cost(ShipType::Fighter, 0);
        let big_fleet = ship_build_cost(ShipType::Fighter, 8);
        assert!(big_fleet > solo);
        assert_eq!(solo, ShipType::Fighter.cost());
    }

    #[test]
    fn test_scout_always_unlocked() {
        assert!(is_ship_unlocked(ShipType::Scout, 1));
    }

    #[test]
    fn test_capital_locked_early() {
        assert!(!is_ship_unlocked(ShipType::Capital, 10));
        assert!(is_ship_unlocked(ShipType::Capital, 30));
    }

    #[test]
    fn test_scout_free() {
        assert_eq!(ship_build_cost(ShipType::Scout, 0), 0);
        // Free ship stays free regardless of fleet size (0 / 4 = 0)
        assert_eq!(ship_build_cost(ShipType::Scout, 10), 0);
    }
}
