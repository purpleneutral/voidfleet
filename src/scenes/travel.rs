use crossterm::event::KeyCode;
use rand::Rng;
use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use ratatui::Frame;
use ratatui::style::{Color, Style};

use crate::engine::ship::{Ship, ShipType};
use crate::rendering::particles::ParticleSystem;
use crate::rendering::starfield::Starfield;
use crate::state::{GamePhase, GameState};
use super::{Scene, SceneAction};

// ── Collectibles ──────────────────────────────────────────────

/// Floating collectible during travel.
struct Collectible {
    x: f32,
    y: f32,
    kind: CollectibleKind,
}

#[derive(Clone, Copy)]
enum CollectibleKind {
    Scrap,   // ◇
    Cargo,   // □
    Beacon,  // ⊕
}

impl CollectibleKind {
    fn char(&self) -> char {
        match self {
            Self::Scrap => '◇',
            Self::Cargo => '□',
            Self::Beacon => '⊕',
        }
    }

    fn color(&self) -> Color {
        match self {
            Self::Scrap => Color::Yellow,
            Self::Cargo => Color::Cyan,
            Self::Beacon => Color::Magenta,
        }
    }

    fn value(&self) -> u64 {
        match self {
            Self::Scrap => 5,
            Self::Cargo => 15,
            Self::Beacon => 2,
        }
    }
}

// ── Nebula clouds ─────────────────────────────────────────────

struct NebulaCloud {
    x: f32,
    y: f32,
    width: u16,
    #[allow(dead_code)]
    height: u16,
    color: Color,
    age: u16,
    max_age: u16,
    /// Pattern rows: each row is a vec of (col_offset, char).
    pattern: Vec<Vec<(u16, char)>>,
}

impl NebulaCloud {
    fn new(x: f32, y: f32, width: u16, height: u16, color: Color) -> Self {
        let mut rng = rand::thread_rng();
        let max_age = rng.gen_range(80..120);

        // Generate an organic-looking cloud pattern
        let mut pattern = Vec::new();
        let chars = ['░', '░', '▒', ' ', '░'];
        for row in 0..height {
            let mut cols = Vec::new();
            for col in 0..width {
                // Elliptical falloff from center
                let cx = width as f32 / 2.0;
                let cy = height as f32 / 2.0;
                let dx = (col as f32 - cx) / cx;
                let dy = (row as f32 - cy) / cy;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < 1.0 && rng.gen_bool((1.0 - dist as f64) * 0.6) {
                    let ch = chars[rng.gen_range(0..chars.len())];
                    cols.push((col, ch));
                }
            }
            pattern.push(cols);
        }

        Self { x, y, width, height, color, age: 0, max_age, pattern }
    }

    fn tick(&mut self) {
        self.x -= 0.15; // Drift left slowly
        self.age += 1;
    }

    fn alive(&self) -> bool {
        self.age < self.max_age && self.x + self.width as f32 > -2.0
    }

    fn fade(&self) -> f32 {
        let ramp = 15.0;
        let fade_in = (self.age as f32 / ramp).min(1.0);
        let fade_out = ((self.max_age - self.age) as f32 / ramp).min(1.0);
        fade_in * fade_out
    }
}

// ── Asteroid field ────────────────────────────────────────────

struct Asteroid {
    x: f32,
    y: f32,
    ch: char,
    color: Color,
}

struct AsteroidField {
    asteroids: Vec<Asteroid>,
    speed: f32,
    age: u16,
    max_age: u16,
}

impl AsteroidField {
    fn new(x: f32, center_y: f32, spread: f32, count: usize) -> Self {
        let mut rng = rand::thread_rng();
        let chars = ['●', '○', '⊕', '◆', '◇'];
        let colors = [Color::DarkGray, Color::Gray, Color::Yellow, Color::Red];
        let asteroids = (0..count)
            .map(|_| Asteroid {
                x: x + rng.gen_range(-3.0..3.0),
                y: center_y + rng.gen_range(-spread..spread),
                ch: chars[rng.gen_range(0..chars.len())],
                color: colors[rng.gen_range(0..colors.len())],
            })
            .collect();
        Self {
            asteroids,
            speed: rng.gen_range(0.3..0.6),
            age: 0,
            max_age: 200,
        }
    }

    fn tick(&mut self) {
        for a in &mut self.asteroids {
            a.x -= self.speed;
        }
        self.age += 1;
    }

    fn alive(&self) -> bool {
        self.age < self.max_age && self.asteroids.iter().any(|a| a.x > -2.0)
    }

    /// Returns the x-range of the field for ship proximity checks.
    fn x_range(&self) -> (f32, f32) {
        let min = self.asteroids.iter().map(|a| a.x).fold(f32::MAX, f32::min);
        let max = self.asteroids.iter().map(|a| a.x).fold(f32::MIN, f32::max);
        (min, max)
    }
}

// ── Colored star (parallax layer 4) ──────────────────────────

struct ColoredStar {
    x: f32,
    y: f32,
    speed: f32,
    ch: char,
    color: Color,
}

// ── Travel events ────────────────────────────────────────────

enum EventOutcome {
    GainScrap(u64),
    GainCredits(u64),
    GainShip(ShipType),
    LoseScrap(u64),
    DamageFleet(u32),
    SkipSectors(u32),
    Nothing,
    StartBattle,
    AddTravelTime(f32),
    HealFleet(u64),
    GainBlueprint,
    GainArtifact,
}

struct TravelEvent {
    title: String,
    description: String,
    options: Vec<(String, EventOutcome)>,
    selected: usize,
    active: bool,
    result_text: Option<String>,
    result_timer: u8,
}

impl TravelEvent {
    fn new(title: &str, description: &str, options: Vec<(String, EventOutcome)>) -> Self {
        Self {
            title: title.to_string(),
            description: description.to_string(),
            options,
            selected: 0,
            active: true,
            result_text: None,
            result_timer: 0,
        }
    }

    fn showing_result(&self) -> bool {
        self.result_text.is_some() && self.result_timer > 0
    }
}

// ── Event type tracking for weighted selection ───────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventType {
    DistressSignal,
    AbandonedWreck,
    TradingPost,
    Wormhole,
    PirateAmbush,
    DerelictStation,
    CosmicStorm,
}

impl EventType {
    const ALL: [EventType; 7] = [
        EventType::DistressSignal,
        EventType::AbandonedWreck,
        EventType::TradingPost,
        EventType::Wormhole,
        EventType::PirateAmbush,
        EventType::DerelictStation,
        EventType::CosmicStorm,
    ];
}

/// Calculate utility-based weights for each event type given the current game state.
fn event_weights(state: &GameState, last_event: Option<EventType>) -> Vec<(EventType, f32)> {
    let hp_pct = if state.fleet_max_hp() > 0 {
        state.fleet_total_hp() as f32 / state.fleet_max_hp() as f32
    } else {
        1.0
    };

    let mut weights: Vec<(EventType, f32)> = Vec::with_capacity(7);

    for event_type in EventType::ALL {
        // Zero weight if same as last event (no repeats)
        if last_event == Some(event_type) {
            weights.push((event_type, 0.0));
            continue;
        }

        let w: f32 = match event_type {
            EventType::DistressSignal => {
                // Small fleet? More distress signals (recruit opportunity)
                if state.fleet.len() <= 2 { 2.5 } else { 1.0 }
            }
            EventType::AbandonedWreck => {
                // Low scrap? More salvage opportunities
                if state.scrap < 100 { 3.0 }
                else if state.scrap < 300 { 2.0 }
                else { 1.0 }
            }
            EventType::TradingPost => {
                // Fleet damaged? More trading posts for healing
                // Died recently? Also helpful
                let mut w = 1.0;
                if hp_pct < 0.6 { w += 2.0; }
                if state.deaths > 0 && state.sector < 10 { w += 1.0; }
                w
            }
            EventType::Wormhole => {
                // Under-leveled for sector? Wormholes less likely (don't skip ahead)
                // Over-leveled? More wormholes to advance faster
                let expected_level = state.sector / 3 + 1;
                if state.level > expected_level + 3 { 2.0 }
                else if state.level < expected_level { 0.5 }
                else { 1.0 }
            }
            EventType::PirateAmbush => {
                // High sector? More dangerous events
                let mut w = 1.0;
                if state.sector > 20 { w += 1.0; }
                if state.sector > 40 { w += 0.5; }
                // Recently died? Fewer ambushes
                if state.deaths > 0 && state.sector < state.highest_sector { w *= 0.5; }
                w
            }
            EventType::DerelictStation => {
                // Low credits? More station exploration
                if state.credits < 200 { 2.0 } else { 1.0 }
            }
            EventType::CosmicStorm => {
                // Higher sectors = more storms
                if state.sector > 15 { 1.5 } else { 1.0 }
            }
        };

        weights.push((event_type, w.max(0.01))); // ensure minimum weight
    }

    weights
}

/// Pick an event type using weighted random selection.
fn pick_event_type(state: &GameState, last_event: Option<EventType>) -> EventType {
    let weights = event_weights(state, last_event);
    let w_values: Vec<f32> = weights.iter().map(|(_, w)| *w).collect();

    if let Ok(dist) = WeightedIndex::new(&w_values) {
        let mut rng = rand::thread_rng();
        let idx = dist.sample(&mut rng);
        weights[idx].0
    } else {
        // Fallback: random pick if weights are all zero somehow
        let mut rng = rand::thread_rng();
        EventType::ALL[rng.gen_range(0..EventType::ALL.len())]
    }
}

/// Sector-based reward scaling multiplier.
fn sector_reward_scale(sector: u32) -> f32 {
    1.0 + sector as f32 / 10.0
}

fn generate_random_event(state: &GameState, last_event: Option<EventType>) -> (TravelEvent, EventType) {
    let mut rng = rand::thread_rng();
    let event_type = pick_event_type(state, last_event);
    let scale = sector_reward_scale(state.sector);

    let event = match event_type {
        EventType::DistressSignal => {
            TravelEvent::new(
                "⚠ Distress Signal",
                "A ship is sending an SOS. The signal is weak\nand could be a trap... or a survivor in need.",
                vec![
                    ("Help the ship".into(), if rng.gen_bool(0.55) {
                        let ships = [ShipType::Scout, ShipType::Fighter, ShipType::Bomber];
                        EventOutcome::GainShip(ships[rng.gen_range(0..ships.len())])
                    } else {
                        let damage = ((rng.gen_range(10..30) as f32) * scale.min(2.0)) as u32;
                        EventOutcome::DamageFleet(damage)
                    }),
                    ("Ignore and continue".into(), EventOutcome::Nothing),
                ],
            )
        }
        EventType::AbandonedWreck => {
            let scrap_amount = ((rng.gen_range(50..201) as f32) * scale) as u64;
            TravelEvent::new(
                "🔧 Abandoned Wreck",
                "You find a drifting hulk. Its hull is breached\nbut the cargo bay might still hold salvage.",
                vec![
                    (format!("Salvage quickly (+{}◇)", scrap_amount), EventOutcome::GainScrap(scrap_amount)),
                    ("Careful search (30% blueprint)".into(), if rng.gen_bool(0.3) {
                        EventOutcome::GainBlueprint
                    } else {
                        let consolation = ((rng.gen_range(20..80) as f32) * scale) as u64;
                        EventOutcome::GainScrap(consolation)
                    }),
                ],
            )
        }
        EventType::TradingPost => {
            // Prices scale with sector
            let heal_cost = (100.0 * (1.0 + state.sector as f32 / 30.0)) as u64;
            let sell_scrap = (50.0 * (1.0 + state.sector as f32 / 40.0)) as u64;
            let sell_credits = (80.0 * scale) as u64;
            TravelEvent::new(
                "💰 Trading Post",
                "A merchant vessel hails you on comms.\n\"Looking to trade, captain?\"",
                vec![
                    (format!("Buy supplies (-{}₿, heal fleet)", heal_cost),
                        if state.credits >= heal_cost { EventOutcome::HealFleet(heal_cost) } else { EventOutcome::Nothing }),
                    (format!("Sell scrap (-{}◇, +{}₿)", sell_scrap, sell_credits),
                        if state.scrap >= sell_scrap { EventOutcome::GainCredits(sell_credits) } else { EventOutcome::Nothing }),
                    ("Ignore".into(), EventOutcome::Nothing),
                ],
            )
        }
        EventType::Wormhole => {
            // Wormhole skip scales: 2-5 early, 3-8 late
            let min_skip = if state.sector > 20 { 3 } else { 2 };
            let max_skip = if state.sector > 20 { 9 } else { 6 };
            let skip = rng.gen_range(min_skip..max_skip);
            TravelEvent::new(
                "🌀 Wormhole",
                "A spatial anomaly tears open before your fleet.\nScanners can't determine where it leads.",
                vec![
                    (format!("Enter the wormhole (skip {} sectors)", skip), EventOutcome::SkipSectors(skip)),
                    ("Navigate around it".into(), EventOutcome::Nothing),
                ],
            )
        }
        EventType::PirateAmbush => {
            // Tribute is percentage-based (15-35% of scrap)
            let tribute_pct = rng.gen_range(15..36) as f64 / 100.0;
            let scrap_cost = (state.scrap as f64 * tribute_pct) as u64;
            TravelEvent::new(
                "☠ Pirate Ambush",
                "Pirates emerge from an asteroid shadow!\n\"Hand over your cargo or we open fire!\"",
                vec![
                    (format!("Pay tribute (-{}◇, {}%)", scrap_cost, (tribute_pct * 100.0) as u32), EventOutcome::LoseScrap(scrap_cost)),
                    ("Fight them off!".into(), EventOutcome::StartBattle),
                ],
            )
        }
        EventType::DerelictStation => {
            let credits = ((rng.gen_range(100..501) as f32) * scale) as u64;
            TravelEvent::new(
                "🏚 Derelict Station",
                "An old station floats nearby, its lights\nflickering in the void. Power still runs.",
                vec![
                    (format!("Explore (+{}₿)", credits), if rng.gen_bool(0.15) {
                        EventOutcome::GainArtifact
                    } else {
                        EventOutcome::GainCredits(credits)
                    }),
                    ("Pass by".into(), EventOutcome::Nothing),
                ],
            )
        }
        EventType::CosmicStorm => {
            let damage = ((rng.gen_range(15..40) as f32) * scale.min(2.5)) as u32;
            let detour_time = 15.0 + (state.sector as f32 * 0.3).min(10.0);
            TravelEvent::new(
                "⚡ Cosmic Storm",
                "A radiation storm approaches! Your shields\nflare as charged particles bombard the fleet.",
                vec![
                    (format!("Brace for impact (-{}hp)", damage), EventOutcome::DamageFleet(damage)),
                    (format!("Take a detour (+{:.0}s travel)", detour_time), EventOutcome::AddTravelTime(detour_time)),
                ],
            )
        }
    };

    (event, event_type)
}

// ── Sector name generation ───────────────────────────────────

fn generate_sector_name(sector: u32) -> String {
    let mut rng = rand::thread_rng();

    let prefixes = [
        "Nebula", "Void Corridor", "Asteroid Belt", "Dark Rift",
        "Ion Storm", "Stellar Wake", "Dust Lane", "Plasma Sea",
        "Gravity Well", "Phantom Reach", "Crystal Veil", "Ember Drift",
    ];
    let suffixes = [
        "Alpha", "Beta", "Gamma", "Delta", "Sigma", "Theta",
        "Omega", "Epsilon", "Zeta", "Kappa", "Lambda", "Tau",
    ];

    // Use sector as part of seed for consistency-ish but still varied
    let prefix = prefixes[rng.gen_range(0..prefixes.len())];
    let suffix = suffixes[rng.gen_range(0..suffixes.len())];
    let num = (sector % 99) + rng.gen_range(1..=12);
    format!("{} {}-{}", prefix, suffix, num)
}

// ── Warp transition constants ────────────────────────────────

const WARP_FRAMES: u8 = 15;
const WARP_FLASH_FRAMES: u8 = 2;

// ── Main scene ───────────────────────────────────────────────

pub struct TravelScene {
    starfield: Starfield,
    collectibles: Vec<Collectible>,
    width: u16,
    height: u16,
    tick_count: u64,
    travel_duration: f32,
    fleet_positions: Vec<(f32, f32)>,

    // Nebula clouds
    nebulae: Vec<NebulaCloud>,

    // Asteroid fields
    asteroid_fields: Vec<AsteroidField>,

    // Warp transition
    warping: bool,
    warp_frame: u8,
    warp_target: GamePhase,

    // Sector name display
    sector_name: String,
    sector_name_fade_tick: u16, // ticks since scene enter, used for fade-in

    // Colored star layer (parallax depth layer 4)
    colored_stars: Vec<ColoredStar>,

    // Random travel events
    event: Option<TravelEvent>,
    event_checked: bool, // true once we've rolled for an event this travel phase
    last_event_type: Option<EventType>, // prevents same event twice in a row
}

impl TravelScene {
    pub fn new() -> Self {
        Self {
            starfield: Starfield::new(80, 24, 60),
            collectibles: Vec::new(),
            width: 80,
            height: 24,
            tick_count: 0,
            travel_duration: 45.0,
            fleet_positions: Vec::new(),
            nebulae: Vec::new(),
            asteroid_fields: Vec::new(),
            warping: false,
            warp_frame: 0,
            warp_target: GamePhase::Battle,
            sector_name: String::new(),
            sector_name_fade_tick: 0,
            colored_stars: Vec::new(),
            event: None,
            event_checked: false,
            last_event_type: None,
        }
    }

    fn spawn_collectible(&mut self) {
        let mut rng = rand::thread_rng();
        let kind = match rng.gen_range(0..10) {
            0..=5 => CollectibleKind::Scrap,
            6..=8 => CollectibleKind::Cargo,
            _ => CollectibleKind::Beacon,
        };
        self.collectibles.push(Collectible {
            x: self.width as f32 + 2.0,
            y: rng.gen_range(1.0..(self.height as f32 - 2.0)),
            kind,
        });
    }

    fn spawn_nebula(&mut self) {
        let mut rng = rand::thread_rng();
        let colors = [
            Color::Rgb(60, 20, 80),  // deep purple
            Color::Rgb(20, 40, 80),  // dark blue
            Color::Rgb(20, 60, 40),  // dark green
            Color::Rgb(40, 20, 60),  // violet
            Color::Rgb(20, 50, 70),  // teal
        ];
        let w = rng.gen_range(12..25);
        let h = rng.gen_range(5..10).min(self.height.saturating_sub(4));
        let y = rng.gen_range(1.0..(self.height as f32 - h as f32 - 1.0));
        let color = colors[rng.gen_range(0..colors.len())];
        self.nebulae.push(NebulaCloud::new(
            self.width as f32 + 2.0,
            y,
            w,
            h,
            color,
        ));
    }

    fn spawn_asteroid_field(&mut self) {
        let mut rng = rand::thread_rng();
        let cy = rng.gen_range(4.0..(self.height as f32 - 4.0));
        let spread = rng.gen_range(3.0..6.0);
        let count = rng.gen_range(6..15);
        self.asteroid_fields.push(AsteroidField::new(
            self.width as f32 + 5.0,
            cy,
            spread,
            count,
        ));
    }

    fn spawn_colored_star(&mut self) {
        let mut rng = rand::thread_rng();
        let colors = [
            Color::Rgb(100, 140, 255), // blue star
            Color::Rgb(255, 160, 60),  // orange star
            Color::Rgb(255, 100, 100), // red giant
            Color::Rgb(200, 200, 255), // bright white-blue
        ];
        let chars = ['✦', '★', '◆', '*'];
        self.colored_stars.push(ColoredStar {
            x: self.width as f32 + rng.gen_range(0.0..5.0),
            y: rng.gen_range(0.0..self.height as f32),
            speed: rng.gen_range(0.2..0.45), // between far and mid layers
            ch: chars[rng.gen_range(0..chars.len())],
            color: colors[rng.gen_range(0..colors.len())],
        });
    }

    /// Check if asteroids are near fleet x-position (~8.0).
    fn asteroids_near_fleet(&self) -> bool {
        for field in &self.asteroid_fields {
            let (min_x, max_x) = field.x_range();
            // Fleet sits around x=8; check if field overlaps with some margin
            if min_x < 20.0 && max_x > 0.0 {
                return true;
            }
        }
        false
    }

    fn calculate_fleet_positions(&mut self, fleet: &[Ship]) {
        self.fleet_positions.clear();
        let cx = 8.0_f32;
        let cy = self.height as f32 / 2.0;

        let asteroid_bobble = if self.asteroids_near_fleet() { 0.8 } else { 0.0 };

        if fleet.len() >= 5 {
            // V-formation: fighters/scouts at the front edges, bigger ships center-back
            let mut sorted_indices: Vec<usize> = (0..fleet.len()).collect();
            sorted_indices.sort_by_key(|&i| ship_formation_priority(&fleet[i].ship_type));

            let total = sorted_indices.len();
            for (rank, &idx) in sorted_indices.iter().enumerate() {
                // rank 0 = front center (smallest/fastest), higher = further back
                let depth = rank as f32 / total.max(1) as f32; // 0..1
                let x = cx - depth * 6.0; // front ships at cx, back ships 6 left
                // Spread in a V: items at front are at center, back items spread out
                let arm = if rank % 2 == 0 { 1.0 } else { -1.0 };
                let spread = (rank as f32 / 2.0).ceil() * 2.5;
                let y = cy + arm * spread;

                // Wave motion + asteroid turbulence
                let wave_amp = 0.3 + asteroid_bobble;
                let wave = (self.tick_count as f32 * 0.05 + idx as f32 * 0.8).sin() * wave_amp;
                let bob = if asteroid_bobble > 0.0 {
                    (self.tick_count as f32 * 0.15 + idx as f32 * 1.5).cos() * 0.5
                } else {
                    0.0
                };
                self.fleet_positions.push((x, y + wave + bob));
            }
        } else {
            // Vertical stack for small fleets
            let spacing = 3.0_f32;
            for (i, _ship) in fleet.iter().enumerate() {
                let row = i as f32;
                let y = cy - (fleet.len() as f32 * spacing / 2.0) + row * spacing;
                let wave_amp = 0.3 + asteroid_bobble;
                let wave = (self.tick_count as f32 * 0.05 + i as f32 * 0.8).sin() * wave_amp;
                let bob = if asteroid_bobble > 0.0 {
                    (self.tick_count as f32 * 0.15 + i as f32 * 1.5).cos() * 0.5
                } else {
                    0.0
                };
                self.fleet_positions.push((cx, y + wave + bob));
            }
        }
    }

    fn emit_scaled_exhaust(&self, particles: &mut ParticleSystem, fleet: &[Ship]) {
        let mut rng = rand::thread_rng();
        for (i, ship) in fleet.iter().enumerate() {
            if i >= self.fleet_positions.len() {
                break;
            }
            let (fx, fy) = self.fleet_positions[i];
            let sprite = ship.ship_type.sprite();
            let sprite_height = sprite.len() as f32;

            match ship_size_class(&ship.ship_type) {
                ShipSize::Small => {
                    // 1 particle, narrow
                    particles.emit(crate::rendering::particles::Particle::new(
                        fx - 1.0,
                        fy,
                        rng.gen_range(-0.8..-0.3),
                        rng.gen_range(-0.1..0.1),
                        rng.gen_range(3..6),
                        '░',
                        Color::DarkGray,
                    ));
                }
                ShipSize::Medium => {
                    // 2 particles, moderate spread
                    for j in 0..2 {
                        let y_off = (j as f32 / 1.0) * sprite_height * 0.5;
                        particles.emit(crate::rendering::particles::Particle::new(
                            fx - 1.0,
                            fy + y_off,
                            rng.gen_range(-1.0..-0.4),
                            rng.gen_range(-0.2..0.2),
                            rng.gen_range(4..8),
                            '░',
                            Color::DarkGray,
                        ));
                    }
                }
                ShipSize::Large => {
                    // 3-4 particles, wide spread, brighter
                    let count = rng.gen_range(3..5);
                    for j in 0..count {
                        let y_off = (j as f32 / count as f32) * sprite_height - sprite_height / 2.0;
                        let ch = if rng.gen_bool(0.3) { '▒' } else { '░' };
                        let color = if rng.gen_bool(0.2) {
                            Color::Rgb(80, 60, 40)
                        } else {
                            Color::DarkGray
                        };
                        particles.emit(crate::rendering::particles::Particle::new(
                            fx - 1.5,
                            fy + sprite_height / 2.0 + y_off,
                            rng.gen_range(-1.2..-0.3),
                            rng.gen_range(-0.3..0.3),
                            rng.gen_range(5..10),
                            ch,
                            color,
                        ));
                    }
                    // Capital ships: extra visible trail particle (longer life)
                    if matches!(ship.ship_type, ShipType::Capital) {
                        particles.emit(crate::rendering::particles::Particle::new(
                            fx - 2.5,
                            fy + sprite_height / 2.0,
                            rng.gen_range(-0.6..-0.2),
                            rng.gen_range(-0.05..0.05),
                            rng.gen_range(10..16),
                            '·',
                            Color::Rgb(60, 40, 30),
                        ));
                    }
                }
            }
        }
    }

    /// Whether a travel event is currently active (blocking input).
    pub fn has_active_event(&self) -> bool {
        self.event.as_ref().is_some_and(|e| e.active || e.showing_result())
    }

    /// Handle input during an active event. Returns true if input was consumed.
    pub fn handle_input(&mut self, key: KeyCode, state: &mut GameState) -> bool {
        let event = match self.event.as_mut() {
            Some(e) if e.active && !e.showing_result() => e,
            _ => return false,
        };

        match key {
            KeyCode::Up => {
                if event.selected > 0 {
                    event.selected -= 1;
                }
                true
            }
            KeyCode::Down => {
                if event.selected + 1 < event.options.len() {
                    event.selected += 1;
                }
                true
            }
            KeyCode::Enter => {
                let idx = event.selected;
                if idx < event.options.len() {
                    // Take the option out to process it
                    let (label, outcome) = event.options.remove(idx);
                    let result = apply_event_outcome(outcome, state, &label);
                    // Reconstruct — we just need the result text now
                    let ev = self.event.as_mut().unwrap();
                    ev.result_text = Some(result);
                    ev.result_timer = 40;
                }
                true
            }
            _ => true, // consume all keys while event is active
        }
    }

    /// Begin warp transition.
    fn start_warp(&mut self, target: GamePhase) {
        self.warping = true;
        self.warp_frame = 0;
        self.warp_target = target;
    }
}

// ── Helpers ──────────────────────────────────────────────────

enum ShipSize {
    Small,
    Medium,
    Large,
}

fn ship_size_class(st: &ShipType) -> ShipSize {
    match st {
        ShipType::Scout | ShipType::Fighter | ShipType::Bomber => ShipSize::Small,
        ShipType::Frigate => ShipSize::Medium,
        ShipType::Destroyer | ShipType::Capital | ShipType::Carrier => ShipSize::Large,
    }
}

/// Priority for V-formation: lower = front of formation.
fn ship_formation_priority(st: &ShipType) -> u8 {
    match st {
        ShipType::Scout => 0,
        ShipType::Fighter => 1,
        ShipType::Bomber => 2,
        ShipType::Frigate => 3,
        ShipType::Destroyer => 4,
        ShipType::Carrier => 5,
        ShipType::Capital => 6,
    }
}

fn apply_event_outcome(outcome: EventOutcome, state: &mut GameState, _label: &str) -> String {
    match outcome {
        EventOutcome::GainScrap(amt) => {
            state.scrap += amt;
            state.total_scrap += amt;
            format!("You salvaged {} scrap!", amt)
        }
        EventOutcome::GainCredits(amt) => {
            state.credits += amt;
            format!("You gained {} credits!", amt)
        }
        EventOutcome::GainShip(ship_type) => {
            let name = ship_type.name().to_string();
            state.fleet.push(Ship::new(ship_type));
            format!("A {} joined your fleet!", name)
        }
        EventOutcome::LoseScrap(amt) => {
            let lost = amt.min(state.scrap);
            state.scrap -= lost;
            format!("You lost {} scrap.", lost)
        }
        EventOutcome::DamageFleet(dmg) => {
            let mut remaining = dmg;
            for ship in state.fleet.iter_mut().rev() {
                if remaining == 0 { break; }
                let take = remaining.min(ship.current_hp);
                ship.current_hp -= take;
                remaining -= take;
            }
            state.fleet.retain(|s| s.current_hp > 0);
            format!("Your fleet took {} damage!", dmg)
        }
        EventOutcome::SkipSectors(n) => {
            state.sector += n;
            format!("Warped through {} sectors!", n)
        }
        EventOutcome::Nothing => {
            "You continue on your way.".to_string()
        }
        EventOutcome::StartBattle => {
            // Will be handled by checking result_text in tick
            "Pirates attack! Brace for combat!".to_string()
        }
        EventOutcome::AddTravelTime(secs) => {
            state.phase_timer += secs;
            format!("Detour adds {:.0} seconds to travel.", secs)
        }
        EventOutcome::HealFleet(cost) => {
            if state.credits >= cost {
                state.credits -= cost;
                for ship in &mut state.fleet {
                    ship.heal_full();
                }
                format!("Fleet fully repaired! (-{}₿)", cost)
            } else {
                "Not enough credits!".to_string()
            }
        }
        EventOutcome::GainBlueprint => {
            state.blueprints += 1;
            "You found a rare blueprint!".to_string()
        }
        EventOutcome::GainArtifact => {
            state.artifacts += 1;
            "You discovered an ancient artifact!".to_string()
        }
    }
}

// ── Scene impl ───────────────────────────────────────────────

impl Scene for TravelScene {
    fn enter(&mut self, state: &GameState, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.starfield = Starfield::new(width, height, (width as usize * height as usize) / 30);
        self.collectibles.clear();
        self.nebulae.clear();
        self.asteroid_fields.clear();
        self.colored_stars.clear();
        self.tick_count = 0;
        self.travel_duration = (45.0 + (state.sector as f32 * 2.0).min(30.0)) * state.pip_travel_bonus();
        self.warping = false;
        self.warp_frame = 0;
        self.sector_name = generate_sector_name(state.sector);
        self.sector_name_fade_tick = 0;
        self.event = None;
        self.event_checked = false;
        // Note: last_event_type is intentionally NOT reset — prevents repeats across sectors
    }

    fn tick(&mut self, state: &mut GameState, particles: &mut ParticleSystem) -> SceneAction {
        self.tick_count += 1;
        self.sector_name_fade_tick = self.sector_name_fade_tick.saturating_add(1);

        // ── Warp transition logic ────────────────────────────
        if self.warping {
            self.warp_frame += 1;

            // During warp, accelerate stars dramatically
            for star in &mut self.starfield.stars {
                let accel = 1.0 + self.warp_frame as f32 * 0.5;
                star.x -= star.speed * accel;
            }

            if self.warp_frame >= WARP_FRAMES + WARP_FLASH_FRAMES {
                return SceneAction::TransitionTo(self.warp_target);
            }
            return SceneAction::Continue;
        }

        // ── Normal travel logic ──────────────────────────────

        // ── Event result timer countdown ─────────────────────
        if let Some(ref mut event) = self.event
            && event.showing_result() {
                event.result_timer -= 1;
                if event.result_timer == 0 {
                    // Check if this was a battle trigger
                    let start_battle = event.result_text.as_deref()
                        == Some("Pirates attack! Brace for combat!");
                    self.event = None;
                    if start_battle {
                        self.start_warp(GamePhase::Battle);
                        return SceneAction::Continue;
                    }
                }
            }

        // ── Event pauses fleet movement ──────────────────────
        if self.has_active_event() {
            // Still animate starfield/particles but don't advance travel
            self.starfield.tick();
            return SceneAction::Continue;
        }

        // ── Random event check at ~50% through travel ────────
        if !self.event_checked {
            let half_duration = self.travel_duration / 2.0;
            let elapsed = self.travel_duration - state.phase_timer;
            if elapsed >= half_duration {
                self.event_checked = true;
                let mut rng = rand::thread_rng();
                // 5-10% chance (scales slightly with sector)
                let chance = 0.05 + (state.sector as f64 * 0.005).min(0.05);
                if rng.gen_bool(chance.min(0.10)) {
                    let (event, event_type) = generate_random_event(state, self.last_event_type);
                    self.last_event_type = Some(event_type);
                    self.event = Some(event);
                    return SceneAction::Continue;
                }
            }
        }

        // Update starfield
        self.starfield.tick();

        // Update colored stars (layer 4)
        for s in &mut self.colored_stars {
            s.x -= s.speed;
        }
        self.colored_stars.retain(|s| s.x > -2.0);
        if self.tick_count.is_multiple_of(60) {
            let mut rng = rand::thread_rng();
            if rng.gen_bool(0.4) {
                self.spawn_colored_star();
            }
        }

        // Update nebulae
        for n in &mut self.nebulae {
            n.tick();
        }
        self.nebulae.retain(|n| n.alive());
        if self.tick_count.is_multiple_of(200) {
            self.spawn_nebula();
        }

        // Update asteroid fields
        for f in &mut self.asteroid_fields {
            f.tick();
        }
        self.asteroid_fields.retain(|f| f.alive());
        if self.tick_count.is_multiple_of(300) {
            let mut rng = rand::thread_rng();
            if rng.gen_bool(0.5) {
                self.spawn_asteroid_field();
            }
        }

        // Update fleet positions
        self.calculate_fleet_positions(&state.fleet);

        // Spawn collectibles
        if self.tick_count.is_multiple_of(40) {
            self.spawn_collectible();
        }

        // Move collectibles left
        self.collectibles.retain_mut(|c| {
            c.x -= 0.4;
            c.x > -2.0
        });

        // Check collection (ship near collectible)
        let fleet_x = 8.0;
        let mut collected_indices = Vec::new();
        for (ci, col) in self.collectibles.iter().enumerate() {
            for &(_, fy) in &self.fleet_positions {
                let dx = (col.x - fleet_x).abs();
                let dy = (col.y - fy).abs();
                if dx < 3.0 && dy < 1.5 {
                    let base_value = col.kind.value();
                    let scrap_value = (base_value as f32 * (1.0 + state.prestige_bonus_scrap)) as u64;
                    state.scrap += scrap_value;
                    state.total_scrap += scrap_value;
                    particles.sparkle(col.x, col.y, col.kind.color());
                    collected_indices.push(ci);
                    break;
                }
            }
        }
        // Remove collected (reverse order to preserve indices)
        collected_indices.sort_unstable();
        for i in collected_indices.into_iter().rev() {
            self.collectibles.remove(i);
        }

        // Engine exhaust — scaled by ship size
        if self.tick_count.is_multiple_of(3) {
            self.emit_scaled_exhaust(particles, &state.fleet);
        }

        // Phase timer
        state.phase_timer -= 0.05; // 20fps * 0.05 = 1 second per 20 ticks
        if state.phase_timer <= 0.0 {
            // Trigger warp transition instead of immediate switch
            let mut rng = rand::thread_rng();
            let next = if rng.gen_bool(0.6) {
                GamePhase::Battle
            } else {
                GamePhase::Raid
            };
            self.start_warp(next);
        }

        SceneAction::Continue
    }

    fn render(&self, frame: &mut Frame, state: &GameState, particles: &ParticleSystem) {
        let area = frame.area();
        let buf = frame.buffer_mut();

        // ── Warp flash (white screen) ────────────────────────
        if self.warping && self.warp_frame > WARP_FRAMES {
            for y in area.y..area.y + area.height {
                for x in area.x..area.x + area.width {
                    let cell = &mut buf[(x, y)];
                    cell.set_char(' ');
                    cell.set_bg(Color::White);
                }
            }
            return;
        }

        // ── Layer 0: Nebula clouds (background) ─────────────
        for nebula in &self.nebulae {
            let fade = nebula.fade();
            if fade <= 0.0 {
                continue;
            }
            for (row_idx, row) in nebula.pattern.iter().enumerate() {
                let sy = (nebula.y + row_idx as f32) as u16;
                if sy >= area.height {
                    continue;
                }
                for &(col_off, ch) in row {
                    let sx = (nebula.x + col_off as f32) as u16;
                    if sx < area.width {
                        let cell = &mut buf[(area.x + sx, area.y + sy)];
                        // Only draw on empty/dark cells to stay behind everything
                        if cell.symbol() == " " {
                            cell.set_char(ch);
                            // Dim the color by fade
                            let (r, g, b) = match nebula.color {
                                Color::Rgb(r, g, b) => (r, g, b),
                                _ => (40, 20, 60),
                            };
                            let f = fade;
                            cell.set_fg(Color::Rgb(
                                (r as f32 * f) as u8,
                                (g as f32 * f) as u8,
                                (b as f32 * f) as u8,
                            ));
                        }
                    }
                }
            }
        }

        // ── Layer 1: Starfield ───────────────────────────────
        if self.warping {
            // Warp stretch effect: stars become horizontal lines
            let stretch = self.warp_frame as u16;
            for star in &self.starfield.stars {
                let sx = star.x as u16;
                let sy = star.y as u16;
                if sy >= area.height {
                    continue;
                }
                // Determine the warp line char and length based on star layer
                let (line_ch, base_len) = if star.speed > 0.5 {
                    ('━', 3 + stretch * 2)   // near stars stretch most
                } else if star.speed > 0.2 {
                    ('─', 2 + stretch)        // mid stars
                } else {
                    ('─', 1 + stretch / 2)    // far stars stretch least
                };
                // Draw the stretched line trailing left from star position
                let start_x = sx.saturating_sub(base_len);
                let end_x = sx.min(area.width.saturating_sub(1));
                for x in start_x..=end_x {
                    if x < area.width {
                        let cell = &mut buf[(area.x + x, area.y + sy)];
                        cell.set_char(line_ch);
                        cell.set_fg(star.color);
                    }
                }
            }
        } else {
            for star in &self.starfield.stars {
                let sx = star.x as u16;
                let sy = star.y as u16;
                if sx < area.width && sy < area.height {
                    let cell = &mut buf[(area.x + sx, area.y + sy)];
                    cell.set_char(star.ch);
                    cell.set_fg(star.color);
                }
            }
        }

        // ── Layer 1.5: Colored stars (parallax depth 4) ─────
        for cs in &self.colored_stars {
            let sx = cs.x as u16;
            let sy = cs.y as u16;
            if sx < area.width && sy < area.height {
                let cell = &mut buf[(area.x + sx, area.y + sy)];
                cell.set_char(cs.ch);
                cell.set_fg(cs.color);
            }
        }

        // ── Layer 1.75: Shooting stars ─────────────────────────
        for (sx, sy, ch, color) in self.starfield.shooting_star_cells() {
            if sx < area.width && sy < area.height {
                let cell = &mut buf[(area.x + sx, area.y + sy)];
                cell.set_char(ch);
                cell.set_fg(color);
            }
        }

        // ── Layer 2: Asteroid fields ─────────────────────────
        for field in &self.asteroid_fields {
            for a in &field.asteroids {
                let ax = a.x as u16;
                let ay = a.y as u16;
                if ax < area.width && ay < area.height {
                    let cell = &mut buf[(area.x + ax, area.y + ay)];
                    cell.set_char(a.ch);
                    cell.set_fg(a.color);
                }
            }
        }

        // ── Layer 3: Collectibles ────────────────────────────
        for col in &self.collectibles {
            let cx = col.x as u16;
            let cy = col.y as u16;
            if cx < area.width && cy < area.height {
                let cell = &mut buf[(area.x + cx, area.y + cy)];
                cell.set_char(col.kind.char());
                cell.set_fg(col.kind.color());
            }
        }

        // ── Layer 4: Fleet ships ─────────────────────────────
        for (i, ship) in state.fleet.iter().enumerate() {
            if i >= self.fleet_positions.len() {
                break;
            }
            let (fx, fy) = self.fleet_positions[i];
            let sprite = ship.ship_type.sprite();
            for (row, line) in sprite.iter().enumerate() {
                let sy = (fy + row as f32) as u16;
                for (col, ch) in line.chars().enumerate() {
                    let sx = (fx + col as f32) as u16;
                    if sx < area.width && sy < area.height && ch != ' ' {
                        let cell = &mut buf[(area.x + sx, area.y + sy)];
                        cell.set_char(ch);
                        cell.set_fg(Color::Cyan);
                    }
                }
            }
        }

        // ── Layer 5: Particles (on top of everything) ───────
        for p in &particles.particles {
            let px = p.x as u16;
            let py = p.y as u16;
            if px < area.width && py < area.height {
                let cell = &mut buf[(area.x + px, area.y + py)];
                cell.set_char(p.render_char());
                cell.set_fg(p.color);
            }
        }

        // ── UI: Sector name (top-right, fades in) ───────────
        if !self.warping {
            let fade_ticks = 40u16; // fade in over ~2 seconds
            let display_ticks = 200u16; // visible for ~10 seconds then fade out
            let alpha = if self.sector_name_fade_tick < fade_ticks {
                self.sector_name_fade_tick as f32 / fade_ticks as f32
            } else if self.sector_name_fade_tick < display_ticks {
                1.0
            } else if self.sector_name_fade_tick < display_ticks + fade_ticks {
                1.0 - (self.sector_name_fade_tick - display_ticks) as f32 / fade_ticks as f32
            } else {
                0.0
            };

            if alpha > 0.01 {
                let name_len = self.sector_name.len() as u16;
                let start_x = area.width.saturating_sub(name_len + 2);
                let name_y = 1;
                let grey = (alpha * 200.0) as u8;
                for (i, ch) in self.sector_name.chars().enumerate() {
                    let x = start_x + i as u16;
                    if x < area.width && name_y < area.height {
                        let cell = &mut buf[(area.x + x, area.y + name_y)];
                        cell.set_char(ch);
                        cell.set_fg(Color::Rgb(grey, grey, grey));
                    }
                }
            }
        }

        // ── UI: Travel event overlay ─────────────────────────
        if let Some(ref event) = self.event
            && event.active {
                // Box dimensions
                let box_w: u16 = 46;
                let desc_lines: Vec<&str> = event.description.lines().collect();
                let option_count = event.options.len() as u16;
                let result_lines = if event.showing_result() { 2 } else { 0 };
                // title(1) + blank(1) + desc + blank(1) + options + result
                let box_h: u16 = 3 + desc_lines.len() as u16
                    + if event.showing_result() { result_lines } else { option_count }
                    + 1; // bottom padding

                let bx = area.width.saturating_sub(box_w) / 2;
                let by = area.height.saturating_sub(box_h) / 2;

                // Draw box background
                for y in by..by + box_h {
                    for x in bx..bx + box_w {
                        if x < area.width && y < area.height {
                            let cell = &mut buf[(area.x + x, area.y + y)];
                            cell.set_char(' ');
                            cell.set_bg(Color::Rgb(15, 15, 25));
                        }
                    }
                }

                // Draw border
                let border_color = Color::Rgb(80, 80, 140);
                for x in bx..bx + box_w {
                    if x < area.width {
                        if by < area.height {
                            let cell = &mut buf[(area.x + x, area.y + by)];
                            cell.set_char('─');
                            cell.set_fg(border_color);
                            cell.set_bg(Color::Rgb(15, 15, 25));
                        }
                        let bot = by + box_h - 1;
                        if bot < area.height {
                            let cell = &mut buf[(area.x + x, area.y + bot)];
                            cell.set_char('─');
                            cell.set_fg(border_color);
                            cell.set_bg(Color::Rgb(15, 15, 25));
                        }
                    }
                }
                for y in by..by + box_h {
                    if y < area.height {
                        if bx < area.width {
                            let cell = &mut buf[(area.x + bx, area.y + y)];
                            cell.set_char('│');
                            cell.set_fg(border_color);
                            cell.set_bg(Color::Rgb(15, 15, 25));
                        }
                        let right = bx + box_w - 1;
                        if right < area.width {
                            let cell = &mut buf[(area.x + right, area.y + y)];
                            cell.set_char('│');
                            cell.set_fg(border_color);
                            cell.set_bg(Color::Rgb(15, 15, 25));
                        }
                    }
                }
                // Corners
                let corners = [(bx, by, '╭'), (bx + box_w - 1, by, '╮'),
                               (bx, by + box_h - 1, '╰'), (bx + box_w - 1, by + box_h - 1, '╯')];
                for (cx, cy, ch) in corners {
                    if cx < area.width && cy < area.height {
                        let cell = &mut buf[(area.x + cx, area.y + cy)];
                        cell.set_char(ch);
                        cell.set_fg(border_color);
                        cell.set_bg(Color::Rgb(15, 15, 25));
                    }
                }

                let mut row = by + 1;
                let text_x = bx + 2;

                // Title
                if row < area.height {
                    for (i, ch) in event.title.chars().enumerate() {
                        let x = text_x + i as u16;
                        if x < bx + box_w - 1 && x < area.width {
                            let cell = &mut buf[(area.x + x, area.y + row)];
                            cell.set_char(ch);
                            cell.set_fg(Color::Yellow);
                            cell.set_bg(Color::Rgb(15, 15, 25));
                        }
                    }
                }
                row += 2; // blank line

                // Description
                for line in &desc_lines {
                    if row < area.height {
                        for (i, ch) in line.chars().enumerate() {
                            let x = text_x + i as u16;
                            if x < bx + box_w - 1 && x < area.width {
                                let cell = &mut buf[(area.x + x, area.y + row)];
                                cell.set_char(ch);
                                cell.set_fg(Color::Rgb(180, 180, 200));
                                cell.set_bg(Color::Rgb(15, 15, 25));
                            }
                        }
                    }
                    row += 1;
                }
                row += 1; // blank line

                if event.showing_result() {
                    // Show result text
                    if let Some(ref text) = event.result_text
                        && row < area.height {
                            for (i, ch) in text.chars().enumerate() {
                                let x = text_x + i as u16;
                                if x < bx + box_w - 1 && x < area.width {
                                    let cell = &mut buf[(area.x + x, area.y + row)];
                                    cell.set_char(ch);
                                    cell.set_fg(Color::Green);
                                    cell.set_bg(Color::Rgb(15, 15, 25));
                                }
                            }
                        }
                } else {
                    // Show options
                    for (i, (label, _)) in event.options.iter().enumerate() {
                        if row < area.height {
                            let selected = i == event.selected;
                            let prefix = if selected { ">> " } else { "   " };
                            let fg = if selected {
                                Color::Cyan
                            } else {
                                Color::Rgb(120, 120, 140)
                            };
                            let text = format!("{}{}", prefix, label);
                            for (j, ch) in text.chars().enumerate() {
                                let x = text_x + j as u16;
                                if x < bx + box_w - 1 && x < area.width {
                                    let cell = &mut buf[(area.x + x, area.y + row)];
                                    cell.set_char(ch);
                                    cell.set_fg(fg);
                                    cell.set_bg(Color::Rgb(15, 15, 25));
                                }
                            }
                        }
                        row += 1;
                    }
                }
            }

        // ── UI: Status bar at bottom ─────────────────────────
        let status_y = area.height.saturating_sub(1);
        let warp_indicator = if self.warping { " ⚡WARP⚡ " } else { "" };
        let status = format!(
            " Sector {} │ Scrap: {} │ Credits: {} │ Fleet: {} │ Lv.{} {}",
            state.sector, state.scrap, state.credits, state.fleet.len(), state.level, warp_indicator
        );
        for (i, ch) in status.chars().enumerate() {
            let x = i as u16;
            if x < area.width {
                let cell = &mut buf[(area.x + x, area.y + status_y)];
                cell.set_char(ch);
                let color = if self.warping {
                    Color::Yellow
                } else {
                    Color::DarkGray
                };
                cell.set_fg(color);
                cell.set_style(Style::default().fg(color));
            }
        }
    }
}
