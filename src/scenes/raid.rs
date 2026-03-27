use rand::Rng;
use ratatui::style::{Color, Style};
use ratatui::Frame;

use crate::rendering::particles::{Particle, ParticleSystem};
use crate::state::{GamePhase, GameState};

use super::{Scene, SceneAction};

// ── Planet types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum PlanetType {
    Desert,
    Urban,
    Ice,
    Volcanic,
}

impl PlanetType {
    fn pick(rng: &mut impl Rng) -> Self {
        match rng.gen_range(0..4) {
            0 => Self::Desert,
            1 => Self::Urban,
            2 => Self::Ice,
            _ => Self::Volcanic,
        }
    }

    /// Fill a terrain row with appropriate block characters.
    fn terrain_chars(&self, rng: &mut impl Rng, width: usize) -> Vec<char> {
        (0..width)
            .map(|_| match self {
                Self::Desert => {
                    let r: u8 = rng.gen_range(0..10);
                    if r < 5 { '▒' } else { '░' }
                }
                Self::Urban => {
                    let r: u8 = rng.gen_range(0..12);
                    if r < 4 {
                        '█'
                    } else if r < 7 {
                        '▓'
                    } else if r < 9 {
                        '▒'
                    } else {
                        '▪'
                    }
                }
                Self::Ice => {
                    let r: u8 = rng.gen_range(0..10);
                    if r < 4 {
                        '░'
                    } else if r < 7 {
                        '·'
                    } else {
                        '▒'
                    }
                }
                Self::Volcanic => {
                    let r: u8 = rng.gen_range(0..10);
                    if r < 4 {
                        '▓'
                    } else if r < 7 {
                        '▒'
                    } else if r < 9 {
                        '~'
                    } else {
                        '░'
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
        }
    }

    fn terrain_bg(&self) -> Color {
        match self {
            Self::Desert => Color::Indexed(94),  // dark yellow/brown
            Self::Urban => Color::DarkGray,
            Self::Ice => Color::Indexed(17),      // dark blue
            Self::Volcanic => Color::Indexed(52), // dark red
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
    bob_offset: f32, // phase offset for bobbing
    flash_frames: u8,
}

// ── Main scene ──────────────────────────────────────────────────────────

const TERRAIN_ROWS: u16 = 4;
const TICK_DT: f32 = 0.05; // 20fps
const BEAM_CHARS: [char; 4] = ['│', '↓', '|', ':'];
const RESOURCE_CHARS: [char; 3] = ['·', '°', '◦'];

pub struct RaidScene {
    width: u16,
    height: u16,
    tick_count: u64,
    planet_type: PlanetType,

    // Terrain grid: [row][col], row 0 is top row of terrain
    terrain: Vec<Vec<char>>,

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
        for _ in 0..TERRAIN_ROWS {
            self.terrain.push(self.planet_type.terrain_chars(&mut rng, w));
        }
    }

    fn position_fleet(&mut self, fleet_size: usize) {
        let mut rng = rand::thread_rng();
        self.ships.clear();

        let top_margin = 2.0_f32;
        let fleet_zone_bottom = (self.surface_top_y() as f32) - 4.0;
        let fleet_zone_height = (fleet_zone_bottom - top_margin).max(4.0);
        let spacing = (fleet_zone_height / fleet_size.max(1) as f32).min(3.0);

        let start_y = top_margin
            + (fleet_zone_height - spacing * fleet_size as f32).max(0.0) / 2.0;

        for i in 0..fleet_size {
            let x = rng.gen_range(4.0..(self.width as f32 * 0.8).max(10.0));
            let base_y = start_y + i as f32 * spacing;
            self.ships.push(HoverShip {
                x,
                base_y,
                bob_offset: rng.gen_range(0.0..std::f32::consts::TAU),
                flash_frames: 0,
            });
        }
    }

    fn spawn_beams(&mut self) {
        self.beams.clear();
        let surface_y = self.surface_top_y();
        for (i, ship) in self.ships.iter().enumerate() {
            let ship_y = ship.base_y as u16 + 1; // beam starts just below ship
            self.beams.push(TractorBeam {
                x: ship.x as u16 + 1, // roughly center of sprite
                ship_y,
                surface_y,
                frame: (i as u8).wrapping_mul(7), // stagger animation
                active: true,
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
        // Pick a random beam contact point and degrade nearby terrain
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
        self.position_fleet(state.fleet.len());
        self.spawn_beams();
        self.spawn_entities();
        self.spawn_turrets(state.sector);
    }

    fn tick(&mut self, state: &mut GameState, particles: &mut ParticleSystem) -> SceneAction {
        self.tick_count += 1;
        self.elapsed += TICK_DT;

        let mut rng = rand::thread_rng();

        // ── Bob ships gently ────────────────────────────────────────────
        let t = self.tick_count as f32 * 0.08;
        for ship in &mut self.ships {
            if ship.flash_frames > 0 {
                ship.flash_frames -= 1;
            }
        }

        // ── Animate beams ───────────────────────────────────────────────
        for (i, beam) in self.beams.iter_mut().enumerate() {
            beam.frame = beam.frame.wrapping_add(1);
            // Update beam x to follow ship bob (horizontal stays fixed, y bobs)
            if let Some(ship) = self.ships.get(i) {
                let bob = (t + ship.bob_offset).sin() * 0.5;
                beam.ship_y = (ship.base_y + bob) as u16 + 1;
            }
        }

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
                let ch = RESOURCE_CHARS[rng.gen_range(0..RESOURCE_CHARS.len())];
                particles.emit(Particle::new(
                    beam.x as f32 + rng.gen_range(-1.0..1.0),
                    beam.surface_y as f32 - 1.0,
                    rng.gen_range(-0.2..0.2),
                    rng.gen_range(-0.6..-0.2),
                    rng.gen_range(8..16),
                    ch,
                    Color::Yellow,
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

        // ── Surface entity movement ─────────────────────────────────────
        let beam_xs: Vec<f32> = self.beams.iter().map(|b| b.x as f32).collect();
        for entity in &mut self.entities {
            // Check if near a beam → scatter
            let near_beam = beam_xs
                .iter()
                .any(|bx| (entity.x - bx).abs() < 6.0);
            entity.scared = near_beam;

            let speed = if entity.scared {
                entity.base_speed * 3.0
            } else {
                entity.base_speed
            };

            // If scared, run away from nearest beam
            if entity.scared {
                if let Some(nearest_bx) = beam_xs
                    .iter()
                    .min_by(|a, b| {
                        (entity.x - **a)
                            .abs()
                            .partial_cmp(&(entity.x - **b).abs())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                {
                    let dir = if entity.x < *nearest_bx { -1.0 } else { 1.0 };
                    entity.x += dir * speed.abs();
                }
            } else {
                entity.x += speed;
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

        // ── Turret firing ───────────────────────────────────────────────
        let fire_y = self.surface_top_y() as f32 - 1.0;
        for turret in &mut self.turrets {
            if turret.cooldown > 0 {
                turret.cooldown -= 1;
            } else {
                // Fire!
                self.projectiles.push(TurretProjectile {
                    x: turret.x as f32,
                    y: fire_y,
                    vy: -0.8,
                });
                turret.cooldown = rng.gen_range(15..40);
            }
        }

        // ── Update turret projectiles ───────────────────────────────────
        self.projectiles.retain_mut(|proj| {
            proj.y += proj.vy;

            // Check hit against ships
            for ship in &mut self.ships {
                let bob = (t + ship.bob_offset).sin() * 0.5;
                let ship_y = ship.base_y + bob;
                let dx = (proj.x - ship.x).abs();
                let dy = (proj.y - ship_y).abs();
                if dx < 3.0 && dy < 1.5 {
                    ship.flash_frames = 6;
                    // Small explosion particle
                    particles.explode(proj.x, proj.y, 4, Color::Red);
                    return false; // consumed
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
        let t = self.tick_count as f32 * 0.08;
        let surface_y = self.surface_top_y();

        // ── Draw terrain ────────────────────────────────────────────────
        let fg = self.planet_type.terrain_fg();
        let bg = self.planet_type.terrain_bg();
        let terrain_style = Style::default().fg(fg).bg(bg);

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
                    // Degraded — show gap with just background
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(bg));
                } else {
                    cell.set_char(ch);
                    cell.set_style(terrain_style);
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

        // ── Draw tractor beams ──────────────────────────────────────────
        for beam in &self.beams {
            if !beam.active {
                continue;
            }
            let x = beam.x;
            if x >= area.width {
                continue;
            }
            for y in beam.ship_y..beam.surface_y {
                if y >= area.height {
                    break;
                }
                // Cycling animation: offset by row to create a flowing effect
                let anim_idx =
                    (beam.frame as usize + y as usize) % BEAM_CHARS.len();
                let ch = BEAM_CHARS[anim_idx];
                let cell = &mut buf[(area.x + x, area.y + y)];
                cell.set_char(ch);
                // Fade beam color: brighter near surface
                let progress =
                    (y - beam.ship_y) as f32 / (beam.surface_y - beam.ship_y).max(1) as f32;
                let color = if progress > 0.7 {
                    Color::LightGreen
                } else if progress > 0.3 {
                    Color::Green
                } else {
                    Color::DarkGray
                };
                cell.set_fg(color);
            }
        }

        // ── Draw fleet ships ────────────────────────────────────────────
        for (i, ship) in state.fleet.iter().enumerate() {
            if i >= self.ships.len() {
                break;
            }
            let hover = &self.ships[i];
            let bob = (t + hover.bob_offset).sin() * 0.5;
            let draw_y = hover.base_y + bob;

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
        let status = format!(
            " RAID │ Sector {} │ {:.0}s │ +{}◇ +{}₿ │ Fleet: {} │ Lv.{} ",
            state.sector,
            state.phase_timer.max(0.0),
            self.scrap_gained,
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
