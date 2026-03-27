use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::engine::abilities::{
    AbilityContext, AbilityEffect, AbilityTrigger, CrewAbility, TriageTarget,
};

// ---------------------------------------------------------------------------
// Crew Classes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrewClass {
    Pilot,     // +piloting, decent gunnery
    Gunner,    // +gunnery, decent piloting
    Engineer,  // +engineering, decent leadership
    Medic,     // +engineering (repair focus), heals between battles
    Captain,   // +leadership, balanced stats, rare
    Navigator, // +piloting, decent leadership, map/travel bonuses
}

impl CrewClass {
    pub const ALL: [CrewClass; 6] = [
        Self::Pilot,
        Self::Gunner,
        Self::Engineer,
        Self::Medic,
        Self::Captain,
        Self::Navigator,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::Pilot => "Pilot",
            Self::Gunner => "Gunner",
            Self::Engineer => "Engineer",
            Self::Medic => "Medic",
            Self::Captain => "Captain",
            Self::Navigator => "Navigator",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Pilot => "\u{2708}",    // ✈
            Self::Gunner => "\u{1f3af}",  // 🎯
            Self::Engineer => "\u{1f527}", // 🔧
            Self::Medic => "\u{2695}",    // ⚕
            Self::Captain => "\u{2b50}",  // ⭐
            Self::Navigator => "\u{1f9ed}", // 🧭
        }
    }

    /// Drop weight for generation (higher = more common).
    pub fn drop_weight(&self) -> u32 {
        match self {
            Self::Pilot => 30,
            Self::Gunner => 30,
            Self::Engineer => 20,
            Self::Medic => 15,
            Self::Captain => 5,
            Self::Navigator => 15, // same rarity as Medic (uncommon)
        }
    }

    /// Base stats: (piloting, gunnery, engineering, leadership).
    fn base_stats(&self) -> (u8, u8, u8, u8) {
        match self {
            Self::Pilot => (30, 15, 8, 5),
            Self::Gunner => (15, 30, 8, 5),
            Self::Engineer => (5, 5, 30, 15),
            Self::Medic => (5, 5, 25, 15),
            Self::Captain => (15, 15, 10, 25),
            Self::Navigator => (35, 10, 15, 25),
        }
    }

    /// All abilities defined for this class, regardless of level.
    pub fn abilities(&self) -> Vec<CrewAbility> {
        match self {
            CrewClass::Pilot => vec![
                CrewAbility {
                    name: "Evasive Maneuvers".into(),
                    description: "Ship becomes untargetable for 30 ticks when HP drops below 25%"
                        .into(),
                    trigger: AbilityTrigger::OnHPBelow(25),
                    effect: AbilityEffect::EvasiveManeuvers {
                        untargetable_ticks: 30,
                    },
                    level_required: 5,
                    cooldown_max: 400,
                    icon: '⚡',
                },
                CrewAbility {
                    name: "Afterburner".into(),
                    description: "Instantly reposition once per battle".into(),
                    trigger: AbilityTrigger::OncePerBattle,
                    effect: AbilityEffect::Afterburner { teleport: true },
                    level_required: 10,
                    cooldown_max: 0,
                    icon: '🚀',
                },
            ],
            CrewClass::Gunner => vec![
                CrewAbility {
                    name: "Lock-On".into(),
                    description: "First shot is a guaranteed critical hit".into(),
                    trigger: AbilityTrigger::OnBattleStart,
                    effect: AbilityEffect::LockOn {
                        guaranteed_crit: true,
                    },
                    level_required: 5,
                    cooldown_max: 0,
                    icon: '🎯',
                },
                CrewAbility {
                    name: "Barrage".into(),
                    description: "Every 5th shot fires a triple burst".into(),
                    trigger: AbilityTrigger::EveryNShots(5),
                    effect: AbilityEffect::Barrage { extra_shots: 2 },
                    level_required: 10,
                    cooldown_max: 0,
                    icon: '💥',
                },
            ],
            CrewClass::Engineer => vec![
                CrewAbility {
                    name: "Passive Regen".into(),
                    description: "Ship regenerates HP during battle".into(),
                    trigger: AbilityTrigger::Passive,
                    effect: AbilityEffect::PassiveRegen { hp_per_tick: 0.1 },
                    level_required: 3,
                    cooldown_max: 0,
                    icon: '🔧',
                },
                CrewAbility {
                    name: "Emergency Repair".into(),
                    description: "Auto-heal 20% HP when dropping below 15%".into(),
                    trigger: AbilityTrigger::OnHPBelow(15),
                    effect: AbilityEffect::EmergencyRepair {
                        heal_percent: 0.20,
                    },
                    level_required: 5,
                    cooldown_max: 600,
                    icon: '⚕',
                },
                CrewAbility {
                    name: "Shield Overclock".into(),
                    description: "Shield equipment effectiveness doubled".into(),
                    trigger: AbilityTrigger::Passive,
                    effect: AbilityEffect::ShieldOverclock { multiplier: 2.0 },
                    level_required: 10,
                    cooldown_max: 0,
                    icon: '🛡',
                },
            ],
            CrewClass::Medic => vec![
                CrewAbility {
                    name: "Triage".into(),
                    description: "Heals the weakest ship in fleet during battle".into(),
                    trigger: AbilityTrigger::Passive,
                    effect: AbilityEffect::Triage {
                        heal_per_tick: 0.15,
                        target: TriageTarget::LowestPercent,
                    },
                    level_required: 5,
                    cooldown_max: 0,
                    icon: '❤',
                },
                CrewAbility {
                    name: "Revive".into(),
                    description: "Once per battle, resurrect a destroyed ship with 25% HP".into(),
                    trigger: AbilityTrigger::OnAllyDestroyed,
                    effect: AbilityEffect::Revive { hp_percent: 0.25 },
                    level_required: 10,
                    cooldown_max: 0,
                    icon: '✦',
                },
            ],
            CrewClass::Captain => vec![
                CrewAbility {
                    name: "Rally".into(),
                    description:
                        "When any ship drops below 30% HP, all ships get +10% damage".into(),
                    trigger: AbilityTrigger::OnHPBelow(30),
                    effect: AbilityEffect::Rally {
                        damage_bonus: 0.10,
                        duration_ticks: 120,
                    },
                    level_required: 5,
                    cooldown_max: 300,
                    icon: '⚔',
                },
                CrewAbility {
                    name: "Inspire".into(),
                    description: "Morale cannot drop below 50 for any crew in fleet".into(),
                    trigger: AbilityTrigger::Passive,
                    effect: AbilityEffect::Inspire { min_morale: 50 },
                    level_required: 10,
                    cooldown_max: 0,
                    icon: '👑',
                },
            ],
            CrewClass::Navigator => vec![
                CrewAbility {
                    name: "Stellar Cartography".into(),
                    description: "Map reveals threat level and loot quality for routes".into(),
                    trigger: AbilityTrigger::Passive,
                    effect: AbilityEffect::StellarCartography,
                    level_required: 5,
                    cooldown_max: 0,
                    icon: '🗺',
                },
                CrewAbility {
                    name: "Shortcut".into(),
                    description: "10% chance to skip a sector entirely".into(),
                    trigger: AbilityTrigger::Passive,
                    effect: AbilityEffect::Shortcut { skip_chance: 0.10 },
                    level_required: 10,
                    cooldown_max: 0,
                    icon: '⚡',
                },
            ],
        }
    }

    /// Abilities available at the given crew level.
    pub fn available_abilities(&self, level: u8) -> Vec<CrewAbility> {
        self.abilities()
            .into_iter()
            .filter(|a| level >= a.level_required)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Personality
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Personality {
    Aggressive, // +10% damage, -5% dodge, morale boost from kills
    Cautious,   // +10% dodge, -5% damage, morale boost from survival
    Loyal,      // +5% all stats when Pip bond > 500
    Greedy,     // +15% loot, -5% morale per battle without loot
    Brave,      // no morale penalty on fleet damage, +10% vs bosses
    Nervous,    // -10% when HP < 50%, +10% when HP > 80%
    Veteran,    // +1% all stats per 10 battles survived
    Reckless,   // +20% damage, +15% chance to take extra damage
}

impl Personality {
    pub const ALL: [Personality; 8] = [
        Self::Aggressive,
        Self::Cautious,
        Self::Loyal,
        Self::Greedy,
        Self::Brave,
        Self::Nervous,
        Self::Veteran,
        Self::Reckless,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::Aggressive => "Aggressive",
            Self::Cautious => "Cautious",
            Self::Loyal => "Loyal",
            Self::Greedy => "Greedy",
            Self::Brave => "Brave",
            Self::Nervous => "Nervous",
            Self::Veteran => "Veteran",
            Self::Reckless => "Reckless",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Aggressive => "+10% dmg, -5% dodge, morale boost from kills",
            Self::Cautious => "+10% dodge, -5% dmg, morale boost from survival",
            Self::Loyal => "+5% all stats when Pip bond > 500",
            Self::Greedy => "+15% loot, -5% morale per battle without loot",
            Self::Brave => "No morale penalty on fleet dmg, +10% vs bosses",
            Self::Nervous => "-10% when HP < 50%, +10% when HP > 80%",
            Self::Veteran => "+1% all stats per 10 battles survived",
            Self::Reckless => "+20% dmg, +15% chance to take extra dmg",
        }
    }
}

// ---------------------------------------------------------------------------
// Crew Member
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewMember {
    pub id: u64,
    pub name: String,
    pub class: CrewClass,
    pub level: u8, // 1-20
    pub xp: u64,
    pub personality: Personality,

    // Stats (0-100, affected by class + level + personality)
    pub piloting: u8,    // dodge, speed
    pub gunnery: u8,     // damage, crit
    pub engineering: u8, // shields, repair
    pub leadership: u8,  // fleet-wide minor bonus

    // State
    pub morale: u8,                   // 0-100, affects performance
    pub assigned_ship: Option<usize>, // index in fleet, None = unassigned
    pub kills: u64,
    pub battles_survived: u64,

    // Grief/Vengeance system
    #[serde(default)]
    pub grief_battles_remaining: u8, // 0 = no grief
    #[serde(default)]
    pub vengeance_battles_remaining: u8, // 0 = no vengeance
    #[serde(default)]
    pub bonded_with_name: Option<String>, // name of lost bonded crew for flavor text

    // Ability state tracking
    #[serde(default)]
    pub ability_cooldowns: Vec<u32>, // cooldown timer per ability
    #[serde(default)]
    pub abilities_used: Vec<bool>, // for OncePerBattle abilities
    #[serde(default)]
    pub shot_counter: u32, // for EveryNShots tracking
}

impl CrewMember {
    /// XP required for next level.
    pub fn xp_to_next(&self) -> u64 {
        100 * (self.level as u64 + 1).pow(2)
    }

    /// Add XP and level up if threshold reached. Returns true if leveled up.
    pub fn add_xp(&mut self, amount: u64) -> bool {
        self.xp += amount;
        let mut leveled = false;
        while self.level < 20 && self.xp >= self.xp_to_next() {
            self.xp -= self.xp_to_next();
            self.level += 1;
            self.improve_stats_on_levelup();
            leveled = true;
        }
        leveled
    }

    fn improve_stats_on_levelup(&mut self) {
        match self.class {
            CrewClass::Pilot => {
                self.piloting = self.piloting.saturating_add(3).min(100);
                self.gunnery = self.gunnery.saturating_add(1).min(100);
            }
            CrewClass::Gunner => {
                self.gunnery = self.gunnery.saturating_add(3).min(100);
                self.piloting = self.piloting.saturating_add(1).min(100);
            }
            CrewClass::Engineer => {
                self.engineering = self.engineering.saturating_add(3).min(100);
                self.leadership = self.leadership.saturating_add(1).min(100);
            }
            CrewClass::Medic => {
                self.engineering = self.engineering.saturating_add(2).min(100);
                self.leadership = self.leadership.saturating_add(2).min(100);
            }
            CrewClass::Captain => {
                self.leadership = self.leadership.saturating_add(2).min(100);
                self.piloting = self.piloting.saturating_add(1).min(100);
                self.gunnery = self.gunnery.saturating_add(1).min(100);
            }
            CrewClass::Navigator => {
                self.piloting = self.piloting.saturating_add(2).min(100);
                self.leadership = self.leadership.saturating_add(2).min(100);
            }
        }
    }

    /// One-line summary for UI display.
    pub fn summary(&self) -> String {
        format!(
            "Lv{} {} {} [P:{} G:{} E:{} L:{}] M:{}",
            self.level,
            self.class.name(),
            self.personality.name(),
            self.piloting,
            self.gunnery,
            self.engineering,
            self.leadership,
            self.morale,
        )
    }

    // ── Ability state management ───────────────────────────────────

    /// Reset all per-battle ability state. Call at the start of each battle.
    pub fn reset_battle_state(&mut self) {
        let count = self.class.abilities().len();
        self.abilities_used = vec![false; count];
        self.shot_counter = 0;
        self.ability_cooldowns = vec![0; count];
    }

    /// Tick all ability cooldowns down by 1.
    pub fn tick_cooldowns(&mut self) {
        for cd in &mut self.ability_cooldowns {
            if *cd > 0 {
                *cd -= 1;
            }
        }
    }

    /// Record that a shot was fired (for EveryNShots tracking).
    pub fn record_shot(&mut self) {
        self.shot_counter = self.shot_counter.wrapping_add(1);
    }

    /// Check which abilities should trigger given the current context.
    /// Returns indices and references to abilities that should activate this tick.
    /// Respects level requirements, cooldowns, and once-per-battle constraints.
    pub fn check_abilities(&self, context: &AbilityContext) -> Vec<(usize, CrewAbility)> {
        let abilities = self.class.abilities();
        let mut triggered = Vec::new();

        for (i, ability) in abilities.into_iter().enumerate() {
            // Level check
            if self.level < ability.level_required {
                continue;
            }

            // Cooldown check (ensure vectors are initialized)
            if i < self.ability_cooldowns.len() && self.ability_cooldowns[i] > 0 {
                continue;
            }

            // Once-per-battle check
            if matches!(ability.trigger, AbilityTrigger::OncePerBattle) {
                if i < self.abilities_used.len() && self.abilities_used[i] {
                    continue;
                }
            }

            // Trigger condition check
            let should_trigger = match ability.trigger {
                AbilityTrigger::OnBattleStart => context.battle_started,
                AbilityTrigger::OnHPBelow(threshold) => {
                    context.ship_hp_percent * 100.0 < threshold as f32
                }
                AbilityTrigger::EveryNShots(n) => {
                    context.shot_fired && n > 0 && self.shot_counter % n as u32 == 0
                }
                AbilityTrigger::OnAllyDestroyed => context.ally_just_destroyed,
                AbilityTrigger::OnEnemyKilled => context.enemy_just_killed,
                AbilityTrigger::OncePerBattle => {
                    // OncePerBattle abilities trigger on first eligible check
                    // (e.g., Afterburner triggers when the player/AI decides to use it)
                    // For auto-trigger: fire on battle start
                    context.battle_started
                }
                AbilityTrigger::Passive => true,
            };

            if should_trigger {
                triggered.push((i, ability));
            }
        }

        triggered
    }

    /// Mark an ability as activated: set cooldown and mark used if once-per-battle.
    pub fn activate_ability(&mut self, index: usize) {
        let abilities = self.class.abilities();
        if index >= abilities.len() {
            return;
        }

        // Ensure vectors are properly sized
        if self.ability_cooldowns.len() <= index {
            self.ability_cooldowns.resize(abilities.len(), 0);
        }
        if self.abilities_used.len() <= index {
            self.abilities_used.resize(abilities.len(), false);
        }

        let ability = &abilities[index];
        if ability.cooldown_max > 0 {
            self.ability_cooldowns[index] = ability.cooldown_max;
        }
        if matches!(ability.trigger, AbilityTrigger::OncePerBattle) {
            self.abilities_used[index] = true;
        }
    }
}

// ---------------------------------------------------------------------------
// Name generation
// ---------------------------------------------------------------------------

const FIRST_NAMES: &[&str] = &[
    "Kira", "Rex", "Nova", "Ash", "Zara", "Cole", "Luna", "Jax", "Vex", "Mira", "Finn", "Sage",
    "Orion", "Lyra", "Kai", "Nyx", "Echo", "Blaze", "Storm", "Drift", "Ember", "Frost", "Hawk",
    "Raven",
];

const LAST_NAMES: &[&str] = &[
    "Voss", "Kane", "Cross", "Steel", "Drake", "Stone", "Wolfe", "Blaze", "Frost", "Storm",
    "Blackwell", "Ashford", "Graves", "Raven", "Thorne", "Ward",
];

// ---------------------------------------------------------------------------
// Crew generation
// ---------------------------------------------------------------------------

fn roll_class(rng: &mut impl Rng) -> CrewClass {
    let weights: Vec<u32> = CrewClass::ALL.iter().map(|c| c.drop_weight()).collect();
    let dist = WeightedIndex::new(&weights).expect("valid weights");
    CrewClass::ALL[dist.sample(rng)]
}

fn roll_personality(rng: &mut impl Rng) -> Personality {
    Personality::ALL[rng.gen_range(0..Personality::ALL.len())]
}

/// Apply random variation (±5) and sector bonus to a base stat.
fn stat_with_variation(rng: &mut impl Rng, base: u8, sector_bonus: u8) -> u8 {
    let offset: i8 = rng.gen_range(-5..=5);
    let varied = (base as i8 + offset).clamp(1, 100) as u8;
    varied.saturating_add(sector_bonus).min(100)
}

/// Generate a crew member appropriate for the given sector.
pub fn generate_crew(sector: u32) -> CrewMember {
    let mut rng = rand::thread_rng();
    generate_crew_with_rng(&mut rng, sector)
}

/// Generate a crew member with a provided RNG (for deterministic testing).
pub fn generate_crew_with_rng(rng: &mut impl Rng, sector: u32) -> CrewMember {
    let first = FIRST_NAMES[rng.gen_range(0..FIRST_NAMES.len())];
    let last = LAST_NAMES[rng.gen_range(0..LAST_NAMES.len())];
    let name = format!("{} {}", first, last);

    let class = roll_class(rng);
    let personality = roll_personality(rng);

    // Level: 1 + sector/5, capped at 10
    let level = (1 + sector / 5).min(10) as u8;

    // Base stats from class
    let (bp, bg, be, bl) = class.base_stats();

    // Sector scaling: +1 per 3 sectors
    let sector_bonus = (sector / 3).min(20) as u8;

    // Random variation: ±5 + sector bonus
    let mut piloting = stat_with_variation(rng, bp, sector_bonus);
    let mut gunnery = stat_with_variation(rng, bg, sector_bonus);
    let mut engineering = stat_with_variation(rng, be, sector_bonus);
    let mut leadership = stat_with_variation(rng, bl, sector_bonus);

    // Simulate level-up stat gains for levels above 1
    for _ in 1..level {
        match class {
            CrewClass::Pilot => {
                piloting = piloting.saturating_add(3).min(100);
                gunnery = gunnery.saturating_add(1).min(100);
            }
            CrewClass::Gunner => {
                gunnery = gunnery.saturating_add(3).min(100);
                piloting = piloting.saturating_add(1).min(100);
            }
            CrewClass::Engineer => {
                engineering = engineering.saturating_add(3).min(100);
                leadership = leadership.saturating_add(1).min(100);
            }
            CrewClass::Medic => {
                engineering = engineering.saturating_add(2).min(100);
                leadership = leadership.saturating_add(2).min(100);
            }
            CrewClass::Captain => {
                leadership = leadership.saturating_add(2).min(100);
                piloting = piloting.saturating_add(1).min(100);
                gunnery = gunnery.saturating_add(1).min(100);
            }
            CrewClass::Navigator => {
                piloting = piloting.saturating_add(2).min(100);
                leadership = leadership.saturating_add(2).min(100);
            }
        }
    }

    CrewMember {
        id: 0, // assigned by GameState::add_crew
        name,
        class,
        level,
        xp: 0,
        personality,
        piloting,
        gunnery,
        engineering,
        leadership,
        morale: 70, // start at decent morale
        assigned_ship: None,
        kills: 0,
        battles_survived: 0,
        grief_battles_remaining: 0,
        vengeance_battles_remaining: 0,
        bonded_with_name: None,
        ability_cooldowns: Vec::new(),
        abilities_used: Vec::new(),
        shot_counter: 0,
    }
}

// ---------------------------------------------------------------------------
// Combat modifiers from crew
// ---------------------------------------------------------------------------

/// Damage modifier from an assigned crew member. Returns 1.0 if no crew.
pub fn crew_damage_modifier(crew: Option<&CrewMember>) -> f32 {
    let Some(c) = crew else { return 1.0 };
    let base = 1.0 + c.gunnery as f32 * 0.003; // up to +30% at 100 gunnery
    let personality = match c.personality {
        Personality::Aggressive => 1.10,
        Personality::Reckless => 1.20,
        Personality::Cautious => 0.95,
        _ => 1.0,
    };
    let morale = 0.8 + c.morale as f32 * 0.004; // 80%-120% based on morale

    // Grief/vengeance modifiers
    let grief_vengeance = if c.grief_battles_remaining > 0 {
        0.80 // -20% during grief
    } else if c.vengeance_battles_remaining > 0 {
        1.15 // +15% during vengeance
    } else {
        1.0
    };

    base * personality * morale * grief_vengeance
}

/// Dodge modifier from an assigned crew member. Returns 1.0 if no crew.
pub fn crew_dodge_modifier(crew: Option<&CrewMember>) -> f32 {
    let Some(c) = crew else { return 1.0 };
    let base = 1.0 + c.piloting as f32 * 0.003;
    let personality = match c.personality {
        Personality::Cautious => 1.10,
        Personality::Aggressive | Personality::Reckless => 0.95,
        _ => 1.0,
    };
    base * personality
}

/// Shield/engineering modifier from an assigned crew member. Returns 1.0 if no crew.
pub fn crew_shield_modifier(crew: Option<&CrewMember>) -> f32 {
    let Some(c) = crew else { return 1.0 };
    1.0 + c.engineering as f32 * 0.003
}

/// Leadership modifier — a fleet-wide minor bonus from crew leadership stat.
pub fn crew_leadership_modifier(crew: Option<&CrewMember>) -> f32 {
    let Some(c) = crew else { return 1.0 };
    1.0 + c.leadership as f32 * 0.001 // up to +10% at 100 leadership
}

// ---------------------------------------------------------------------------
// Personality synergies
// ---------------------------------------------------------------------------

/// Returns fleet-wide modifier when two personalities coexist in the same fleet.
/// Call for each pair of crew — multiply all pair results for total synergy.
pub fn personality_synergy(a: Personality, b: Personality) -> f32 {
    // Normalize order for symmetric matching
    let (x, y) = if (a as u8) <= (b as u8) { (a, b) } else { (b, a) };
    match (x, y) {
        (Personality::Aggressive, Personality::Cautious) => 0.95, // tension
        (Personality::Loyal, Personality::Brave) => 1.05,         // courage + loyalty
        (Personality::Loyal, Personality::Greedy) => 0.97,        // conflicting values
        (Personality::Nervous, Personality::Reckless) => 0.93,    // anxiety
        (Personality::Brave, Personality::Veteran) => 1.08,       // experience + courage
        _ => 1.0,
    }
}

/// Calculate total fleet synergy modifier from all crew personality pairs.
pub fn fleet_synergy(crew: &[&CrewMember]) -> f32 {
    let mut modifier = 1.0f32;
    for i in 0..crew.len() {
        for j in (i + 1)..crew.len() {
            modifier *= personality_synergy(crew[i].personality, crew[j].personality);
        }
    }
    modifier
}

// ---------------------------------------------------------------------------
// Crew Relationships (Bond System)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewBond {
    pub crew_a_id: u64,
    pub crew_b_id: u64,
    pub battles_together: u32,
    pub bond_type: BondType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BondType {
    None,
    Acquaintance,   // 5+ battles together
    BattleBrothers, // 10+ battles together → +3% damage when both alive
    Respect,        // 20+ battles with conflicting personalities → conflict penalty halved
    Rivals,         // conflicting personalities, <10 battles → -2% each
}

impl BondType {
    pub fn damage_modifier(&self) -> f32 {
        match self {
            BondType::BattleBrothers => 1.03,
            BondType::Rivals => 0.98,
            _ => 1.0,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            BondType::None => "Strangers",
            BondType::Acquaintance => "Acquaintances",
            BondType::BattleBrothers => "Battle Brothers (+3% DMG)",
            BondType::Respect => "Mutual Respect (conflict halved)",
            BondType::Rivals => "Rivals (-2% each)",
        }
    }
}

/// Returns true if two personalities are considered conflicting.
pub fn personalities_conflict(a: Personality, b: Personality) -> bool {
    let synergy = personality_synergy(a, b);
    synergy < 1.0
}

/// Determine bond type based on battles together and personality compatibility.
fn determine_bond_type(battles: u32, a_personality: Personality, b_personality: Personality) -> BondType {
    let conflict = personalities_conflict(a_personality, b_personality);

    if conflict && battles < 10 {
        BondType::Rivals
    } else if conflict && battles >= 20 {
        BondType::Respect
    } else if !conflict && battles >= 10 {
        BondType::BattleBrothers
    } else if battles >= 5 {
        BondType::Acquaintance
    } else {
        BondType::None
    }
}

/// Find a bond between two crew members. Returns the index in the bonds vec.
pub fn find_bond(bonds: &[CrewBond], a_id: u64, b_id: u64) -> Option<usize> {
    bonds.iter().position(|b| {
        (b.crew_a_id == a_id && b.crew_b_id == b_id)
            || (b.crew_a_id == b_id && b.crew_b_id == a_id)
    })
}

/// Get the bond type between two crew members.
pub fn get_bond_type(bonds: &[CrewBond], a_id: u64, b_id: u64) -> BondType {
    find_bond(bonds, a_id, b_id)
        .map(|idx| bonds[idx].bond_type)
        .unwrap_or(BondType::None)
}

/// Get the total bond damage modifier for a crew member based on all bonds
/// with other currently-assigned (alive) crew.
pub fn bond_damage_modifier(bonds: &[CrewBond], crew_id: u64, assigned_crew_ids: &[u64]) -> f32 {
    let mut modifier = 1.0f32;
    for &other_id in assigned_crew_ids {
        if other_id == crew_id {
            continue;
        }
        let bond_type = get_bond_type(bonds, crew_id, other_id);
        modifier *= bond_type.damage_modifier();
    }
    modifier
}

/// Update crew bonds after a battle. All assigned crew who survived gain
/// battles_together increments and bond type recalculation.
/// Returns a list of newly formed/changed bonds for event emission.
pub fn update_crew_bonds(
    crew_roster: &[CrewMember],
    bonds: &mut Vec<CrewBond>,
) -> Vec<(String, String, BondType)> {
    // Collect assigned crew (survived this battle)
    let assigned: Vec<(u64, Personality)> = crew_roster
        .iter()
        .filter(|c| c.assigned_ship.is_some())
        .map(|c| (c.id, c.personality))
        .collect();

    let mut changed = Vec::new();

    // For every pair of assigned crew
    for i in 0..assigned.len() {
        for j in (i + 1)..assigned.len() {
            let (id_a, pers_a) = assigned[i];
            let (id_b, pers_b) = assigned[j];

            let bond_idx = find_bond(bonds, id_a, id_b);
            match bond_idx {
                Some(idx) => {
                    let old_type = bonds[idx].bond_type;
                    bonds[idx].battles_together += 1;
                    let new_type = determine_bond_type(bonds[idx].battles_together, pers_a, pers_b);
                    bonds[idx].bond_type = new_type;
                    if old_type != new_type {
                        let name_a = crew_roster.iter().find(|c| c.id == id_a)
                            .map(|c| c.name.clone()).unwrap_or_default();
                        let name_b = crew_roster.iter().find(|c| c.id == id_b)
                            .map(|c| c.name.clone()).unwrap_or_default();
                        changed.push((name_a, name_b, new_type));
                    }
                }
                None => {
                    let new_type = determine_bond_type(1, pers_a, pers_b);
                    bonds.push(CrewBond {
                        crew_a_id: id_a,
                        crew_b_id: id_b,
                        battles_together: 1,
                        bond_type: new_type,
                    });
                    if new_type != BondType::None {
                        let name_a = crew_roster.iter().find(|c| c.id == id_a)
                            .map(|c| c.name.clone()).unwrap_or_default();
                        let name_b = crew_roster.iter().find(|c| c.id == id_b)
                            .map(|c| c.name.clone()).unwrap_or_default();
                        changed.push((name_a, name_b, new_type));
                    }
                }
            }
        }
    }

    changed
}

/// Process grief/vengeance state after a battle tick.
/// Call this for each crew member after a battle.
/// Returns events: (crew_name, event_type) where event_type is "grief_end" or "vengeance_end".
pub fn tick_grief_vengeance(crew: &mut CrewMember) -> Option<&'static str> {
    if crew.grief_battles_remaining > 0 {
        crew.grief_battles_remaining -= 1;
        if crew.grief_battles_remaining == 0 {
            // Grief ends → vengeance begins
            crew.vengeance_battles_remaining = 10;
            crew.morale = crew.morale.saturating_add(10).min(100);
            return Some("vengeance_start");
        }
    } else if crew.vengeance_battles_remaining > 0 {
        crew.vengeance_battles_remaining -= 1;
        if crew.vengeance_battles_remaining == 0 {
            crew.bonded_with_name = None;
            return Some("vengeance_end");
        }
    }
    None
}

/// Trigger grief on a crew member when their battle brother dies.
pub fn trigger_grief(crew: &mut CrewMember, fallen_name: &str) {
    crew.grief_battles_remaining = 5;
    crew.vengeance_battles_remaining = 0;
    crew.morale = crew.morale.saturating_sub(20);
    crew.bonded_with_name = Some(fallen_name.to_string());
}

/// Check if a crew member is eligible for a specific crew event type.
pub fn crew_eligible_spotter(crew: &CrewMember) -> bool {
    crew.piloting > 40 || matches!(crew.class, CrewClass::Pilot | CrewClass::Navigator)
}

pub fn crew_eligible_challenger(crew: &CrewMember) -> bool {
    matches!(crew.personality, Personality::Aggressive | Personality::Brave)
}

pub fn crew_eligible_signal_detector(crew: &CrewMember) -> bool {
    matches!(crew.class, CrewClass::Engineer | CrewClass::Navigator)
}

pub fn crew_eligible_homesick(crew: &CrewMember) -> bool {
    crew.morale < 40
}

/// Find two crew members with conflicting personalities.
pub fn find_conflicting_pair(roster: &[CrewMember]) -> Option<(usize, usize)> {
    for i in 0..roster.len() {
        for j in (i + 1)..roster.len() {
            if roster[i].assigned_ship.is_some()
                && roster[j].assigned_ship.is_some()
                && personalities_conflict(roster[i].personality, roster[j].personality)
            {
                return Some((i, j));
            }
        }
    }
    None
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

    // ── Generation tests ───────────────────────────────────────────

    #[test]
    fn generate_crew_has_name() {
        let mut rng = test_rng();
        let crew = generate_crew_with_rng(&mut rng, 1);
        assert!(crew.name.contains(' '), "name should have first + last");
        assert!(crew.name.len() >= 5);
    }

    #[test]
    fn generate_crew_sector_1_level() {
        let mut rng = test_rng();
        for _ in 0..100 {
            let crew = generate_crew_with_rng(&mut rng, 1);
            assert_eq!(crew.level, 1, "sector 1 crew should be level 1");
        }
    }

    #[test]
    fn generate_crew_higher_sector_higher_level() {
        let mut rng = test_rng();
        let crew_s1 = generate_crew_with_rng(&mut rng, 1);
        let crew_s25 = generate_crew_with_rng(&mut rng, 25);
        assert!(crew_s25.level > crew_s1.level);
    }

    #[test]
    fn generate_crew_level_capped_at_10() {
        let mut rng = test_rng();
        let crew = generate_crew_with_rng(&mut rng, 100);
        assert_eq!(crew.level, 10);
    }

    #[test]
    fn generate_crew_stats_in_range() {
        let mut rng = test_rng();
        for _ in 0..200 {
            let crew = generate_crew_with_rng(&mut rng, 15);
            assert!(crew.piloting >= 1 && crew.piloting <= 100);
            assert!(crew.gunnery >= 1 && crew.gunnery <= 100);
            assert!(crew.engineering >= 1 && crew.engineering <= 100);
            assert!(crew.leadership >= 1 && crew.leadership <= 100);
            assert!(crew.morale <= 100);
        }
    }

    #[test]
    fn generate_crew_class_specialization() {
        // Pilots should generally have higher piloting than gunnery
        let mut rng = test_rng();
        let mut pilot_higher = 0u32;
        let total = 200;
        for _ in 0..total {
            let crew = generate_crew_with_rng(&mut rng, 1);
            if crew.class == CrewClass::Pilot && crew.piloting > crew.gunnery {
                pilot_higher += 1;
            }
        }
        // At least some should be pilots with higher piloting
        assert!(pilot_higher > 0, "pilots should favor piloting stat");
    }

    #[test]
    fn generate_crew_class_distribution() {
        let mut rng = test_rng();
        let mut counts = [0u32; CrewClass::ALL.len()];
        let n = 5000;
        for _ in 0..n {
            let crew = generate_crew_with_rng(&mut rng, 5);
            let idx = CrewClass::ALL
                .iter()
                .position(|c| *c == crew.class)
                .unwrap();
            counts[idx] += 1;
        }
        // All classes should appear
        for (i, count) in counts.iter().enumerate() {
            assert!(
                *count > 0,
                "class {:?} had zero generations",
                CrewClass::ALL[i]
            );
        }
        // Captain should be rarest
        let captain_idx = CrewClass::ALL
            .iter()
            .position(|c| *c == CrewClass::Captain)
            .unwrap();
        let pilot_idx = CrewClass::ALL
            .iter()
            .position(|c| *c == CrewClass::Pilot)
            .unwrap();
        assert!(
            counts[captain_idx] < counts[pilot_idx],
            "Captain ({}) should be rarer than Pilot ({})",
            counts[captain_idx],
            counts[pilot_idx],
        );
    }

    #[test]
    fn generate_crew_starts_unassigned() {
        let mut rng = test_rng();
        let crew = generate_crew_with_rng(&mut rng, 5);
        assert!(crew.assigned_ship.is_none());
        assert_eq!(crew.kills, 0);
        assert_eq!(crew.battles_survived, 0);
    }

    // ── XP and leveling tests ──────────────────────────────────────

    #[test]
    fn xp_to_next_scales() {
        let mut rng = test_rng();
        let mut crew = generate_crew_with_rng(&mut rng, 1);
        crew.level = 1;
        let xp_1 = crew.xp_to_next();
        crew.level = 5;
        let xp_5 = crew.xp_to_next();
        assert!(xp_5 > xp_1, "higher level needs more XP");
    }

    #[test]
    fn add_xp_levels_up() {
        let mut rng = test_rng();
        let mut crew = generate_crew_with_rng(&mut rng, 1);
        crew.level = 1;
        crew.xp = 0;
        let initial_piloting = crew.piloting;
        let needed = crew.xp_to_next();
        let leveled = crew.add_xp(needed);
        assert!(leveled);
        assert_eq!(crew.level, 2);
        // Stats should have increased if class benefits
        if crew.class == CrewClass::Pilot {
            assert!(crew.piloting > initial_piloting);
        }
    }

    #[test]
    fn add_xp_multi_level() {
        let mut rng = test_rng();
        let mut crew = generate_crew_with_rng(&mut rng, 1);
        crew.level = 1;
        crew.xp = 0;
        // Give a huge amount of XP
        crew.add_xp(100_000);
        assert!(crew.level > 2, "should have gained multiple levels");
    }

    #[test]
    fn level_cap_at_20() {
        let mut rng = test_rng();
        let mut crew = generate_crew_with_rng(&mut rng, 1);
        crew.level = 1;
        crew.xp = 0;
        crew.add_xp(10_000_000); // massive XP
        assert_eq!(crew.level, 20);
    }

    #[test]
    fn stats_capped_at_100() {
        let mut rng = test_rng();
        let mut crew = generate_crew_with_rng(&mut rng, 50);
        crew.piloting = 98;
        crew.gunnery = 98;
        crew.engineering = 98;
        crew.leadership = 98;
        crew.level = 1;
        crew.xp = 0;
        crew.add_xp(10_000_000);
        assert!(crew.piloting <= 100);
        assert!(crew.gunnery <= 100);
        assert!(crew.engineering <= 100);
        assert!(crew.leadership <= 100);
    }

    // ── Combat modifier tests ──────────────────────────────────────

    #[test]
    fn crew_damage_modifier_none() {
        assert!((crew_damage_modifier(None) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn crew_dodge_modifier_none() {
        assert!((crew_dodge_modifier(None) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn crew_shield_modifier_none() {
        assert!((crew_shield_modifier(None) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn aggressive_boosts_damage() {
        let crew = CrewMember {
            id: 1,
            name: "Test".into(),
            class: CrewClass::Gunner,
            level: 5,
            xp: 0,
            personality: Personality::Aggressive,
            piloting: 20,
            gunnery: 50,
            engineering: 10,
            leadership: 10,
            morale: 70,
            assigned_ship: Some(0),
            kills: 0,
            battles_survived: 0,
            ability_cooldowns: vec![],
            abilities_used: vec![],
            shot_counter: 0,
            grief_battles_remaining: 0,
            vengeance_battles_remaining: 0,
            bonded_with_name: None,
        };
        let dmg = crew_damage_modifier(Some(&crew));
        // Aggressive gives 1.10 personality bonus
        assert!(dmg > 1.0, "aggressive should boost damage, got {}", dmg);

        let mut cautious = crew.clone();
        cautious.personality = Personality::Cautious;
        let dmg_cautious = crew_damage_modifier(Some(&cautious));
        assert!(
            dmg > dmg_cautious,
            "aggressive ({}) should beat cautious ({})",
            dmg,
            dmg_cautious
        );
    }

    #[test]
    fn cautious_boosts_dodge() {
        let crew = CrewMember {
            id: 1,
            name: "Test".into(),
            class: CrewClass::Pilot,
            level: 5,
            xp: 0,
            personality: Personality::Cautious,
            piloting: 50,
            gunnery: 20,
            engineering: 10,
            leadership: 10,
            morale: 70,
            assigned_ship: Some(0),
            kills: 0,
            battles_survived: 0,
            ability_cooldowns: vec![],
            abilities_used: vec![],
            shot_counter: 0,
            grief_battles_remaining: 0,
            vengeance_battles_remaining: 0,
            bonded_with_name: None,
        };
        let dodge = crew_dodge_modifier(Some(&crew));
        assert!(dodge > 1.0, "cautious should boost dodge, got {}", dodge);

        let mut aggressive = crew.clone();
        aggressive.personality = Personality::Aggressive;
        let dodge_agg = crew_dodge_modifier(Some(&aggressive));
        assert!(
            dodge > dodge_agg,
            "cautious ({}) should dodge better than aggressive ({})",
            dodge,
            dodge_agg
        );
    }

    #[test]
    fn high_gunnery_more_damage() {
        let make = |gunnery: u8| CrewMember {
            id: 1,
            name: "Test".into(),
            class: CrewClass::Gunner,
            level: 5,
            xp: 0,
            personality: Personality::Brave,
            piloting: 10,
            gunnery,
            engineering: 10,
            leadership: 10,
            morale: 70,
            assigned_ship: Some(0),
            kills: 0,
            battles_survived: 0,
            ability_cooldowns: vec![],
            abilities_used: vec![],
            shot_counter: 0,
            grief_battles_remaining: 0,
            vengeance_battles_remaining: 0,
            bonded_with_name: None,
        };
        let low = crew_damage_modifier(Some(&make(10)));
        let high = crew_damage_modifier(Some(&make(90)));
        assert!(high > low, "90 gunnery ({}) > 10 gunnery ({})", high, low);
    }

    #[test]
    fn morale_affects_damage() {
        let make = |morale: u8| CrewMember {
            id: 1,
            name: "Test".into(),
            class: CrewClass::Gunner,
            level: 5,
            xp: 0,
            personality: Personality::Brave,
            piloting: 10,
            gunnery: 50,
            engineering: 10,
            leadership: 10,
            morale,
            assigned_ship: Some(0),
            kills: 0,
            battles_survived: 0,
            ability_cooldowns: vec![],
            abilities_used: vec![],
            shot_counter: 0,
            grief_battles_remaining: 0,
            vengeance_battles_remaining: 0,
            bonded_with_name: None,
        };
        let low_morale = crew_damage_modifier(Some(&make(10)));
        let high_morale = crew_damage_modifier(Some(&make(90)));
        assert!(
            high_morale > low_morale,
            "high morale ({}) should boost dmg over low ({})",
            high_morale,
            low_morale
        );
    }

    #[test]
    fn engineering_boosts_shields() {
        let make = |eng: u8| CrewMember {
            id: 1,
            name: "Test".into(),
            class: CrewClass::Engineer,
            level: 5,
            xp: 0,
            personality: Personality::Brave,
            piloting: 10,
            gunnery: 10,
            engineering: eng,
            leadership: 10,
            morale: 70,
            assigned_ship: Some(0),
            kills: 0,
            battles_survived: 0,
            ability_cooldowns: vec![],
            abilities_used: vec![],
            shot_counter: 0,
            grief_battles_remaining: 0,
            vengeance_battles_remaining: 0,
            bonded_with_name: None,
        };
        let low = crew_shield_modifier(Some(&make(10)));
        let high = crew_shield_modifier(Some(&make(90)));
        assert!(high > low, "90 eng ({}) > 10 eng ({})", high, low);
    }

    // ── Synergy tests ──────────────────────────────────────────────

    #[test]
    fn brave_loyal_synergy() {
        let s = personality_synergy(Personality::Brave, Personality::Loyal);
        assert!((s - 1.05).abs() < f32::EPSILON);
        // Symmetric
        let s2 = personality_synergy(Personality::Loyal, Personality::Brave);
        assert!((s2 - 1.05).abs() < f32::EPSILON);
    }

    #[test]
    fn aggressive_cautious_conflict() {
        let s = personality_synergy(Personality::Aggressive, Personality::Cautious);
        assert!((s - 0.95).abs() < f32::EPSILON);
    }

    #[test]
    fn reckless_nervous_conflict() {
        let s = personality_synergy(Personality::Nervous, Personality::Reckless);
        assert!((s - 0.93).abs() < f32::EPSILON);
    }

    #[test]
    fn neutral_synergy() {
        let s = personality_synergy(Personality::Aggressive, Personality::Aggressive);
        assert!((s - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn fleet_synergy_empty() {
        let crew: Vec<&CrewMember> = vec![];
        let s = fleet_synergy(&crew);
        assert!((s - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn fleet_synergy_single() {
        let c = CrewMember {
            id: 1,
            name: "Test".into(),
            class: CrewClass::Pilot,
            level: 1,
            xp: 0,
            personality: Personality::Brave,
            piloting: 30,
            gunnery: 15,
            engineering: 8,
            leadership: 5,
            morale: 70,
            assigned_ship: None,
            kills: 0,
            battles_survived: 0,
            ability_cooldowns: vec![],
            abilities_used: vec![],
            shot_counter: 0,
            grief_battles_remaining: 0,
            vengeance_battles_remaining: 0,
            bonded_with_name: None,
        };
        let s = fleet_synergy(&[&c]);
        assert!((s - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn fleet_synergy_positive_pair() {
        let brave = CrewMember {
            id: 1,
            name: "A".into(),
            class: CrewClass::Pilot,
            level: 1,
            xp: 0,
            personality: Personality::Brave,
            piloting: 30,
            gunnery: 15,
            engineering: 8,
            leadership: 5,
            morale: 70,
            assigned_ship: None,
            kills: 0,
            battles_survived: 0,
            ability_cooldowns: vec![],
            abilities_used: vec![],
            shot_counter: 0,
            grief_battles_remaining: 0,
            vengeance_battles_remaining: 0,
            bonded_with_name: None,
        };
        let loyal = CrewMember {
            personality: Personality::Loyal,
            id: 2,
            name: "B".into(),
            ..brave.clone()
        };
        let s = fleet_synergy(&[&brave, &loyal]);
        assert!(s > 1.0, "brave + loyal should be positive synergy: {}", s);
    }

    #[test]
    fn fleet_synergy_multiple_pairs() {
        let make = |p: Personality, id: u64| CrewMember {
            id,
            name: format!("Crew{}", id),
            class: CrewClass::Pilot,
            level: 1,
            xp: 0,
            personality: p,
            piloting: 30,
            gunnery: 15,
            engineering: 8,
            leadership: 5,
            morale: 70,
            assigned_ship: None,
            kills: 0,
            battles_survived: 0,
            ability_cooldowns: vec![],
            abilities_used: vec![],
            shot_counter: 0,
            grief_battles_remaining: 0,
            vengeance_battles_remaining: 0,
            bonded_with_name: None,
        };
        let brave = make(Personality::Brave, 1);
        let loyal = make(Personality::Loyal, 2);
        let veteran = make(Personality::Veteran, 3);
        // brave+loyal = 1.05, brave+veteran = 1.08, loyal+veteran = 1.0
        let s = fleet_synergy(&[&brave, &loyal, &veteran]);
        let expected = 1.05 * 1.08 * 1.0;
        assert!(
            (s - expected).abs() < 0.001,
            "expected ~{}, got {}",
            expected,
            s
        );
    }

    // ── Serialization test ─────────────────────────────────────────

    #[test]
    fn serialization_roundtrip() {
        let mut rng = test_rng();
        let crew = generate_crew_with_rng(&mut rng, 10);
        let json = serde_json::to_string(&crew).expect("serialize");
        let deserialized: CrewMember = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.id, crew.id);
        assert_eq!(deserialized.name, crew.name);
        assert_eq!(deserialized.class, crew.class);
        assert_eq!(deserialized.level, crew.level);
        assert_eq!(deserialized.personality, crew.personality);
    }

    // ── Summary test ───────────────────────────────────────────────

    #[test]
    fn summary_non_empty() {
        let mut rng = test_rng();
        let crew = generate_crew_with_rng(&mut rng, 5);
        let summary = crew.summary();
        assert!(!summary.is_empty());
        assert!(summary.contains("Lv"));
    }

    // ── Bond system tests ──────────────────────────────────────

    fn make_crew(id: u64, personality: Personality, assigned: bool) -> CrewMember {
        let mut crew = generate_crew_with_rng(&mut test_rng(), 5);
        crew.id = id;
        crew.name = format!("Crew{}", id);
        crew.class = CrewClass::Gunner;
        crew.personality = personality;
        crew.piloting = 30;
        crew.gunnery = 50;
        crew.engineering = 10;
        crew.leadership = 10;
        crew.morale = 70;
        crew.assigned_ship = if assigned { Some(0) } else { None };
        crew.kills = 0;
        crew.battles_survived = 0;
        crew.grief_battles_remaining = 0;
        crew.vengeance_battles_remaining = 0;
        crew.bonded_with_name = None;
        crew
    }

    #[test]
    fn bond_starts_as_none() {
        let bonds: Vec<CrewBond> = vec![];
        assert_eq!(get_bond_type(&bonds, 1, 2), BondType::None);
    }

    #[test]
    fn bond_progression_to_acquaintance() {
        let roster = vec![
            make_crew(1, Personality::Brave, true),
            make_crew(2, Personality::Loyal, true),
        ];
        let mut bonds = Vec::new();
        // 5 battles together → Acquaintance
        for _ in 0..5 {
            update_crew_bonds(&roster, &mut bonds);
        }
        assert_eq!(bonds.len(), 1);
        assert_eq!(bonds[0].battles_together, 5);
        assert_eq!(bonds[0].bond_type, BondType::Acquaintance);
    }

    #[test]
    fn bond_progression_to_battle_brothers() {
        let roster = vec![
            make_crew(1, Personality::Brave, true),
            make_crew(2, Personality::Loyal, true),
        ];
        let mut bonds = Vec::new();
        for _ in 0..10 {
            update_crew_bonds(&roster, &mut bonds);
        }
        assert_eq!(bonds[0].bond_type, BondType::BattleBrothers);
    }

    #[test]
    fn conflicting_personalities_become_rivals() {
        let roster = vec![
            make_crew(1, Personality::Aggressive, true),
            make_crew(2, Personality::Cautious, true),
        ];
        let mut bonds = Vec::new();
        update_crew_bonds(&roster, &mut bonds);
        // First battle with conflicting personalities → Rivals (< 10 battles)
        assert_eq!(bonds[0].bond_type, BondType::Rivals);
    }

    #[test]
    fn conflicting_personalities_gain_respect() {
        let roster = vec![
            make_crew(1, Personality::Aggressive, true),
            make_crew(2, Personality::Cautious, true),
        ];
        let mut bonds = Vec::new();
        for _ in 0..20 {
            update_crew_bonds(&roster, &mut bonds);
        }
        assert_eq!(bonds[0].bond_type, BondType::Respect);
    }

    #[test]
    fn bond_damage_modifier_battle_brothers() {
        let bonds = vec![CrewBond {
            crew_a_id: 1,
            crew_b_id: 2,
            battles_together: 10,
            bond_type: BondType::BattleBrothers,
        }];
        let mod_val = bond_damage_modifier(&bonds, 1, &[1, 2]);
        assert!((mod_val - 1.03).abs() < 0.001);
    }

    #[test]
    fn bond_damage_modifier_rivals() {
        let bonds = vec![CrewBond {
            crew_a_id: 1,
            crew_b_id: 2,
            battles_together: 3,
            bond_type: BondType::Rivals,
        }];
        let mod_val = bond_damage_modifier(&bonds, 1, &[1, 2]);
        assert!((mod_val - 0.98).abs() < 0.001);
    }

    #[test]
    fn bond_changes_reported() {
        let roster = vec![
            make_crew(1, Personality::Brave, true),
            make_crew(2, Personality::Loyal, true),
        ];
        let mut bonds = Vec::new();
        // First 4 battles: no bond change reported (None → None)
        for _ in 0..4 {
            let changes = update_crew_bonds(&roster, &mut bonds);
            assert!(changes.is_empty() || changes[0].2 == BondType::None);
        }
        // 5th battle: should transition to Acquaintance
        let changes = update_crew_bonds(&roster, &mut bonds);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].2, BondType::Acquaintance);
    }

    #[test]
    fn unassigned_crew_dont_bond() {
        let roster = vec![
            make_crew(1, Personality::Brave, true),
            make_crew(2, Personality::Loyal, false), // unassigned
        ];
        let mut bonds = Vec::new();
        for _ in 0..10 {
            update_crew_bonds(&roster, &mut bonds);
        }
        assert!(bonds.is_empty());
    }

    // ── Grief/vengeance tests ──────────────────────────────────

    #[test]
    fn grief_reduces_damage() {
        let mut crew = make_crew(1, Personality::Brave, true);
        crew.grief_battles_remaining = 3;
        let dmg = crew_damage_modifier(Some(&crew));
        let mut normal = make_crew(2, Personality::Brave, true);
        normal.morale = crew.morale;
        normal.gunnery = crew.gunnery;
        let normal_dmg = crew_damage_modifier(Some(&normal));
        assert!(dmg < normal_dmg, "grief should reduce damage: {} vs {}", dmg, normal_dmg);
    }

    #[test]
    fn vengeance_boosts_damage() {
        let mut crew = make_crew(1, Personality::Brave, true);
        crew.vengeance_battles_remaining = 5;
        let dmg = crew_damage_modifier(Some(&crew));
        let mut normal = make_crew(2, Personality::Brave, true);
        normal.morale = crew.morale;
        normal.gunnery = crew.gunnery;
        let normal_dmg = crew_damage_modifier(Some(&normal));
        assert!(dmg > normal_dmg, "vengeance should boost damage: {} vs {}", dmg, normal_dmg);
    }

    #[test]
    fn grief_transitions_to_vengeance() {
        let mut crew = make_crew(1, Personality::Brave, true);
        trigger_grief(&mut crew, "FallenHero");
        assert_eq!(crew.grief_battles_remaining, 5);
        assert_eq!(crew.morale, 50); // 70 - 20

        // Tick through grief
        for i in 0..4 {
            let result = tick_grief_vengeance(&mut crew);
            assert!(result.is_none(), "no event at grief tick {}", i);
        }
        // 5th tick: grief ends → vengeance starts
        let result = tick_grief_vengeance(&mut crew);
        assert_eq!(result, Some("vengeance_start"));
        assert_eq!(crew.grief_battles_remaining, 0);
        assert_eq!(crew.vengeance_battles_remaining, 10);
    }

    #[test]
    fn vengeance_expires() {
        let mut crew = make_crew(1, Personality::Brave, true);
        crew.vengeance_battles_remaining = 1;
        crew.bonded_with_name = Some("OldFriend".to_string());
        let result = tick_grief_vengeance(&mut crew);
        assert_eq!(result, Some("vengeance_end"));
        assert_eq!(crew.vengeance_battles_remaining, 0);
        assert!(crew.bonded_with_name.is_none());
    }

    // ── Event eligibility tests ────────────────────────────────

    #[test]
    fn spotter_eligibility() {
        let mut crew = make_crew(1, Personality::Brave, true);
        crew.piloting = 50;
        assert!(crew_eligible_spotter(&crew));

        crew.piloting = 30;
        crew.class = CrewClass::Pilot;
        assert!(crew_eligible_spotter(&crew));

        crew.piloting = 30;
        crew.class = CrewClass::Gunner;
        assert!(!crew_eligible_spotter(&crew));
    }

    #[test]
    fn challenger_eligibility() {
        let crew_agg = make_crew(1, Personality::Aggressive, true);
        assert!(crew_eligible_challenger(&crew_agg));

        let crew_brave = make_crew(2, Personality::Brave, true);
        assert!(crew_eligible_challenger(&crew_brave));

        let crew_cautious = make_crew(3, Personality::Cautious, true);
        assert!(!crew_eligible_challenger(&crew_cautious));
    }

    #[test]
    fn homesick_eligibility() {
        let mut crew = make_crew(1, Personality::Nervous, true);
        crew.morale = 30;
        assert!(crew_eligible_homesick(&crew));

        crew.morale = 50;
        assert!(!crew_eligible_homesick(&crew));
    }

    #[test]
    fn find_conflicting_pair_works() {
        let roster = vec![
            make_crew(1, Personality::Aggressive, true),
            make_crew(2, Personality::Cautious, true),
        ];
        assert!(find_conflicting_pair(&roster).is_some());

        let roster2 = vec![
            make_crew(1, Personality::Brave, true),
            make_crew(2, Personality::Loyal, true),
        ];
        assert!(find_conflicting_pair(&roster2).is_none());
    }

    #[test]
    fn find_conflicting_pair_requires_assigned() {
        let roster = vec![
            make_crew(1, Personality::Aggressive, true),
            make_crew(2, Personality::Cautious, false), // unassigned
        ];
        assert!(find_conflicting_pair(&roster).is_none());
    }

    // ── Navigator class tests ──────────────────────────────────────

    #[test]
    fn navigator_in_all_classes() {
        assert!(CrewClass::ALL.contains(&CrewClass::Navigator));
    }

    #[test]
    fn navigator_name_and_icon() {
        assert_eq!(CrewClass::Navigator.name(), "Navigator");
        assert_eq!(CrewClass::Navigator.icon(), "🧭");
    }

    #[test]
    fn navigator_base_stats() {
        let (p, g, e, l) = CrewClass::Navigator.base_stats();
        assert_eq!(p, 35); // piloting primary
        assert_eq!(g, 10);
        assert_eq!(e, 15);
        assert_eq!(l, 25); // leadership secondary
    }

    #[test]
    fn navigator_drop_weight_matches_medic() {
        assert_eq!(
            CrewClass::Navigator.drop_weight(),
            CrewClass::Medic.drop_weight()
        );
    }

    #[test]
    fn navigator_levelup_stats() {
        let mut crew = make_crew(1, Personality::Brave, true);
        crew.class = CrewClass::Navigator;
        crew.piloting = 35;
        crew.leadership = 25;
        crew.level = 1;
        crew.xp = 0;
        let old_piloting = crew.piloting;
        let old_leadership = crew.leadership;
        crew.add_xp(crew.xp_to_next());
        assert_eq!(crew.piloting, old_piloting + 2);
        assert_eq!(crew.leadership, old_leadership + 2);
    }

    #[test]
    fn navigator_generates_in_pool() {
        let mut rng = test_rng();
        let mut found_navigator = false;
        for _ in 0..5000 {
            let crew = generate_crew_with_rng(&mut rng, 10);
            if crew.class == CrewClass::Navigator {
                found_navigator = true;
                break;
            }
        }
        assert!(found_navigator, "Navigator should appear in generation pool");
    }

    #[test]
    fn navigator_combat_modifiers() {
        let mut crew = make_crew(1, Personality::Brave, true);
        crew.class = CrewClass::Navigator;
        crew.piloting = 50;
        crew.gunnery = 10;
        // Navigator should have neutral-ish damage (low gunnery)
        let dmg = crew_damage_modifier(Some(&crew));
        assert!(dmg > 0.8 && dmg < 1.5, "Navigator damage modifier: {}", dmg);
        // Navigator should have decent dodge (high piloting)
        let dodge = crew_dodge_modifier(Some(&crew));
        assert!(dodge > 1.0, "Navigator should have dodge bonus: {}", dodge);
    }

    // ── Ability definition tests ───────────────────────────────────

    #[test]
    fn all_classes_have_abilities() {
        for class in CrewClass::ALL {
            let abilities = class.abilities();
            assert!(
                !abilities.is_empty(),
                "{:?} should have at least one ability",
                class
            );
        }
    }

    #[test]
    fn pilot_has_two_abilities() {
        let abilities = CrewClass::Pilot.abilities();
        assert_eq!(abilities.len(), 2);
        assert_eq!(abilities[0].name, "Evasive Maneuvers");
        assert_eq!(abilities[1].name, "Afterburner");
    }

    #[test]
    fn gunner_has_two_abilities() {
        let abilities = CrewClass::Gunner.abilities();
        assert_eq!(abilities.len(), 2);
        assert_eq!(abilities[0].name, "Lock-On");
        assert_eq!(abilities[1].name, "Barrage");
    }

    #[test]
    fn engineer_has_three_abilities() {
        let abilities = CrewClass::Engineer.abilities();
        assert_eq!(abilities.len(), 3);
        assert_eq!(abilities[0].name, "Passive Regen");
        assert_eq!(abilities[1].name, "Emergency Repair");
        assert_eq!(abilities[2].name, "Shield Overclock");
    }

    #[test]
    fn medic_has_two_abilities() {
        let abilities = CrewClass::Medic.abilities();
        assert_eq!(abilities.len(), 2);
        assert_eq!(abilities[0].name, "Triage");
        assert_eq!(abilities[1].name, "Revive");
    }

    #[test]
    fn captain_has_two_abilities() {
        let abilities = CrewClass::Captain.abilities();
        assert_eq!(abilities.len(), 2);
        assert_eq!(abilities[0].name, "Rally");
        assert_eq!(abilities[1].name, "Inspire");
    }

    #[test]
    fn navigator_has_two_abilities() {
        let abilities = CrewClass::Navigator.abilities();
        assert_eq!(abilities.len(), 2);
        assert_eq!(abilities[0].name, "Stellar Cartography");
        assert_eq!(abilities[1].name, "Shortcut");
    }

    #[test]
    fn available_abilities_level_gating() {
        // Engineer: Passive Regen at lv3, Emergency Repair at lv5, Shield Overclock at lv10
        assert_eq!(CrewClass::Engineer.available_abilities(1).len(), 0);
        assert_eq!(CrewClass::Engineer.available_abilities(3).len(), 1);
        assert_eq!(CrewClass::Engineer.available_abilities(5).len(), 2);
        assert_eq!(CrewClass::Engineer.available_abilities(10).len(), 3);
    }

    #[test]
    fn available_abilities_at_max_level() {
        for class in CrewClass::ALL {
            let all = class.abilities();
            let available = class.available_abilities(20);
            assert_eq!(
                available.len(),
                all.len(),
                "{:?} at level 20 should have all abilities",
                class
            );
        }
    }

    // ── Ability state tracking tests ───────────────────────────────

    #[test]
    fn reset_battle_state_initializes() {
        let mut crew = make_crew(1, Personality::Brave, true);
        crew.class = CrewClass::Engineer; // 3 abilities
        crew.shot_counter = 99;
        crew.reset_battle_state();
        let count = CrewClass::Engineer.abilities().len();
        assert_eq!(crew.ability_cooldowns.len(), count);
        assert_eq!(crew.abilities_used.len(), count);
        assert_eq!(crew.shot_counter, 0);
        assert!(crew.ability_cooldowns.iter().all(|&cd| cd == 0));
        assert!(crew.abilities_used.iter().all(|&u| !u));
    }

    #[test]
    fn tick_cooldowns_decrements() {
        let mut crew = make_crew(1, Personality::Brave, true);
        crew.ability_cooldowns = vec![5, 0, 3];
        crew.tick_cooldowns();
        assert_eq!(crew.ability_cooldowns, vec![4, 0, 2]);
        crew.tick_cooldowns();
        assert_eq!(crew.ability_cooldowns, vec![3, 0, 1]);
    }

    #[test]
    fn tick_cooldowns_floors_at_zero() {
        let mut crew = make_crew(1, Personality::Brave, true);
        crew.ability_cooldowns = vec![1, 0];
        crew.tick_cooldowns();
        assert_eq!(crew.ability_cooldowns, vec![0, 0]);
        crew.tick_cooldowns(); // should not underflow
        assert_eq!(crew.ability_cooldowns, vec![0, 0]);
    }

    #[test]
    fn record_shot_increments() {
        let mut crew = make_crew(1, Personality::Brave, true);
        assert_eq!(crew.shot_counter, 0);
        crew.record_shot();
        assert_eq!(crew.shot_counter, 1);
        crew.record_shot();
        assert_eq!(crew.shot_counter, 2);
    }

    #[test]
    fn check_abilities_passive_triggers_always() {
        let mut crew = make_crew(1, Personality::Brave, true);
        crew.class = CrewClass::Engineer;
        crew.level = 5; // has Passive Regen (lv3) and Emergency Repair (lv5)
        crew.reset_battle_state();
        let ctx = AbilityContext {
            ship_hp_percent: 0.80, // healthy
            ..AbilityContext::default()
        };
        let triggered = crew.check_abilities(&ctx);
        // Passive Regen should trigger (passive, no HP condition)
        assert!(
            triggered.iter().any(|(_, a)| a.name == "Passive Regen"),
            "Passive Regen should trigger: {:?}",
            triggered.iter().map(|(i, a)| (&a.name, *i)).collect::<Vec<_>>()
        );
        // Emergency Repair should NOT trigger (HP > 15%)
        assert!(
            !triggered.iter().any(|(_, a)| a.name == "Emergency Repair"),
            "Emergency Repair should not trigger at 80% HP"
        );
    }

    #[test]
    fn check_abilities_hp_threshold_triggers() {
        let mut crew = make_crew(1, Personality::Brave, true);
        crew.class = CrewClass::Engineer;
        crew.level = 5;
        crew.reset_battle_state();
        let ctx = AbilityContext {
            ship_hp_percent: 0.10, // below 15% threshold
            ..AbilityContext::default()
        };
        let triggered = crew.check_abilities(&ctx);
        assert!(
            triggered.iter().any(|(_, a)| a.name == "Emergency Repair"),
            "Emergency Repair should trigger at 10% HP"
        );
    }

    #[test]
    fn check_abilities_level_requirement() {
        let mut crew = make_crew(1, Personality::Brave, true);
        crew.class = CrewClass::Engineer;
        crew.level = 2; // below Passive Regen's lv3 requirement
        crew.reset_battle_state();
        let ctx = AbilityContext::default();
        let triggered = crew.check_abilities(&ctx);
        assert!(
            triggered.is_empty(),
            "No abilities should trigger below level requirement"
        );
    }

    #[test]
    fn check_abilities_cooldown_blocks() {
        let mut crew = make_crew(1, Personality::Brave, true);
        crew.class = CrewClass::Engineer;
        crew.level = 5;
        crew.reset_battle_state();
        // Activate Emergency Repair (index 1), which has cooldown 600
        crew.activate_ability(1);
        assert_eq!(crew.ability_cooldowns[1], 600);
        let ctx = AbilityContext {
            ship_hp_percent: 0.10,
            ..AbilityContext::default()
        };
        let triggered = crew.check_abilities(&ctx);
        assert!(
            !triggered.iter().any(|(_, a)| a.name == "Emergency Repair"),
            "Emergency Repair should be on cooldown"
        );
    }

    #[test]
    fn check_abilities_once_per_battle() {
        let mut crew = make_crew(1, Personality::Brave, true);
        crew.class = CrewClass::Pilot;
        crew.level = 10; // has Afterburner (OncePerBattle)
        crew.reset_battle_state();
        // First check on battle start — should trigger
        let ctx = AbilityContext {
            battle_started: true,
            ..AbilityContext::default()
        };
        let triggered = crew.check_abilities(&ctx);
        assert!(
            triggered.iter().any(|(_, a)| a.name == "Afterburner"),
            "Afterburner should trigger on first battle start"
        );
        // Mark it used
        let afterburner_idx = triggered
            .iter()
            .find(|(_, a)| a.name == "Afterburner")
            .unwrap()
            .0;
        crew.activate_ability(afterburner_idx);
        // Second check — should NOT trigger
        let triggered2 = crew.check_abilities(&ctx);
        assert!(
            !triggered2.iter().any(|(_, a)| a.name == "Afterburner"),
            "Afterburner should not trigger twice per battle"
        );
    }

    #[test]
    fn check_abilities_every_n_shots() {
        let mut crew = make_crew(1, Personality::Brave, true);
        crew.class = CrewClass::Gunner;
        crew.level = 10; // has Barrage (EveryNShots(5))
        crew.reset_battle_state();
        let shot_ctx = |counter: u32| {
            let mut c = make_crew(1, Personality::Brave, true);
            c.class = CrewClass::Gunner;
            c.level = 10;
            c.reset_battle_state();
            c.shot_counter = counter;
            c
        };
        // Shots 1-4 should NOT trigger Barrage
        for n in 1..5 {
            let mut c = shot_ctx(0);
            for _ in 0..n {
                c.record_shot();
            }
            let ctx = AbilityContext {
                shot_fired: true,
                ..AbilityContext::default()
            };
            let triggered = c.check_abilities(&ctx);
            let has_barrage = triggered.iter().any(|(_, a)| a.name == "Barrage");
            if n % 5 == 0 {
                assert!(has_barrage, "Barrage should trigger on shot {}", n);
            } else {
                assert!(!has_barrage, "Barrage should NOT trigger on shot {}", n);
            }
        }
        // Shot 5 SHOULD trigger
        let mut c = shot_ctx(0);
        for _ in 0..5 {
            c.record_shot();
        }
        let ctx = AbilityContext {
            shot_fired: true,
            ..AbilityContext::default()
        };
        let triggered = c.check_abilities(&ctx);
        assert!(
            triggered.iter().any(|(_, a)| a.name == "Barrage"),
            "Barrage should trigger on shot 5"
        );
    }

    #[test]
    fn check_abilities_on_battle_start() {
        let mut crew = make_crew(1, Personality::Brave, true);
        crew.class = CrewClass::Gunner;
        crew.level = 5; // has Lock-On (OnBattleStart)
        crew.reset_battle_state();
        let ctx = AbilityContext {
            battle_started: true,
            ..AbilityContext::default()
        };
        let triggered = crew.check_abilities(&ctx);
        assert!(
            triggered.iter().any(|(_, a)| a.name == "Lock-On"),
            "Lock-On should trigger on battle start"
        );
    }

    #[test]
    fn check_abilities_on_ally_destroyed() {
        let mut crew = make_crew(1, Personality::Brave, true);
        crew.class = CrewClass::Medic;
        crew.level = 10; // has Revive (OnAllyDestroyed)
        crew.reset_battle_state();
        let ctx = AbilityContext {
            ally_just_destroyed: true,
            ..AbilityContext::default()
        };
        let triggered = crew.check_abilities(&ctx);
        assert!(
            triggered.iter().any(|(_, a)| a.name == "Revive"),
            "Revive should trigger on ally destroyed"
        );
    }

    #[test]
    fn activate_ability_sets_cooldown() {
        let mut crew = make_crew(1, Personality::Brave, true);
        crew.class = CrewClass::Engineer;
        crew.level = 5;
        crew.reset_battle_state();
        // Emergency Repair (index 1) has cooldown_max 600
        crew.activate_ability(1);
        assert_eq!(crew.ability_cooldowns[1], 600);
    }

    #[test]
    fn activate_ability_marks_once_per_battle() {
        let mut crew = make_crew(1, Personality::Brave, true);
        crew.class = CrewClass::Pilot;
        crew.level = 10;
        crew.reset_battle_state();
        // Afterburner (index 1) is OncePerBattle
        crew.activate_ability(1);
        assert!(crew.abilities_used[1]);
    }

    #[test]
    fn activate_ability_out_of_bounds_safe() {
        let mut crew = make_crew(1, Personality::Brave, true);
        crew.class = CrewClass::Pilot;
        crew.reset_battle_state();
        // Index 99 is way out of bounds — should not panic
        crew.activate_ability(99);
    }
}
