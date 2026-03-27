//! Combat utility functions — tech-aware damage, HP, and fire rate calculations.
//! Equipment bonuses are layered on top of tech bonuses.

use crate::engine::ship::{Ship, ShipType};

/// Calculate effective damage for a ship considering tech bonuses, equipment,
/// and voyage permanent bonuses.
pub fn effective_damage(ship: &Ship, tech_lasers: u8) -> u32 {
    effective_damage_with_voyage(ship, tech_lasers, 0.0)
}

/// Calculate effective damage including voyage permanent damage bonus.
pub fn effective_damage_with_voyage(ship: &Ship, tech_lasers: u8, voyage_bonus: f32) -> u32 {
    let base = ship.damage();
    let tech_bonus = (base as f32 * tech_lasers as f32 * 0.1) as u32;
    let (flat, pct) = ship.total_damage_bonus();
    let before_flat = (base + tech_bonus) as f32 * (1.0 + pct + voyage_bonus);
    (before_flat + flat as f32).max(0.0) as u32
}

/// Calculate effective max HP considering tech bonuses, equipment,
/// and voyage permanent bonuses.
pub fn effective_hp(ship: &Ship, tech_shields: u8) -> u32 {
    effective_hp_with_voyage(ship, tech_shields, 0.0)
}

/// Calculate effective max HP including voyage permanent HP bonus.
pub fn effective_hp_with_voyage(ship: &Ship, tech_shields: u8, voyage_bonus: f32) -> u32 {
    let base = ship.max_hp();
    let tech_bonus = (base as f32 * tech_shields as f32 * 0.12) as u32;
    let (flat, pct) = ship.total_hp_bonus();
    let before_flat = (base + tech_bonus) as f32 * (1.0 + pct + voyage_bonus);
    (before_flat + flat as f32).max(1.0) as u32
}

/// Calculate fire rate (ticks between shots) — lower is faster.
/// Equipment speed bonuses reduce fire rate further.
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
    let tech_factor = 1.0 - tech_engines as f32 * 0.05;
    let speed_bonus = ship.total_speed_bonus();
    // Each point of speed bonus reduces fire rate by ~3%
    let equip_factor = (1.0 - speed_bonus * 0.03).max(0.3);
    (base as f32 * tech_factor * equip_factor).max(5.0) as u32
}

/// Roll whether this attack is a critical hit based on equipped crit chance.
pub fn roll_crit(ship: &Ship) -> bool {
    let chance = ship.total_crit_chance();
    if chance <= 0.0 {
        return false;
    }
    rand::random::<f32>() < chance
}

/// Calculate critical hit damage (2x multiplier).
pub fn crit_damage(base_damage: u32) -> u32 {
    base_damage.saturating_mul(2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::equipment::{Equipment, Modifiers, Rarity, Slot};

    fn make_weapon(flat_damage: i32, pct_damage: f32, crit: f32) -> Equipment {
        Equipment {
            id: 1,
            name: "Test Weapon".to_string(),
            slot: Slot::Weapon,
            rarity: Rarity::Common,
            level: 1,
            modifiers: Modifiers {
                flat_damage,
                pct_damage,
                crit_chance: crit,
                ..Modifiers::default()
            },
            set_id: None,
            special_effect: None,
        }
    }

    fn make_shield(flat_hp: i32, pct_hp: f32) -> Equipment {
        Equipment {
            id: 2,
            name: "Test Shield".to_string(),
            slot: Slot::Shield,
            rarity: Rarity::Common,
            level: 1,
            modifiers: Modifiers {
                flat_hp,
                pct_hp,
                ..Modifiers::default()
            },
            set_id: None,
            special_effect: None,
        }
    }

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
    fn test_effective_damage_with_equipment() {
        let mut ship = Ship::new(ShipType::Fighter);
        let base_dmg = effective_damage(&ship, 0);
        ship.equip(make_weapon(10, 0.0, 0.0));
        let equipped_dmg = effective_damage(&ship, 0);
        assert_eq!(equipped_dmg, base_dmg + 10);
    }

    #[test]
    fn test_effective_damage_with_pct_equipment() {
        let mut ship = Ship::new(ShipType::Fighter);
        let base_dmg = effective_damage(&ship, 0);
        ship.equip(make_weapon(0, 0.5, 0.0)); // +50%
        let equipped_dmg = effective_damage(&ship, 0);
        assert_eq!(equipped_dmg, (base_dmg as f32 * 1.5) as u32);
    }

    #[test]
    fn test_effective_hp_with_tech() {
        let ship = Ship::new(ShipType::Frigate);
        let hp = effective_hp(&ship, 3);
        assert!(hp > ship.max_hp());
    }

    #[test]
    fn test_effective_hp_with_equipment() {
        let mut ship = Ship::new(ShipType::Frigate);
        let base_hp = effective_hp(&ship, 0);
        ship.equip(make_shield(20, 0.0));
        let equipped_hp = effective_hp(&ship, 0);
        assert_eq!(equipped_hp, base_hp + 20);
    }

    #[test]
    fn test_fire_rate_clamped() {
        let ship = Ship::new(ShipType::Scout);
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

    #[test]
    fn test_crit_damage() {
        assert_eq!(crit_damage(50), 100);
        assert_eq!(crit_damage(0), 0);
    }

    #[test]
    fn test_crit_chance_capped() {
        let mut ship = Ship::new(ShipType::Fighter);
        // Equip items with huge crit
        ship.equip(Equipment {
            id: 1,
            name: "Crit Monster".to_string(),
            slot: Slot::Weapon,
            rarity: Rarity::Legendary,
            level: 1,
            modifiers: Modifiers {
                crit_chance: 0.9,
                ..Modifiers::default()
            },
            set_id: None,
            special_effect: None,
        });
        assert!((ship.total_crit_chance() - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn test_roll_crit_zero_chance() {
        let ship = Ship::new(ShipType::Fighter);
        // With no equipment, crit chance is 0 — should never crit
        assert!(!roll_crit(&ship));
    }
}
