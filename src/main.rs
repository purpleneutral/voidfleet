 // Game systems defined for future wiring — intentional

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
use engine::events::{EventBus, pip_commentary, process_events};
use engine::voyage::VoyageStats;
use engine::{missions, trade};
use rendering::particles::ParticleSystem;
use scenes::{Scene, SceneAction};
use scenes::battle::BattleScene;
use scenes::loot::LootScene;
use scenes::raid::RaidScene;
use scenes::stats::StatsScreen;
use scenes::title::TitleScreen;
use scenes::travel::TravelScene;
use scenes::bridge::BridgeScene;
use scenes::inventory::InventoryScreen;
use scenes::crew::CrewScreen;
use scenes::diplomacy::DiplomacyScreen;
use scenes::trade::TradeScreen;
use scenes::missions::MissionScreen;
use scenes::upgrades::UpgradeScreen;
use scenes::map::MapScreen;
use scenes::help::HelpScreen;
use scenes::gamelog::GameLogScreen;
use scenes::voyage::VoyageScreen;
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

    // Event bus (decouples game systems)
    let mut event_bus = EventBus::new();

    // Overlay screens
    let mut upgrades = UpgradeScreen::new();
    let mut stats = StatsScreen::new();
    let mut bridge = BridgeScene::new();
    let mut inventory = InventoryScreen::new();
    let mut map_screen = MapScreen::new();
    let mut crew_screen = CrewScreen::new();
    let mut diplomacy = DiplomacyScreen::new();
    let mut trade_screen = TradeScreen::new();
    let mut mission_screen = MissionScreen::new();
    let mut gamelog = GameLogScreen::new();
    let mut voyage_screen = VoyageScreen::new();
    let mut help_screen = HelpScreen::new();

    // Achievement popup display
    let mut popup_text: Option<String> = None;
    let mut popup_timer: u8 = 0;

    // Pip commentary bubble (HUD-level, separate from bridge speech)
    let mut pip_comment: Option<String> = None;
    let mut pip_comment_timer: u8 = 0;

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
                            if help_screen.open {
                                help_screen.handle_input(key.code);
                            } else if voyage_screen.active {
                                if key.code == KeyCode::Enter {
                                    voyage_screen.handle_enter();
                                }
                            } else if gamelog.open {
                                match key.code {
                                    KeyCode::Esc | KeyCode::Char('l') | KeyCode::Char('L') => gamelog.toggle(),
                                    other => gamelog.handle_input(other),
                                }
                            } else if trade_screen.open {
                                match key.code {
                                    KeyCode::Esc => { trade_screen.open = false; }
                                    other => trade_screen.handle_input(other, &mut state),
                                }
                            } else if mission_screen.open {
                                match key.code {
                                    KeyCode::Esc => { mission_screen.open = false; }
                                    other => mission_screen.handle_input(other, &mut state),
                                }
                            } else if upgrades.open {
                                match key.code {
                                    KeyCode::Esc => upgrades.toggle(),
                                    other => upgrades.handle_input(other, &mut state),
                                }
                            } else if diplomacy.open {
                                diplomacy.handle_input(key.code);
                            } else if map_screen.open {
                                map_screen.handle_input(key.code, &mut state);
                            } else if stats.open {
                                match key.code {
                                    KeyCode::Esc | KeyCode::Tab => stats.toggle(),
                                    _ => {}
                                }
                            } else if inventory.open {
                                match key.code {
                                    KeyCode::Esc => inventory.toggle(),
                                    other => inventory.handle_input(other, &mut state),
                                }
                            } else if crew_screen.open {
                                crew_screen.handle_input(key.code, &mut state);
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
                                    KeyCode::Char('i') | KeyCode::Char('I') => inventory.toggle(),
                                    KeyCode::Char('c') | KeyCode::Char('C') => crew_screen.toggle(&state),
                                    KeyCode::Char('f') | KeyCode::Char('F') => diplomacy.toggle(),
                                    KeyCode::Char('m') | KeyCode::Char('M') => map_screen.toggle(&state),
                                    KeyCode::Char('t') | KeyCode::Char('T') => trade_screen.toggle(&state),
                                    KeyCode::Char('j') | KeyCode::Char('J') => mission_screen.toggle(&mut state),
                                    KeyCode::Char('l') | KeyCode::Char('L') => gamelog.toggle(),
                                    KeyCode::Char('?') => help_screen.toggle(),
                                    KeyCode::Char('p') | KeyCode::Char('P') => {
                                        // Voyage info display
                                        let info = crate::engine::voyage::VoyageInfo::for_voyage(state.voyage);
                                        popup_text = Some(format!(
                                            "◈ Voyage {}: {} — Target: Sector {} (Current: {})",
                                            state.voyage, info.display_name(), info.target_sector, state.sector,
                                        ));
                                        popup_timer = 60;
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

                // ── Voyage cinematic (blocks normal gameplay when active) ──
                if voyage_screen.active {
                    let completed = voyage_screen.tick();
                    if completed {
                        // Voyage cinematic finished — reset game state
                        state.complete_voyage();
                        let size = terminal.size()?;
                        particles.particles.clear();
                        get_scene_mut(
                            &mut travel, &mut battle, &mut raid, &mut loot,
                            state.phase,
                        )
                        .enter(&state, size.width, size.height);
                        state.save();
                    }
                    // Render only the voyage screen
                    terminal.draw(|frame| {
                        voyage_screen.render(frame);
                    })?;

                    // Frame rate control
                    let elapsed = tick_start.elapsed();
                    if elapsed < TICK_RATE {
                        std::thread::sleep(TICK_RATE - elapsed);
                    }
                    continue;
                }

                // Tick particles
                particles.tick();

                // Tick current scene
                let action = get_scene_mut(
                    &mut travel, &mut battle, &mut raid, &mut loot, state.phase,
                )
                .tick(&mut state, &mut particles, &mut event_bus);

                // Handle scene transitions
                if let SceneAction::TransitionTo(next_phase) = action {
                    // Update highest sector
                    if state.sector > state.highest_sector {
                        state.highest_sector = state.sector;
                    }

                    // Pip loot notification (battle win/loss handled via events)
                    if next_phase == GamePhase::Loot {
                        bridge.notify_loot();
                    }

                    // ── Voyage boss defeated → activate cinematic ──────
                    if state.phase == GamePhase::Loot && next_phase == GamePhase::Travel && state.voyage_boss_defeated {
                        state.voyage_boss_defeated = false;
                        let size = terminal.size()?;
                        let stats = VoyageStats {
                            sectors_cleared: state.sector.saturating_sub(1),
                            battles_won: state.total_battles,
                            enemies_destroyed: state.enemies_destroyed,
                            ships_built: state.voyage_ships_built,
                            crew_recruited: state.voyage_crew_recruited,
                            equipment_found: state.voyage_equipment_found,
                            credits_earned: state.voyage_credits_earned,
                            time_played_secs: state.time_played_secs,
                        };
                        voyage_screen.activate(
                            state.voyage,
                            stats,
                            &state.voyage_bonuses,
                            size.width,
                            size.height,
                        );
                        // Don't proceed with the normal Loot→Travel transition;
                        // the voyage cinematic will handle complete_voyage() when done
                        continue;
                    }

                    // ── Sector transition logic (Loot → Travel) ──────────
                    // When transitioning from Loot to Travel, the sector has been
                    // cleared and advanced. Run mission/contraband/faction checks.
                    if state.phase == GamePhase::Loot && next_phase == GamePhase::Travel {
                        // Check mission progress
                        // Note: boss_killed/raid_completed tracked via events;
                        // sector-based missions (Delivery, Escort, Exploration) progress here.
                        let mission_updates = state.check_mission_progress(
                            state.sector,
                            true,   // battle was won (we came from loot, not death)
                            false,  // boss status not tracked across scenes yet
                            false,  // raid status handled separately
                            false,  // fleet ship loss
                        );
                        for update in &mission_updates {
                            match &update.update_type {
                                missions::MissionUpdateType::Completed {
                                    reward_credits, reward_rep, ..
                                } => {
                                    event_bus.emit(engine::events::GameEvent::MissionCompleted {
                                        title: update.title.clone(),
                                        reward_credits: *reward_credits,
                                        reward_rep: *reward_rep,
                                    });
                                    // Apply faction rep for mission completion
                                    if let Some(mission) = state.active_missions.iter()
                                        .chain(state.available_missions.iter())
                                        .find(|m| m.id == update.mission_id)
                                    {
                                        let faction = mission.faction;
                                        state.change_reputation(faction, *reward_rep);
                                    }
                                }
                                missions::MissionUpdateType::Failed { reason } => {
                                    event_bus.emit(engine::events::GameEvent::MissionFailed {
                                        title: update.title.clone(),
                                        reason: reason.clone(),
                                    });
                                }
                                missions::MissionUpdateType::Progress { .. } => {}
                            }
                        }

                        // Expire old missions
                        state.fail_expired_missions();

                        // Contraband check when entering new sector
                        let sector_faction = state.sector_faction(state.sector);
                        let contraband_results = trade::check_contraband(
                            state.sector,
                            &sector_faction,
                            &state.cargo,
                            state.sector + state.total_battles as u32,
                        );
                        for result in contraband_results {
                            event_bus.emit(engine::events::GameEvent::ContrabandDetected {
                                good: result.good.name().to_string(),
                                faction: sector_faction.name().to_string(),
                                fine: result.fine,
                            });
                        }

                        // Refresh available missions for new sector
                        state.refresh_available_missions(state.sector);
                    }

                    // Track whether we're entering loot from a raid
                    let prev_phase = state.phase;
                    if next_phase == GamePhase::Loot {
                        loot.from_raid = prev_phase == GamePhase::Raid;
                        loot.raid_sector = state.sector;
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
                        event_bus.emit(engine::events::GameEvent::AchievementUnlocked {
                            id: ach.id.to_string(),
                            name: format!("{} — {}", ach.name, ach.description),
                            icon: ach.icon,
                        });
                    }
                }

                // Process queued events (central hub for cross-system effects)
                if event_bus.has_pending() {
                    let events = event_bus.drain();
                    // Pip commentary — pick the last relevant comment
                    for event in &events {
                        if let Some(comment) = pip_commentary(event, &state) {
                            pip_comment = Some(comment);
                            pip_comment_timer = 60; // 3 seconds at 20fps
                        }
                    }
                    process_events(&events, &mut state, &mut bridge, &mut popup_text, &mut popup_timer);
                    // Convert events to log entries
                    for event in &events {
                        let log_entries = engine::events::event_to_log_entries(event, &state);
                        gamelog.add_entries(log_entries);
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
                    let active_overlay = if help_screen.open {
                        "help"
                    } else if gamelog.open {
                        "log"
                    } else if trade_screen.open {
                        "trade"
                    } else if mission_screen.open {
                        "missions"
                    } else if inventory.open {
                        "inventory"
                    } else if crew_screen.open {
                        "crew"
                    } else if diplomacy.open {
                        "diplomacy"
                    } else if bridge.open {
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

                    // Pip commentary bubble (bottom-right, above HUD)
                    if let Some(ref comment) = pip_comment
                        && pip_comment_timer > 0 {
                            render_pip_bubble(frame, comment, pip_comment_timer);
                        }

                    // Overlays
                    if gamelog.open {
                        gamelog.render(frame);
                    }
                    if trade_screen.open {
                        trade_screen.render(frame, &state);
                    }
                    if mission_screen.open {
                        mission_screen.render(frame, &state);
                    }
                    if inventory.open {
                        inventory.render(frame, &state);
                    }
                    if crew_screen.open {
                        crew_screen.render(frame, &state);
                    }
                    if bridge.open {
                        bridge.render(frame, &state);
                    }
                    if diplomacy.open {
                        diplomacy.render(frame, &state);
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
                    if help_screen.open {
                        help_screen.render(frame);
                    }
                })?;

                // Tick popup timer
                if popup_timer > 0 {
                    popup_timer -= 1;
                    if popup_timer == 0 {
                        popup_text = None;
                    }
                }

                // Tick pip comment timer
                if pip_comment_timer > 0 {
                    pip_comment_timer -= 1;
                    if pip_comment_timer == 0 {
                        pip_comment = None;
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
        "trade" => format!(
            " MARKET │ Cargo: {}/{} │ ₿{} │ [?]Help [Esc]Close ",
            state.cargo_total(), state.cargo_capacity, state.credits
        ),
        "missions" => format!(
            " MISSIONS │ Active: {} │ [?]Help [Esc]Close ",
            state.active_missions.len()
        ),
        "crew" => format!(
            " CREW │ {}/{} │ ₿{} │ [?]Help [Esc]Close ",
            state.crew_roster.len(), state.crew_capacity, state.credits
        ),
        "inventory" => format!(
            " INVENTORY │ {}/{} │ ◇{} │ [?]Help [Esc]Close ",
            state.inventory.len(), state.inventory_capacity, state.scrap
        ),
        "bridge" => format!(
            " BRIDGE │ Pip Lv.{} │ [?]Help [Esc]Close ",
            state.pip_level
        ),
        "upgrades" => format!(
            " UPGRADES │ ◇{} │ ₿{} │ [?]Help [Esc]Close ",
            state.scrap, state.credits
        ),
        "diplomacy" => " FACTIONS │ [?]Help [Esc]Close ".to_string(),
        "stats" => " STATS │ [Esc/Tab]Close ".to_string(),
        "map" => " SECTOR MAP │ [?]Help [Esc]Close ".to_string(),
        "log" => " SHIP LOG │ [?]Help [Esc]Close ".to_string(),
        "event" => " EVENT │ [↑↓]Select [Enter]Choose ".to_string(),
        "help" => " HELP │ [↑↓]Scroll [Esc]Close ".to_string(),
        _ => {
            let voyage_target = crate::engine::voyage::voyage_target_sector(state.voyage);
            format!(
                " V{} {} │ Sec:{}/{} │ Lv.{} │ ◇{} │ ₿{} │ Ships:{} │ [?]Help ",
                state.voyage, phase_name, state.sector, voyage_target,
                state.level, state.scrap, state.credits, state.fleet.len()
            )
        }
    };

    // Determine label color based on context
    let (label, label_color) = match overlay {
        "trade" => ("MARKET", Color::Yellow),
        "missions" => ("MISSIONS", Color::Cyan),
        "crew" => ("CREW", Color::Magenta),
        "diplomacy" => ("FACTIONS", Color::LightBlue),
        "inventory" => ("INVENTORY", Color::Yellow),
        "bridge" => ("BRIDGE", Color::Magenta),
        "upgrades" => ("UPGRADES", Color::Green),
        "stats" => ("STATS", Color::Cyan),
        "map" => ("SECTOR MAP", Color::Yellow),
        "log" => ("SHIP LOG", Color::White),
        "help" => ("HELP", Color::White),
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

fn render_pip_bubble(frame: &mut Frame, text: &str, timer: u8) {
    let area = frame.area();
    let buf = frame.buffer_mut();

    // Small face + bubble in bottom-right, above HUD bar
    let face = "(◕◕)";
    let bubble = format!("\u{300c}{}\u{300d}", text);
    let face_width = face.chars().count() as u16;
    let bubble_width = bubble.chars().count() as u16;
    let total_width = face_width + 1 + bubble_width;
    let x = area.width.saturating_sub(total_width + 2);
    let y = area.height.saturating_sub(3); // above HUD

    if y == 0 || x == 0 {
        return;
    }

    // Draw face
    for (i, ch) in face.chars().enumerate() {
        let cx = x + i as u16;
        if cx < area.width && y < area.height {
            let cell = &mut buf[(cx, y)];
            cell.set_char(ch);
            cell.set_fg(Color::Green); // Pip color
        }
    }

    // Draw bubble with fade
    let bx = x + face_width + 1;
    let fade = if timer > 40 {
        Color::White
    } else if timer > 20 {
        Color::Gray
    } else {
        Color::DarkGray
    };
    for (i, ch) in bubble.chars().enumerate() {
        let cx = bx + i as u16;
        if cx < area.width && y < area.height {
            let cell = &mut buf[(cx, y)];
            cell.set_char(ch);
            cell.set_fg(fade);
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
