use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use rand::Rng;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Crew Classes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrewClass {
    Pilot,    // +piloting, decent gunnery
    Gunner,   // +gunnery, decent piloting
    Engineer, // +engineering, decent leadership
    Medic,    // +engineering (repair focus), heals between battles
    Captain,  // +leadership, balanced stats, rare
}

impl CrewClass {
    pub const ALL: [CrewClass; 5] = [
        Self::Pilot,
        Self::Gunner,
        Self::Engineer,
        Self::Medic,
        Self::Captain,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::Pilot => "Pilot",
            Self::Gunner => "Gunner",
            Self::Engineer => "Engineer",
            Self::Medic => "Medic",
            Self::Captain => "Captain",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Pilot => "\u{2708}",    // ✈
            Self::Gunner => "\u{1f3af}",  // 🎯
            Self::Engineer => "\u{1f527}", // 🔧
            Self::Medic => "\u{2695}",    // ⚕
            Self::Captain => "\u{2b50}",  // ⭐
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
        }
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
    base * personality * morale
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
        let mut counts = [0u32; 5];
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
}
