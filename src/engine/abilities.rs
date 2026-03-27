use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Ability Triggers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbilityTrigger {
    OnBattleStart,
    OnHPBelow(u8),     // percentage threshold (e.g., 25)
    EveryNShots(u8),   // periodic (e.g., every 5th shot)
    OnAllyDestroyed,
    OnEnemyKilled,
    OncePerBattle,
    Passive, // always active, checked every tick
}

// ---------------------------------------------------------------------------
// Ability Effects
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AbilityEffect {
    // Pilot abilities
    EvasiveManeuvers { untargetable_ticks: u32 },
    Afterburner { teleport: bool },

    // Gunner abilities
    LockOn { guaranteed_crit: bool },
    Barrage { extra_shots: u8 },
    HomingShots { homing_strength: f32 },

    // Engineer abilities
    PassiveRegen { hp_per_tick: f32 },
    EmergencyRepair { heal_percent: f32 },
    ShieldOverclock { multiplier: f32 },

    // Medic abilities
    Triage {
        heal_per_tick: f32,
        target: TriageTarget,
    },
    Revive { hp_percent: f32 },
    MoraleAura { min_morale: u8 },

    // Captain abilities
    Rally {
        damage_bonus: f32,
        duration_ticks: u32,
    },
    Inspire { min_morale: u8 },
    LeadershipAura { stat_bonus: u8 },

    // Navigator abilities
    StellarCartography,
    Shortcut { skip_chance: f32 },
    EventMagnet { bonus_chance: f32 },
    BetterPrices { discount: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriageTarget {
    LowestHP,
    LowestPercent,
}

// ---------------------------------------------------------------------------
// Crew Ability Definition
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewAbility {
    pub name: String,
    pub description: String,
    pub trigger: AbilityTrigger,
    pub effect: AbilityEffect,
    pub level_required: u8,
    pub cooldown_max: u32, // ticks between activations (0 = no cooldown)
    pub icon: char,
}

// ---------------------------------------------------------------------------
// Ability Context — passed to check_abilities each tick
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AbilityContext {
    pub ship_hp_percent: f32,
    pub battle_started: bool,
    pub ally_just_destroyed: bool,
    pub enemy_just_killed: bool,
    pub shot_fired: bool,
}

impl Default for AbilityContext {
    fn default() -> Self {
        Self {
            ship_hp_percent: 1.0,
            battle_started: false,
            ally_just_destroyed: false,
            enemy_just_killed: false,
            shot_fired: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ability_context_default() {
        let ctx = AbilityContext::default();
        assert!((ctx.ship_hp_percent - 1.0).abs() < f32::EPSILON);
        assert!(!ctx.battle_started);
        assert!(!ctx.ally_just_destroyed);
        assert!(!ctx.enemy_just_killed);
        assert!(!ctx.shot_fired);
    }

    #[test]
    fn crew_ability_serialization_roundtrip() {
        let ability = CrewAbility {
            name: "Test Ability".into(),
            description: "Does testing things".into(),
            trigger: AbilityTrigger::OnHPBelow(25),
            effect: AbilityEffect::EmergencyRepair { heal_percent: 0.20 },
            level_required: 5,
            cooldown_max: 600,
            icon: '⚕',
        };
        let json = serde_json::to_string(&ability).expect("serialize");
        let deserialized: CrewAbility = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.name, "Test Ability");
        assert_eq!(deserialized.level_required, 5);
        assert_eq!(deserialized.cooldown_max, 600);
    }

    #[test]
    fn trigger_variants_serialize() {
        let triggers = vec![
            AbilityTrigger::OnBattleStart,
            AbilityTrigger::OnHPBelow(25),
            AbilityTrigger::EveryNShots(5),
            AbilityTrigger::OnAllyDestroyed,
            AbilityTrigger::OnEnemyKilled,
            AbilityTrigger::OncePerBattle,
            AbilityTrigger::Passive,
        ];
        for trigger in triggers {
            let json = serde_json::to_string(&trigger).expect("serialize trigger");
            let back: AbilityTrigger = serde_json::from_str(&json).expect("deserialize trigger");
            assert_eq!(back, trigger);
        }
    }

    #[test]
    fn effect_variants_serialize() {
        let effects: Vec<AbilityEffect> = vec![
            AbilityEffect::EvasiveManeuvers {
                untargetable_ticks: 30,
            },
            AbilityEffect::Afterburner { teleport: true },
            AbilityEffect::LockOn {
                guaranteed_crit: true,
            },
            AbilityEffect::Barrage { extra_shots: 2 },
            AbilityEffect::HomingShots {
                homing_strength: 0.8,
            },
            AbilityEffect::PassiveRegen { hp_per_tick: 0.1 },
            AbilityEffect::EmergencyRepair { heal_percent: 0.20 },
            AbilityEffect::ShieldOverclock { multiplier: 2.0 },
            AbilityEffect::Triage {
                heal_per_tick: 0.15,
                target: TriageTarget::LowestPercent,
            },
            AbilityEffect::Revive { hp_percent: 0.25 },
            AbilityEffect::MoraleAura { min_morale: 50 },
            AbilityEffect::Rally {
                damage_bonus: 0.10,
                duration_ticks: 120,
            },
            AbilityEffect::Inspire { min_morale: 50 },
            AbilityEffect::LeadershipAura { stat_bonus: 5 },
            AbilityEffect::StellarCartography,
            AbilityEffect::Shortcut { skip_chance: 0.10 },
            AbilityEffect::EventMagnet { bonus_chance: 0.15 },
            AbilityEffect::BetterPrices { discount: 0.10 },
        ];
        for effect in effects {
            let json = serde_json::to_string(&effect).expect("serialize effect");
            let _back: AbilityEffect = serde_json::from_str(&json).expect("deserialize effect");
        }
    }

    #[test]
    fn triage_target_serialize() {
        let targets = [TriageTarget::LowestHP, TriageTarget::LowestPercent];
        for t in targets {
            let json = serde_json::to_string(&t).expect("serialize");
            let back: TriageTarget = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, t);
        }
    }
}
