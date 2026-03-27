pub mod title;
pub mod travel;
pub mod battle;
pub mod raid;
pub mod loot;
pub mod upgrades;
pub mod bridge;
pub mod help;
pub mod stats;
pub mod map;
pub mod inventory;
pub mod crew;
pub mod diplomacy;
pub mod trade;
pub mod missions;
pub mod gamelog;
pub mod voyage;

use ratatui::Frame;
use crate::engine::events::EventBus;
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
    fn tick(&mut self, state: &mut GameState, particles: &mut ParticleSystem, events: &mut EventBus) -> SceneAction;

    /// Render the scene to the terminal frame.
    fn render(&self, frame: &mut Frame, state: &GameState, particles: &ParticleSystem);
}
