use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShipType {
    Scout,
    Fighter,
    Bomber,
    Frigate,
    Destroyer,
    Capital,
    Carrier,
}

impl ShipType {
    pub fn base_hp(&self) -> u32 {
        match self {
            Self::Scout => 10,
            Self::Fighter => 20,
            Self::Bomber => 30,
            Self::Frigate => 80,
            Self::Destroyer => 150,
            Self::Capital => 500,
            Self::Carrier => 300,
        }
    }

    pub fn base_dmg(&self) -> u32 {
        match self {
            Self::Scout => 3,
            Self::Fighter => 8,
            Self::Bomber => 20,
            Self::Frigate => 15,
            Self::Destroyer => 35,
            Self::Capital => 80,
            Self::Carrier => 10,
        }
    }

    pub fn base_speed(&self) -> f32 {
        match self {
            Self::Scout => 10.0,
            Self::Fighter => 8.0,
            Self::Bomber => 4.0,
            Self::Frigate => 5.0,
            Self::Destroyer => 3.0,
            Self::Capital => 2.0,
            Self::Carrier => 2.0,
        }
    }

    pub fn cost(&self) -> u64 {
        match self {
            Self::Scout => 0,
            Self::Fighter => 100,
            Self::Bomber => 300,
            Self::Frigate => 500,
            Self::Destroyer => 1500,
            Self::Capital => 5000,
            Self::Carrier => 8000,
        }
    }

    pub fn unlock_level(&self) -> u32 {
        match self {
            Self::Scout => 1,
            Self::Fighter => 3,
            Self::Bomber => 7,
            Self::Frigate => 10,
            Self::Destroyer => 18,
            Self::Capital => 30,
            Self::Carrier => 25,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Scout => "Scout",
            Self::Fighter => "Fighter",
            Self::Bomber => "Bomber",
            Self::Frigate => "Frigate",
            Self::Destroyer => "Destroyer",
            Self::Capital => "Capital Ship",
            Self::Carrier => "Carrier",
        }
    }

    /// Multi-line ASCII sprite for rendering. Each line is one row.
    /// Ships face right (→ direction).
    pub fn sprite(&self) -> &'static [&'static str] {
        match self {
            Self::Scout => &["=>"],
            Self::Fighter => &["═╝►"],
            Self::Bomber => &["═══►"],
            Self::Frigate => &["═══╗", "═══╝►"],
            Self::Destroyer => &["╔═══╗", "╣███╠►", "╚═══╝"],
            Self::Capital => &["  ╔════╗", "╔╣██████╠═►", "  ╚════╝"],
            Self::Carrier => &["╔══════╗", "║██░░██║►", "╚══════╝"],
        }
    }
}

/// A single ship instance in the player's fleet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ship {
    pub ship_type: ShipType,
    pub current_hp: u32,
    pub upgrade_level: u8, // 0-10, multiplies stats

    // Runtime position (not saved, set by scene)
    #[serde(skip)]
    pub x: f32,
    #[serde(skip)]
    pub y: f32,
}

impl Ship {
    pub fn new(ship_type: ShipType) -> Self {
        let hp = ship_type.base_hp();
        Self {
            ship_type,
            current_hp: hp,
            upgrade_level: 0,
            x: 0.0,
            y: 0.0,
        }
    }

    pub fn max_hp(&self) -> u32 {
        let base = self.ship_type.base_hp();
        base + (base as f32 * self.upgrade_level as f32 * 0.15) as u32
    }

    pub fn damage(&self) -> u32 {
        let base = self.ship_type.base_dmg();
        base + (base as f32 * self.upgrade_level as f32 * 0.12) as u32
    }

    pub fn speed(&self) -> f32 {
        self.ship_type.base_speed() + self.upgrade_level as f32 * 0.3
    }

    pub fn dps(&self) -> f32 {
        self.damage() as f32 * (0.5 + self.speed() * 0.05)
    }

    pub fn is_alive(&self) -> bool {
        self.current_hp > 0
    }

    pub fn heal_full(&mut self) {
        self.current_hp = self.max_hp();
    }

    pub fn upgrade_cost(&self) -> u64 {
        let base = self.ship_type.cost().max(50);
        base * (self.upgrade_level as u64 + 1)
    }
}
