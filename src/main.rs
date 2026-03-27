mod engine;
mod rendering;
mod scenes;
mod state;

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::prelude::*;

use rendering::particles::ParticleSystem;
use scenes::{Scene, SceneAction};
use scenes::travel::TravelScene;
use scenes::battle::BattleScene;
use scenes::raid::RaidScene;
use scenes::loot::LootScene;
use scenes::upgrades::UpgradeScreen;
use state::{GamePhase, GameState};

const TICK_RATE: Duration = Duration::from_millis(50); // 20 fps

fn main() -> io::Result<()> {
    // Setup terminal
    terminal::enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Load or create game state
    let mut state = GameState::load();

    // Create scenes
    let mut travel = TravelScene::new();
    let mut battle = BattleScene::new();
    let mut raid = RaidScene::new();
    let mut loot = LootScene::new();

    // Particle system (shared across scenes)
    let mut particles = ParticleSystem::new();

    // Upgrade screen overlay
    let mut upgrades = UpgradeScreen::new();

    // Enter initial scene
    let size = terminal.size()?;
    get_scene_mut(&mut travel, &mut battle, &mut raid, &mut loot, state.phase)
        .enter(&state, size.width, size.height);

    let mut last_save = Instant::now();

    // Main game loop
    loop {
        let tick_start = Instant::now();

        // Handle input (non-blocking)
        if event::poll(Duration::ZERO)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if upgrades.open {
                        match key.code {
                            KeyCode::Esc => upgrades.toggle(),
                            other => upgrades.handle_input(other, &mut state),
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Char('Q') => break,
                            KeyCode::Esc => break,
                            KeyCode::Char('u') | KeyCode::Char('U') => upgrades.toggle(),
                            _ => {}
                        }
                    }
                }
            }
        }

        // Tick particles
        particles.tick();

        // Tick current scene
        let action = get_scene_mut(&mut travel, &mut battle, &mut raid, &mut loot, state.phase)
            .tick(&mut state, &mut particles);

        // Handle scene transitions
        if let SceneAction::TransitionTo(next_phase) = action {
            state.phase = next_phase;
            // Set phase timer for next phase
            match next_phase {
                GamePhase::Travel => state.phase_timer = 45.0,
                GamePhase::Battle => state.phase_timer = 20.0,
                GamePhase::Raid => state.phase_timer = 15.0,
                GamePhase::Loot => state.phase_timer = 4.0,
            }
            let size = terminal.size()?;
            particles.particles.clear();
            get_scene_mut(&mut travel, &mut battle, &mut raid, &mut loot, state.phase)
                .enter(&state, size.width, size.height);
        }

        // Render
        let phase = state.phase;
        terminal.draw(|frame| {
            // Clear with black background
            let area = frame.area();
            let buf = frame.buffer_mut();
            for y in area.top()..area.bottom() {
                for x in area.left()..area.right() {
                    let cell = &mut buf[(x, y)];
                    cell.set_char(' ');
                    cell.set_fg(Color::White);
                    cell.set_bg(Color::Black);
                }
            }

            match phase {
                GamePhase::Travel => travel.render(frame, &state, &particles),
                GamePhase::Battle => battle.render(frame, &state, &particles),
                GamePhase::Raid => raid.render(frame, &state, &particles),
                GamePhase::Loot => loot.render(frame, &state, &particles),
            }

            // Upgrade overlay on top
            if upgrades.open {
                upgrades.render(frame, &state);
            }
        })?;

        // Auto-save every 60 seconds
        if last_save.elapsed() > Duration::from_secs(60) {
            state.save();
            last_save = Instant::now();
        }

        // Frame rate control
        let elapsed = tick_start.elapsed();
        if elapsed < TICK_RATE {
            std::thread::sleep(TICK_RATE - elapsed);
        }
    }

    // Save on exit
    state.save();

    // Restore terminal
    terminal::disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn get_scene_mut<'a>(
    travel: &'a mut TravelScene,
    battle: &'a mut BattleScene,
    raid: &'a mut RaidScene,
    loot: &'a mut LootScene,
    phase: GamePhase,
) -> &'a mut dyn Scene {
    match phase {
        GamePhase::Travel => travel,
        GamePhase::Battle => battle,
        GamePhase::Raid => raid,
        GamePhase::Loot => loot,
    }
}
