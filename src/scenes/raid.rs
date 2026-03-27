use ratatui::Frame;
use crate::rendering::particles::ParticleSystem;
use crate::state::GameState;
use super::{Scene, SceneAction};

pub struct RaidScene;

impl RaidScene {
    pub fn new() -> Self { Self }
}

impl Scene for RaidScene {
    fn enter(&mut self, _state: &GameState, _width: u16, _height: u16) {}

    fn tick(&mut self, state: &mut GameState, _particles: &mut ParticleSystem) -> SceneAction {
        state.phase_timer -= 0.05;
        if state.phase_timer <= 0.0 {
            state.total_raids += 1;
            SceneAction::TransitionTo(crate::state::GamePhase::Loot)
        } else {
            SceneAction::Continue
        }
    }

    fn render(&self, frame: &mut Frame, state: &GameState, _particles: &ParticleSystem) {
        let area = frame.area();
        let buf = frame.buffer_mut();
        let msg = format!("🌍 RAID — Sector {} — {:.0}s remaining", state.sector, state.phase_timer);
        for (i, ch) in msg.chars().enumerate() {
            let x = (area.width / 2).saturating_sub(msg.len() as u16 / 2) + i as u16;
            let y = area.height / 2;
            if x < area.width {
                buf[(area.x + x, area.y + y)].set_char(ch);
                buf[(area.x + x, area.y + y)].set_fg(ratatui::style::Color::Green);
            }
        }
    }
}
