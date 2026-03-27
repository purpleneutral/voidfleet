/// Combat utility functions — tech-aware damage, HP, and fire rate calculations.

use crate::engine::ship::{Ship, ShipType};

/// Calculate effective damage for a ship considering tech bonuses
pub fn effective_damage(ship: &Ship, tech_lasers: u8) -> u32 {
    let base = ship.damage();
    base + (base as f32 * tech_lasers as f32 * 0.1) as u32
}

/// Calculate effective max HP considering tech bonuses
pub fn effective_hp(ship: &Ship, tech_shields: u8) -> u32 {
    let base = ship.max_hp();
    base + (base as f32 * tech_shields as f32 * 0.12) as u32
}

/// Calculate fire rate (ticks between shots) — lower is faster
pub fn fire_rate(ship: &Ship, tech_engines: u8) -> u32 {
    let base = match ship.ship_type {
        ShipType::Scout => 15,
        ShipType::Fighter => 12,
        ShipType::Bomber => 25,
        ShipType::Frigate => 18,
        ShipType::Destroyer => 20,
        ShipType::Capital => 30,
        ShipType::Carrier => 35,
    };
    (base as f32 * (1.0 - tech_engines as f32 * 0.05)).max(5.0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effective_damage_no_tech() {
        let ship = Ship::new(ShipType::Fighter);
        assert_eq!(effective_damage(&ship, 0), ship.damage());
    }

    #[test]
    fn test_effective_damage_with_tech() {
        let ship = Ship::new(ShipType::Fighter);
        let dmg = effective_damage(&ship, 5);
        assert!(dmg > ship.damage());
    }

    #[test]
    fn test_effective_hp_with_tech() {
        let ship = Ship::new(ShipType::Frigate);
        let hp = effective_hp(&ship, 3);
        assert!(hp > ship.max_hp());
    }

    #[test]
    fn test_fire_rate_clamped() {
        let ship = Ship::new(ShipType::Scout);
        // Even with absurd tech, fire rate floors at 5
        let rate = fire_rate(&ship, 20);
        assert!(rate >= 5);
    }

    #[test]
    fn test_fire_rate_improves_with_tech() {
        let ship = Ship::new(ShipType::Capital);
        let base_rate = fire_rate(&ship, 0);
        let upgraded_rate = fire_rate(&ship, 5);
        assert!(upgraded_rate < base_rate);
    }
}
