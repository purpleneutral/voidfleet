use ratatui::Frame;
use crate::rendering::particles::ParticleSystem;
use crate::state::GameState;
use super::{Scene, SceneAction};

pub struct LootScene {
    timer: f32,
}

impl LootScene {
    pub fn new() -> Self {
        Self { timer: 4.0 }
    }
}

impl Scene for LootScene {
    fn enter(&mut self, state: &GameState, _width: u16, _height: u16) {
        self.timer = 4.0;
        // Loot is calculated here but applied in tick
        let _ = state;
    }

    fn tick(&mut self, state: &mut GameState, _particles: &mut ParticleSystem) -> SceneAction {
        self.timer -= 0.05;

        // Award loot on first tick
        if self.timer > 3.9 {
            let sector_mult = state.sector as u64;
            state.credits += 20 + sector_mult * 5;
            state.scrap += 10 + sector_mult * 3;
            state.add_xp(15 + sector_mult * 2);

            // Rare blueprint drop
            if rand::random::<f32>() < 0.1 {
                state.blueprints += 1;
            }

            // Heal fleet
            for ship in &mut state.fleet {
                ship.heal_full();
            }

            // Advance sector
            state.sector += 1;
        }

        if self.timer <= 0.0 {
            state.phase_timer = 45.0;
            state.save();
            SceneAction::TransitionTo(crate::state::GamePhase::Travel)
        } else {
            SceneAction::Continue
        }
    }

    fn render(&self, frame: &mut Frame, state: &GameState, _particles: &ParticleSystem) {
        let area = frame.area();
        let buf = frame.buffer_mut();
        let cy = area.height / 2;

        let lines = [
            format!("═══ SECTOR {} CLEAR ═══", state.sector.saturating_sub(1)),
            String::new(),
            format!("  Credits: +{}", 20 + (state.sector.saturating_sub(1)) as u64 * 5),
            format!("  Scrap:   +{}", 10 + (state.sector.saturating_sub(1)) as u64 * 3),
            format!("  XP:      +{}", 15 + (state.sector.saturating_sub(1)) as u64 * 2),
            String::new(),
            format!("  Fleet restored to full HP"),
        ];

        for (row, line) in lines.iter().enumerate() {
            let y = cy.saturating_sub(3) + row as u16;
            let x_start = (area.width / 2).saturating_sub(line.len() as u16 / 2);
            for (i, ch) in line.chars().enumerate() {
                let x = x_start + i as u16;
                if x < area.width && y < area.height {
                    buf[(area.x + x, area.y + y)].set_char(ch);
                    buf[(area.x + x, area.y + y)].set_fg(ratatui::style::Color::Yellow);
                }
            }
        }
    }
}
