use rand::Rng;
use ratatui::style::{Color, Style};
use ratatui::Frame;

use crate::rendering::particles::{Particle, ParticleSystem};
use crate::state::{GamePhase, GameState};

use super::{Scene, SceneAction};

// ── Planet types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlanetType {
    Desert,
    Urban,
    Ice,
    Volcanic,
    Ocean,
}

impl PlanetType {
    fn pick(rng: &mut impl Rng) -> Self {
        match rng.gen_range(0..5) {
            0 => Self::Desert,
            1 => Self::Urban,
            2 => Self::Ice,
            3 => Self::Volcanic,
            _ => Self::Ocean,
        }
    }

    /// Fill a terrain row with appropriate block characters.
    fn terrain_chars(&self, rng: &mut impl Rng, width: usize, row: usize) -> Vec<char> {
        (0..width)
            .map(|_col| match self {
                Self::Desert => {
                    let r: u8 = rng.gen_range(0..20);
                    if r < 6 {
                        '░'
                    } else if r < 12 {
                        '▒'
                    } else if r == 12 && row == 0 {
                        'ψ' // cactus on surface row
                    } else {
                        '░'
                    }
                }
                Self::Urban => {
                    // Buildings are handled separately via building structs
                    let r: u8 = rng.gen_range(0..10);
                    if r < 4 {
                        '░'
                    } else if r < 7 {
                        '▒'
                    } else {
                        '·'
                    }
                }
                Self::Ice => {
                    let r: u8 = rng.gen_range(0..15);
                    if r < 5 {
                        '░'
                    } else if r < 8 {
                        '·'
                    } else if r < 11 {
                        '∙'
                    } else {
                        '▒'
                    }
                }
                Self::Volcanic => {
                    let r: u8 = rng.gen_range(0..15);
                    if r < 4 {
                        '▓'
                    } else if r < 8 {
                        '▒'
                    } else if r < 11 {
                        '~'
                    } else {
                        '░'
                    }
                }
                Self::Ocean => {
                    let r: u8 = rng.gen_range(0..15);
                    if r < 5 {
                        '≈'
                    } else if r < 9 {
                        '∼'
                    } else if r < 12 {
                        '~'
                    } else if r == 12 && row == 0 {
                        '▓' // small island
                    } else {
                        '≈'
                    }
                }
            })
            .collect()
    }

    fn terrain_fg(&self) -> Color {
        match self {
            Self::Desert => Color::Yellow,
            Self::Urban => Color::Gray,
            Self::Ice => Color::Cyan,
            Self::Volcanic => Color::Red,
            Self::Ocean => Color::Blue,
        }
    }

    fn terrain_bg(&self) -> Color {
        match self {
            Self::Desert => Color::Indexed(94),   // dark yellow/brown
            Self::Urban => Color::DarkGray,
            Self::Ice => Color::Indexed(17),       // dark blue
            Self::Volcanic => Color::Indexed(52),  // dark red
            Self::Ocean => Color::Indexed(18),     // deep blue
        }
    }

    fn resource_char(&self) -> char {
        match self {
            Self::Desert => '·',
            Self::Urban => '◇',
            Self::Ice => '✦',
            Self::Volcanic => '◆',
            Self::Ocean => '○',
        }
    }

    fn resource_color(&self) -> Color {
        match self {
            Self::Desert => Color::Yellow,
            Self::Urban => Color::LightYellow,
            Self::Ice => Color::LightCyan,
            Self::Volcanic => Color::LightRed,
            Self::Ocean => Color::LightGreen,
        }
    }
}

// ── Surface entities ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct SurfaceEntity {
    x: f32,
    base_speed: f32,
    kind: EntityKind,
    scared: bool,
    frozen_timer: u8,     // frames of freeze-in-fear before running
    panic_exclaim: u8,    // frames to show ! above head
}

#[derive(Debug, Clone, Copy)]
enum EntityKind {
    Person,
    Vehicle,
}

impl EntityKind {
    fn ch(&self) -> char {
        match self {
            Self::Person => 'o',
            Self::Vehicle => '=',
        }
    }

    fn color(&self) -> Color {
        match self {
            Self::Person => Color::White,
            Self::Vehicle => Color::LightYellow,
        }
    }
}

// ── Buildings (urban planet) ────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Building {
    x: u16,
    width: u8,       // 1-3 columns
    height: u8,      // 2-4 rows tall (current, shrinks under beam)
    #[allow(dead_code)]
    max_height: u8,
    destroyed: bool,
    crumble_timer: u8, // ticks until next crumble step
}

// ── Defense turrets ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Turret {
    x: u16,
    cooldown: u8,
}

#[derive(Debug, Clone)]
struct TurretProjectile {
    x: f32,
    y: f32,
    vy: f32,
}

// ── Tractor beam state ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct TractorBeam {
    x: u16,           // column on screen
    ship_y: u16,      // top of beam (ship position)
    surface_y: u16,   // bottom of beam (surface)
    frame: u8,        // animation frame counter
    active: bool,
}

// ── Hovering ship position ──────────────────────────────────────────────

#[derive(Debug, Clone)]
struct HoverShip {
    x: f32,
    base_y: f32,
    current_y: f32,    // for entry animation
    bob_offset: f32,   // phase offset for bobbing
    flash_frames: u8,
}

// ── Main scene ──────────────────────────────────────────────────────────

const TERRAIN_ROWS: u16 = 4;
const TICK_DT: f32 = 0.05; // 20fps
const BEAM_CHARS: [char; 4] = ['│', '┃', '║', ':'];
const ENTRY_FRAMES: u64 = 20;

pub struct RaidScene {
    width: u16,
    height: u16,
    tick_count: u64,
    planet_type: PlanetType,

    // Terrain grid: [row][col], row 0 is top row of terrain
    terrain: Vec<Vec<char>>,

    // Buildings (urban only)
    buildings: Vec<Building>,

    // Ships hovering
    ships: Vec<HoverShip>,

    // Tractor beams
    beams: Vec<TractorBeam>,

    // Surface entities (people/vehicles)
    entities: Vec<SurfaceEntity>,

    // Turrets and their projectiles
    turrets: Vec<Turret>,
    projectiles: Vec<TurretProjectile>,

    // Raid rewards accumulator
    scrap_gained: u64,
    credits_gained: u64,

    // Degradation tracking: number of terrain cells destroyed
    cells_degraded: u32,

    // Raid duration for reward calculation
    raid_duration: f32,
    elapsed: f32,
}

impl RaidScene {
    pub fn new() -> Self {
        Self {
            width: 80,
            height: 24,
            tick_count: 0,
            planet_type: PlanetType::Desert,
            terrain: Vec::new(),
            buildings: Vec::new(),
            ships: Vec::new(),
            beams: Vec::new(),
            entities: Vec::new(),
            turrets: Vec::new(),
            projectiles: Vec::new(),
            scrap_gained: 0,
            credits_gained: 0,
            cells_degraded: 0,
            raid_duration: 20.0,
            elapsed: 0.0,
        }
    }

    fn surface_top_y(&self) -> u16 {
        self.height.saturating_sub(TERRAIN_ROWS + 1) // +1 for status bar
    }

    fn generate_terrain(&mut self) {
        let mut rng = rand::thread_rng();
        self.terrain.clear();
        let w = self.width as usize;
        for row in 0..TERRAIN_ROWS as usize {
            self.terrain
                .push(self.planet_type.terrain_chars(&mut rng, w, row));
        }
    }

    fn generate_buildings(&mut self) {
        let mut rng = rand::thread_rng();
        self.buildings.clear();
        if self.planet_type != PlanetType::Urban {
            return;
        }
        // Place buildings across the width
        let mut x: u16 = rng.gen_range(2..6);
        while x < self.width.saturating_sub(4) {
            let w = rng.gen_range(1..=3_u8);
            let h = rng.gen_range(2..=4_u8);
            self.buildings.push(Building {
                x,
                width: w,
                height: h,
                max_height: h,
                destroyed: false,
                crumble_timer: 0,
            });
            x += w as u16 + rng.gen_range(2..6);
        }
    }

    fn position_fleet(&mut self, fleet_size: usize) {
        let mut rng = rand::thread_rng();
        self.ships.clear();

        // Spread ships horizontally across the top
        let hover_y = 3.0_f32; // stable hover row
        let usable_width = (self.width as f32 - 8.0).max(10.0);
        let n = fleet_size.max(1);
        let spacing = usable_width / n as f32;

        for i in 0..fleet_size {
            let x = 4.0 + (i as f32 + 0.5) * spacing;
            self.ships.push(HoverShip {
                x,
                base_y: hover_y,
                current_y: -3.0, // start above screen for entry animation
                bob_offset: rng.gen_range(0.0..std::f32::consts::TAU),
                flash_frames: 0,
            });
        }
    }

    fn spawn_beams(&mut self) {
        self.beams.clear();
        let surface_y = self.surface_top_y();
        for (i, ship) in self.ships.iter().enumerate() {
            let ship_y = ship.base_y as u16 + 2; // beam starts just below ship
            self.beams.push(TractorBeam {
                x: ship.x as u16 + 1,
                ship_y,
                surface_y,
                frame: (i as u8).wrapping_mul(7), // stagger animation
                active: false, // activated after entry
            });
        }
    }

    fn spawn_entities(&mut self) {
        let mut rng = rand::thread_rng();
        self.entities.clear();
        let count = rng.gen_range(3..8_u32);
        for _ in 0..count {
            let kind = if rng.gen_bool(0.6) {
                EntityKind::Person
            } else {
                EntityKind::Vehicle
            };
            let base_speed = match kind {
                EntityKind::Person => rng.gen_range(0.1..0.3),
                EntityKind::Vehicle => rng.gen_range(0.3..0.6),
            };
            self.entities.push(SurfaceEntity {
                x: rng.gen_range(2.0..(self.width as f32 - 2.0)),
                base_speed: if rng.gen_bool(0.5) {
                    base_speed
                } else {
                    -base_speed
                },
                kind,
                scared: false,
                frozen_timer: 0,
                panic_exclaim: 0,
            });
        }
    }

    fn spawn_turrets(&mut self, sector: u32) {
        let mut rng = rand::thread_rng();
        self.turrets.clear();
        self.projectiles.clear();

        // No turrets in early sectors
        if sector < 3 {
            return;
        }

        let max_turrets = ((sector - 2) as usize).min(5);
        let count = rng.gen_range(1..=max_turrets);
        for _ in 0..count {
            self.turrets.push(Turret {
                x: rng.gen_range(3..self.width.saturating_sub(3)),
                cooldown: rng.gen_range(10..30),
            });
        }
    }

    fn degrade_terrain(&mut self) {
        let mut rng = rand::thread_rng();
        if self.terrain.is_empty() {
            return;
        }
        for beam in &self.beams {
            if !beam.active {
                continue;
            }
            if rng.gen_range(0..20) == 0 {
                let col = beam.x as usize;
                let row = rng.gen_range(0..self.terrain.len());
                let offset = rng.gen_range(0..3_usize);
                let target_col = col.wrapping_add(offset).wrapping_sub(1);
                if row < self.terrain.len() {
                    if let Some(cell) = self.terrain[row].get_mut(target_col) {
                        if *cell != ' ' {
                            *cell = ' ';
                            self.cells_degraded += 1;
                        }
                    }
                }
            }
        }
    }

    fn crumble_buildings(&mut self, particles: &mut ParticleSystem) {
        if self.planet_type != PlanetType::Urban {
            return;
        }
        let mut rng = rand::thread_rng();
        let beam_xs: Vec<u16> = self
            .beams
            .iter()
            .filter(|b| b.active)
            .map(|b| b.x)
            .collect();
        let surface_y = self.surface_top_y();

        for bld in &mut self.buildings {
            if bld.destroyed {
                continue;
            }
            // Check if any beam overlaps this building
            let under_beam = beam_xs.iter().any(|&bx| {
                bx >= bld.x && bx < bld.x + bld.width as u16
            });
            if !under_beam {
                bld.crumble_timer = 0;
                continue;
            }
            bld.crumble_timer += 1;
            // Crumble every ~20 ticks (1 second)
            if bld.crumble_timer >= 20 {
                bld.crumble_timer = 0;
                if bld.height > 0 {
                    bld.height -= 1;
                    // Debris particles
                    let cx = bld.x as f32 + bld.width as f32 / 2.0;
                    let cy = surface_y as f32 - bld.height as f32;
                    for _ in 0..5 {
                        particles.emit(Particle::new(
                            cx + rng.gen_range(-1.5..1.5),
                            cy,
                            rng.gen_range(-0.5..0.5),
                            rng.gen_range(-0.3..0.3),
                            rng.gen_range(5..12),
                            if rng.gen_bool(0.5) { '▪' } else { '·' },
                            Color::DarkGray,
                        ));
                    }
                }
                if bld.height == 0 {
                    bld.destroyed = true;
                }
            }
        }
    }

    /// Emit atmosphere entry particles below descending ships.
    fn emit_entry_particles(&self, particles: &mut ParticleSystem) {
        let mut rng = rand::thread_rng();
        for ship in &self.ships {
            if ship.current_y >= ship.base_y {
                continue;
            }
            // Re-entry flame below ship
            for _ in 0..3 {
                let colors = [Color::Red, Color::LightRed, Color::Yellow, Color::LightYellow];
                particles.emit(Particle::new(
                    ship.x + rng.gen_range(-1.0..3.0),
                    ship.current_y + 2.0,
                    rng.gen_range(-0.3..0.3),
                    rng.gen_range(0.3..1.0),
                    rng.gen_range(3..8),
                    if rng.gen_bool(0.5) { '▓' } else { '░' },
                    colors[rng.gen_range(0..colors.len())],
                ));
            }
        }
    }

    /// Emit ambient particles based on planet type.
    fn emit_ambient_particles(&self, particles: &mut ParticleSystem) {
        let mut rng = rand::thread_rng();
        let surface_y = self.surface_top_y();
        match self.planet_type {
            PlanetType::Ice => {
                // Snowflakes drifting down
                if rng.gen_range(0..4) == 0 {
                    particles.emit(Particle::new(
                        rng.gen_range(0.0..self.width as f32),
                        rng.gen_range(1.0..4.0),
                        rng.gen_range(-0.15..0.15),
                        rng.gen_range(0.1..0.3),
                        rng.gen_range(20..40),
                        '❄',
                        if rng.gen_bool(0.5) {
                            Color::White
                        } else {
                            Color::Cyan
                        },
                    ));
                }
            }
            PlanetType::Volcanic => {
                // Embers floating up from surface
                if rng.gen_range(0..5) == 0 {
                    let lava_cols: Vec<usize> = self
                        .terrain
                        .first()
                        .map(|row| {
                            row.iter()
                                .enumerate()
                                .filter(|&(_, c)| *c == '~')
                                .map(|(i, _)| i)
                                .collect()
                        })
                        .unwrap_or_default();
                    if !lava_cols.is_empty() {
                        let col = lava_cols[rng.gen_range(0..lava_cols.len())];
                        particles.emit(Particle::new(
                            col as f32,
                            surface_y as f32 - 1.0,
                            rng.gen_range(-0.1..0.1),
                            rng.gen_range(-0.4..-0.1),
                            rng.gen_range(6..14),
                            if rng.gen_bool(0.6) { '·' } else { '*' },
                            if rng.gen_bool(0.5) {
                                Color::LightRed
                            } else {
                                Color::Yellow
                            },
                        ));
                    }
                }
            }
            PlanetType::Ocean => {
                // Fish jumping
                if rng.gen_range(0..30) == 0 {
                    let x = rng.gen_range(2.0..(self.width as f32 - 2.0));
                    particles.emit(Particle::new(
                        x,
                        surface_y as f32 - 1.0,
                        rng.gen_range(-0.2..0.2),
                        rng.gen_range(-0.5..-0.2),
                        rng.gen_range(4..8),
                        '♓',
                        Color::LightCyan,
                    ));
                }
            }
            _ => {}
        }
    }
}

impl Scene for RaidScene {
    fn enter(&mut self, state: &GameState, width: u16, height: u16) {
        let mut rng = rand::thread_rng();

        self.width = width;
        self.height = height;
        self.tick_count = 0;
        self.scrap_gained = 0;
        self.credits_gained = 0;
        self.cells_degraded = 0;
        self.elapsed = 0.0;

        self.planet_type = PlanetType::pick(&mut rng);
        self.raid_duration = 15.0 + (state.sector as f32 * 0.5).min(15.0);

        self.generate_terrain();
        self.generate_buildings();
        self.position_fleet(state.fleet.len());
        self.spawn_beams();
        self.spawn_entities();
        self.spawn_turrets(state.sector);
    }

    fn tick(&mut self, state: &mut GameState, particles: &mut ParticleSystem) -> SceneAction {
        self.tick_count += 1;
        self.elapsed += TICK_DT;

        let mut rng = rand::thread_rng();
        let in_entry = self.tick_count <= ENTRY_FRAMES;

        // ── Atmosphere entry animation ──────────────────────────────────
        if in_entry {
            let progress = self.tick_count as f32 / ENTRY_FRAMES as f32;
            for ship in &mut self.ships {
                // Lerp from above-screen to base_y
                ship.current_y = -3.0 + (ship.base_y + 3.0) * progress;
            }
            self.emit_entry_particles(particles);

            // Activate beams once entry completes
            if self.tick_count == ENTRY_FRAMES {
                for beam in &mut self.beams {
                    beam.active = true;
                }
            }

            // Don't run game logic during entry
            return SceneAction::Continue;
        }

        // ── Ship movement: patrol + bob ────────────────────────────────
        let t = self.tick_count as f32 * 0.08;
        let w = self.width as f32;
        for (i, ship) in self.ships.iter_mut().enumerate() {
            // Vertical bob
            let bob = (t + ship.bob_offset).sin() * 0.5;
            ship.current_y = ship.base_y + bob;

            // Horizontal patrol — each ship drifts along a sine wave
            // with different frequency so they spread out and weave
            let freq = 0.02 + (i as f32 * 0.007);
            let amp = (w * 0.3).min(15.0);
            let center = w / 2.0;
            let patrol_x = center + (t * freq + ship.bob_offset).sin() * amp;
            // Smoothly drift toward patrol target
            let dx = patrol_x - ship.x;
            ship.x += dx * 0.03;
            // Clamp to screen bounds
            ship.x = ship.x.clamp(3.0, w - 6.0);

            // Slight y dip when moving fast horizontally (swooping feel)
            let speed = dx.abs();
            if speed > 2.0 {
                ship.current_y += 0.15;
            }

            if ship.flash_frames > 0 {
                ship.flash_frames -= 1;
            }
        }

        // ── Animate beams ───────────────────────────────────────────────
        for (i, beam) in self.beams.iter_mut().enumerate() {
            beam.frame = beam.frame.wrapping_add(1);
            if let Some(ship) = self.ships.get(i) {
                beam.ship_y = ship.current_y as u16 + 2;
                beam.x = ship.x as u16 + 1;
            }
        }

        // ── Ambient particles ───────────────────────────────────────────
        self.emit_ambient_particles(particles);

        // ── Resource extraction (ticking up) ────────────────────────────
        if self.tick_count % 10 == 0 {
            let per_beam = 1 + state.sector as u64 / 5;
            let active_beams = self.beams.iter().filter(|b| b.active).count() as u64;
            let scrap_tick = per_beam * active_beams;
            let credits_tick = (per_beam * active_beams) / 2;
            state.scrap += scrap_tick;
            state.credits += credits_tick;
            self.scrap_gained += scrap_tick;
            self.credits_gained += credits_tick;
            state.total_scrap += scrap_tick;
        }

        // ── Resource particles rising from beam contact points ──────────
        if self.tick_count % 4 == 0 {
            for beam in &self.beams {
                if !beam.active {
                    continue;
                }
                let ch = self.planet_type.resource_char();
                let color = self.planet_type.resource_color();
                particles.emit(Particle::new(
                    beam.x as f32 + rng.gen_range(-1.0..1.0),
                    beam.surface_y as f32 - 1.0,
                    rng.gen_range(-0.2..0.2),
                    rng.gen_range(-0.6..-0.2),
                    rng.gen_range(8..16),
                    ch,
                    color,
                ));
            }
        }

        // ── Sparkle at beam contact points ──────────────────────────────
        if self.tick_count % 12 == 0 {
            for beam in &self.beams {
                if beam.active {
                    particles.sparkle(beam.x as f32, beam.surface_y as f32, Color::LightYellow);
                }
            }
        }

        // ── Surface entity movement with panic behavior ─────────────────
        let beam_xs: Vec<f32> = self
            .beams
            .iter()
            .filter(|b| b.active)
            .map(|b| b.x as f32)
            .collect();
        for entity in &mut self.entities {
            // Check if near a beam → trigger fear
            let near_beam = beam_xs.iter().any(|bx| (entity.x - bx).abs() < 8.0);
            let was_scared = entity.scared;
            entity.scared = near_beam;

            // First time getting scared: freeze briefly, then run
            if near_beam && !was_scared {
                entity.frozen_timer = rng.gen_range(3..10); // freeze 3-10 frames
                entity.panic_exclaim = 12; // show ! for 12 frames
            }

            // Tick down panic indicators
            if entity.panic_exclaim > 0 {
                entity.panic_exclaim -= 1;
            }

            // Frozen: don't move
            if entity.frozen_timer > 0 {
                entity.frozen_timer -= 1;
                continue;
            }

            let speed = if entity.scared {
                entity.base_speed.abs() * 4.0 // run FASTER
            } else {
                entity.base_speed.abs()
            };

            // If scared, run OPPOSITE direction from nearest beam
            if entity.scared {
                if let Some(nearest_bx) = beam_xs.iter().min_by(|a, b| {
                    (entity.x - **a)
                        .abs()
                        .partial_cmp(&(entity.x - **b).abs())
                        .unwrap_or(std::cmp::Ordering::Equal)
                }) {
                    let dir = if entity.x < *nearest_bx { -1.0 } else { 1.0 };
                    entity.x += dir * speed;
                }
            } else {
                let dir = if entity.base_speed >= 0.0 { 1.0 } else { -1.0 };
                entity.x += dir * speed;
            }

            // Bounce at edges
            if entity.x < 1.0 {
                entity.x = 1.0;
                entity.base_speed = entity.base_speed.abs();
            } else if entity.x >= (self.width as f32 - 1.0) {
                entity.x = self.width as f32 - 2.0;
                entity.base_speed = -entity.base_speed.abs();
            }
        }

        // ── Building destruction ────────────────────────────────────────
        self.crumble_buildings(particles);

        // ── Turret firing ───────────────────────────────────────────────
        let fire_y = self.surface_top_y() as f32 - 1.0;
        for turret in &mut self.turrets {
            if turret.cooldown > 0 {
                turret.cooldown -= 1;
            } else {
                self.projectiles.push(TurretProjectile {
                    x: turret.x as f32,
                    y: fire_y,
                    vy: -0.8,
                });
                turret.cooldown = rng.gen_range(15..40);
            }
        }

        // ── Update turret projectiles ───────────────────────────────────
        let ships = &mut self.ships;
        self.projectiles.retain_mut(|proj| {
            proj.y += proj.vy;

            for ship in ships.iter_mut() {
                let dx = (proj.x - ship.x).abs();
                let dy = (proj.y - ship.current_y).abs();
                if dx < 3.0 && dy < 1.5 {
                    ship.flash_frames = 6;
                    particles.explode(proj.x, proj.y, 4, Color::Red);
                    return false;
                }
            }

            proj.y > 0.0
        });

        // ── Degrade terrain ─────────────────────────────────────────────
        self.degrade_terrain();

        // ── Phase timer ─────────────────────────────────────────────────
        state.phase_timer -= TICK_DT;
        if state.phase_timer <= 0.0 {
            state.total_raids += 1;
            return SceneAction::TransitionTo(GamePhase::Loot);
        }

        SceneAction::Continue
    }

    fn render(&self, frame: &mut Frame, state: &GameState, particles: &ParticleSystem) {
        let area = frame.area();
        let buf = frame.buffer_mut();
        let surface_y = self.surface_top_y();

        // ── Draw terrain ────────────────────────────────────────────────
        let fg = self.planet_type.terrain_fg();
        let bg = self.planet_type.terrain_bg();
        let terrain_style = Style::default().fg(fg).bg(bg);

        // For volcanic: lava pools glow with cycling color
        let lava_glow = if self.planet_type == PlanetType::Volcanic {
            if self.tick_count % 8 < 4 {
                Color::LightRed
            } else {
                Color::Yellow
            }
        } else {
            fg
        };

        // For ocean: water shifts subtly
        let water_fg = if self.planet_type == PlanetType::Ocean {
            if self.tick_count % 10 < 5 {
                Color::Blue
            } else {
                Color::LightBlue
            }
        } else {
            fg
        };

        for (row_idx, row) in self.terrain.iter().enumerate() {
            let y = surface_y + row_idx as u16;
            if y >= area.height {
                break;
            }
            for (col_idx, &ch) in row.iter().enumerate() {
                let x = col_idx as u16;
                if x >= area.width {
                    break;
                }
                let cell = &mut buf[(area.x + x, area.y + y)];
                if ch == ' ' {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(bg));
                } else {
                    cell.set_char(ch);
                    match self.planet_type {
                        PlanetType::Volcanic if ch == '~' => {
                            cell.set_style(Style::default().fg(lava_glow).bg(bg));
                        }
                        PlanetType::Ocean if ch == '≈' || ch == '∼' || ch == '~' => {
                            cell.set_style(Style::default().fg(water_fg).bg(bg));
                        }
                        PlanetType::Desert if ch == 'ψ' => {
                            cell.set_style(Style::default().fg(Color::Green).bg(bg));
                        }
                        PlanetType::Ocean if ch == '▓' => {
                            // Island
                            cell.set_style(
                                Style::default()
                                    .fg(Color::Yellow)
                                    .bg(Color::Indexed(94)),
                            );
                        }
                        PlanetType::Ice if ch == '∙' => {
                            cell.set_style(Style::default().fg(Color::White).bg(bg));
                        }
                        _ => {
                            cell.set_style(terrain_style);
                        }
                    }
                }
            }
        }

        // ── Draw buildings (urban) ──────────────────────────────────────
        if self.planet_type == PlanetType::Urban {
            for bld in &self.buildings {
                if bld.destroyed {
                    // Rubble
                    let rubble_y = surface_y;
                    for dx in 0..bld.width as u16 {
                        let bx = bld.x + dx;
                        if bx < area.width && rubble_y < area.height {
                            let cell = &mut buf[(area.x + bx, area.y + rubble_y)];
                            let rubble_chars = ['.', ':', '.'];
                            cell.set_char(
                                rubble_chars[(bx as usize + self.tick_count as usize / 10)
                                    % rubble_chars.len()],
                            );
                            cell.set_fg(Color::DarkGray);
                        }
                    }
                    continue;
                }
                // Draw building columns
                for h in 0..bld.height as u16 {
                    let by = surface_y.saturating_sub(h + 1);
                    for dx in 0..bld.width as u16 {
                        let bx = bld.x + dx;
                        if bx < area.width && by < area.height && by > 0 {
                            let cell = &mut buf[(area.x + bx, area.y + by)];
                            // Outer vs inner
                            if h == bld.height as u16 - 1 {
                                cell.set_char('▓');
                            } else {
                                // Flickering windows
                                let is_window = (bx as usize + h as usize) % 2 == 0;
                                let window_on = is_window
                                    && ((self.tick_count as usize / 6 + bx as usize + h as usize)
                                        % 7
                                        != 0);
                                if window_on {
                                    cell.set_char('▪');
                                    cell.set_fg(Color::LightYellow);
                                } else {
                                    cell.set_char('█');
                                    cell.set_fg(Color::Gray);
                                }
                            }
                            if h == bld.height as u16 - 1 {
                                cell.set_fg(Color::DarkGray);
                            }
                        }
                    }
                }
            }
        }

        // ── Draw surface entities ───────────────────────────────────────
        let entity_y = surface_y; // entities walk on top row of terrain
        for entity in &self.entities {
            let ex = entity.x as u16;
            if ex < area.width && entity_y < area.height {
                let cell = &mut buf[(area.x + ex, area.y + entity_y)];
                cell.set_char(entity.kind.ch());
                cell.set_fg(if entity.scared {
                    Color::LightRed
                } else {
                    entity.kind.color()
                });

                // Exclamation mark above panicking entities
                if entity.panic_exclaim > 0 || (entity.scared && entity.frozen_timer > 0) {
                    let ey = entity_y.saturating_sub(1);
                    if ey > 0 && ey < area.height && ex < area.width {
                        let exclaim_cell = &mut buf[(area.x + ex, area.y + ey)];
                        exclaim_cell.set_char('!');
                        exclaim_cell.set_fg(Color::LightRed);
                    }
                }
            }
        }

        // ── Draw turrets ────────────────────────────────────────────────
        for turret in &self.turrets {
            let tx = turret.x;
            let ty = surface_y;
            if tx < area.width && ty < area.height {
                let cell = &mut buf[(area.x + tx, area.y + ty)];
                cell.set_char('▲');
                cell.set_fg(Color::LightRed);
            }
        }

        // ── Draw turret projectiles ─────────────────────────────────────
        for proj in &self.projectiles {
            let px = proj.x as u16;
            let py = proj.y as u16;
            if px < area.width && py < area.height {
                let cell = &mut buf[(area.x + px, area.y + py)];
                cell.set_char('↑');
                cell.set_fg(Color::LightRed);
            }
        }

        // ── Draw tractor beams with color gradient ──────────────────────
        for beam in &self.beams {
            if !beam.active {
                continue;
            }
            let x = beam.x;
            if x >= area.width {
                continue;
            }
            let beam_len = beam.surface_y.saturating_sub(beam.ship_y).max(1) as f32;
            for y in beam.ship_y..beam.surface_y {
                if y >= area.height {
                    break;
                }
                let anim_idx =
                    (beam.frame as usize + y as usize) % BEAM_CHARS.len();
                let ch = BEAM_CHARS[anim_idx];
                let cell = &mut buf[(area.x + x, area.y + y)];
                cell.set_char(ch);

                // Gradient: bright cyan at ship → dim blue at surface
                let progress = (y - beam.ship_y) as f32 / beam_len;
                let color = if progress < 0.2 {
                    Color::LightCyan
                } else if progress < 0.4 {
                    Color::Cyan
                } else if progress < 0.65 {
                    Color::Blue
                } else if progress < 0.85 {
                    Color::Indexed(25) // dim blue
                } else {
                    Color::DarkGray
                };
                // Cycle: shift the gradient based on frame
                let cycle_offset = (beam.frame as f32 * 0.1).sin() * 0.15;
                let adj_progress = (progress + cycle_offset).clamp(0.0, 1.0);
                let final_color = if adj_progress < 0.2 {
                    Color::LightCyan
                } else if adj_progress < 0.4 {
                    Color::Cyan
                } else if adj_progress < 0.65 {
                    Color::Blue
                } else if adj_progress < 0.85 {
                    Color::Indexed(25)
                } else {
                    color // fallback to original
                };
                cell.set_fg(final_color);
            }
        }

        // ── Draw fleet ships ────────────────────────────────────────────
        for (i, ship) in state.fleet.iter().enumerate() {
            if i >= self.ships.len() {
                break;
            }
            let hover = &self.ships[i];
            let draw_y = hover.current_y;

            let is_flashing = hover.flash_frames > 0 && hover.flash_frames % 2 == 0;
            let ship_color = if is_flashing {
                Color::LightRed
            } else {
                Color::Cyan
            };

            let sprite = ship.ship_type.sprite();
            for (row, line) in sprite.iter().enumerate() {
                let sy = (draw_y + row as f32) as u16;
                for (col, ch) in line.chars().enumerate() {
                    if ch == ' ' {
                        continue;
                    }
                    let sx = (hover.x + col as f32) as u16;
                    if sx < area.width && sy < area.height {
                        let cell = &mut buf[(area.x + sx, area.y + sy)];
                        cell.set_char(ch);
                        cell.set_fg(ship_color);
                    }
                }
            }
        }

        // ── Draw particles ──────────────────────────────────────────────
        for p in &particles.particles {
            let px = p.x as u16;
            let py = p.y as u16;
            if px < area.width && py < area.height {
                let cell = &mut buf[(area.x + px, area.y + py)];
                cell.set_char(p.render_char());
                cell.set_fg(p.color);
            }
        }

        // ── HUD: status bar at bottom ───────────────────────────────────
        let status_y = area.height.saturating_sub(1);
        let res_char = self.planet_type.resource_char();
        let planet_label = match self.planet_type {
            PlanetType::Desert => "DESERT",
            PlanetType::Urban => "URBAN",
            PlanetType::Ice => "ICE",
            PlanetType::Volcanic => "VOLCANIC",
            PlanetType::Ocean => "OCEAN",
        };
        let status = format!(
            " RAID │ {} │ Sector {} │ {:.0}s │ +{}{} +{}₿ │ Fleet: {} │ Lv.{} ",
            planet_label,
            state.sector,
            state.phase_timer.max(0.0),
            self.scrap_gained,
            res_char,
            self.credits_gained,
            state.fleet.len(),
            state.level,
        );
        for (i, ch) in status.chars().enumerate() {
            let x = i as u16;
            if x < area.width && status_y < area.height {
                let cell = &mut buf[(area.x + x, area.y + status_y)];
                cell.set_char(ch);
                cell.set_style(Style::default().fg(Color::Green).bg(Color::Black));
            }
        }
    }
}
