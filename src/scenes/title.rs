use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::canvas::{Canvas, Context};
use crate::engine::voyage::VoyageInfo;
use crate::rendering::starfield::Starfield;
use crate::state::GameState;

const LOGO: &[&str] = &[
    r" ██╗   ██╗ ██████╗ ██╗██████╗     ███████╗██╗     ███████╗███████╗████████╗",
    r" ██║   ██║██╔═══██╗██║██╔══██╗    ██╔════╝██║     ██╔════╝██╔════╝╚══██╔══╝",
    r" ██║   ██║██║   ██║██║██║  ██║    █████╗  ██║     █████╗  █████╗     ██║   ",
    r" ╚██╗ ██╔╝██║   ██║██║██║  ██║    ██╔══╝  ██║     ██╔══╝  ██╔══╝     ██║   ",
    r"  ╚████╔╝ ╚██████╔╝██║██████╔╝    ██║     ███████╗███████╗███████╗   ██║   ",
    r"   ╚═══╝   ╚═════╝ ╚═╝╚═════╝     ╚═╝     ╚══════╝╚══════╝╚══════╝   ╚═╝   ",
];

const SUBTITLE: &str = "Space Conquest Idle";
const VERSION: &str = "v0.1.0";
const BLINK_INTERVAL: u32 = 15;

/// Standalone title screen — not a Scene trait impl.
pub struct TitleScreen {
    starfield: Starfield,
    blink_timer: u32,
    blink_visible: bool,
    #[allow(dead_code)] // Stored for future resize support
    width: u16,
    #[allow(dead_code)]
    height: u16,
}

impl TitleScreen {
    pub fn new(width: u16, height: u16) -> Self {
        let density = (width as usize * height as usize) / 20;
        Self {
            starfield: Starfield::new(width, height, density),
            blink_timer: 0,
            blink_visible: true,
            width,
            height,
        }
    }

    /// Update starfield animation and blink timer.
    pub fn tick(&mut self) {
        self.starfield.tick();
        self.blink_timer += 1;
        if self.blink_timer >= BLINK_INTERVAL {
            self.blink_timer = 0;
            self.blink_visible = !self.blink_visible;
        }
    }

    /// Render the title screen.
    pub fn render(&self, frame: &mut Frame, has_save: bool, state: &GameState) {
        let area = frame.area();

        // ── Starfield background ──────────────────────────────
        self.render_starfield(frame, area);

        // ── ASCII logo (centered) ─────────────────────────────
        let logo_height = LOGO.len() as u16;

        // Vertical layout: logo near top third, subtitle below, prompt lower
        let logo_y = area.y + area.height.saturating_sub(logo_height + 10) / 3;

        for (i, line) in LOGO.iter().enumerate() {
            let line_len = line.chars().count() as u16;
            let x = area.x + area.width.saturating_sub(line_len) / 2;
            let y = logo_y + i as u16;
            if y < area.y + area.height {
                let buf = frame.buffer_mut();
                let mut col = x;
                for ch in line.chars() {
                    if col < area.x + area.width {
                        buf[(col, y)].set_char(ch).set_style(
                            Style::default().fg(Color::Cyan),
                        );
                        col += 1;
                    }
                }
            }
        }

        // ── Subtitle ─────────────────────────────────────────
        let subtitle_y = logo_y + logo_height + 1;
        if subtitle_y < area.y + area.height {
            let sub_len = SUBTITLE.len() as u16;
            let x = area.x + area.width.saturating_sub(sub_len) / 2;
            let buf = frame.buffer_mut();
            for (i, ch) in SUBTITLE.chars().enumerate() {
                let col = x + i as u16;
                if col < area.x + area.width {
                    buf[(col, subtitle_y)].set_char(ch).set_style(
                        Style::default().fg(Color::DarkGray),
                    );
                }
            }
        }

        // ── Save info / prompt ────────────────────────────────
        let info_y = subtitle_y + 3;
        if info_y < area.y + area.height && has_save {
            let voyage_info = VoyageInfo::for_voyage(state.voyage);
            let voyage_line = format!(
                "Voyage {}: {}",
                voyage_info.number, voyage_info.name
            );
            self.draw_centered_line(frame, area, info_y, &voyage_line, Color::Rgb(255, 215, 0));

            let detail_line = format!(
                "Sector {} | Level {} | {} ships",
                state.sector, state.level, state.fleet.len()
            );
            if info_y + 1 < area.y + area.height {
                self.draw_centered_line(frame, area, info_y + 1, &detail_line, Color::White);
            }

            // Show permanent bonuses if any
            let bonus_str = state.voyage_bonuses.hud_string();
            if !bonus_str.is_empty() && info_y + 2 < area.y + area.height {
                let bonus_line = format!("Permanent: {}", bonus_str);
                self.draw_centered_line(frame, area, info_y + 2, &bonus_line, Color::Rgb(160, 160, 180));
            }
        } else if info_y < area.y + area.height {
            self.draw_centered_line(frame, area, info_y, "New Game", Color::White);
        }

        // ── Blinking prompt ───────────────────────────────────
        if self.blink_visible {
            let prompt_y = info_y + 2;
            if prompt_y < area.y + area.height {
                let prompt = if has_save {
                    "Press [C] to continue / [N] for new game"
                } else {
                    "Press [ENTER] to start"
                };
                let prompt_len = prompt.len() as u16;
                let x = area.x + area.width.saturating_sub(prompt_len) / 2;
                let buf = frame.buffer_mut();
                for (i, ch) in prompt.chars().enumerate() {
                    let col = x + i as u16;
                    if col < area.x + area.width {
                        buf[(col, prompt_y)].set_char(ch).set_style(
                            Style::default().fg(Color::Yellow),
                        );
                    }
                }
            }
        }

        // ── Version (bottom-right) ───────────────────────────
        let ver_y = area.y + area.height.saturating_sub(1);
        let ver_len = VERSION.len() as u16;
        let ver_x = area.x + area.width.saturating_sub(ver_len + 1);
        if ver_y >= area.y {
            let buf = frame.buffer_mut();
            for (i, ch) in VERSION.chars().enumerate() {
                let col = ver_x + i as u16;
                if col < area.x + area.width {
                    buf[(col, ver_y)].set_char(ch).set_style(
                        Style::default().fg(Color::DarkGray),
                    );
                }
            }
        }
    }

    /// Draw a centered line of text at a given row.
    fn draw_centered_line(&self, frame: &mut Frame, area: Rect, y: u16, text: &str, color: Color) {
        let text_len = text.len() as u16;
        let x = area.x + area.width.saturating_sub(text_len) / 2;
        let buf = frame.buffer_mut();
        for (i, ch) in text.chars().enumerate() {
            let col = x + i as u16;
            if col < area.x + area.width {
                buf[(col, y)].set_char(ch).set_style(
                    Style::default().fg(color),
                );
            }
        }
    }

    /// Draw the starfield as a canvas background.
    fn render_starfield(&self, frame: &mut Frame, area: Rect) {
        let stars = &self.starfield.stars;
        let w = area.width as f64;
        let h = area.height as f64;

        let canvas = Canvas::default()
            .x_bounds([0.0, w])
            .y_bounds([0.0, h])
            .paint(|ctx: &mut Context<'_>| {
                for star in stars {
                    let sx = star.x as f64;
                    // Canvas y is bottom-up, terminal y is top-down
                    let sy = h - star.y as f64;
                    if sx >= 0.0 && sx < w && sy >= 0.0 && sy < h {
                        ctx.print(sx, sy, ratatui::text::Span::styled(
                            star.ch.to_string(),
                            Style::default().fg(star.color),
                        ));
                    }
                }
            });

        frame.render_widget(canvas, area);
    }
}
