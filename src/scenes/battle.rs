use rand::Rng;
use ratatui::style::Color;
use ratatui::Frame;

use crate::engine::ship::{Ship, ShipAbility, ShipType};
use crate::rendering::particles::{Particle, ParticleSystem};
use crate::state::{GamePhase, GameState};

use super::{Scene, SceneAction};

// ---------------------------------------------------------------------------
// AI Strategy — each enemy ship gets a tactical behavior
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AIStrategy {
    FocusWeak,  // target lowest HP player ship
    FocusDPS,   // target highest damage player ship
    Aggressive, // charge forward, close distance, high fire rate
    Flanker,    // move to top/bottom edges, attack from angles
    Retreater,  // stay far right, pull back when HP < 30%
    Bomber,     // slow approach, massive damage, suicide run
}

impl AIStrategy {
    /// Assign strategy based on enemy tier and a random seed.
    fn for_tier(tier: u32, rng: &mut impl Rng) -> Self {
        match tier {
            0 => {
                // Small ships: flanker or aggressive
                if rng.gen_bool(0.5) {
                    AIStrategy::Flanker
                } else {
                    AIStrategy::Aggressive
                }
            }
            1 => {
                // Medium ships: mixed
                let roll: f32 = rng.gen_range(0.0..1.0);
                if roll < 0.3 {
                    AIStrategy::Aggressive
                } else if roll < 0.6 {
                    AIStrategy::Flanker
                } else {
                    AIStrategy::FocusWeak
                }
            }
            2 => {
                // Big ships: focus or retreat
                if rng.gen_bool(0.5) {
                    AIStrategy::FocusDPS
                } else {
                    AIStrategy::Retreater
                }
            }
            3 => {
                // Warships: focus DPS or retreat
                if rng.gen_bool(0.6) {
                    AIStrategy::FocusDPS
                } else {
                    AIStrategy::Retreater
                }
            }
            _ => AIStrategy::FocusDPS,
        }
    }
}

// ---------------------------------------------------------------------------
// Enemy ship — local struct mirroring player Ship but simpler
// ---------------------------------------------------------------------------

struct EnemyShip {
    x: f32,
    y: f32,
    /// Base y position for layout (before bob/tactical movement)
    base_y: f32,
    hp: u32,
    max_hp: u32,
    damage: u32,
    speed: f32,
    #[allow(dead_code)]
    name: &'static str,
    sprite: &'static [&'static str],
    /// Tick counter for fire‐rate cadence (fires when countdown reaches 0).
    fire_cooldown: u32,
    fire_rate: u32,
    /// Tier for projectile type selection (0=pirate, 1=militia, 2=cruiser, 3=warship).
    tier: u32,
    /// Dodge offset applied when evading incoming projectiles.
    dodge_offset: f32,
    /// AI strategy for this ship.
    strategy: AIStrategy,
    /// Current target index into player fleet (-1 = none).
    target_idx: i32,
    /// Ticks until next target recalculation.
    retarget_cooldown: u32,
    /// Tactical x offset from base position (for aggressive/retreater movement).
    tactical_x_offset: f32,
    /// Tactical y offset from base position (for flanker movement).
    tactical_y_offset: f32,
    /// Death animation frame (0 = alive, 1+ = dying).
    death_frame: u8,
    /// Whether this ship has already triggered its death explosion sequence.
    death_exploded: bool,
}

impl EnemyShip {
    fn is_alive(&self) -> bool {
        self.hp > 0
    }

    /// Ship is in death animation (hp=0, death_frame still playing).
    fn is_dying(&self) -> bool {
        self.hp == 0 && self.death_frame > 0 && self.death_frame <= 15
    }

    /// Ship is fully dead and done animating.
    fn is_done(&self) -> bool {
        self.hp == 0 && self.death_frame > 15
    }

    /// Height of the sprite in rows.
    fn height(&self) -> usize {
        self.sprite.len()
    }

    fn is_big(&self) -> bool {
        self.tier >= 2
    }

    fn hp_ratio(&self) -> f32 {
        if self.max_hp == 0 {
            0.0
        } else {
            self.hp as f32 / self.max_hp as f32
        }
    }
}

// Enemy templates — sprites face left (mirrored player ships).
const ENEMY_PIRATE: &[&str] = &["◄="];
const ENEMY_MILITIA: &[&str] = &["◄╚═"];
const ENEMY_CRUISER: &[&str] = &["◄══╗", "◄══╝"];
const ENEMY_WARSHIP: &[&str] = &["╔═══►", "╣███◄", "╚═══►"];

fn enemy_template(sector: u32, rng: &mut impl Rng) -> EnemyShip {
    // Difficulty scales with sector — higher sectors get tougher ships + stat boost
    let scale = 1.0 + (sector as f32 - 1.0) * 0.15;
    let tier = rng.gen_range(0..=(sector.min(20) / 5).min(3));

    let (name, sprite, base_hp, base_dmg, base_speed) = match tier {
        0 => ("Pirate Scout", ENEMY_PIRATE, 8u32, 2u32, 9.0f32),
        1 => ("Militia Fighter", ENEMY_MILITIA, 18, 6, 7.0),
        2 => ("Cruiser", ENEMY_CRUISER, 50, 12, 4.0),
        _ => ("Warship", ENEMY_WARSHIP, 120, 28, 3.0),
    };

    let hp = (base_hp as f32 * scale) as u32;
    let fire_rate = fire_rate_ticks(base_speed, sector);
    let strategy = AIStrategy::for_tier(tier, rng);

    EnemyShip {
        x: 0.0, // set by enter()
        y: 0.0,
        base_y: 0.0,
        hp,
        max_hp: hp,
        damage: (base_dmg as f32 * scale) as u32,
        speed: base_speed,
        name,
        sprite,
        fire_cooldown: rng.gen_range(0..fire_rate), // stagger first volley
        fire_rate,
        tier,
        dodge_offset: 0.0,
        strategy,
        target_idx: -1,
        retarget_cooldown: 0,
        tactical_x_offset: 0.0,
        tactical_y_offset: 0.0,
        death_frame: 0,
        death_exploded: false,
    }
}

/// Convert speed stat → ticks between shots (faster = fewer ticks = higher fire‐rate).
/// At higher sectors, fire rate increases across the board.
fn fire_rate_ticks(speed: f32, sector: u32) -> u32 {
    let base = (40.0 / speed) as u32;
    let sector_mult = (1.0 - (sector as f32 - 1.0) * 0.02).max(0.6);
    ((base as f32 * sector_mult) as u32).max(3)
}

// ---------------------------------------------------------------------------
// Projectile — enhanced with trailing visuals and homing
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum ProjectileKind {
    ScoutLaser,    // Scout/Fighter: ─→ cyan, fast, low damage
    FrigateLaser,  // Frigate: ──→ cyan, medium
    CapitalBeam,   // Capital: ━━━━► bright cyan, slow, massive, leaves trail
    BomberMissile, // Bomber: ══► yellow, slow, high damage, slight homing
    EnemyLaser,    // Enemy standard: ◄── red
    EnemyHeavy,    // Enemy big: ◄══ magenta
    // -- Ability projectiles --
    HeavyPayload,    // Bomber ability: ◉ slow, big AOE on hit
    BroadsideShot,   // Destroyer ability: spread projectile
    TempFighterShot, // Temp fighter laser
}

struct Projectile {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32, // vertical velocity for homing/spread
    damage: u32,
    friendly: bool,
    kind: ProjectileKind,
    /// Target y for homing missiles (-1.0 = no homing)
    homing_target_y: f32,
}

impl Projectile {
    fn trail_chars(&self) -> &[char] {
        match self.kind {
            ProjectileKind::ScoutLaser => &['→', '─'],
            ProjectileKind::FrigateLaser => &['→', '─', '─'],
            ProjectileKind::CapitalBeam => &['►', '━', '━', '━', '━'],
            ProjectileKind::BomberMissile => &['►', '═', '═'],
            ProjectileKind::EnemyLaser => &['◄', '─', '─'],
            ProjectileKind::EnemyHeavy => &['◄', '═', '═', '─'],
            ProjectileKind::HeavyPayload => &['◉', '○', '·'],
            ProjectileKind::BroadsideShot => &['»', '─'],
            ProjectileKind::TempFighterShot => &['→', '·'],
        }
    }

    fn color(&self) -> Color {
        match self.kind {
            ProjectileKind::ScoutLaser => Color::Cyan,
            ProjectileKind::FrigateLaser => Color::Cyan,
            ProjectileKind::CapitalBeam => Color::Rgb(100, 220, 255),
            ProjectileKind::BomberMissile => Color::Yellow,
            ProjectileKind::EnemyLaser => Color::Red,
            ProjectileKind::EnemyHeavy => Color::Magenta,
            ProjectileKind::HeavyPayload => Color::Rgb(255, 200, 50),
            ProjectileKind::BroadsideShot => Color::Rgb(180, 220, 255),
            ProjectileKind::TempFighterShot => Color::Rgb(100, 255, 180),
        }
    }

    fn trail_color(&self) -> Color {
        match self.kind {
            ProjectileKind::ScoutLaser => Color::Rgb(0, 140, 180),
            ProjectileKind::FrigateLaser => Color::Rgb(0, 140, 180),
            ProjectileKind::CapitalBeam => Color::Rgb(60, 160, 200),
            ProjectileKind::BomberMissile => Color::Rgb(180, 140, 0),
            ProjectileKind::EnemyLaser => Color::Rgb(180, 60, 60),
            ProjectileKind::EnemyHeavy => Color::Rgb(140, 50, 100),
            ProjectileKind::HeavyPayload => Color::Rgb(200, 140, 30),
            ProjectileKind::BroadsideShot => Color::Rgb(120, 160, 200),
            ProjectileKind::TempFighterShot => Color::Rgb(60, 180, 120),
        }
    }
}

// ---------------------------------------------------------------------------
// Battle phase enum
// ---------------------------------------------------------------------------

/// Phase of the battle sequence.
#[derive(Clone, Copy, PartialEq, Eq)]
enum BattlePhase {
    /// Fleets sliding in from edges (first ~40 ticks / 2 seconds at 20 ticks/sec).
    SlideIn,
    /// Active combat.
    Combat,
    /// Victory formation before transition.
    VictoryPose(u8),
    /// End-of-battle freeze — counts down frames before transition.
    Freeze(u8),
}

// ---------------------------------------------------------------------------
// Death animation state for player ships
// ---------------------------------------------------------------------------

struct PlayerShipState {
    fire_cooldown: u32,
    flash: u8,
    dodge: f32,
    death_frame: u8,
    death_exploded: bool,
    /// Tactical y offset for formation behavior
    formation_offset: f32,
    /// Ability cooldown counter — fires when it reaches the trigger threshold.
    ability_cooldown: u32,
    /// Shield active timer (ticks remaining, 0 = inactive). Halves incoming damage.
    shield_timer: u32,
    /// Beam weapon charge counter (0 = not charging). Counts up to 60, then fires.
    beam_charge: u32,
    /// Beam weapon active timer (ticks remaining, 0 = inactive). Renders beam across screen.
    beam_active: u32,
}

// ---------------------------------------------------------------------------
// Temporary fighter spawned by Carrier's LaunchFighters ability
// ---------------------------------------------------------------------------

struct TempFighter {
    x: f32,
    y: f32,
    hp: u32,
    damage: u32,
    fire_cooldown: u32,
    lifetime: u32, // ticks remaining
    target_idx: i32,
}

impl TempFighter {
    fn is_alive(&self) -> bool {
        self.hp > 0 && self.lifetime > 0
    }
}

// ---------------------------------------------------------------------------
// Active beam weapon visual state (for rendering)
// ---------------------------------------------------------------------------

struct ActiveBeam {
    /// Player ship index that owns this beam
    #[allow(dead_code)]
    ship_idx: usize,
    /// Y position of the beam
    y: f32,
    /// Starting x position (muzzle)
    start_x: f32,
    /// Ticks remaining
    ticks_left: u32,
}

// ---------------------------------------------------------------------------
// BattleScene
// ---------------------------------------------------------------------------

pub struct BattleScene {
    enemies: Vec<EnemyShip>,
    projectiles: Vec<Projectile>,
    width: u16,
    height: u16,
    tick_count: u64,
    sector: u32,
    /// Per-ship state for the player fleet (indexed same as state.fleet).
    player_states: Vec<PlayerShipState>,
    /// Tracks whether battle was won (all enemies dead) or lost.
    player_won: bool,
    /// Current battle phase.
    phase: BattlePhase,
    /// Slide-in progress (0.0 = offscreen, 1.0 = in position).
    slide_progress: f32,
    /// Coordinated focus target for enemies (-1 = no coordination).
    focus_fire_target: i32,
    /// Ticks until focus fire is re-evaluated.
    focus_fire_cooldown: u32,
    /// Temporary fighters spawned by Carrier ability.
    temp_fighters: Vec<TempFighter>,
    /// Active beam weapons being rendered.
    active_beams: Vec<ActiveBeam>,
    /// Whether any player ship has Scan ability (show all enemy HP).
    has_scan: bool,
}

impl BattleScene {
    pub fn new() -> Self {
        Self {
            enemies: Vec::new(),
            projectiles: Vec::new(),
            width: 80,
            height: 24,
            tick_count: 0,
            sector: 1,
            player_states: Vec::new(),
            player_won: false,
            phase: BattlePhase::SlideIn,
            slide_progress: 0.0,
            focus_fire_target: -1,
            focus_fire_cooldown: 0,
            temp_fighters: Vec::new(),
            active_beams: Vec::new(),
            has_scan: false,
        }
    }

    // -- helpers --

    fn player_x_start(&self) -> f32 {
        5.0
    }

    fn enemy_x_start(&self) -> f32 {
        (self.width as f32 - 15.0).max(30.0)
    }

    /// Lay out the player fleet vertically, centered, with dodge + formation offsets.
    fn player_positions(&self, fleet: &[Ship]) -> Vec<(f32, f32)> {
        let base_x = self.player_x_start();
        let cy = self.height as f32 / 2.0;
        let spacing = 3.0_f32;
        let total = fleet.len() as f32 * spacing;

        // During slide-in, ships come from offscreen left
        let slide_offset = if let BattlePhase::SlideIn = self.phase {
            -(1.0 - self.slide_progress) * (base_x + 20.0)
        } else {
            0.0
        };

        // During victory pose, form a V-shape
        let victory = matches!(self.phase, BattlePhase::VictoryPose(_));

        fleet
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let base_y = cy - total / 2.0 + i as f32 * spacing;
                let bob = (self.tick_count as f32 * 0.06 + i as f32 * 1.1).sin() * 0.4;
                let dodge = if i < self.player_states.len() {
                    self.player_states[i].dodge
                } else {
                    0.0
                };
                let formation = if i < self.player_states.len() {
                    self.player_states[i].formation_offset
                } else {
                    0.0
                };

                if victory {
                    // V-formation: center ship forward, others angled back
                    let center = fleet.len() as f32 / 2.0;
                    let dist_from_center = (i as f32 - center).abs();
                    let vx = base_x + 10.0 - dist_from_center * 3.0 + slide_offset;
                    (vx, base_y + bob)
                } else {
                    (base_x + slide_offset, base_y + bob + dodge + formation)
                }
            })
            .collect()
    }

    fn layout_enemies(&mut self) {
        let cx = self.enemy_x_start();
        let cy = self.height as f32 / 2.0;
        let spacing = 3.0_f32;
        let total = self.enemies.len() as f32 * spacing;

        for (i, e) in self.enemies.iter_mut().enumerate() {
            e.x = cx;
            let y = cy - total / 2.0 + i as f32 * spacing;
            e.y = y;
            e.base_y = y;
        }
    }

    fn enemy_alive_count(&self) -> usize {
        self.enemies.iter().filter(|e| e.is_alive()).count()
    }

    fn player_alive_count(fleet: &[Ship]) -> usize {
        fleet.iter().filter(|s| s.is_alive()).count()
    }

    fn enemy_total_hp(&self) -> u32 {
        self.enemies.iter().map(|e| e.hp).sum()
    }

    fn enemy_max_hp(&self) -> u32 {
        self.enemies.iter().map(|e| e.max_hp).sum()
    }

    // -- projectile kind from ship type --

    fn player_projectile_kind(ship_type: ShipType) -> ProjectileKind {
        match ship_type {
            ShipType::Scout => ProjectileKind::ScoutLaser,
            ShipType::Fighter => ProjectileKind::ScoutLaser,
            ShipType::Bomber => ProjectileKind::BomberMissile,
            ShipType::Frigate => ProjectileKind::FrigateLaser,
            ShipType::Destroyer | ShipType::Capital => ProjectileKind::CapitalBeam,
            ShipType::Carrier => ProjectileKind::FrigateLaser,
        }
    }

    fn enemy_projectile_kind(tier: u32) -> ProjectileKind {
        match tier {
            2 | 3 => ProjectileKind::EnemyHeavy,
            _ => ProjectileKind::EnemyLaser,
        }
    }

    // -- muzzle flash particles --

    fn emit_muzzle_flash(particles: &mut ParticleSystem, x: f32, y: f32, facing_right: bool) {
        let mut rng = rand::thread_rng();
        let dir = if facing_right { 1.0 } else { -1.0 };
        for _ in 0..3 {
            particles.emit(Particle::new(
                x,
                y,
                dir * rng.gen_range(0.3..0.8),
                rng.gen_range(-0.2..0.2),
                3,
                if rng.gen_bool(0.5) { '✦' } else { '*' },
                if rng.gen_bool(0.6) {
                    Color::White
                } else {
                    Color::Yellow
                },
            ));
        }
    }

    // -- capital beam trail particles --

    fn emit_beam_trail(particles: &mut ParticleSystem, x: f32, y: f32) {
        let mut rng = rand::thread_rng();
        particles.emit(Particle::new(
            x,
            y,
            rng.gen_range(-0.1..0.1),
            rng.gen_range(-0.15..0.15),
            rng.gen_range(3..6),
            if rng.gen_bool(0.5) { '·' } else { '∙' },
            Color::Rgb(60, 160, 200),
        ));
    }

    // -- chain explosion for destroyed ships --

    fn chain_explosion(particles: &mut ParticleSystem, x: f32, y: f32, big: bool) {
        let mut rng = rand::thread_rng();

        // Phase 1: Bright white core burst
        let core_count = if big { 8 } else { 4 };
        for _ in 0..core_count {
            let angle: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
            let speed: f32 = rng.gen_range(0.2..0.6);
            particles.emit(Particle::new(
                x,
                y,
                angle.cos() * speed,
                angle.sin() * speed * 0.5,
                4,
                '█',
                Color::White,
            ));
        }

        // Phase 2: Expanding ring of orange/red
        let ring_count = if big { 16 } else { 8 };
        for _ in 0..ring_count {
            let angle: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
            let speed: f32 = rng.gen_range(0.5..1.8);
            let life: u8 = rng.gen_range(8..16);
            let color = if rng.gen_bool(0.5) {
                Color::Rgb(255, 140, 0)
            } else {
                Color::LightRed
            };
            particles.emit(Particle::new(
                x,
                y,
                angle.cos() * speed,
                angle.sin() * speed * 0.5,
                life,
                if rng.gen_bool(0.4) { '✦' } else { '◆' },
                color,
            ));
        }

        // Phase 3: Debris chars that linger and drift slowly
        let debris_chars = ['▪', '▫', '◦', '∙', '·'];
        let debris_count = if big { 12 } else { 5 };
        for _ in 0..debris_count {
            let angle: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
            let speed: f32 = rng.gen_range(0.1..0.5);
            let life: u8 = rng.gen_range(15..30);
            let ch = debris_chars[rng.gen_range(0..debris_chars.len())];
            particles.emit(Particle::new(
                x,
                y,
                angle.cos() * speed,
                angle.sin() * speed * 0.3,
                life,
                ch,
                Color::DarkGray,
            ));
        }
    }

    /// Secondary explosion for big ships (chain explosions at offsets).
    fn chain_explosion_sequence(
        particles: &mut ParticleSystem,
        x: f32,
        y: f32,
        frame: u8,
    ) {
        let mut rng = rand::thread_rng();
        // Trigger sub-explosions at frames 4, 7, 10
        match frame {
            4 => Self::chain_explosion(
                particles,
                x + rng.gen_range(-2.0..2.0),
                y + rng.gen_range(-1.0..1.0),
                false,
            ),
            7 => Self::chain_explosion(
                particles,
                x + rng.gen_range(-3.0..3.0),
                y + rng.gen_range(-1.5..1.5),
                false,
            ),
            10 => Self::chain_explosion(
                particles,
                x + rng.gen_range(-2.0..2.0),
                y + rng.gen_range(-1.0..1.0),
                true,
            ),
            _ => {}
        }
    }

    // -- dodge logic: compute dodge offsets for each ship --

    fn compute_player_dodge(
        projectiles: &[Projectile],
        positions: &[(f32, f32)],
        fleet: &[Ship],
        states: &mut [PlayerShipState],
    ) {
        let dodge_x_range = 8.0_f32;
        let dodge_y_range = 2.0_f32;
        let base_dodge_strength = 0.3_f32;
        let decay = 0.85_f32;

        for (i, (px, py)) in positions.iter().enumerate() {
            if i >= states.len() || i >= fleet.len() || !fleet[i].is_alive() {
                continue;
            }

            // Speed-scaled dodge effectiveness
            let speed_mult = fleet[i].speed() / 10.0;
            let dodge_strength = base_dodge_strength * speed_mult;

            // Find nearest threatening enemy projectile
            let mut nearest_dy: Option<f32> = None;
            let base_y = *py - states[i].dodge;
            for p in projectiles.iter() {
                if p.friendly {
                    continue;
                }
                // Only dodge projectiles heading toward us (within x range)
                let dx = p.x - *px;
                if dx.abs() > dodge_x_range || dx > 0.0 {
                    // Projectile must be to our right (heading left toward us) and close
                    continue;
                }
                let dy = p.y - base_y;
                if dy.abs() < dodge_y_range {
                    match nearest_dy {
                        None => nearest_dy = Some(dy),
                        Some(prev) => {
                            if dy.abs() < prev.abs() {
                                nearest_dy = Some(dy);
                            }
                        }
                    }
                }
            }

            if let Some(dy) = nearest_dy {
                let dodge_dir = if dy > 0.0 {
                    -dodge_strength
                } else {
                    dodge_strength
                };
                states[i].dodge = (states[i].dodge + dodge_dir).clamp(-2.0, 2.0);
            } else {
                states[i].dodge *= decay;
                if states[i].dodge.abs() < 0.05 {
                    states[i].dodge = 0.0;
                }
            }
        }
    }

    fn compute_enemy_dodge(
        projectiles: &[Projectile],
        enemies: &mut [EnemyShip],
    ) {
        let dodge_range = 3.0_f32;
        let dodge_strength = 0.6_f32;
        let decay = 0.8_f32;

        for e in enemies.iter_mut() {
            if !e.is_alive() {
                continue;
            }

            let mut nearest_dy: Option<f32> = None;
            for p in projectiles.iter() {
                if !p.friendly {
                    continue;
                }
                let dy = p.y - (e.y - e.dodge_offset);
                if dy.abs() < dodge_range {
                    match nearest_dy {
                        None => nearest_dy = Some(dy),
                        Some(prev) => {
                            if dy.abs() < prev.abs() {
                                nearest_dy = Some(dy);
                            }
                        }
                    }
                }
            }

            if let Some(dy) = nearest_dy {
                let dodge_dir = if dy > 0.0 {
                    -dodge_strength
                } else {
                    dodge_strength
                };
                e.dodge_offset = (e.dodge_offset + dodge_dir).clamp(-2.0, 2.0);
            } else {
                e.dodge_offset *= decay;
                if e.dodge_offset.abs() < 0.05 {
                    e.dodge_offset = 0.0;
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // AI: Utility-based target selection for enemies
    // -----------------------------------------------------------------------

    fn enemy_select_target(
        strategy: AIStrategy,
        enemy_y: f32,
        fleet: &[Ship],
        positions: &[(f32, f32)],
        all_enemy_ys: &[f32],
    ) -> i32 {
        if fleet.is_empty() {
            return -1;
        }

        let mut best_idx: i32 = -1;
        let mut best_score: f32 = f32::NEG_INFINITY;

        for (i, ship) in fleet.iter().enumerate() {
            if !ship.is_alive() || i >= positions.len() {
                continue;
            }

            let score = match strategy {
                AIStrategy::FocusWeak => {
                    if ship.current_hp == 0 {
                        f32::NEG_INFINITY
                    } else {
                        1.0 / ship.current_hp as f32
                    }
                }
                AIStrategy::FocusDPS => ship.damage() as f32,
                AIStrategy::Aggressive => {
                    let (px, py) = positions[i];
                    let dist =
                        ((enemy_y - py).powi(2) + (px - 30.0).powi(2)).sqrt();
                    if dist < 0.01 {
                        1000.0
                    } else {
                        1.0 / dist
                    }
                }
                AIStrategy::Flanker => {
                    // Prefer targets far from other enemies' y positions
                    let (_, py) = positions[i];
                    let min_enemy_dist = all_enemy_ys
                        .iter()
                        .filter(|&&ey| (ey - enemy_y).abs() > 1.0) // exclude self
                        .map(|&ey| (ey - py).abs())
                        .fold(f32::INFINITY, f32::min);
                    // Higher score = farther from other enemies
                    if min_enemy_dist.is_infinite() {
                        1.0 // only enemy, pick anyone
                    } else {
                        min_enemy_dist
                    }
                }
                AIStrategy::Retreater => {
                    // Prefer closest (minimize waste on retreating shots)
                    let (_, py) = positions[i];
                    let dist = (enemy_y - py).abs();
                    if dist < 0.01 {
                        1000.0
                    } else {
                        1.0 / dist
                    }
                }
                AIStrategy::Bomber => {
                    // Target highest HP (most value from suicide)
                    ship.current_hp as f32
                }
            };

            if score > best_score {
                best_score = score;
                best_idx = i as i32;
            }
        }

        best_idx
    }

    // -----------------------------------------------------------------------
    // AI: Player smart targeting
    // -----------------------------------------------------------------------

    fn player_select_target(
        ship: &Ship,
        enemies: &[EnemyShip],
    ) -> i32 {
        let mut best_idx: i32 = -1;
        let mut best_score: f32 = f32::NEG_INFINITY;

        let is_capital = matches!(
            ship.ship_type,
            ShipType::Capital | ShipType::Destroyer | ShipType::Carrier
        );

        for (i, enemy) in enemies.iter().enumerate() {
            if !enemy.is_alive() {
                continue;
            }

            let mut score: f32 = 0.0;

            // Threat priority: aggressive and bomber enemies are highest priority
            match enemy.strategy {
                AIStrategy::Bomber => score += 50.0,
                AIStrategy::Aggressive => score += 30.0,
                _ => {}
            }

            // Capital ships prefer enemy big ships
            if is_capital && enemy.is_big() {
                score += 20.0;
            }

            // Secondary: lowest HP enemy (finish kills)
            if enemy.max_hp > 0 {
                let hp_ratio = enemy.hp as f32 / enemy.max_hp as f32;
                score += (1.0 - hp_ratio) * 15.0;
            }

            if score > best_score {
                best_score = score;
                best_idx = i as i32;
            }
        }

        best_idx
    }

    // -----------------------------------------------------------------------
    // AI: Focus fire coordination
    // -----------------------------------------------------------------------

    fn update_focus_fire(&mut self, fleet: &[Ship]) {
        if self.focus_fire_cooldown > 0 {
            self.focus_fire_cooldown -= 1;
            return;
        }

        self.focus_fire_cooldown = 20; // re-evaluate every 20 ticks
        let alive_enemies = self.enemy_alive_count();

        if alive_enemies < 3 {
            self.focus_fire_target = -1;
            return;
        }

        let mut rng = rand::thread_rng();

        // 60% chance enemies coordinate on same target
        if !rng.gen_bool(0.6) {
            self.focus_fire_target = -1;
            return;
        }

        // Check if any player ship is below 25% HP — all switch to finish it
        for (i, ship) in fleet.iter().enumerate() {
            if !ship.is_alive() {
                continue;
            }
            let ratio = ship.current_hp as f32 / ship.max_hp() as f32;
            if ratio < 0.25 {
                self.focus_fire_target = i as i32;
                return;
            }
        }

        // Otherwise pick the weakest alive ship
        let mut weakest_idx: i32 = -1;
        let mut weakest_hp = u32::MAX;
        for (i, ship) in fleet.iter().enumerate() {
            if ship.is_alive() && ship.current_hp < weakest_hp {
                weakest_hp = ship.current_hp;
                weakest_idx = i as i32;
            }
        }
        self.focus_fire_target = weakest_idx;
    }

    // -----------------------------------------------------------------------
    // AI: Enemy tactical movement
    // -----------------------------------------------------------------------

    fn update_enemy_movement(&mut self) {
        let height = self.height as f32;
        let quarter = height / 4.0;

        for e in self.enemies.iter_mut() {
            if !e.is_alive() {
                continue;
            }

            match e.strategy {
                AIStrategy::Flanker => {
                    // Move to top or bottom quarter of screen
                    let target_y = if e.base_y < height / 2.0 {
                        quarter // top quarter
                    } else {
                        height - quarter // bottom quarter
                    };
                    let dy = target_y - (e.base_y + e.tactical_y_offset);
                    e.tactical_y_offset += dy.clamp(-0.15, 0.15);
                    e.tactical_y_offset = e.tactical_y_offset.clamp(-height / 3.0, height / 3.0);
                }
                AIStrategy::Aggressive => {
                    // Drift left toward player fleet
                    e.tactical_x_offset -= 0.1;
                    // Clamp so they don't go past midscreen
                    let max_advance = e.x - 15.0;
                    if e.tactical_x_offset < -max_advance {
                        e.tactical_x_offset = -max_advance;
                    }
                }
                AIStrategy::Retreater => {
                    if e.hp_ratio() < 0.3 {
                        // Retreat: drift right
                        e.tactical_x_offset += 0.15;
                        let max_retreat = (self.width as f32 - e.x - 2.0).max(0.0);
                        if e.tactical_x_offset > max_retreat {
                            e.tactical_x_offset = max_retreat;
                        }
                    }
                }
                AIStrategy::Bomber => {
                    // Charge forward at 0.3/tick
                    e.tactical_x_offset -= 0.3;
                    // If reached player fleet x range, it will "explode" in tick
                    let max_advance = e.x - 8.0;
                    if e.tactical_x_offset < -max_advance {
                        e.tactical_x_offset = -max_advance;
                    }
                }
                _ => {
                    // FocusWeak, FocusDPS: gentle drift toward target y
                    // (handled by gentle oscillation only)
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // AI: Player fleet formation behavior
    // -----------------------------------------------------------------------

    fn update_player_formation(
        fleet: &[Ship],
        enemies: &[EnemyShip],
        states: &mut [PlayerShipState],
    ) {
        for (i, ship) in fleet.iter().enumerate() {
            if !ship.is_alive() || i >= states.len() {
                continue;
            }

            let is_fighter = matches!(
                ship.ship_type,
                ShipType::Scout | ShipType::Fighter
            );

            if is_fighter {
                // Fighters drift toward nearest alive enemy's y position
                let mut nearest_enemy_y: Option<f32> = None;
                let mut nearest_dist = f32::INFINITY;
                for e in enemies.iter() {
                    if !e.is_alive() {
                        continue;
                    }
                    let ey = e.y + e.tactical_y_offset;
                    // We don't have exact player y here, but formation offset pulls toward enemy
                    let dist = ey.abs(); // rough proxy
                    if dist < nearest_dist {
                        nearest_dist = dist;
                        nearest_enemy_y = Some(ey);
                    }
                }

                if let Some(ey) = nearest_enemy_y {
                    // Pull formation offset toward enemy y (relative to base position)
                    let target_offset = (ey - 12.0).clamp(-4.0, 4.0) * 0.3;
                    let diff = target_offset - states[i].formation_offset;
                    states[i].formation_offset += diff.clamp(-0.1, 0.1);
                }
            } else {
                // Capital/heavy ships: stay center, decay formation offset
                states[i].formation_offset *= 0.95;
                if states[i].formation_offset.abs() < 0.05 {
                    states[i].formation_offset = 0.0;
                }
            }
        }
    }

    // -- convert all remaining projectiles into particles --

    fn projectiles_to_particles(
        projectiles: &mut Vec<Projectile>,
        particles: &mut ParticleSystem,
    ) {
        let mut rng = rand::thread_rng();
        for p in projectiles.drain(..) {
            let color = if p.friendly {
                Color::Cyan
            } else {
                Color::Red
            };
            particles.emit(Particle::new(
                p.x,
                p.y,
                p.vx * 0.3 + rng.gen_range(-0.3..0.3),
                rng.gen_range(-0.3..0.3),
                rng.gen_range(4..8),
                '✦',
                color,
            ));
        }
    }

    // -- bomber suicide explosion --

    fn bomber_explode(
        particles: &mut ParticleSystem,
        x: f32,
        y: f32,
        damage: u32,
        fleet: &mut [Ship],
        positions: &[(f32, f32)],
        states: &mut [PlayerShipState],
    ) -> u64 {
        // Area damage to all player ships within range
        let blast_radius = 6.0_f32;
        let mut destroyed = 0u64;

        for (i, ship) in fleet.iter_mut().enumerate() {
            if !ship.is_alive() || i >= positions.len() {
                continue;
            }
            let (sx, sy) = positions[i];
            let dist = ((x - sx).powi(2) + (y - sy).powi(2)).sqrt();
            if dist < blast_radius {
                let falloff = 1.0 - (dist / blast_radius);
                let dmg = (damage as f32 * falloff) as u32;
                let actual = dmg.min(ship.current_hp);
                ship.current_hp -= actual;

                if i < states.len() {
                    states[i].flash = 4;
                }

                if !ship.is_alive() {
                    let is_big = matches!(
                        ship.ship_type,
                        ShipType::Destroyer
                            | ShipType::Capital
                            | ShipType::Carrier
                            | ShipType::Frigate
                    );
                    let sprite_w = ship.ship_type.sprite()[0].chars().count() as f32;
                    Self::chain_explosion(particles, sx + sprite_w / 2.0, sy, is_big);
                    destroyed += 1;
                }
            }
        }

        // Big explosion at bomber location
        Self::chain_explosion(particles, x, y, true);

        destroyed
    }
}

impl Scene for BattleScene {
    fn enter(&mut self, state: &GameState, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.tick_count = 0;
        self.sector = state.sector;
        self.projectiles.clear();
        self.player_won = false;
        self.phase = BattlePhase::SlideIn;
        self.slide_progress = 0.0;
        self.focus_fire_target = -1;
        self.focus_fire_cooldown = 0;
        self.temp_fighters.clear();
        self.active_beams.clear();
        self.has_scan = state.fleet.iter().any(|s| {
            s.is_alive() && s.ship_type.ability() == Some(ShipAbility::Scan)
        });

        // Generate enemy fleet based on sector
        let mut rng = rand::thread_rng();
        let count = (2 + state.sector / 3).min(8) as usize;
        self.enemies.clear();
        for _ in 0..count {
            self.enemies.push(enemy_template(state.sector, &mut rng));
        }
        self.layout_enemies();

        // Player ship states
        self.player_states = state
            .fleet
            .iter()
            .map(|s| {
                let rate = fire_rate_ticks(s.speed(), state.sector);
                // Stagger ability cooldowns so they don't all fire at once
                let ability_start = match s.ship_type.ability() {
                    Some(a) => rng.gen_range(0..a.cooldown_ticks().max(1)),
                    None => 0,
                };
                PlayerShipState {
                    fire_cooldown: rng.gen_range(0..rate),
                    flash: 0,
                    dodge: 0.0,
                    death_frame: 0,
                    death_exploded: false,
                    formation_offset: 0.0,
                    ability_cooldown: ability_start,
                    shield_timer: 0,
                    beam_charge: 0,
                    beam_active: 0,
                }
            })
            .collect();
    }

    fn tick(&mut self, state: &mut GameState, particles: &mut ParticleSystem) -> SceneAction {
        self.tick_count += 1;

        // ── Slide-in phase ─────────────────────────────────────────────
        if let BattlePhase::SlideIn = self.phase {
            self.slide_progress += 0.025; // ~40 ticks to complete
            if self.slide_progress >= 1.0 {
                self.slide_progress = 1.0;
                self.phase = BattlePhase::Combat;
            }
            // During slide-in, enemies also slide from right
            let slide = self.slide_progress;
            let base_x = self.enemy_x_start();
            let offscreen_x = self.width as f32 + 10.0;
            for e in self.enemies.iter_mut() {
                e.x = offscreen_x + (base_x - offscreen_x) * slide;
            }
            return SceneAction::Continue;
        }

        // ── Victory pose ───────────────────────────────────────────────
        if let BattlePhase::VictoryPose(ref mut frames) = self.phase {
            if *frames == 0 {
                self.phase = BattlePhase::Freeze(10);
                return SceneAction::Continue;
            }
            *frames -= 1;
            return SceneAction::Continue;
        }

        // ── Handle end-of-battle freeze ────────────────────────────────
        if let BattlePhase::Freeze(ref mut frames) = self.phase {
            if *frames == 0 {
                state.total_battles += 1;
                return SceneAction::TransitionTo(GamePhase::Loot);
            }
            *frames -= 1;
            return SceneAction::Continue;
        }

        // ══════════════════════════════════════════════════════════════
        // COMBAT PHASE
        // ══════════════════════════════════════════════════════════════

        // ── Update death animations ────────────────────────────────────
        for e in self.enemies.iter_mut() {
            if e.hp == 0 && e.death_frame > 0 && e.death_frame <= 15 {
                e.death_frame += 1;
                if e.is_big() {
                    let sprite_w = e.sprite[0].chars().count() as f32;
                    Self::chain_explosion_sequence(
                        particles,
                        e.x + sprite_w / 2.0,
                        e.y,
                        e.death_frame,
                    );
                }
            }
        }
        {
            let player_pos = self.player_positions(&state.fleet);
            for (i, ps) in self.player_states.iter_mut().enumerate() {
                if i < state.fleet.len() && !state.fleet[i].is_alive() && ps.death_frame > 0 && ps.death_frame <= 15 {
                    ps.death_frame += 1;
                    let is_big = matches!(
                        state.fleet[i].ship_type,
                        ShipType::Destroyer | ShipType::Capital | ShipType::Carrier | ShipType::Frigate
                    );
                    if is_big && i < player_pos.len() {
                        let sprite = state.fleet[i].ship_type.sprite();
                        let sprite_w = sprite[0].chars().count() as f32;
                        let (px, py) = player_pos[i];
                        Self::chain_explosion_sequence(
                            particles,
                            px + sprite_w / 2.0,
                            py,
                            ps.death_frame,
                        );
                    }
                }
            }
        }

        // ── Focus fire coordination ────────────────────────────────────
        self.update_focus_fire(&state.fleet);

        // ── Enemy AI: target selection (every 30 ticks) ────────────────
        let positions = self.player_positions(&state.fleet);
        let all_enemy_ys: Vec<f32> = self
            .enemies
            .iter()
            .filter(|e| e.is_alive())
            .map(|e| e.y + e.tactical_y_offset)
            .collect();

        for e in self.enemies.iter_mut() {
            if !e.is_alive() {
                continue;
            }
            if e.retarget_cooldown == 0 {
                e.retarget_cooldown = 30;

                // If focus fire is active and this enemy is eligible, use it
                if self.focus_fire_target >= 0
                    && (self.focus_fire_target as usize) < state.fleet.len()
                    && state.fleet[self.focus_fire_target as usize].is_alive()
                {
                    e.target_idx = self.focus_fire_target;
                } else {
                    e.target_idx = Self::enemy_select_target(
                        e.strategy,
                        e.y + e.tactical_y_offset,
                        &state.fleet,
                        &positions,
                        &all_enemy_ys,
                    );
                }
            } else {
                e.retarget_cooldown -= 1;
            }
        }

        // ── Enemy tactical movement ────────────────────────────────────
        self.update_enemy_movement();

        // ── Enemy bobbing + positioning ────────────────────────────────
        let base_x = self.enemy_x_start();
        for (i, e) in self.enemies.iter_mut().enumerate() {
            if !e.is_alive() {
                continue;
            }
            let bob = (self.tick_count as f32 * 0.05 + i as f32 * 1.3).sin() * 0.4;
            e.x = base_x + e.tactical_x_offset;
            e.y = e.base_y + bob + e.dodge_offset + e.tactical_y_offset;
            // Clamp to screen
            e.y = e.y.clamp(1.0, self.height as f32 - 3.0);
            e.x = e.x.clamp(10.0, self.width as f32 - 2.0);
        }

        // ── Compute enemy dodge offsets ────────────────────────────────
        Self::compute_enemy_dodge(&self.projectiles, &mut self.enemies);

        // ── Compute player dodge offsets ───────────────────────────────
        {
            let positions = self.player_positions(&state.fleet);
            Self::compute_player_dodge(
                &self.projectiles,
                &positions,
                &state.fleet,
                &mut self.player_states,
            );
        }

        // ── Player fleet formation AI ──────────────────────────────────
        Self::update_player_formation(&state.fleet, &self.enemies, &mut self.player_states);

        // ── Ship special abilities ─────────────────────────────────────
        {
            let positions = self.player_positions(&state.fleet);
            let mut new_projectiles: Vec<Projectile> = Vec::new();
            let mut new_temp_fighters: Vec<TempFighter> = Vec::new();

            for (i, ship) in state.fleet.iter().enumerate() {
                if !ship.is_alive() || i >= self.player_states.len() || i >= positions.len() {
                    continue;
                }
                let ability = match ship.ship_type.ability() {
                    Some(a) => a,
                    None => continue,
                };
                // Scan is passive — no cooldown logic needed
                if ability == ShipAbility::Scan {
                    continue;
                }

                let ps = &mut self.player_states[i];
                let (px, py) = positions[i];
                let sprite_w = ship.ship_type.sprite()[0].chars().count() as f32;
                let muzzle_x = px + sprite_w + 1.0;

                // BeamWeapon has a charge phase before firing
                if ability == ShipAbility::BeamWeapon {
                    if ps.beam_active > 0 {
                        // Beam is currently firing — damage enemies along the line
                        ps.beam_active -= 1;
                        // Damage all enemies at roughly the same y
                        for enemy in self.enemies.iter_mut() {
                            if !enemy.is_alive() {
                                continue;
                            }
                            let ey = enemy.y;
                            if (ey - py).abs() < 1.5 && enemy.x > px {
                                let beam_dmg = (ship.damage() / 3).max(1);
                                let actual = beam_dmg.min(enemy.hp);
                                enemy.hp -= actual;
                                if !enemy.is_alive() {
                                    enemy.death_frame = 1;
                                    if !enemy.is_big() {
                                        let cx = enemy.x + enemy.sprite[0].chars().count() as f32 / 2.0;
                                        Self::chain_explosion(particles, cx, ey, false);
                                        enemy.death_exploded = true;
                                    }
                                    state.enemies_destroyed += 1;
                                }
                            }
                        }
                        if ps.beam_active == 0 {
                            ps.ability_cooldown = 0; // reset to count back up
                        }
                        continue;
                    }

                    if ps.beam_charge > 0 {
                        ps.beam_charge += 1;
                        if ps.beam_charge >= 60 {
                            // Fire the beam!
                            ps.beam_charge = 0;
                            ps.beam_active = 10;
                            self.active_beams.push(ActiveBeam {
                                ship_idx: i,
                                y: py,
                                start_x: muzzle_x,
                                ticks_left: 10,
                            });
                            // Big flash at muzzle
                            particles.explode(muzzle_x, py, 12, Color::White);
                        }
                        continue;
                    }

                    // Count up toward ability trigger
                    ps.ability_cooldown += 1;
                    if ps.ability_cooldown >= ability.cooldown_ticks() {
                        // Start charging
                        ps.beam_charge = 1;
                        ps.ability_cooldown = 0;
                    }
                    continue;
                }

                // All other active abilities: count up cooldown
                ps.ability_cooldown += 1;
                if ps.ability_cooldown < ability.cooldown_ticks() {
                    continue;
                }
                // Ability fires! Reset cooldown.
                ps.ability_cooldown = 0;

                match ability {
                    ShipAbility::HeavyPayload => {
                        // Fire a slow, large AOE projectile
                        new_projectiles.push(Projectile {
                            x: muzzle_x,
                            y: py,
                            vx: 0.5,
                            vy: 0.0,
                            damage: ship.damage() * 2,
                            friendly: true,
                            kind: ProjectileKind::HeavyPayload,
                            homing_target_y: -1.0,
                        });
                        Self::emit_muzzle_flash(particles, muzzle_x, py, true);
                        // Extra muzzle particles for drama
                        particles.explode(muzzle_x, py, 6, Color::Rgb(255, 200, 50));
                    }
                    ShipAbility::Shield => {
                        // Activate shield: 50% damage reduction for 100 ticks
                        ps.shield_timer = 100;
                        // Shield activation flash
                        particles.explode(px + sprite_w / 2.0, py, 8, Color::Rgb(80, 180, 255));
                    }
                    ShipAbility::Broadside => {
                        // Fire 5 projectiles in a vertical spread
                        let offsets: [f32; 5] = [-2.0, -1.0, 0.0, 1.0, 2.0];
                        for &y_off in &offsets {
                            new_projectiles.push(Projectile {
                                x: muzzle_x,
                                y: py + y_off,
                                vx: 1.5,
                                vy: y_off * 0.05, // slight spread
                                damage: ship.damage(),
                                friendly: true,
                                kind: ProjectileKind::BroadsideShot,
                                homing_target_y: -1.0,
                            });
                        }
                        // Big muzzle flash
                        Self::emit_muzzle_flash(particles, muzzle_x, py, true);
                        particles.explode(muzzle_x, py, 10, Color::Rgb(180, 220, 255));
                    }
                    ShipAbility::LaunchFighters => {
                        // Spawn 2 temporary fighter allies
                        for offset in [-2.0_f32, 2.0] {
                            new_temp_fighters.push(TempFighter {
                                x: px + sprite_w / 2.0,
                                y: py + offset,
                                hp: 15,
                                damage: 6,
                                fire_cooldown: 5,
                                lifetime: 200,
                                target_idx: -1,
                            });
                        }
                        // Launch particles
                        particles.explode(px + sprite_w, py, 8, Color::Rgb(100, 255, 180));
                    }
                    // Scan and BeamWeapon handled above
                    _ => {}
                }
            }

            self.projectiles.extend(new_projectiles);
            self.temp_fighters.extend(new_temp_fighters);
        }

        // ── Update shield timers ───────────────────────────────────────
        for ps in self.player_states.iter_mut() {
            if ps.shield_timer > 0 {
                ps.shield_timer -= 1;
            }
        }

        // ── Update active beams ────────────────────────────────────────
        self.active_beams.retain_mut(|beam| {
            if beam.ticks_left == 0 {
                return false;
            }
            beam.ticks_left -= 1;
            true
        });

        // ── Temp fighter AI: move, target, fire ────────────────────────
        {
            let mut temp_projectiles: Vec<Projectile> = Vec::new();
            for tf in self.temp_fighters.iter_mut() {
                if !tf.is_alive() {
                    continue;
                }
                tf.lifetime -= 1;

                // Pick target (nearest alive enemy)
                if tf.target_idx < 0 || self.tick_count % 30 == 0 {
                    let mut best = -1i32;
                    let mut best_dist = f32::INFINITY;
                    for (ei, enemy) in self.enemies.iter().enumerate() {
                        if !enemy.is_alive() {
                            continue;
                        }
                        let dx = enemy.x - tf.x;
                        let dy = enemy.y - tf.y;
                        let dist = dx * dx + dy * dy;
                        if dist < best_dist {
                            best_dist = dist;
                            best = ei as i32;
                        }
                    }
                    tf.target_idx = best;
                }

                // Move toward target
                if tf.target_idx >= 0 {
                    let tidx = tf.target_idx as usize;
                    if tidx < self.enemies.len() && self.enemies[tidx].is_alive() {
                        let target = &self.enemies[tidx];
                        let dx = target.x - tf.x;
                        let dy = target.y - tf.y;
                        let dist = (dx * dx + dy * dy).sqrt().max(0.01);
                        // Move at fighter speed but stay behind enemy
                        let desired_x = target.x - 10.0;
                        let move_dx = (desired_x - tf.x).clamp(-0.8, 0.8);
                        let move_dy = (dy / dist * 0.5).clamp(-0.4, 0.4);
                        tf.x += move_dx;
                        tf.y += move_dy;
                    }
                }

                // Clamp to screen
                tf.x = tf.x.clamp(2.0, self.width as f32 - 5.0);
                tf.y = tf.y.clamp(1.0, self.height as f32 - 2.0);

                // Fire
                if tf.fire_cooldown == 0 {
                    temp_projectiles.push(Projectile {
                        x: tf.x + 2.0,
                        y: tf.y,
                        vx: 1.5,
                        vy: 0.0,
                        damage: tf.damage,
                        friendly: true,
                        kind: ProjectileKind::TempFighterShot,
                        homing_target_y: -1.0,
                    });
                    tf.fire_cooldown = 8; // fast fire rate
                } else {
                    tf.fire_cooldown -= 1;
                }
            }
            self.projectiles.extend(temp_projectiles);

            // Remove expired temp fighters with particle poof
            self.temp_fighters.retain(|tf| {
                if !tf.is_alive() {
                    // Note: can't access particles here, handled separately
                    return false;
                }
                true
            });
        }

        // ── Player fleet fires ─────────────────────────────────────────
        let positions = self.player_positions(&state.fleet);
        for (i, ship) in state.fleet.iter().enumerate() {
            if !ship.is_alive() || i >= self.player_states.len() {
                continue;
            }
            if self.player_states[i].fire_cooldown == 0 {
                let (px, py) = positions[i];
                let sprite_w = ship.ship_type.sprite()[0].chars().count() as f32;
                let muzzle_x = px + sprite_w + 1.0;
                let kind = Self::player_projectile_kind(ship.ship_type);
                let speed = match kind {
                    ProjectileKind::BomberMissile => 0.8 + ship.speed() * 0.05,
                    ProjectileKind::CapitalBeam => 0.6 + ship.speed() * 0.04,
                    _ => 1.2 + ship.speed() * 0.08,
                };

                // Smart targeting: aim toward selected enemy
                let target_idx = Self::player_select_target(ship, &self.enemies);
                let vy = if target_idx >= 0 {
                    let tidx = target_idx as usize;
                    if tidx < self.enemies.len() && self.enemies[tidx].is_alive() {
                        let ey = self.enemies[tidx].y;
                        let dy = ey - py;
                        // Slight vertical aim, more for homing missiles
                        match kind {
                            ProjectileKind::BomberMissile => dy.clamp(-0.3, 0.3),
                            _ => dy.clamp(-0.1, 0.1),
                        }
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };

                // Homing target for bomber missiles
                let homing_y = if kind == ProjectileKind::BomberMissile && target_idx >= 0 {
                    let tidx = target_idx as usize;
                    if tidx < self.enemies.len() {
                        self.enemies[tidx].y
                    } else {
                        -1.0
                    }
                } else {
                    -1.0
                };

                self.projectiles.push(Projectile {
                    x: muzzle_x,
                    y: py,
                    vx: speed,
                    vy,
                    damage: ship.damage(),
                    friendly: true,
                    kind,
                    homing_target_y: homing_y,
                });

                Self::emit_muzzle_flash(particles, muzzle_x, py, true);
                self.player_states[i].fire_cooldown = fire_rate_ticks(ship.speed(), self.sector);
            } else {
                self.player_states[i].fire_cooldown -= 1;
            }
        }

        // ── Enemy fleet fires ──────────────────────────────────────────
        let positions = self.player_positions(&state.fleet);
        let player_x = self.player_x_start();
        for e in self.enemies.iter_mut() {
            if !e.is_alive() {
                continue;
            }

            // Bomber suicide check: if reached player fleet x range
            if e.strategy == AIStrategy::Bomber && e.x <= player_x + 12.0 {
                let damage = e.damage * 3; // triple damage on suicide
                e.hp = 0;
                e.death_frame = 1;
                // Explode with area damage
                let bomber_x = e.x;
                let bomber_y = e.y;
                let destroyed = Self::bomber_explode(
                    particles,
                    bomber_x,
                    bomber_y,
                    damage,
                    &mut state.fleet,
                    &positions,
                    &mut self.player_states,
                );
                state.enemies_destroyed += 1 + destroyed;
                continue;
            }

            if e.fire_cooldown == 0 {
                let kind = Self::enemy_projectile_kind(e.tier);
                let speed = match kind {
                    ProjectileKind::EnemyHeavy => 0.6 + e.speed * 0.04,
                    _ => 0.8 + e.speed * 0.06,
                };
                let muzzle_x = e.x - 1.0;

                // Aim at target
                let vy = if e.target_idx >= 0 {
                    let tidx = e.target_idx as usize;
                    if tidx < positions.len() && tidx < state.fleet.len() && state.fleet[tidx].is_alive() {
                        let (_, ty) = positions[tidx];
                        let dy = ty - e.y;
                        dy.clamp(-0.15, 0.15)
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };

                self.projectiles.push(Projectile {
                    x: muzzle_x,
                    y: e.y,
                    vx: -speed,
                    vy,
                    damage: e.damage,
                    friendly: false,
                    kind,
                    homing_target_y: -1.0,
                });

                Self::emit_muzzle_flash(particles, muzzle_x, e.y, false);

                // Aggressive enemies fire faster
                let rate_mult = if e.strategy == AIStrategy::Aggressive {
                    0.7
                } else {
                    1.0
                };
                e.fire_cooldown = ((e.fire_rate as f32 * rate_mult) as u32).max(2);
            } else {
                e.fire_cooldown -= 1;
            }
        }

        // ── Move projectiles ───────────────────────────────────────────
        for p in self.projectiles.iter_mut() {
            p.x += p.vx;
            p.y += p.vy;

            // Homing: bomber missiles adjust vy toward target
            if p.kind == ProjectileKind::BomberMissile && p.homing_target_y >= 0.0 {
                let dy = p.homing_target_y - p.y;
                p.vy += dy.clamp(-0.02, 0.02);
                p.vy = p.vy.clamp(-0.4, 0.4);
            }

            // Capital beam trail particles
            if p.kind == ProjectileKind::CapitalBeam && self.tick_count % 2 == 0 {
                Self::emit_beam_trail(particles, p.x - 2.0, p.y);
            }
        }

        // Remove off-screen
        let w = self.width as f32;
        let h = self.height as f32;
        self.projectiles
            .retain(|p| p.x >= -4.0 && p.x <= w + 4.0 && p.y >= -2.0 && p.y <= h + 2.0);

        // ── Hit detection: friendly projectiles → enemies ──────────────
        let mut to_remove_proj: Vec<usize> = Vec::new();
        // Collect AOE hits to process after the loop (avoids borrow issues)
        let mut aoe_hits: Vec<(f32, f32, u32)> = Vec::new(); // (x, y, damage)
        for (pi, proj) in self.projectiles.iter().enumerate() {
            if !proj.friendly {
                continue;
            }
            for (_ei, enemy) in self.enemies.iter_mut().enumerate() {
                if !enemy.is_alive() {
                    continue;
                }
                let sprite_w = enemy.sprite[0].chars().count() as f32;
                let hit_x = proj.x >= enemy.x && proj.x <= enemy.x + sprite_w;
                let hit_y = (proj.y - enemy.y).abs() < (enemy.height() as f32 * 0.5 + 0.5);
                if hit_x && hit_y {
                    let dmg = proj.damage.min(enemy.hp);
                    enemy.hp -= dmg;
                    to_remove_proj.push(pi);

                    // HeavyPayload: AOE explosion on hit
                    if proj.kind == ProjectileKind::HeavyPayload {
                        aoe_hits.push((proj.x, proj.y, proj.damage));
                        // Big dramatic explosion
                        particles.explode(proj.x, proj.y, 20, Color::Rgb(255, 200, 50));
                        Self::chain_explosion(particles, proj.x, proj.y, true);
                    } else {
                        // Normal hit spark
                        particles.emit(Particle::new(
                            proj.x,
                            proj.y,
                            0.0,
                            0.0,
                            2,
                            '✦',
                            Color::White,
                        ));
                    }

                    // Death? Start death animation
                    if !enemy.is_alive() {
                        enemy.death_frame = 1;
                        if !enemy.is_big() {
                            // Small ships: immediate full explosion
                            let center_x = enemy.x + sprite_w / 2.0;
                            Self::chain_explosion(particles, center_x, enemy.y, false);
                            enemy.death_exploded = true;
                        }
                        state.enemies_destroyed += 1;
                    }
                    break;
                }
            }
        }

        // ── Process HeavyPayload AOE damage ────────────────────────────
        for (aoe_x, aoe_y, aoe_dmg) in aoe_hits {
            for enemy in self.enemies.iter_mut() {
                if !enemy.is_alive() {
                    continue;
                }
                // Damage all enemies within 3 y-units of the impact
                let dy = (enemy.y - aoe_y).abs();
                if dy <= 3.0 && (enemy.x - aoe_x).abs() < 10.0 {
                    let falloff = 1.0 - (dy / 3.0);
                    let splash = ((aoe_dmg as f32 * 0.6 * falloff) as u32).max(1);
                    let actual = splash.min(enemy.hp);
                    enemy.hp -= actual;
                    if !enemy.is_alive() {
                        enemy.death_frame = 1;
                        let sprite_w = enemy.sprite[0].chars().count() as f32;
                        if !enemy.is_big() {
                            let cx = enemy.x + sprite_w / 2.0;
                            Self::chain_explosion(particles, cx, enemy.y, false);
                            enemy.death_exploded = true;
                        }
                        state.enemies_destroyed += 1;
                    }
                }
            }
        }

        // ── Hit detection: enemy projectiles → player ships ────────────
        let positions = self.player_positions(&state.fleet);
        for (pi, proj) in self.projectiles.iter().enumerate() {
            if proj.friendly {
                continue;
            }
            for (si, ship) in state.fleet.iter_mut().enumerate() {
                if !ship.is_alive() || si >= positions.len() {
                    continue;
                }
                let (sx, sy) = positions[si];
                let sprite = ship.ship_type.sprite();
                let sprite_w = sprite[0].chars().count() as f32;
                let sprite_h = sprite.len() as f32;
                let hit_x = proj.x >= sx && proj.x <= sx + sprite_w;
                let hit_y = (proj.y - sy).abs() < (sprite_h * 0.5 + 0.5);
                if hit_x && hit_y {
                    // Shield damage reduction
                    let raw_dmg = proj.damage;
                    let effective_dmg = if si < self.player_states.len()
                        && self.player_states[si].shield_timer > 0
                    {
                        raw_dmg / 2 // 50% reduction
                    } else {
                        raw_dmg
                    };
                    let dmg = effective_dmg.min(ship.current_hp);
                    ship.current_hp -= dmg;
                    if !to_remove_proj.contains(&pi) {
                        to_remove_proj.push(pi);
                    }

                    // Flash (blue if shielded, red if not)
                    if si < self.player_states.len() {
                        self.player_states[si].flash = 3;
                    }

                    // Hit spark
                    particles.emit(Particle::new(
                        proj.x,
                        proj.y,
                        0.0,
                        0.0,
                        2,
                        '✦',
                        Color::White,
                    ));

                    // Death? Start death animation
                    if !ship.is_alive() && si < self.player_states.len() {
                        self.player_states[si].death_frame = 1;
                        let is_big = matches!(
                            ship.ship_type,
                            ShipType::Destroyer
                                | ShipType::Capital
                                | ShipType::Carrier
                                | ShipType::Frigate
                        );
                        if !is_big {
                            Self::chain_explosion(
                                particles,
                                sx + sprite_w / 2.0,
                                sy,
                                false,
                            );
                            self.player_states[si].death_exploded = true;
                        }
                    }
                    break;
                }
            }
        }

        // Remove hit projectiles (reverse order)
        to_remove_proj.sort_unstable();
        to_remove_proj.dedup();
        for i in to_remove_proj.into_iter().rev() {
            if i < self.projectiles.len() {
                self.projectiles.swap_remove(i);
            }
        }

        // ── Decrement flash timers ─────────────────────────────────────
        for ps in self.player_states.iter_mut() {
            ps.flash = ps.flash.saturating_sub(1);
        }

        // ── Win / lose check ───────────────────────────────────────────
        // All enemies must be fully dead (done animating)
        let all_enemies_dead = self.enemies.iter().all(|e| e.hp == 0);
        let all_enemies_done = self.enemies.iter().all(|e| e.is_done() || (e.hp == 0 && e.death_frame == 0));

        if all_enemies_dead && (all_enemies_done || self.enemies.iter().all(|e| e.hp == 0)) {
            // Mark all as done if they haven't started death anim
            for e in self.enemies.iter_mut() {
                if e.death_frame == 0 && e.hp == 0 {
                    e.death_frame = 16; // skip animation
                }
            }
            self.player_won = true;
            Self::projectiles_to_particles(&mut self.projectiles, particles);
            self.phase = BattlePhase::VictoryPose(20); // ~1 second victory formation
            return SceneAction::Continue;
        }

        if Self::player_alive_count(&state.fleet) == 0 {
            self.player_won = false;
            Self::projectiles_to_particles(&mut self.projectiles, particles);
            self.phase = BattlePhase::Freeze(10);
            return SceneAction::Continue;
        }

        // Fallback timeout
        state.phase_timer -= 0.05;
        if state.phase_timer <= 0.0 {
            self.player_won = true;
            state.total_battles += 1;
            return SceneAction::TransitionTo(GamePhase::Loot);
        }

        SceneAction::Continue
    }

    fn render(&self, frame: &mut Frame, state: &GameState, particles: &ParticleSystem) {
        let area = frame.area();
        let buf = frame.buffer_mut();

        // ── Player ships (left side) ───────────────────────────────────
        let positions = self.player_positions(&state.fleet);
        for (i, ship) in state.fleet.iter().enumerate() {
            if i >= positions.len() {
                continue;
            }
            if i < self.player_states.len() {
                let ps = &self.player_states[i];

                // Death animation rendering
                if !ship.is_alive() {
                    if ps.death_frame > 0 && ps.death_frame <= 3 {
                        // Frame 1-3: bright white block
                        let (fx, fy) = positions[i];
                        let sprite = ship.ship_type.sprite();
                        for (row, line) in sprite.iter().enumerate() {
                            let sy = (fy + row as f32) as u16;
                            for (col, _) in line.chars().enumerate() {
                                let sx = (fx + col as f32) as u16;
                                if sx < area.width && sy < area.height {
                                    let cell = &mut buf[(area.x + sx, area.y + sy)];
                                    cell.set_char('█');
                                    cell.set_fg(Color::White);
                                    cell.set_bg(Color::Reset);
                                }
                            }
                        }
                    }
                    // Frame 4+: handled by particles
                    continue;
                }
            } else if !ship.is_alive() {
                continue;
            }

            let (fx, fy) = positions[i];
            let sprite = ship.ship_type.sprite();
            let flashing = i < self.player_states.len() && self.player_states[i].flash > 0;
            let damaged = ship.current_hp < ship.max_hp();

            let fg = if flashing {
                Color::White
            } else if damaged && self.tick_count % 6 < 2 {
                Color::DarkGray
            } else {
                Color::Cyan
            };
            let bg = if flashing {
                Color::Red
            } else {
                Color::Reset
            };

            for (row, line) in sprite.iter().enumerate() {
                let sy = (fy + row as f32) as u16;
                for (col, ch) in line.chars().enumerate() {
                    let sx = (fx + col as f32) as u16;
                    if sx < area.width && sy < area.height {
                        let cell = &mut buf[(area.x + sx, area.y + sy)];
                        cell.set_char(ch);
                        cell.set_fg(fg);
                        cell.set_bg(bg);
                    }
                }
            }
        }

        // ── Enemy ships (right side) ──────────────────────────────────
        for (_i, enemy) in self.enemies.iter().enumerate() {
            // Death animation rendering
            if !enemy.is_alive() {
                if enemy.death_frame > 0 && enemy.death_frame <= 3 {
                    // Frame 1-3: bright white block
                    for (row, line) in enemy.sprite.iter().enumerate() {
                        let sy = (enemy.y + row as f32) as u16;
                        for (col, _) in line.chars().enumerate() {
                            let sx = (enemy.x + col as f32) as u16;
                            if sx < area.width && sy < area.height {
                                let cell = &mut buf[(area.x + sx, area.y + sy)];
                                cell.set_char('█');
                                cell.set_fg(Color::White);
                                cell.set_bg(Color::Reset);
                            }
                        }
                    }
                }
                continue;
            }

            let damaged = enemy.hp < enemy.max_hp;

            // Color based on strategy for visual variety
            let base_fg = match enemy.strategy {
                AIStrategy::Bomber => Color::Yellow,
                AIStrategy::Aggressive => Color::Rgb(255, 100, 100),
                AIStrategy::Flanker => Color::Rgb(255, 130, 180),
                _ => Color::LightRed,
            };

            let fg = if damaged && self.tick_count % 6 < 2 {
                Color::DarkGray
            } else {
                base_fg
            };

            for (row, line) in enemy.sprite.iter().enumerate() {
                let sy = (enemy.y + row as f32) as u16;
                for (col, ch) in line.chars().enumerate() {
                    let sx = (enemy.x + col as f32) as u16;
                    if sx < area.width && sy < area.height {
                        let cell = &mut buf[(area.x + sx, area.y + sy)];
                        cell.set_char(ch);
                        cell.set_fg(fg);
                        cell.set_bg(Color::Reset);
                    }
                }
            }
        }

        // ── Projectiles with trailing chars ───────────────────────────
        for p in &self.projectiles {
            let py = p.y as u16;
            if py >= area.height {
                continue;
            }

            let trail = p.trail_chars();
            let head_color = p.color();
            let trail_color = p.trail_color();

            for (idx, &ch) in trail.iter().enumerate() {
                let offset = if p.vx > 0.0 {
                    -(idx as f32)
                } else {
                    idx as f32
                };
                let tx = (p.x + offset) as i32;
                if tx >= 0 && (tx as u16) < area.width {
                    let cell = &mut buf[(area.x + tx as u16, area.y + py)];
                    cell.set_char(ch);
                    cell.set_fg(if idx == 0 { head_color } else { trail_color });
                }
            }
        }

        // ── Shield visuals ─────────────────────────────────────────────
        let positions = self.player_positions(&state.fleet);
        for (i, ship) in state.fleet.iter().enumerate() {
            if !ship.is_alive() || i >= self.player_states.len() || i >= positions.len() {
                continue;
            }
            let ps = &self.player_states[i];
            if ps.shield_timer > 0 {
                let (fx, fy) = positions[i];
                let sprite = ship.ship_type.sprite();
                let sprite_h = sprite.len();
                // Render `[` shield char in front of ship for each row
                let shield_x = (fx - 1.0) as u16;
                let pulse = if self.tick_count % 6 < 3 {
                    Color::Rgb(80, 180, 255)
                } else {
                    Color::Rgb(40, 120, 200)
                };
                for row in 0..sprite_h {
                    let sy = (fy + row as f32) as u16;
                    if shield_x < area.width && sy < area.height {
                        let cell = &mut buf[(area.x + shield_x, area.y + sy)];
                        cell.set_char('[');
                        cell.set_fg(pulse);
                    }
                }
            }

            // ── Beam charge visual ─────────────────────────────────────
            if ps.beam_charge > 0 {
                let (fx, fy) = positions[i];
                let sprite_w = ship.ship_type.sprite()[0].chars().count() as f32;
                let muzzle_x = (fx + sprite_w + 1.0) as u16;
                let charge_y = fy as u16;
                // Growing ▸▸▸ based on charge progress
                let num_arrows = ((ps.beam_charge as f32 / 60.0) * 6.0).ceil() as usize;
                let charge_color = if self.tick_count % 4 < 2 {
                    Color::Rgb(100, 220, 255)
                } else {
                    Color::White
                };
                for c in 0..num_arrows.min(6) {
                    let cx = muzzle_x + c as u16;
                    if cx < area.width && charge_y < area.height {
                        let cell = &mut buf[(area.x + cx, area.y + charge_y)];
                        cell.set_char('▸');
                        cell.set_fg(charge_color);
                    }
                }
            }
        }

        // ── Active beam weapons ────────────────────────────────────────
        for beam in &self.active_beams {
            let by = beam.y as u16;
            if by >= area.height {
                continue;
            }
            // Beam flickers between white and cyan
            let beam_color = if self.tick_count % 3 == 0 {
                Color::White
            } else {
                Color::Rgb(100, 220, 255)
            };
            let start = beam.start_x as u16;
            for bx in start..area.width {
                let cell = &mut buf[(area.x + bx, area.y + by)];
                cell.set_char('━');
                cell.set_fg(beam_color);
            }
        }

        // ── Temp fighters ──────────────────────────────────────────────
        for tf in &self.temp_fighters {
            if !tf.is_alive() {
                continue;
            }
            let tx = tf.x as u16;
            let ty = tf.y as u16;
            // Render as `=>`
            let fg = if tf.lifetime < 30 && self.tick_count % 4 < 2 {
                Color::DarkGray // flicker when about to expire
            } else {
                Color::Rgb(100, 255, 180)
            };
            if tx < area.width.saturating_sub(1) && ty < area.height {
                let cell = &mut buf[(area.x + tx, area.y + ty)];
                cell.set_char('=');
                cell.set_fg(fg);
                let cell2 = &mut buf[(area.x + tx + 1, area.y + ty)];
                cell2.set_char('>');
                cell2.set_fg(fg);
            }
        }

        // ── Enemy HP bars (when Scan ability active) ───────────────────
        if self.has_scan {
            for enemy in &self.enemies {
                if !enemy.is_alive() {
                    continue;
                }
                let bar_x = enemy.x as u16;
                let bar_y = (enemy.y - 1.0) as u16;
                if bar_y == 0 || bar_y >= area.height || bar_x >= area.width {
                    continue;
                }
                let hp_ratio = enemy.hp_ratio();
                let bar_len = 6usize.min((area.width - bar_x) as usize);
                let filled = (hp_ratio * bar_len as f32) as usize;
                let hp_color = if hp_ratio > 0.5 {
                    Color::Red
                } else if hp_ratio > 0.25 {
                    Color::Rgb(255, 140, 0)
                } else {
                    Color::Rgb(255, 60, 60)
                };
                for bi in 0..bar_len {
                    let bx = bar_x + bi as u16;
                    if bx < area.width {
                        let cell = &mut buf[(area.x + bx, area.y + bar_y)];
                        if bi < filled {
                            cell.set_char('▬');
                            cell.set_fg(hp_color);
                        } else {
                            cell.set_char('▬');
                            cell.set_fg(Color::DarkGray);
                        }
                    }
                }
            }
        }

        // ── Particles ─────────────────────────────────────────────────
        for p in &particles.particles {
            let px = p.x as u16;
            let py = p.y as u16;
            if px < area.width && py < area.height {
                let cell = &mut buf[(area.x + px, area.y + py)];
                cell.set_char(p.render_char());
                cell.set_fg(p.color);
            }
        }

        // ── Focus fire indicator ──────────────────────────────────────
        if self.focus_fire_target >= 0 {
            let tidx = self.focus_fire_target as usize;
            if tidx < positions.len() && tidx < state.fleet.len() && state.fleet[tidx].is_alive() {
                let (_tx, ty) = positions[tidx];
                let indicator_x = 1u16;
                let indicator_y = ty as u16;
                if indicator_x < area.width && indicator_y < area.height {
                    let cell = &mut buf[(area.x + indicator_x, area.y + indicator_y)];
                    cell.set_char('⚠');
                    cell.set_fg(Color::Red);
                }
            }
        }

        // ── Health bars at bottom ──────────────────────────────────────
        let bar_y = area.height.saturating_sub(2);
        let bar_width = (area.width / 2).saturating_sub(4) as usize;

        // Player fleet HP
        let player_hp: u32 = state.fleet.iter().map(|s| s.current_hp).sum();
        let player_max: u32 = state.fleet.iter().map(|s| s.max_hp()).sum();
        let player_ratio = if player_max > 0 {
            player_hp as f32 / player_max as f32
        } else {
            0.0
        };
        let player_filled = (player_ratio * bar_width as f32) as usize;

        let label = "FLEET ";
        for (i, ch) in label.chars().enumerate() {
            let x = 1 + i as u16;
            if x < area.width && bar_y < area.height {
                let cell = &mut buf[(area.x + x, area.y + bar_y)];
                cell.set_char(ch);
                cell.set_fg(Color::Green);
            }
        }
        let bar_start = 1 + label.len() as u16;
        for i in 0..bar_width {
            let x = bar_start + i as u16;
            if x < area.width && bar_y < area.height {
                let cell = &mut buf[(area.x + x, area.y + bar_y)];
                if i < player_filled {
                    cell.set_char('█');
                    cell.set_fg(Color::Green);
                } else {
                    cell.set_char('░');
                    cell.set_fg(Color::DarkGray);
                }
            }
        }
        let hp_text = format!(" {}/{}", player_hp, player_max);
        let hp_x = bar_start + bar_width as u16;
        for (i, ch) in hp_text.chars().enumerate() {
            let x = hp_x + i as u16;
            if x < area.width && bar_y < area.height {
                let cell = &mut buf[(area.x + x, area.y + bar_y)];
                cell.set_char(ch);
                cell.set_fg(Color::Green);
            }
        }

        // Enemy fleet HP
        let enemy_hp = self.enemy_total_hp();
        let enemy_max = self.enemy_max_hp();
        let enemy_ratio = if enemy_max > 0 {
            enemy_hp as f32 / enemy_max as f32
        } else {
            0.0
        };
        let enemy_filled = (enemy_ratio * bar_width as f32) as usize;

        let right_start = area.width / 2 + 2;
        let elabel = "ENEMY ";
        for (i, ch) in elabel.chars().enumerate() {
            let x = right_start + i as u16;
            if x < area.width && bar_y < area.height {
                let cell = &mut buf[(area.x + x, area.y + bar_y)];
                cell.set_char(ch);
                cell.set_fg(Color::Red);
            }
        }
        let ebar_start = right_start + elabel.len() as u16;
        for i in 0..bar_width {
            let x = ebar_start + i as u16;
            if x < area.width && bar_y < area.height {
                let cell = &mut buf[(area.x + x, area.y + bar_y)];
                if i < enemy_filled {
                    cell.set_char('█');
                    cell.set_fg(Color::Red);
                } else {
                    cell.set_char('░');
                    cell.set_fg(Color::DarkGray);
                }
            }
        }
        let ehp_text = format!(" {}/{}", enemy_hp, enemy_max);
        let ehp_x = ebar_start + bar_width as u16;
        for (i, ch) in ehp_text.chars().enumerate() {
            let x = ehp_x + i as u16;
            if x < area.width && bar_y < area.height {
                let cell = &mut buf[(area.x + x, area.y + bar_y)];
                cell.set_char(ch);
                cell.set_fg(Color::Red);
            }
        }

        // ── Battle header ──────────────────────────────────────────────
        let phase_label = match self.phase {
            BattlePhase::SlideIn => " — Engaging...",
            BattlePhase::VictoryPose(_) => " — VICTORY!",
            BattlePhase::Freeze(_) => {
                if self.player_won {
                    " — VICTORY!"
                } else {
                    " — DEFEATED"
                }
            }
            BattlePhase::Combat => "",
        };
        let header = format!(
            "⚔ BATTLE — Sector {} — {:.0}s{}",
            state.sector, state.phase_timer, phase_label
        );
        let hx = (area.width / 2).saturating_sub(header.len() as u16 / 2);
        for (i, ch) in header.chars().enumerate() {
            let x = hx + i as u16;
            if x < area.width && area.height > 0 {
                let cell = &mut buf[(area.x + x, area.y)];
                cell.set_char(ch);
                cell.set_fg(Color::Yellow);
            }
        }
    }
}
