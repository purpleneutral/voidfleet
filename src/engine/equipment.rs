use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use rand::Rng;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Rarity
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl Rarity {
    pub fn color(&self) -> Color {
        match self {
            Self::Common => Color::Gray,
            Self::Uncommon => Color::Green,
            Self::Rare => Color::Blue,
            Self::Epic => Color::Magenta,
            Self::Legendary => Color::Yellow,
        }
    }

    pub fn stat_multiplier(&self) -> f32 {
        match self {
            Self::Common => 1.0,
            Self::Uncommon => 1.3,
            Self::Rare => 1.6,
            Self::Epic => 2.0,
            Self::Legendary => 2.5,
        }
    }

    pub fn drop_weight(&self) -> u32 {
        match self {
            Self::Common => 50,
            Self::Uncommon => 30,
            Self::Rare => 14,
            Self::Epic => 5,
            Self::Legendary => 1,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Common => "Common",
            Self::Uncommon => "Uncommon",
            Self::Rare => "Rare",
            Self::Epic => "Epic",
            Self::Legendary => "Legendary",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Common => "\u{00b7}",    // ·
            Self::Uncommon => "\u{25cb}",  // ○
            Self::Rare => "\u{25cf}",      // ●
            Self::Epic => "\u{25c6}",      // ◆
            Self::Legendary => "\u{2605}", // ★
        }
    }

    pub const ALL: [Rarity; 5] = [
        Self::Common,
        Self::Uncommon,
        Self::Rare,
        Self::Epic,
        Self::Legendary,
    ];
}

// ---------------------------------------------------------------------------
// Equipment Slots
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Slot {
    Weapon,
    Shield,
    Engine,
    Special,
}

impl Slot {
    pub const ALL: [Slot; 4] = [Self::Weapon, Self::Shield, Self::Engine, Self::Special];

    pub fn name(&self) -> &'static str {
        match self {
            Self::Weapon => "Weapon",
            Self::Shield => "Shield",
            Self::Engine => "Engine",
            Self::Special => "Special",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Weapon => "\u{2694}",  // ⚔
            Self::Shield => "\u{1f6e1}", // 🛡
            Self::Engine => "\u{26a1}",  // ⚡
            Self::Special => "\u{2726}", // ✦
        }
    }
}

// ---------------------------------------------------------------------------
// Special Effects
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpecialEffect {
    /// Hit bounces to N nearby enemies for a percentage of original damage.
    ChainLightning { targets: u8, damage_pct: f32 },
    /// Heal a percentage of damage dealt.
    Leech { percent: f32 },
    /// AOE explosion on hit.
    Explosive { radius: f32, damage: i32 },
    /// Projectile continues through target with a given chance.
    Piercing { chance: f32 },
    /// Increased damage but hurts self each shot.
    Overcharge { damage_mult: f32, self_damage: i32 },
    /// Passive HP regeneration per second.
    AutoRepair { hp_per_sec: f32 },
    /// Temporary invisibility.
    Cloak { duration_secs: f32 },
    /// Stuns nearby enemies briefly.
    EMPBurst { stun_ticks: u32 },
}

impl SpecialEffect {
    pub fn description(&self) -> String {
        match self {
            Self::ChainLightning { targets, damage_pct } => {
                format!(
                    "Chain Lightning: bounces to {} targets for {:.0}% dmg",
                    targets,
                    damage_pct * 100.0
                )
            }
            Self::Leech { percent } => {
                format!("Leech: heal {:.0}% of damage dealt", percent * 100.0)
            }
            Self::Explosive { radius, damage } => {
                format!("Explosive: {:.1} radius, {} AOE damage", radius, damage)
            }
            Self::Piercing { chance } => {
                format!("Piercing: {:.0}% chance to pass through", chance * 100.0)
            }
            Self::Overcharge {
                damage_mult,
                self_damage,
            } => {
                format!(
                    "Overcharge: {:.0}% more dmg, {} self-damage",
                    (damage_mult - 1.0) * 100.0,
                    self_damage
                )
            }
            Self::AutoRepair { hp_per_sec } => {
                format!("Auto Repair: +{:.1} HP/sec", hp_per_sec)
            }
            Self::Cloak { duration_secs } => {
                format!("Cloak: {:.1}s invisibility", duration_secs)
            }
            Self::EMPBurst { stun_ticks } => {
                format!("EMP Burst: stun enemies for {} ticks", stun_ticks)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Stat Modifiers (nested struct for equipment stats)
// ---------------------------------------------------------------------------

/// Stat modifiers from a single piece of equipment. Ships read these fields
/// via `item.modifiers.*` when computing totals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Modifiers {
    pub flat_damage: i32,
    pub pct_damage: f32,
    pub flat_hp: i32,
    pub pct_hp: f32,
    pub speed: f32,
    pub fire_rate: f32,
    pub crit_chance: f32,
    pub dodge: f32,
    pub shield_regen: f32,
}

impl Default for Modifiers {
    fn default() -> Self {
        Self {
            flat_damage: 0,
            pct_damage: 0.0,
            flat_hp: 0,
            pct_hp: 0.0,
            speed: 0.0,
            fire_rate: 0.0,
            crit_chance: 0.0,
            dodge: 0.0,
            shield_regen: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Equipment Item
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Equipment {
    pub id: u64,
    pub name: String,
    pub slot: Slot,
    pub rarity: Rarity,
    pub level: u32,

    /// All stat bonuses from this item.
    pub modifiers: Modifiers,

    // Special properties
    pub set_id: Option<String>,
    pub special_effect: Option<SpecialEffect>,
}

impl Equipment {
    /// Scrap value when salvaging this item.
    pub fn salvage_value(&self) -> u64 {
        let base = 5u64 + self.level as u64 * 3;
        let rarity_mult = match self.rarity {
            Rarity::Common => 1,
            Rarity::Uncommon => 2,
            Rarity::Rare => 4,
            Rarity::Epic => 8,
            Rarity::Legendary => 15,
        };
        base * rarity_mult
    }

    /// One-line summary for inventory display.
    pub fn summary(&self) -> String {
        let m = &self.modifiers;
        let mut parts = vec![format!("[Lv{}]", self.level)];

        if m.flat_damage != 0 {
            parts.push(format!("+{} dmg", m.flat_damage));
        }
        if m.pct_damage > 0.0 {
            parts.push(format!("+{:.0}% dmg", m.pct_damage * 100.0));
        }
        if m.flat_hp != 0 {
            parts.push(format!("+{} HP", m.flat_hp));
        }
        if m.pct_hp > 0.0 {
            parts.push(format!("+{:.0}% HP", m.pct_hp * 100.0));
        }
        if m.speed != 0.0 {
            let sign = if m.speed > 0.0 { "+" } else { "" };
            parts.push(format!("{}{:.1} spd", sign, m.speed));
        }
        if m.fire_rate != 0.0 {
            let sign = if m.fire_rate > 0.0 { "+" } else { "" };
            parts.push(format!("{}{:.1} rate", sign, m.fire_rate));
        }
        if m.crit_chance > 0.0 {
            parts.push(format!("{:.0}% crit", m.crit_chance * 100.0));
        }
        if m.dodge > 0.0 {
            parts.push(format!("+{:.0}% dodge", m.dodge * 100.0));
        }
        if m.shield_regen > 0.0 {
            parts.push(format!("+{:.1} regen", m.shield_regen));
        }

        parts.join(" ")
    }

    /// Detailed stat lines for the detail panel (one stat per line).
    pub fn detail_lines(&self) -> Vec<String> {
        let m = &self.modifiers;
        let mut lines = Vec::new();

        if m.flat_damage != 0 {
            lines.push(format!("+{} Damage", m.flat_damage));
        }
        if m.pct_damage > 0.0 {
            lines.push(format!("+{:.0}% Damage", m.pct_damage * 100.0));
        }
        if m.flat_hp != 0 {
            lines.push(format!("+{} HP", m.flat_hp));
        }
        if m.pct_hp > 0.0 {
            lines.push(format!("+{:.0}% HP", m.pct_hp * 100.0));
        }
        if m.speed != 0.0 {
            let sign = if m.speed > 0.0 { "+" } else { "" };
            lines.push(format!("{}{:.1} Speed", sign, m.speed));
        }
        if m.fire_rate != 0.0 {
            let sign = if m.fire_rate > 0.0 { "+" } else { "" };
            lines.push(format!("{}{:.1} Fire Rate", sign, m.fire_rate));
        }
        if m.crit_chance > 0.0 {
            lines.push(format!("+{:.0}% Critical Chance", m.crit_chance * 100.0));
        }
        if m.dodge > 0.0 {
            lines.push(format!("+{:.0}% Dodge", m.dodge * 100.0));
        }
        if m.shield_regen > 0.0 {
            lines.push(format!("+{:.1} Shield Regen", m.shield_regen));
        }
        lines
    }
}

// ---------------------------------------------------------------------------
// Equipment Sets
// ---------------------------------------------------------------------------

pub struct SetBonus {
    pub set_id: &'static str,
    pub set_name: &'static str,
    pub pieces_required: u8,
    pub bonus_description: &'static str,
    pub damage_percent: f32,
    pub hp_percent: f32,
    pub speed_bonus: f32,
}

pub const SET_BONUSES: &[SetBonus] = &[
    SetBonus {
        set_id: "void_walker",
        set_name: "Void Walker",
        pieces_required: 3,
        bonus_description: "+20% speed, +10% dodge",
        damage_percent: 0.0,
        hp_percent: 0.0,
        speed_bonus: 0.20,
    },
    SetBonus {
        set_id: "iron_fortress",
        set_name: "Iron Fortress",
        pieces_required: 3,
        bonus_description: "+30% HP, +15% shield regen",
        damage_percent: 0.0,
        hp_percent: 0.30,
        speed_bonus: 0.0,
    },
    SetBonus {
        set_id: "plasma_core",
        set_name: "Plasma Core",
        pieces_required: 3,
        bonus_description: "+25% damage, +10% crit",
        damage_percent: 0.25,
        hp_percent: 0.0,
        speed_bonus: 0.0,
    },
    SetBonus {
        set_id: "ghost_tech",
        set_name: "Ghost Tech",
        pieces_required: 4,
        bonus_description: "+30% dodge, +20% speed, cloak on kill",
        damage_percent: 0.0,
        hp_percent: 0.0,
        speed_bonus: 0.20,
    },
    SetBonus {
        set_id: "dreadnought",
        set_name: "Dreadnought",
        pieces_required: 4,
        bonus_description: "+40% HP, +20% damage, -15% speed",
        damage_percent: 0.20,
        hp_percent: 0.40,
        speed_bonus: -0.15,
    },
];

/// Count set pieces in a collection and return active bonuses.
pub fn active_set_bonuses(equipment: &[Equipment]) -> Vec<&'static SetBonus> {
    let mut bonuses = Vec::new();
    for set in SET_BONUSES {
        let count = equipment
            .iter()
            .filter(|e| e.set_id.as_deref() == Some(set.set_id))
            .count();
        if count >= set.pieces_required as usize {
            bonuses.push(set);
        }
    }
    bonuses
}

// ---------------------------------------------------------------------------
// Name generation
// ---------------------------------------------------------------------------

const WEAPON_BASES: &[&str] = &[
    "Plasma Cannon",
    "Pulse Emitter",
    "Railgun",
    "Laser Array",
    "Ion Blaster",
    "Photon Lance",
    "Beam Projector",
    "Gatling Turret",
    "Missile Pod",
    "Disruptor",
];

const SHIELD_BASES: &[&str] = &[
    "Barrier Matrix",
    "Deflector Array",
    "Energy Ward",
    "Force Screen",
    "Phase Shield",
    "Hull Plating",
    "Armor Shell",
    "Bulwark Module",
    "Aegis Generator",
    "Repulsor Grid",
];

const ENGINE_BASES: &[&str] = &[
    "Drive Core",
    "Thrust Module",
    "Warp Coil",
    "Ion Thruster",
    "Impulse Engine",
    "Phase Drive",
    "Afterburner",
    "Fusion Plant",
    "Gravity Well",
    "Propulsion Array",
];

const SPECIAL_BASES: &[&str] = &[
    "Targeting Link",
    "Field Generator",
    "Sensor Suite",
    "Overloader",
    "Amplifier Node",
    "Cloaking Device",
    "Signal Jammer",
    "Nanite Swarm",
    "Chrono Capacitor",
    "EMP Module",
];

const PREFIXES_UNCOMMON: &[&str] = &["Refined", "Enhanced", "Tuned", "Polished"];
const PREFIXES_RARE: &[&str] = &["Superior", "Advanced", "Precision", "Augmented"];
const PREFIXES_EPIC: &[&str] = &["Masterwork", "Prototype", "Experimental", "Elite"];
const PREFIXES_LEGENDARY: &[&str] = &["Legendary", "Mythic", "Ancient", "Ascendant"];

const ADJECTIVES: &[&str] = &[
    "Searing", "Void", "Quantum", "Phantom", "Hardened", "Overclocked", "Neural", "Stealth",
    "Crimson", "Abyssal", "Solar", "Cryo", "Inferno", "Spectral", "Tachyon",
];

fn generate_name(rng: &mut impl Rng, slot: Slot, rarity: Rarity) -> String {
    let bases = match slot {
        Slot::Weapon => WEAPON_BASES,
        Slot::Shield => SHIELD_BASES,
        Slot::Engine => ENGINE_BASES,
        Slot::Special => SPECIAL_BASES,
    };

    let base = bases[rng.gen_range(0..bases.len())];
    let adjective = ADJECTIVES[rng.gen_range(0..ADJECTIVES.len())];

    let prefix = match rarity {
        Rarity::Common => None,
        Rarity::Uncommon => Some(PREFIXES_UNCOMMON[rng.gen_range(0..PREFIXES_UNCOMMON.len())]),
        Rarity::Rare => Some(PREFIXES_RARE[rng.gen_range(0..PREFIXES_RARE.len())]),
        Rarity::Epic => Some(PREFIXES_EPIC[rng.gen_range(0..PREFIXES_EPIC.len())]),
        Rarity::Legendary => {
            Some(PREFIXES_LEGENDARY[rng.gen_range(0..PREFIXES_LEGENDARY.len())])
        }
    };

    match prefix {
        Some(p) => format!("{} {} {}", p, adjective, base),
        None => format!("{} {}", adjective, base),
    }
}

// ---------------------------------------------------------------------------
// ID generation (simple atomic counter)
// ---------------------------------------------------------------------------

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn next_equipment_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Loot generation
// ---------------------------------------------------------------------------

fn roll_rarity(rng: &mut impl Rng) -> Rarity {
    let weights: Vec<u32> = Rarity::ALL.iter().map(|r| r.drop_weight()).collect();
    let dist = WeightedIndex::new(&weights).expect("valid weights");
    Rarity::ALL[dist.sample(rng)]
}

fn roll_rarity_min(rng: &mut impl Rng, min: Rarity) -> Rarity {
    loop {
        let r = roll_rarity(rng);
        if r >= min {
            return r;
        }
    }
}

fn roll_special_effect(rng: &mut impl Rng, slot: Slot) -> SpecialEffect {
    match slot {
        Slot::Weapon => match rng.gen_range(0..5u8) {
            0 => SpecialEffect::ChainLightning {
                targets: rng.gen_range(2..=4),
                damage_pct: rng.gen_range(0.2..=0.5),
            },
            1 => SpecialEffect::Leech {
                percent: rng.gen_range(0.05..=0.20),
            },
            2 => SpecialEffect::Explosive {
                radius: rng.gen_range(1.5..=4.0),
                damage: rng.gen_range(5..=30),
            },
            3 => SpecialEffect::Piercing {
                chance: rng.gen_range(0.15..=0.50),
            },
            _ => SpecialEffect::Overcharge {
                damage_mult: rng.gen_range(1.3..=1.8),
                self_damage: rng.gen_range(2..=8),
            },
        },
        Slot::Shield => match rng.gen_range(0..2u8) {
            0 => SpecialEffect::AutoRepair {
                hp_per_sec: rng.gen_range(1.0..=5.0),
            },
            _ => SpecialEffect::EMPBurst {
                stun_ticks: rng.gen_range(10..=30),
            },
        },
        Slot::Engine => match rng.gen_range(0..2u8) {
            0 => SpecialEffect::Cloak {
                duration_secs: rng.gen_range(2.0..=6.0),
            },
            _ => SpecialEffect::AutoRepair {
                hp_per_sec: rng.gen_range(0.5..=2.0),
            },
        },
        Slot::Special => match rng.gen_range(0..4u8) {
            0 => SpecialEffect::ChainLightning {
                targets: rng.gen_range(2..=5),
                damage_pct: rng.gen_range(0.3..=0.6),
            },
            1 => SpecialEffect::EMPBurst {
                stun_ticks: rng.gen_range(15..=40),
            },
            2 => SpecialEffect::Cloak {
                duration_secs: rng.gen_range(3.0..=8.0),
            },
            _ => SpecialEffect::Leech {
                percent: rng.gen_range(0.10..=0.25),
            },
        },
    }
}

/// Build stat modifiers for a given slot, rarity, and level.
fn build_modifiers(slot: Slot, rarity: Rarity, level: u32) -> Modifiers {
    let mult = rarity.stat_multiplier();
    let level_scale = 1.0 + level as f32 * 0.05;

    match slot {
        Slot::Weapon => Modifiers {
            flat_damage: (5.0 * mult * level_scale) as i32,
            pct_damage: 0.05 * mult * level_scale,
            crit_chance: 0.03 * mult,
            fire_rate: -0.05 * mult, // negative = faster
            ..Modifiers::default()
        },
        Slot::Shield => Modifiers {
            flat_hp: (15.0 * mult * level_scale) as i32,
            pct_hp: 0.08 * mult * level_scale,
            shield_regen: 0.5 * mult * level_scale,
            dodge: 0.02 * mult,
            ..Modifiers::default()
        },
        Slot::Engine => Modifiers {
            speed: 0.5 * mult * level_scale,
            dodge: 0.04 * mult,
            flat_hp: (5.0 * mult * level_scale) as i32,
            ..Modifiers::default()
        },
        Slot::Special => Modifiers {
            flat_damage: (3.0 * mult * level_scale) as i32,
            pct_damage: 0.03 * mult * level_scale,
            flat_hp: (8.0 * mult * level_scale) as i32,
            crit_chance: 0.05 * mult,
            speed: 0.2 * mult,
            ..Modifiers::default()
        },
    }
}

/// Generate a single piece of equipment appropriate for the given sector.
pub fn generate_equipment(sector: u32, slot: Option<Slot>) -> Equipment {
    let mut rng = rand::thread_rng();
    generate_equipment_with_rng(&mut rng, sector, slot)
}

/// Generate equipment using a provided RNG (for deterministic testing).
pub fn generate_equipment_with_rng(
    rng: &mut impl Rng,
    sector: u32,
    slot: Option<Slot>,
) -> Equipment {
    let slot = slot.unwrap_or_else(|| Slot::ALL[rng.gen_range(0..Slot::ALL.len())]);
    let rarity = roll_rarity(rng);

    // Item level = sector ± some variance, minimum 1
    let level_offset: i32 = rng.gen_range(-2..=3);
    let level = (sector as i32 + level_offset).max(1) as u32;

    let modifiers = build_modifiers(slot, rarity, level);

    // Special effect chance scales with rarity
    let special_chance = match rarity {
        Rarity::Common => 0.05,
        Rarity::Uncommon => 0.10,
        Rarity::Rare => 0.18,
        Rarity::Epic => 0.30,
        Rarity::Legendary => 0.50,
    };
    let special_effect = if rng.gen_range(0.0..1.0f32) < special_chance {
        Some(roll_special_effect(rng, slot))
    } else {
        None
    };

    // 5% chance of set piece, only Rare+
    let set_id = if rarity >= Rarity::Rare && rng.gen_range(0.0..1.0f32) < 0.05 {
        let set = &SET_BONUSES[rng.gen_range(0..SET_BONUSES.len())];
        Some(set.set_id.to_string())
    } else {
        None
    };

    let name = generate_name(rng, slot, rarity);

    Equipment {
        id: next_equipment_id(),
        name,
        slot,
        rarity,
        level,
        modifiers,
        set_id,
        special_effect,
    }
}

/// Generate loot drops from a battle encounter.
pub fn generate_battle_drops(sector: u32, enemy_count: u32, boss: bool) -> Vec<Equipment> {
    let mut rng = rand::thread_rng();
    generate_battle_drops_with_rng(&mut rng, sector, enemy_count, boss)
}

/// Generate battle drops with a provided RNG (for deterministic testing).
pub fn generate_battle_drops_with_rng(
    rng: &mut impl Rng,
    sector: u32,
    enemy_count: u32,
    boss: bool,
) -> Vec<Equipment> {
    // Base 1-2 drops
    let mut count = rng.gen_range(1..=2u32);
    // +1 per 3 enemies killed
    count += enemy_count / 3;

    let mut drops = Vec::new();

    // Boss: guaranteed Rare+ drop first, plus an extra regular drop
    if boss {
        let rarity = roll_rarity_min(rng, Rarity::Rare);
        let slot = Slot::ALL[rng.gen_range(0..Slot::ALL.len())];
        let level_offset: i32 = rng.gen_range(-2..=3);
        let level = (sector as i32 + level_offset).max(1) as u32;
        let modifiers = build_modifiers(slot, rarity, level);
        let name = generate_name(rng, slot, rarity);

        let special_chance = match rarity {
            Rarity::Common => 0.05,
            Rarity::Uncommon => 0.10,
            Rarity::Rare => 0.18,
            Rarity::Epic => 0.30,
            Rarity::Legendary => 0.50,
        };
        let special_effect = if rng.gen_range(0.0..1.0f32) < special_chance {
            Some(roll_special_effect(rng, slot))
        } else {
            None
        };

        let set_id = if rng.gen_range(0.0..1.0f32) < 0.05 {
            let set = &SET_BONUSES[rng.gen_range(0..SET_BONUSES.len())];
            Some(set.set_id.to_string())
        } else {
            None
        };

        drops.push(Equipment {
            id: next_equipment_id(),
            name,
            slot,
            rarity,
            level,
            modifiers,
            set_id,
            special_effect,
        });

        count += 1;
    }

    for _ in 0..count {
        drops.push(generate_equipment_with_rng(rng, sector, None));
    }

    drops
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn test_rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    #[test]
    fn rarity_ordering() {
        assert!(Rarity::Common < Rarity::Uncommon);
        assert!(Rarity::Uncommon < Rarity::Rare);
        assert!(Rarity::Rare < Rarity::Epic);
        assert!(Rarity::Epic < Rarity::Legendary);
    }

    #[test]
    fn rarity_stat_multipliers_increase() {
        let mults: Vec<f32> = Rarity::ALL.iter().map(|r| r.stat_multiplier()).collect();
        for w in mults.windows(2) {
            assert!(w[1] > w[0], "multipliers must increase with rarity");
        }
    }

    #[test]
    fn rarity_drop_weights_decrease() {
        let weights: Vec<u32> = Rarity::ALL.iter().map(|r| r.drop_weight()).collect();
        for w in weights.windows(2) {
            assert!(w[1] < w[0], "drop weights must decrease with rarity");
        }
    }

    #[test]
    fn generate_single_equipment() {
        let mut rng = test_rng();
        let item = generate_equipment_with_rng(&mut rng, 5, Some(Slot::Weapon));
        assert_eq!(item.slot, Slot::Weapon);
        assert!(item.level >= 1);
        assert!(!item.name.is_empty());
        assert!(item.id > 0);
    }

    #[test]
    fn generate_all_slots() {
        let mut rng = test_rng();
        for slot in Slot::ALL {
            let item = generate_equipment_with_rng(&mut rng, 10, Some(slot));
            assert_eq!(item.slot, slot);
        }
    }

    #[test]
    fn item_level_near_sector() {
        let mut rng = test_rng();
        for _ in 0..100 {
            let sector = 20;
            let item = generate_equipment_with_rng(&mut rng, sector, None);
            // level = sector + (-2..=3), so 18..=23
            assert!(item.level >= 18, "level {} too low", item.level);
            assert!(item.level <= 23, "level {} too high", item.level);
        }
    }

    #[test]
    fn weapon_has_damage_stats() {
        let mut rng = test_rng();
        for _ in 0..50 {
            let item = generate_equipment_with_rng(&mut rng, 10, Some(Slot::Weapon));
            assert!(
                item.modifiers.flat_damage > 0,
                "weapon should have flat_damage"
            );
            assert!(
                item.modifiers.pct_damage > 0.0,
                "weapon should have pct_damage"
            );
        }
    }

    #[test]
    fn shield_has_hp_stats() {
        let mut rng = test_rng();
        for _ in 0..50 {
            let item = generate_equipment_with_rng(&mut rng, 10, Some(Slot::Shield));
            assert!(item.modifiers.flat_hp > 0, "shield should have flat_hp");
            assert!(item.modifiers.pct_hp > 0.0, "shield should have pct_hp");
            assert!(
                item.modifiers.shield_regen > 0.0,
                "shield should have shield_regen"
            );
        }
    }

    #[test]
    fn engine_has_speed_stats() {
        let mut rng = test_rng();
        for _ in 0..50 {
            let item = generate_equipment_with_rng(&mut rng, 10, Some(Slot::Engine));
            assert!(item.modifiers.speed > 0.0, "engine should have speed");
        }
    }

    #[test]
    fn higher_rarity_means_higher_stats() {
        let mut rng = test_rng();
        let mut totals: [f64; 5] = [0.0; 5];
        let mut counts: [u32; 5] = [0; 5];

        for _ in 0..10_000 {
            let item = generate_equipment_with_rng(&mut rng, 10, Some(Slot::Weapon));
            let idx = Rarity::ALL.iter().position(|r| *r == item.rarity).unwrap();
            totals[idx] += item.modifiers.flat_damage as f64;
            counts[idx] += 1;
        }

        let avgs: Vec<f64> = totals
            .iter()
            .zip(counts.iter())
            .map(|(t, c)| if *c > 0 { t / *c as f64 } else { 0.0 })
            .collect();

        for w in avgs.windows(2) {
            if w[0] > 0.0 && w[1] > 0.0 {
                assert!(
                    w[1] >= w[0],
                    "higher rarity should have higher avg damage: {:?}",
                    avgs
                );
            }
        }
    }

    #[test]
    fn rarity_distribution_roughly_correct() {
        let mut rng = test_rng();
        let mut counts = [0u32; 5];
        let n = 10_000;

        for _ in 0..n {
            let r = roll_rarity(&mut rng);
            let idx = Rarity::ALL.iter().position(|x| *x == r).unwrap();
            counts[idx] += 1;
        }

        let common_pct = counts[0] as f32 / n as f32;
        assert!(
            common_pct > 0.40,
            "Common too rare: {:.1}%",
            common_pct * 100.0
        );

        let legendary_pct = counts[4] as f32 / n as f32;
        assert!(
            legendary_pct < 0.03,
            "Legendary too common: {:.1}%",
            legendary_pct * 100.0
        );

        for (i, count) in counts.iter().enumerate() {
            assert!(*count > 0, "Rarity {:?} had zero drops", Rarity::ALL[i]);
        }
    }

    #[test]
    fn battle_drops_basic() {
        let mut rng = test_rng();
        let drops = generate_battle_drops_with_rng(&mut rng, 5, 3, false);
        assert!(!drops.is_empty());
        assert!(drops.len() <= 10, "too many drops: {}", drops.len());
    }

    #[test]
    fn boss_drops_guaranteed_rare_plus() {
        let mut rng = test_rng();
        for _ in 0..50 {
            let drops = generate_battle_drops_with_rng(&mut rng, 10, 1, true);
            assert!(
                drops[0].rarity >= Rarity::Rare,
                "boss first drop was {:?}, expected Rare+",
                drops[0].rarity
            );
            assert!(drops.len() >= 3, "boss should give at least 3 drops");
        }
    }

    #[test]
    fn boss_drops_more_than_normal() {
        let mut rng = test_rng();
        let normal_total: usize = (0..100)
            .map(|_| generate_battle_drops_with_rng(&mut rng, 10, 3, false).len())
            .sum();
        let boss_total: usize = (0..100)
            .map(|_| generate_battle_drops_with_rng(&mut rng, 10, 3, true).len())
            .sum();

        assert!(
            boss_total > normal_total,
            "boss should give more loot: boss={}, normal={}",
            boss_total, normal_total
        );
    }

    #[test]
    fn unique_ids() {
        let mut rng = test_rng();
        let items: Vec<Equipment> = (0..100)
            .map(|_| generate_equipment_with_rng(&mut rng, 5, None))
            .collect();

        let mut ids: Vec<u64> = items.iter().map(|i| i.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 100, "all equipment IDs should be unique");
    }

    #[test]
    fn set_pieces_only_rare_plus() {
        let mut rng = test_rng();
        for _ in 0..10_000 {
            let item = generate_equipment_with_rng(&mut rng, 10, None);
            if item.set_id.is_some() {
                assert!(
                    item.rarity >= Rarity::Rare,
                    "set piece was {:?}, expected Rare+",
                    item.rarity
                );
            }
        }
    }

    #[test]
    fn set_bonus_detection() {
        let mut rng = test_rng();
        let mut pieces: Vec<Equipment> = (0..3)
            .map(|_| {
                let mut item = generate_equipment_with_rng(&mut rng, 10, None);
                item.set_id = Some("void_walker".to_string());
                item
            })
            .collect();

        let bonuses = active_set_bonuses(&pieces);
        assert_eq!(bonuses.len(), 1);
        assert_eq!(bonuses[0].set_id, "void_walker");

        pieces.pop();
        let bonuses = active_set_bonuses(&pieces);
        assert!(bonuses.is_empty());
    }

    #[test]
    fn special_effect_description() {
        let effect = SpecialEffect::ChainLightning {
            targets: 3,
            damage_pct: 0.4,
        };
        let desc = effect.description();
        assert!(desc.contains("Chain Lightning"));
        assert!(desc.contains("3 targets"));
    }

    #[test]
    fn equipment_summary_non_empty() {
        let mut rng = test_rng();
        for slot in Slot::ALL {
            let item = generate_equipment_with_rng(&mut rng, 10, Some(slot));
            let summary = item.summary();
            assert!(!summary.is_empty());
            assert!(summary.contains("Lv"));
        }
    }

    #[test]
    fn serialization_roundtrip() {
        let mut rng = test_rng();
        let item = generate_equipment_with_rng(&mut rng, 10, Some(Slot::Weapon));
        let json = serde_json::to_string(&item).expect("serialize");
        let deserialized: Equipment = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.id, item.id);
        assert_eq!(deserialized.name, item.name);
        assert_eq!(deserialized.slot, item.slot);
        assert_eq!(deserialized.rarity, item.rarity);
        assert_eq!(deserialized.level, item.level);
    }

    #[test]
    fn sector_1_items_low_level() {
        let mut rng = test_rng();
        for _ in 0..100 {
            let item = generate_equipment_with_rng(&mut rng, 1, None);
            assert!(
                item.level <= 4,
                "sector 1 item level {} too high",
                item.level
            );
        }
    }

    #[test]
    fn name_generation_contains_base() {
        let mut rng = test_rng();
        for _ in 0..100 {
            let item = generate_equipment_with_rng(&mut rng, 10, None);
            assert!(item.name.len() >= 5, "name too short: {}", item.name);
            assert!(item.name.len() <= 60, "name too long: {}", item.name);
        }
    }

    #[test]
    fn salvage_value_scales_with_rarity() {
        // Generate items at same level, check salvage increases with rarity
        let common = Equipment {
            id: 1,
            name: "Test".into(),
            slot: Slot::Weapon,
            rarity: Rarity::Common,
            level: 10,
            modifiers: Modifiers::default(),
            set_id: None,
            special_effect: None,
        };
        let legendary = Equipment {
            rarity: Rarity::Legendary,
            ..common.clone()
        };
        assert!(
            legendary.salvage_value() > common.salvage_value(),
            "legendary salvage {} should exceed common {}",
            legendary.salvage_value(),
            common.salvage_value()
        );
    }

    #[test]
    fn salvage_value_scales_with_level() {
        let low = Equipment {
            id: 1,
            name: "Test".into(),
            slot: Slot::Weapon,
            rarity: Rarity::Rare,
            level: 1,
            modifiers: Modifiers::default(),
            set_id: None,
            special_effect: None,
        };
        let high = Equipment {
            level: 20,
            ..low.clone()
        };
        assert!(
            high.salvage_value() > low.salvage_value(),
            "level 20 salvage {} should exceed level 1 {}",
            high.salvage_value(),
            low.salvage_value()
        );
    }
}
