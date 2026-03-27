pub mod travel;
pub mod battle;
pub mod raid;
pub mod loot;
pub mod upgrades;

use ratatui::Frame;
use crate::rendering::particles::ParticleSystem;
use crate::state::GameState;

/// What the scene wants the main loop to do after a tick.
pub enum SceneAction {
    Continue,
    TransitionTo(crate::state::GamePhase),
}

/// Trait for all game scenes.
pub trait Scene {
    /// Called once when entering this scene.
    fn enter(&mut self, state: &GameState, width: u16, height: u16);

    /// Update scene state (called every tick ~50ms).
    fn tick(&mut self, state: &mut GameState, particles: &mut ParticleSystem) -> SceneAction;

    /// Render the scene to the terminal frame.
    fn render(&self, frame: &mut Frame, state: &GameState, particles: &ParticleSystem);
}
