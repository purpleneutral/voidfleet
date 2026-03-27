use rand::Rng;
use ratatui::style::Color;
use ratatui::Frame;

use crate::rendering::particles::ParticleSystem;
use crate::state::GameState;

use super::{Scene, SceneAction};

/// Pre-computed loot data snapshot taken at scene entry.
#[derive(Debug, Clone, Default)]
struct LootData {
    sector_cleared: u32,
    credits_gained: u64,
    scrap_gained: u64,
    xp_gained: u64,
    blueprint_drop: bool,
    artifact_drop: bool,
    new_record: bool,
    level_up: bool,
    old_level: u32,
    new_level: u32,
    /// XP bar progress at scene start (before gain), 0.0–1.0
    xp_start_ratio: f32,
    /// XP bar progress at scene end (after gain), 0.0–1.0
    xp_end_ratio: f32,
    /// Fleet status: (ship_name, alive)
    fleet_status: Vec<(&'static str, bool)>,
}

/// Sparkle particle for rare loot effects within the loot box.
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

pub struct LootScene {
    tick_count: u16,
    loot: LootData,
    sparkles: Vec<Sparkle>,
    /// Which line index we've revealed so far (increments every 15 ticks).
    lines_revealed: usize,
    /// XP bar animated fill progress (0.0–1.0).
    xp_anim: f32,
    /// Flash timer for level-up text.
    level_up_flash: u8,
}

impl LootScene {
    pub fn new() -> Self {
        Self {
            tick_count: 0,
            loot: LootData::default(),
            sparkles: Vec::new(),
            lines_revealed: 0,
            xp_anim: 0.0,
            level_up_flash: 0,
        }
    }
}

impl Scene for LootScene {
    fn enter(&mut self, state: &GameState, _width: u16, _height: u16) {
        self.tick_count = 0;
        self.sparkles.clear();
        self.lines_revealed = 0;
        self.level_up_flash = 0;

        // Compute loot before mutating state (state is immutable here,
        // actual mutations happen in tick on frame 1).
        let mut rng = rand::thread_rng();
        let sector = state.sector;
        let sector_mult = sector as u64;
        let credits = 20 + sector_mult * 5;
        let scrap = 10 + sector_mult * 3;
        let xp = 15 + sector_mult * 2;
        let bp_drop = rng.gen_range(0.0f32..1.0) < 0.1;
        let art_drop = rng.gen_range(0.0f32..1.0) < 0.03;
        let new_record = sector >= state.highest_sector && sector > 1;

        // Pre-compute level-up detection
        let mut sim_xp = state.xp;
        let mut sim_xp_next = state.xp_to_next;
        let mut sim_level = state.level;
        let xp_start_ratio = if sim_xp_next > 0 {
            sim_xp as f32 / sim_xp_next as f32
        } else {
            0.0
        };

        sim_xp += xp;
        let leveled_up = sim_xp >= sim_xp_next;
        let old_level = sim_level;
        while sim_xp >= sim_xp_next {
            sim_xp -= sim_xp_next;
            sim_level += 1;
            sim_xp_next = (sim_xp_next as f64 * 1.3) as u64;
        }
        let xp_end_ratio = if sim_xp_next > 0 {
            sim_xp as f32 / sim_xp_next as f32
        } else {
            0.0
        };

        let fleet_status: Vec<(&'static str, bool)> = state
            .fleet
            .iter()
            .map(|s| (s.ship_type.name(), s.is_alive()))
            .collect();

        self.loot = LootData {
            sector_cleared: sector,
            credits_gained: credits,
            scrap_gained: scrap,
            xp_gained: xp,
            blueprint_drop: bp_drop,
            artifact_drop: art_drop,
            new_record,
            level_up: leveled_up,
            old_level,
            new_level: sim_level,
            xp_start_ratio,
            xp_end_ratio,
            fleet_status,
        };

        self.xp_anim = xp_start_ratio;
    }

    fn tick(&mut self, state: &mut GameState, _particles: &mut ParticleSystem) -> SceneAction {
        self.tick_count += 1;

        // Award loot on first tick
        if self.tick_count == 1 {
            state.credits += self.loot.credits_gained;
            state.scrap += self.loot.scrap_gained;
            state.add_xp(self.loot.xp_gained);

            if self.loot.blueprint_drop {
                state.blueprints += 1;
            }
            if self.loot.artifact_drop {
                state.artifacts += 1;
            }

            // Heal fleet
            for ship in &mut state.fleet {
                ship.heal_full();
            }

            // Advance sector
            state.sector += 1;
        }

        // Reveal lines progressively (every 15 ticks)
        let total_lines = self.total_display_lines();
        let target_reveal = ((self.tick_count / 15) as usize + 1).min(total_lines);
        if target_reveal > self.lines_revealed {
            self.lines_revealed = target_reveal;
        }

        // Animate XP bar fill
        let target_xp = if self.loot.level_up {
            // If leveled up, fill to 100% first then jump to new ratio
            if self.xp_anim < 0.98 {
                1.0
            } else {
                self.loot.xp_end_ratio
            }
        } else {
            self.loot.xp_end_ratio
        };
        self.xp_anim += (target_xp - self.xp_anim) * 0.08;
        if (self.xp_anim - target_xp).abs() < 0.01 {
            self.xp_anim = target_xp;
        }

        // Level-up flash
        if self.loot.level_up && self.xp_anim >= 0.95 && self.level_up_flash == 0 {
            self.level_up_flash = 30; // 1.5 seconds of flashing
        }
        if self.level_up_flash > 0 {
            self.level_up_flash -= 1;
        }

        // Tick sparkles
        self.sparkles.retain_mut(|s| {
            s.x += s.vx;
            s.y += s.vy;
            s.vy += 0.02; // slight gravity
            if s.life == 0 {
                return false;
            }
            s.life -= 1;
            true
        });

        // Spawn sparkles for rare drops when they appear
        if self.loot.blueprint_drop && self.lines_revealed >= 3 && self.tick_count % 4 == 0 {
            self.spawn_sparkles(Color::Cyan);
        }
        if self.loot.artifact_drop && self.lines_revealed >= 4 && self.tick_count % 4 == 0 {
            self.spawn_sparkles(Color::Magenta);
        }

        // Auto-advance after 100 ticks (5 seconds)
        if self.tick_count >= 100 {
            state.phase_timer = 45.0;
            state.save();
            SceneAction::TransitionTo(crate::state::GamePhase::Travel)
        } else {
            SceneAction::Continue
        }
    }

    fn render(&self, frame: &mut Frame, _state: &GameState, _particles: &ParticleSystem) {
        let area = frame.area();
        let buf = frame.buffer_mut();

        // Build display lines with colors
        let lines = self.build_lines();
        let visible = &lines[..self.lines_revealed.min(lines.len())];

        // Box dimensions
        let box_width: u16 = 30;
        let box_height = lines.len() as u16 + 2; // +2 for top/bottom border
        let bx = area.width.saturating_sub(box_width) / 2;
        let by = area.height.saturating_sub(box_height) / 2;

        // Draw box border
        self.draw_box(buf, area, bx, by, box_width, box_height);

        // Draw visible content lines
        for (row, (text, color)) in visible.iter().enumerate() {
            let y = by + 1 + row as u16;
            if y >= area.y + area.height {
                break;
            }
            // Center text within box
            let text_width = text.chars().count() as u16;
            let x_start = bx + (box_width.saturating_sub(text_width)) / 2;
            for (i, ch) in text.chars().enumerate() {
                let x = x_start + i as u16;
                if x > bx && x < bx + box_width - 1 && y < area.y + area.height {
                    let cell = &mut buf[(area.x + x, area.y + y)];
                    cell.set_char(ch);
                    cell.set_fg(*color);
                }
            }
        }

        // Draw XP bar (always at a fixed row in the box)
        let xp_row = by + lines.len() as u16 - 3; // 3 from bottom of content
        if self.lines_revealed >= lines.len().saturating_sub(4) {
            self.draw_xp_bar(buf, area, bx, xp_row, box_width);
        }

        // Draw sparkles
        for sparkle in &self.sparkles {
            let sx = sparkle.x.round() as u16;
            let sy = sparkle.y.round() as u16;
            if sx < area.width && sy < area.height && sx > 0 {
                let fade = sparkle.life as f32 / 10.0;
                let cell = &mut buf[(area.x + sx, area.y + sy)];
                cell.set_char(sparkle.ch);
                let color = if fade > 0.5 {
                    sparkle.color
                } else {
                    Color::DarkGray
                };
                cell.set_fg(color);
            }
        }

        // NEW RECORD banner
        if self.loot.new_record && self.tick_count > 20 {
            let record_y = by.saturating_sub(2);
            let record_text = "★ NEW RECORD ★";
            let rx = area.width.saturating_sub(record_text.len() as u16) / 2;
            let flash = if self.tick_count % 10 < 5 {
                Color::Yellow
            } else {
                Color::Rgb(255, 200, 50)
            };
            for (i, ch) in record_text.chars().enumerate() {
                let x = rx + i as u16;
                if x < area.width && record_y < area.height {
                    let cell = &mut buf[(area.x + x, area.y + record_y)];
                    cell.set_char(ch);
                    cell.set_fg(flash);
                }
            }
        }
    }
}

impl LootScene {
    fn total_display_lines(&self) -> usize {
        let mut count = 2; // header + blank
        count += 1; // scrap
        count += 1; // credits
        if self.loot.blueprint_drop {
            count += 1;
        }
        if self.loot.artifact_drop {
            count += 1;
        }
        count += 1; // blank
        count += 1; // XP bar
        if self.loot.level_up {
            count += 1; // LEVEL UP text
        }
        count += 1; // blank
        count += 1; // fleet line
        count
    }

    fn build_lines(&self) -> Vec<(String, Color)> {
        let mut lines = Vec::new();

        // Header
        lines.push((
            format!("SECTOR {} CLEAR", self.loot.sector_cleared),
            Color::White,
        ));
        lines.push((String::new(), Color::White));

        // Loot items
        lines.push((
            format!("  {} Scrap    +{}", '\u{25C7}', self.loot.scrap_gained),
            Color::Gray,
        ));
        lines.push((
            format!(
                "  {} Credits  +{}",
                '\u{20BF}', self.loot.credits_gained
            ),
            Color::Yellow,
        ));
        if self.loot.blueprint_drop {
            lines.push((
                "  Blueprint!  +1".to_string(),
                Color::Cyan,
            ));
        }
        if self.loot.artifact_drop {
            lines.push((
                "  Artifact!   +1".to_string(),
                Color::Magenta,
            ));
        }

        lines.push((String::new(), Color::White));

        // XP bar placeholder (rendered separately for animation)
        lines.push((
            format!("  XP: +{}", self.loot.xp_gained),
            Color::Green,
        ));
        if self.loot.level_up {
            let lu_color = if self.level_up_flash > 0 && self.level_up_flash % 6 < 3 {
                Color::Yellow
            } else if self.level_up_flash > 0 {
                Color::White
            } else {
                Color::Green
            };
            lines.push((
                format!(
                    "  LEVEL UP! {} -> {}",
                    self.loot.old_level, self.loot.new_level
                ),
                lu_color,
            ));
        }

        lines.push((String::new(), Color::White));

        // Fleet status
        let fleet_str: String = self
            .loot
            .fleet_status
            .iter()
            .map(|(name, alive)| {
                if *alive {
                    // Show abbreviated ship type
                    let abbr = match *name {
                        "Scout" => "=>",
                        "Fighter" => "=|>",
                        "Bomber" => "==>",
                        "Frigate" => "==>|",
                        "Destroyer" => "[==]>",
                        "Capital Ship" => "[===]>",
                        "Carrier" => "[..]>",
                        _ => "?>",
                    };
                    abbr.to_string()
                } else {
                    "\u{2620}".to_string() // ☠ skull
                }
            })
            .collect::<Vec<_>>()
            .join("  ");
        lines.push((format!("  Fleet: {}", fleet_str), Color::Rgb(120, 180, 255)));

        lines
    }

    fn draw_box(
        &self,
        buf: &mut ratatui::buffer::Buffer,
        area: ratatui::layout::Rect,
        bx: u16,
        by: u16,
        w: u16,
        h: u16,
    ) {
        let border_color = Color::Rgb(100, 100, 160);

        // Top border
        if by < area.height {
            for x in bx..bx + w {
                if x < area.width {
                    let cell = &mut buf[(area.x + x, area.y + by)];
                    let ch = if x == bx {
                        '╔'
                    } else if x == bx + w - 1 {
                        '╗'
                    } else {
                        '═'
                    };
                    cell.set_char(ch);
                    cell.set_fg(border_color);
                }
            }
        }

        // Bottom border
        let bottom = by + h - 1;
        if bottom < area.height {
            for x in bx..bx + w {
                if x < area.width {
                    let cell = &mut buf[(area.x + x, area.y + bottom)];
                    let ch = if x == bx {
                        '╚'
                    } else if x == bx + w - 1 {
                        '╝'
                    } else {
                        '═'
                    };
                    cell.set_char(ch);
                    cell.set_fg(border_color);
                }
            }
        }

        // Side borders + fill interior
        for y in by + 1..by + h - 1 {
            if y >= area.height {
                break;
            }
            for x in bx..bx + w {
                if x < area.width {
                    let cell = &mut buf[(area.x + x, area.y + y)];
                    if x == bx || x == bx + w - 1 {
                        cell.set_char('║');
                        cell.set_fg(border_color);
                    } else {
                        cell.set_bg(Color::Rgb(10, 10, 20));
                    }
                }
            }
        }
    }

    fn draw_xp_bar(
        &self,
        buf: &mut ratatui::buffer::Buffer,
        area: ratatui::layout::Rect,
        bx: u16,
        y: u16,
        box_width: u16,
    ) {
        if y >= area.height {
            return;
        }
        let bar_width = (box_width - 6) as usize; // margin
        let filled = (self.xp_anim * bar_width as f32).round() as usize;
        let bar_x = bx + 3;

        for i in 0..bar_width {
            let x = bar_x + i as u16;
            if x < area.width && x < bx + box_width - 1 {
                let cell = &mut buf[(area.x + x, area.y + y)];
                if i < filled {
                    cell.set_char('█');
                    cell.set_fg(Color::Green);
                } else {
                    cell.set_char('░');
                    cell.set_fg(Color::Rgb(40, 40, 40));
                }
                cell.set_bg(Color::Rgb(10, 10, 20));
            }
        }
    }

    fn spawn_sparkles(&mut self, color: Color) {
        let mut rng = rand::thread_rng();
        let sparkle_chars = ['✦', '✧', '·', '*', '⁺'];
        for _ in 0..3 {
            // Spawn around center of screen roughly where loot text is
            self.sparkles.push(Sparkle {
                x: rng.gen_range(20.0..60.0),
                y: rng.gen_range(8.0..18.0),
                vx: rng.gen_range(-0.5..0.5),
                vy: rng.gen_range(-0.4..0.1),
                life: rng.gen_range(6..12),
                ch: sparkle_chars[rng.gen_range(0..sparkle_chars.len())],
                color,
            });
        }
    }
}
