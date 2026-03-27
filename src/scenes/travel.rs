use ratatui::Frame;
use ratatui::style::{Color, Style};

use crate::engine::ship::Ship;
use crate::rendering::particles::ParticleSystem;
use crate::rendering::starfield::Starfield;
use crate::state::{GamePhase, GameState};
use super::{Scene, SceneAction};

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

pub struct TravelScene {
    starfield: Starfield,
    collectibles: Vec<Collectible>,
    width: u16,
    height: u16,
    tick_count: u64,
    travel_duration: f32,
    fleet_positions: Vec<(f32, f32)>,
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
        }
    }

    fn spawn_collectible(&mut self) {
        use rand::Rng;
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

    fn calculate_fleet_positions(&mut self, fleet: &[Ship]) {
        self.fleet_positions.clear();
        let cx = 8.0_f32;
        let cy = self.height as f32 / 2.0;
        let spacing = 3.0_f32;

        for (i, _ship) in fleet.iter().enumerate() {
            let row = i as f32;
            let y = cy - (fleet.len() as f32 * spacing / 2.0) + row * spacing;
            // Slight wave motion
            let wave = (self.tick_count as f32 * 0.05 + i as f32 * 0.8).sin() * 0.3;
            self.fleet_positions.push((cx, y + wave));
        }
    }
}

impl Scene for TravelScene {
    fn enter(&mut self, state: &GameState, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.starfield = Starfield::new(width, height, (width as usize * height as usize) / 30);
        self.collectibles.clear();
        self.tick_count = 0;
        self.travel_duration = 45.0 + (state.sector as f32 * 2.0).min(30.0);
    }

    fn tick(&mut self, state: &mut GameState, particles: &mut ParticleSystem) -> SceneAction {
        self.tick_count += 1;

        // Update starfield
        self.starfield.tick();

        // Update fleet positions
        self.calculate_fleet_positions(&state.fleet);

        // Spawn collectibles
        if self.tick_count % 40 == 0 {
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
                    state.scrap += col.kind.value();
                    state.total_scrap += col.kind.value();
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

        // Engine exhaust for each ship
        if self.tick_count % 3 == 0 {
            for &(fx, fy) in &self.fleet_positions {
                particles.exhaust(fx - 1.0, fy);
            }
        }

        // Phase timer
        state.phase_timer -= 0.05; // 20fps * 0.05 = 1 second per 20 ticks
        if state.phase_timer <= 0.0 {
            // Transition to battle or raid
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let next = if rng.gen_bool(0.6) {
                GamePhase::Battle
            } else {
                GamePhase::Raid
            };
            return SceneAction::TransitionTo(next);
        }

        SceneAction::Continue
    }

    fn render(&self, frame: &mut Frame, state: &GameState, particles: &ParticleSystem) {
        let area = frame.area();
        let buf = frame.buffer_mut();

        // Draw starfield
        for star in &self.starfield.stars {
            let sx = star.x as u16;
            let sy = star.y as u16;
            if sx < area.width && sy < area.height {
                let cell = &mut buf[(area.x + sx, area.y + sy)];
                cell.set_char(star.ch);
                cell.set_fg(star.color);
            }
        }

        // Draw collectibles
        for col in &self.collectibles {
            let cx = col.x as u16;
            let cy = col.y as u16;
            if cx < area.width && cy < area.height {
                let cell = &mut buf[(area.x + cx, area.y + cy)];
                cell.set_char(col.kind.char());
                cell.set_fg(col.kind.color());
            }
        }

        // Draw fleet ships
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
                    if sx < area.width && sy < area.height {
                        let cell = &mut buf[(area.x + sx, area.y + sy)];
                        cell.set_char(ch);
                        cell.set_fg(Color::Cyan);
                    }
                }
            }
        }

        // Draw particles
        for p in &particles.particles {
            let px = p.x as u16;
            let py = p.y as u16;
            if px < area.width && py < area.height {
                let cell = &mut buf[(area.x + px, area.y + py)];
                cell.set_char(p.render_char());
                cell.set_fg(p.color);
            }
        }

        // Status bar at bottom
        let status_y = area.height.saturating_sub(1);
        let status = format!(
            " Sector {} │ Scrap: {} │ Credits: {} │ Fleet: {} │ Lv.{} ",
            state.sector, state.scrap, state.credits, state.fleet.len(), state.level
        );
        for (i, ch) in status.chars().enumerate() {
            let x = i as u16;
            if x < area.width {
                let cell = &mut buf[(area.x + x, area.y + status_y)];
                cell.set_char(ch);
                cell.set_fg(Color::DarkGray);
                cell.set_style(Style::default().fg(Color::DarkGray));
            }
        }
    }
}
