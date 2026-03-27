/// Economy — cost calculations and unlock gates.

use crate::engine::ship::ShipType;

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
