#![allow(dead_code)] // Game systems defined for future wiring — intentional

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

use engine::achievements::check_achievements;
use rendering::particles::ParticleSystem;
use scenes::{Scene, SceneAction};
use scenes::battle::BattleScene;
use scenes::loot::LootScene;
use scenes::raid::RaidScene;
use scenes::stats::StatsScreen;
use scenes::title::TitleScreen;
use scenes::travel::TravelScene;
use scenes::bridge::BridgeScene;
use scenes::upgrades::UpgradeScreen;
use scenes::map::MapScreen;
use state::{GamePhase, GameState};

const TICK_RATE: Duration = Duration::from_millis(50); // 20 fps

/// App mode — title screen vs playing
enum AppMode {
    Title,
    Playing,
}

fn main() -> io::Result<()> {
    // Setup terminal
    terminal::enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let size = terminal.size()?;

    // Load or create game state
    let mut state = GameState::load();
    let has_save = state.sector > 1 || state.scrap > 0 || state.total_battles > 0;

    // Title screen
    let mut title = TitleScreen::new(size.width, size.height);
    let mut mode = AppMode::Title;

    // Create scenes
    let mut travel = TravelScene::new();
    let mut battle = BattleScene::new();
    let mut raid = RaidScene::new();
    let mut loot = LootScene::new();

    // Particle system (shared across scenes)
    let mut particles = ParticleSystem::new();

    // Overlay screens
    let mut upgrades = UpgradeScreen::new();
    let mut stats = StatsScreen::new();
    let mut bridge = BridgeScene::new();
    let mut map_screen = MapScreen::new();

    // Achievement popup display
    let mut popup_text: Option<String> = None;
    let mut popup_timer: u8 = 0;

    // Time tracking
    let mut last_save = Instant::now();
    let mut last_time_tick = Instant::now();
    let mut tick_count: u64 = 0;

    // Main game loop
    loop {
        let tick_start = Instant::now();

        // Track play time (1 second increments)
        if last_time_tick.elapsed() >= Duration::from_secs(1) {
            if matches!(mode, AppMode::Playing) {
                state.time_played_secs += 1;
            }
            last_time_tick = Instant::now();
        }

        // Handle input (non-blocking)
        if event::poll(Duration::ZERO)?
            && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press {
                    match mode {
                        AppMode::Title => {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Char('Q') => break,
                                KeyCode::Esc => break,
                                KeyCode::Enter | KeyCode::Char('c') | KeyCode::Char('C') => {
                                    // Continue existing save
                                    mode = AppMode::Playing;
                                    let size = terminal.size()?;
                                    get_scene_mut(
                                        &mut travel, &mut battle, &mut raid, &mut loot,
                                        state.phase,
                                    )
                                    .enter(&state, size.width, size.height);
                                }
                                KeyCode::Char('n') | KeyCode::Char('N') => {
                                    // New game
                                    state = GameState::new();
                                    mode = AppMode::Playing;
                                    let size = terminal.size()?;
                                    get_scene_mut(
                                        &mut travel, &mut battle, &mut raid, &mut loot,
                                        state.phase,
                                    )
                                    .enter(&state, size.width, size.height);
                                }
                                _ => {}
                            }
                        }
                        AppMode::Playing => {
                            if upgrades.open {
                                match key.code {
                                    KeyCode::Esc => upgrades.toggle(),
                                    other => upgrades.handle_input(other, &mut state),
                                }
                            } else if map_screen.open {
                                map_screen.handle_input(key.code, &mut state);
                            } else if stats.open {
                                match key.code {
                                    KeyCode::Esc | KeyCode::Tab => stats.toggle(),
                                    _ => {}
                                }
                            } else if bridge.open {
                                bridge.handle_input(key.code, &mut state);
                            } else if travel.has_active_event() {
                                travel.handle_input(key.code, &mut state);
                            } else {
                                match key.code {
                                    KeyCode::Char('q') | KeyCode::Char('Q') => break,
                                    KeyCode::Esc => break,
                                    KeyCode::Char('u') | KeyCode::Char('U') => upgrades.toggle(),
                                    KeyCode::Tab => stats.toggle(),
                                    KeyCode::Char('b') | KeyCode::Char('B') => bridge.toggle(&mut state),
                                    KeyCode::Char('m') | KeyCode::Char('M') => map_screen.toggle(&state),
                                    KeyCode::Char('p') | KeyCode::Char('P') => {
                                        if state.prestige() {
                                            // Prestige resets state, re-enter travel scene
                                            let size = terminal.size()?;
                                            particles.particles.clear();
                                            get_scene_mut(
                                                &mut travel, &mut battle, &mut raid, &mut loot,
                                                state.phase,
                                            )
                                            .enter(&state, size.width, size.height);
                                            popup_text = Some(format!(
                                                "★ PRESTIGE {} ★ — XP +{}% Credits +{}% Scrap +{}%",
                                                state.prestige_level,
                                                (state.prestige_bonus_xp * 100.0) as u32,
                                                (state.prestige_bonus_credits * 100.0) as u32,
                                                (state.prestige_bonus_scrap * 100.0) as u32,
                                            ));
                                            popup_timer = 80;
                                            state.save();
                                        }
                                    }
                                    KeyCode::Char(' ') => {
                                        state.phase_timer = 0.1;
                                    }
                                    KeyCode::Char('s') | KeyCode::Char('S') => {
                                        state.save();
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }

        match mode {
            AppMode::Title => {
                title.tick();
                let state_ref = &state;
                terminal.draw(|frame| {
                    title.render(frame, has_save, state_ref);
                })?;
            }
            AppMode::Playing => {
                tick_count += 1;

                // Tick particles
                particles.tick();

                // Tick current scene
                let action = get_scene_mut(
                    &mut travel, &mut battle, &mut raid, &mut loot, state.phase,
                )
                .tick(&mut state, &mut particles);

                // Handle scene transitions
                if let SceneAction::TransitionTo(next_phase) = action {
                    // Update highest sector
                    if state.sector > state.highest_sector {
                        state.highest_sector = state.sector;
                    }

                    // Notify Pip of transitions
                    if state.phase == GamePhase::Battle && next_phase == GamePhase::Loot {
                        if state.fleet_total_hp() == 0 {
                            bridge.notify_battle_loss(&mut state);
                        } else {
                            bridge.notify_battle_win(&mut state);
                        }
                    }
                    if next_phase == GamePhase::Loot {
                        bridge.notify_loot();
                    }

                    state.phase = next_phase;
                    match next_phase {
                        GamePhase::Travel => state.phase_timer = 45.0,
                        GamePhase::Battle => state.phase_timer = 20.0,
                        GamePhase::Raid => state.phase_timer = 15.0,
                        GamePhase::Loot => state.phase_timer = 4.0,
                    }
                    let size = terminal.size()?;
                    particles.particles.clear();
                    get_scene_mut(
                        &mut travel, &mut battle, &mut raid, &mut loot, state.phase,
                    )
                    .enter(&state, size.width, size.height);
                }

                // Tick Pip (always, even when bridge isn't open)
                bridge.tick(&mut state);

                // Check achievements (throttled to every 20 ticks)
                if tick_count.is_multiple_of(20) {
                    let new_achievements = check_achievements(&state);
                    for ach in new_achievements {
                        state.achievements_unlocked.push(ach.id.to_string());
                        popup_text = Some(format!("{} Achievement: {} — {}", ach.icon, ach.name, ach.description));
                        popup_timer = 60; // 3 seconds at 20fps
                        bridge.notify_achievement(&mut state);
                    }
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

                    // Scene
                    match phase {
                        GamePhase::Travel => travel.render(frame, &state, &particles),
                        GamePhase::Battle => battle.render(frame, &state, &particles),
                        GamePhase::Raid => raid.render(frame, &state, &particles),
                        GamePhase::Loot => loot.render(frame, &state, &particles),
                    }

                    // HUD bar
                    // Determine active overlay for context-aware HUD
                    let active_overlay = if bridge.open {
                        "bridge"
                    } else if upgrades.open {
                        "upgrades"
                    } else if stats.open {
                        "stats"
                    } else if map_screen.open {
                        "map"
                    } else if travel.has_active_event() {
                        "event"
                    } else {
                        "game"
                    };
                    render_hud(frame, &state, phase, active_overlay);

                    // Achievement popup banner
                    if let Some(ref text) = popup_text
                        && popup_timer > 0 {
                            render_popup(frame, text, popup_timer);
                        }

                    // Overlays
                    if bridge.open {
                        bridge.render(frame, &state);
                    }
                    if map_screen.open {
                        map_screen.render(frame, &state);
                    }
                    if upgrades.open {
                        upgrades.render(frame, &state);
                    }
                    if stats.open {
                        stats.render(frame, &state);
                    }
                })?;

                // Tick popup timer
                if popup_timer > 0 {
                    popup_timer -= 1;
                    if popup_timer == 0 {
                        popup_text = None;
                    }
                }

                // Auto-save every 60 seconds
                if last_save.elapsed() > Duration::from_secs(60) {
                    state.save();
                    last_save = Instant::now();
                }
            }
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

fn render_hud(frame: &mut Frame, state: &GameState, phase: GamePhase, overlay: &str) {
    let area = frame.area();
    let hud_y = area.height.saturating_sub(1);
    let buf = frame.buffer_mut();

    let phase_name = match phase {
        GamePhase::Travel => "TRAVEL",
        GamePhase::Battle => "BATTLE",
        GamePhase::Raid => "RAID",
        GamePhase::Loot => "LOOT",
    };
    let phase_color = match phase {
        GamePhase::Travel => Color::Cyan,
        GamePhase::Battle => Color::Red,
        GamePhase::Raid => Color::Green,
        GamePhase::Loot => Color::Yellow,
    };

    let controls = match overlay {
        "bridge" => format!(
            " BRIDGE │ Pip Lv.{} │ ◇{} │ ₿{} │ [F]eed [P]et [G]ifts [Esc]Close ",
            state.pip_level, state.scrap, state.credits
        ),
        "upgrades" => format!(
            " UPGRADES │ ◇{} │ ₿{} │ [↑↓]Navigate [←→/Tab]Tabs [Enter]Buy [Esc]Close ",
            state.scrap, state.credits
        ),
        "stats" => " STATS │ [Esc/Tab]Close ".to_string(),
        "map" => " SECTOR MAP │ [A/B/C]Choose Route [Esc]Close ".to_string(),
        "event" => " EVENT │ [↑↓]Select [Enter]Choose ".to_string(),
        _ => {
            let mut hud = format!(
                " {} │ Sec:{} │ Lv.{} │ ◇{} │ ₿{} │ Ships:{} │ [U]pgrade [B]ridge [M]ap [Tab]Stats [Space]Skip ",
                phase_name, state.sector, state.level, state.scrap, state.credits, state.fleet.len()
            );
            if state.sector >= 30 {
                hud.push_str("[P]restige ");
            }
            hud.push_str("[Q]uit ");
            hud
        }
    };

    // Determine label color based on context
    let (label, label_color) = match overlay {
        "bridge" => ("BRIDGE", Color::Magenta),
        "upgrades" => ("UPGRADES", Color::Green),
        "stats" => ("STATS", Color::Cyan),
        "map" => ("SECTOR MAP", Color::Yellow),
        "event" => ("EVENT", Color::Yellow),
        _ => (phase_name, phase_color),
    };
    let label_len = label.len() + 1; // +1 for leading space

    for (i, ch) in controls.chars().enumerate() {
        let x = i as u16;
        if x < area.width {
            let cell = &mut buf[(area.x + x, area.y + hud_y)];
            cell.set_char(ch);
            if i < label_len {
                cell.set_fg(label_color);
            } else {
                cell.set_fg(Color::DarkGray);
            }
            cell.set_bg(Color::Rgb(20, 20, 30));
        }
    }
    for x in controls.len() as u16..area.width {
        let cell = &mut buf[(area.x + x, area.y + hud_y)];
        cell.set_char(' ');
        cell.set_bg(Color::Rgb(20, 20, 30));
    }
}

fn render_popup(frame: &mut Frame, text: &str, timer: u8) {
    let area = frame.area();
    let buf = frame.buffer_mut();

    // Banner at top of screen, centered
    let y = 1_u16;
    let pad = 2;
    let text_len = text.len() as u16;
    let box_width = text_len + pad * 2;
    let x_start = area.width.saturating_sub(box_width) / 2;

    // Fade: bright at start, dimmer near end
    let fg = if timer > 40 {
        Color::Yellow
    } else if timer > 20 {
        Color::DarkGray
    } else {
        Color::Rgb(60, 60, 60)
    };

    // Draw background
    for x in x_start..x_start + box_width {
        if x < area.width && y < area.height {
            let cell = &mut buf[(area.x + x, area.y + y)];
            cell.set_char(' ');
            cell.set_bg(Color::Rgb(30, 30, 10));
        }
    }

    // Draw text
    for (i, ch) in text.chars().enumerate() {
        let x = x_start + pad + i as u16;
        if x < area.width && y < area.height {
            let cell = &mut buf[(area.x + x, area.y + y)];
            cell.set_char(ch);
            cell.set_fg(fg);
            cell.set_bg(Color::Rgb(30, 30, 10));
        }
    }
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
