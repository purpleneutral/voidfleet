//! Voyage completion cinematic — dramatic multi-phase screen shown when
//! the voyage boss is defeated. Starfield background with sparkle effects.

use rand::Rng;
use ratatui::style::Color;
use ratatui::Frame;

use crate::engine::voyage::{VoyageBonuses, VoyageInfo, VoyageStats};
use crate::rendering::starfield::Starfield;

/// Sparkle particle for bonus reveals.
#[derive(Debug, Clone)]
struct Sparkle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: u8,
    ch: char,
    color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VoyagePhase {
    FadeOut,    // 30 frames: screen dims
    Title,      // 60 frames: "VOYAGE X COMPLETE" in big text
    Stats,      // 80 frames: show journey stats one by one
    Bonuses,    // 60 frames: show permanent bonuses earned
    NextVoyage, // Holds until Enter pressed
    FadeIn,     // 30 frames: fade back to game
}

pub struct VoyageScreen {
    pub active: bool,
    tick: u64,
    phase: VoyagePhase,
    phase_tick: u64,
    voyage_completed: u32,
    voyage_name: String,
    next_voyage_name: String,
    next_voyage_subtitle: String,
    bonuses_earned: VoyageBonuses,
    total_bonuses: VoyageBonuses,
    stats: VoyageStats,
    starfield: Starfield,
    sparkles: Vec<Sparkle>,
    enter_pressed: bool,
}

impl VoyageScreen {
    pub fn new() -> Self {
        Self {
            active: false,
            tick: 0,
            phase: VoyagePhase::FadeOut,
            phase_tick: 0,
            voyage_completed: 1,
            voyage_name: String::new(),
            next_voyage_name: String::new(),
            next_voyage_subtitle: String::new(),
            bonuses_earned: VoyageBonuses::default(),
            total_bonuses: VoyageBonuses::default(),
            stats: VoyageStats::default(),
            starfield: Starfield::new(80, 24, 200),
            sparkles: Vec::new(),
            enter_pressed: false,
        }
    }

    /// Activate the voyage completion cinematic.
    pub fn activate(
        &mut self,
        voyage_number: u32,
        stats: VoyageStats,
        current_bonuses: &VoyageBonuses,
        width: u16,
        height: u16,
    ) {
        self.active = true;
        self.tick = 0;
        self.phase = VoyagePhase::FadeOut;
        self.phase_tick = 0;
        self.voyage_completed = voyage_number;
        self.enter_pressed = false;
        self.sparkles.clear();

        let current_info = VoyageInfo::for_voyage(voyage_number);
        self.voyage_name = current_info.name.to_string();

        let next_info = VoyageInfo::for_voyage(voyage_number + 1);
        self.next_voyage_name = format!("VOYAGE {}: {}", next_info.number, next_info.name.to_uppercase());
        self.next_voyage_subtitle = next_info.subtitle.to_string();

        self.bonuses_earned = VoyageBonuses::for_completion(voyage_number);
        // Total = current accumulated + what we'll earn
        self.total_bonuses = current_bonuses.clone();
        self.total_bonuses.accumulate(&self.bonuses_earned);

        // Also compute from the legacy percentage fields if bonuses struct is empty
        // (covers the case where voyage_bonuses wasn't populated yet)
        if self.total_bonuses.damage_pct == 0.0 && current_bonuses.damage_pct == 0.0 {
            self.total_bonuses = self.bonuses_earned.clone();
        }

        self.stats = stats;

        let density = (width as usize * height as usize) / 15;
        self.starfield = Starfield::new(width, height, density);
    }

    /// Handle Enter key press.
    pub fn handle_enter(&mut self) {
        if self.phase == VoyagePhase::NextVoyage {
            self.enter_pressed = true;
        }
    }

    /// Tick the cinematic. Returns true when the cinematic is complete
    /// and the game should call `state.complete_voyage()`.
    pub fn tick(&mut self) -> bool {
        if !self.active {
            return false;
        }

        self.tick += 1;
        self.phase_tick += 1;
        self.starfield.tick();

        // Tick sparkles
        self.sparkles.retain_mut(|s| {
            s.x += s.vx;
            s.y += s.vy;
            s.vy += 0.02;
            if s.life == 0 {
                return false;
            }
            s.life -= 1;
            true
        });

        // Spawn sparkles during Bonuses phase
        if self.phase == VoyagePhase::Bonuses && self.tick.is_multiple_of(3) {
            self.spawn_sparkles();
        }

        // Phase transitions
        let advance = match self.phase {
            VoyagePhase::FadeOut => self.phase_tick >= 30,
            VoyagePhase::Title => self.phase_tick >= 60,
            VoyagePhase::Stats => self.phase_tick >= 80,
            VoyagePhase::Bonuses => self.phase_tick >= 60,
            VoyagePhase::NextVoyage => self.enter_pressed,
            VoyagePhase::FadeIn => self.phase_tick >= 30,
        };

        if advance {
            let next = match self.phase {
                VoyagePhase::FadeOut => VoyagePhase::Title,
                VoyagePhase::Title => VoyagePhase::Stats,
                VoyagePhase::Stats => VoyagePhase::Bonuses,
                VoyagePhase::Bonuses => VoyagePhase::NextVoyage,
                VoyagePhase::NextVoyage => VoyagePhase::FadeIn,
                VoyagePhase::FadeIn => {
                    self.active = false;
                    return true; // Signal completion
                }
            };
            self.phase = next;
            self.phase_tick = 0;
        }

        false
    }

    /// Render the cinematic.
    pub fn render(&self, frame: &mut Frame) {
        if !self.active {
            return;
        }

        let area = frame.area();
        let buf = frame.buffer_mut();

        // Clear to black
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let cell = &mut buf[(x, y)];
                cell.set_char(' ');
                cell.set_fg(Color::White);
                cell.set_bg(Color::Black);
            }
        }

        // Starfield background (dimmed during FadeOut)
        let star_dim = match self.phase {
            VoyagePhase::FadeOut => {
                let progress = self.phase_tick as f32 / 30.0;
                1.0 - progress * 0.7
            }
            VoyagePhase::FadeIn => {
                let progress = self.phase_tick as f32 / 30.0;
                0.3 + progress * 0.7
            }
            _ => 0.3,
        };
        for star in &self.starfield.stars {
            let sx = star.x.round() as u16;
            let sy = star.y.round() as u16;
            if sx < area.width && sy < area.height.saturating_sub(1) {
                let cell = &mut buf[(area.x + sx, area.y + sy)];
                cell.set_char(star.ch);
                let base_brightness = match star.color {
                    Color::White => 255,
                    Color::Gray => 160,
                    _ => 80,
                };
                let b = (base_brightness as f32 * star_dim) as u8;
                cell.set_fg(Color::Rgb(b, b, b));
            }
        }

        // Shooting stars
        for (sx, sy, ch, _color) in self.starfield.shooting_star_cells() {
            if sx < area.width && sy < area.height.saturating_sub(1) {
                let cell = &mut buf[(area.x + sx, area.y + sy)];
                cell.set_char(ch);
                let b = (200.0 * star_dim) as u8;
                cell.set_fg(Color::Rgb(b, b, b + 30));
            }
        }

        // Render sparkles
        for sparkle in &self.sparkles {
            let sx = sparkle.x.round() as u16;
            let sy = sparkle.y.round() as u16;
            if sx < area.width && sy < area.height.saturating_sub(1) && sx > 0 {
                let cell = &mut buf[(area.x + sx, area.y + sy)];
                cell.set_char(sparkle.ch);
                let fade = sparkle.life as f32 / 12.0;
                cell.set_fg(if fade > 0.5 { sparkle.color } else { Color::DarkGray });
            }
        }

        // Phase-specific content
        match self.phase {
            VoyagePhase::FadeOut => {}
            VoyagePhase::Title => self.render_title(buf, area),
            VoyagePhase::Stats => self.render_stats(buf, area),
            VoyagePhase::Bonuses => self.render_bonuses(buf, area),
            VoyagePhase::NextVoyage => self.render_next_voyage(buf, area),
            VoyagePhase::FadeIn => {}
        }
    }

    fn render_title(&self, buf: &mut ratatui::buffer::Buffer, area: ratatui::layout::Rect) {
        let fade_in = (self.phase_tick as f32 / 20.0).min(1.0);
        let gold_r = (255.0 * fade_in) as u8;
        let gold_g = (215.0 * fade_in) as u8;
        let gold_b = (0.0 * fade_in) as u8;
        let gold = Color::Rgb(gold_r, gold_g, gold_b);
        let dim = Color::Rgb(
            (140.0 * fade_in) as u8,
            (140.0 * fade_in) as u8,
            (160.0 * fade_in) as u8,
        );

        let cy = area.height / 2;

        // Divider above
        let divider = "══════════════════════════════";
        self.draw_centered(buf, area, cy.saturating_sub(3), divider, dim);

        // Title
        let title = format!("VOYAGE {} COMPLETE", self.voyage_completed);
        self.draw_centered(buf, area, cy.saturating_sub(1), &title, gold);

        // Subtitle
        self.draw_centered(buf, area, cy, &self.voyage_name, dim);

        // Divider below
        self.draw_centered(buf, area, cy + 2, divider, dim);
    }

    fn render_stats(&self, buf: &mut ratatui::buffer::Buffer, area: ratatui::layout::Rect) {
        // Show title dimly at top
        let gold_dim = Color::Rgb(160, 130, 0);
        let title = format!("VOYAGE {} COMPLETE", self.voyage_completed);
        self.draw_centered(buf, area, 3, &title, gold_dim);

        let stat_lines = [
            format!("Sectors cleared:     {}", self.stats.sectors_cleared),
            format!("Battles won:         {}", self.stats.battles_won),
            format!("Enemies destroyed:   {}", self.stats.enemies_destroyed),
            format!("Ships built:         {}", self.stats.ships_built),
            format!("Crew recruited:      {}", self.stats.crew_recruited),
            format!("Equipment found:     {}", self.stats.equipment_found),
            format!("Credits earned:      {}", format_with_commas(self.stats.credits_earned)),
            format!("Time played:         {}", format_duration(self.stats.time_played_secs)),
        ];

        let start_y = area.height / 2 - 5;
        let lines_to_show = ((self.phase_tick / 10) as usize + 1).min(stat_lines.len());

        for (i, line) in stat_lines.iter().take(lines_to_show).enumerate() {
            let color = if i < lines_to_show.saturating_sub(1) {
                Color::Rgb(180, 180, 200)
            } else {
                // Newest line is brighter
                Color::White
            };
            self.draw_centered(buf, area, start_y + i as u16, line, color);
        }
    }

    fn render_bonuses(&self, buf: &mut ratatui::buffer::Buffer, area: ratatui::layout::Rect) {
        let cy = area.height / 2;
        let gold = Color::Rgb(255, 215, 0);
        let bright = Color::Rgb(200, 200, 220);
        let dim = Color::Rgb(120, 120, 140);

        // Header
        self.draw_centered(buf, area, cy.saturating_sub(5), "★ PERMANENT BONUSES EARNED ★", gold);

        let bonus_lines = [
            format!(
                "+{:.0}% Damage    (total: +{:.0}%)",
                self.bonuses_earned.damage_pct, self.total_bonuses.damage_pct
            ),
            format!(
                "+{:.0}% Hull HP   (total: +{:.0}%)",
                self.bonuses_earned.hull_hp_pct, self.total_bonuses.hull_hp_pct
            ),
            format!(
                "+{:.0}% Speed     (total: +{:.0}%)",
                self.bonuses_earned.speed_pct, self.total_bonuses.speed_pct
            ),
            format!(
                "+{:.0}% Critical  (total: +{:.0}%)",
                self.bonuses_earned.crit_pct, self.total_bonuses.crit_pct
            ),
        ];

        let lines_to_show = ((self.phase_tick / 12) as usize + 1).min(bonus_lines.len());
        for (i, line) in bonus_lines.iter().take(lines_to_show).enumerate() {
            self.draw_centered(buf, area, cy.saturating_sub(2) + i as u16, line, bright);
        }

        // Carry-over notes (appear after bonuses)
        if self.phase_tick > 40 {
            self.draw_centered(buf, area, cy + 4, "Pip bond carries over ♥", dim);
            self.draw_centered(buf, area, cy + 5, "Achievements preserved", dim);
        }
    }

    fn render_next_voyage(&self, buf: &mut ratatui::buffer::Buffer, area: ratatui::layout::Rect) {
        let cy = area.height / 2;
        let gold = Color::Rgb(255, 215, 0);
        let bright = Color::Rgb(200, 200, 220);
        let dim = Color::Rgb(120, 120, 140);

        let divider = "════════════════════════════";
        self.draw_centered(buf, area, cy.saturating_sub(4), divider, dim);

        self.draw_centered(buf, area, cy.saturating_sub(2), &self.next_voyage_name, gold);
        self.draw_centered(buf, area, cy.saturating_sub(1), &self.next_voyage_subtitle, bright);

        self.draw_centered(buf, area, cy + 1, "New enemies await...", dim);
        self.draw_centered(buf, area, cy + 2, "New equipment tiers unlocked...", dim);

        // Blinking prompt
        if (self.phase_tick / 15).is_multiple_of(2) {
            self.draw_centered(
                buf, area, cy + 4,
                "Press [ENTER] to begin",
                Color::Yellow,
            );
        }

        self.draw_centered(buf, area, cy + 6, divider, dim);
    }

    fn draw_centered(
        &self,
        buf: &mut ratatui::buffer::Buffer,
        area: ratatui::layout::Rect,
        y: u16,
        text: &str,
        color: Color,
    ) {
        if y >= area.height {
            return;
        }
        let text_len = text.chars().count() as u16;
        let x_start = area.x + area.width.saturating_sub(text_len) / 2;
        for (i, ch) in text.chars().enumerate() {
            let x = x_start + i as u16;
            if x < area.x + area.width {
                let cell = &mut buf[(x, area.y + y)];
                cell.set_char(ch);
                cell.set_fg(color);
            }
        }
    }

    fn spawn_sparkles(&mut self) {
        let mut rng = rand::thread_rng();
        let sparkle_chars = ['✦', '✧', '·', '*', '⁺', '★'];
        let colors = [
            Color::Rgb(255, 215, 0),   // gold
            Color::Rgb(255, 255, 150),  // bright yellow
            Color::Rgb(200, 180, 100),  // warm
        ];
        for _ in 0..2 {
            self.sparkles.push(Sparkle {
                x: rng.gen_range(15.0..65.0),
                y: rng.gen_range(6.0..18.0),
                vx: rng.gen_range(-0.5..0.5),
                vy: rng.gen_range(-0.4..0.1),
                life: rng.gen_range(8..14),
                ch: sparkle_chars[rng.gen_range(0..sparkle_chars.len())],
                color: colors[rng.gen_range(0..colors.len())],
            });
        }
    }
}

fn format_with_commas(n: u64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(ch);
    }
    result
}

fn format_duration(secs: u64) -> String {
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m {}s", mins, secs % 60)
    }
}
