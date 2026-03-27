use ratatui::prelude::*;

/// The kind of screen transition effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionKind {
    /// Screen dims to black over N frames.
    FadeOut,
    /// Screen brightens from black over N frames.
    FadeIn,
    /// Black bar sweeps left to right.
    WipeLeft,
    /// Bright white flash then fade (for explosions/impacts).
    Flash,
}

/// Manages a screen transition overlay effect.
#[derive(Debug)]
pub struct ScreenTransition {
    pub active: bool,
    pub frame: u8,
    pub max_frames: u8,
    pub kind: TransitionKind,
}

impl ScreenTransition {
    pub fn new() -> Self {
        Self {
            active: false,
            frame: 0,
            max_frames: 0,
            kind: TransitionKind::FadeOut,
        }
    }

    /// Start a transition effect.
    pub fn start(&mut self, kind: TransitionKind, frames: u8) {
        self.active = true;
        self.frame = 0;
        self.max_frames = frames.max(1);
        self.kind = kind;
    }

    /// Advance by one frame. Returns `true` when the transition has completed.
    pub fn tick(&mut self) -> bool {
        if !self.active {
            return false;
        }
        self.frame += 1;
        if self.frame >= self.max_frames {
            self.active = false;
            return true;
        }
        false
    }

    /// Progress ratio from 0.0 (start) to 1.0 (end).
    fn progress(&self) -> f32 {
        if self.max_frames == 0 {
            return 1.0;
        }
        self.frame as f32 / self.max_frames as f32
    }

    /// Apply the transition effect to the already-rendered frame buffer.
    pub fn apply(&self, frame: &mut Frame) {
        if !self.active {
            return;
        }
        match self.kind {
            TransitionKind::FadeOut => self.apply_fade_out(frame),
            TransitionKind::FadeIn => self.apply_fade_in(frame),
            TransitionKind::WipeLeft => self.apply_wipe_left(frame),
            TransitionKind::Flash => self.apply_flash(frame),
        }
    }

    /// FadeOut: progressively darken all cells toward black.
    fn apply_fade_out(&self, frame: &mut Frame) {
        let t = self.progress(); // 0→1 means fully bright→fully dark
        let area = frame.area();
        let buf = frame.buffer_mut();
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let cell = &mut buf[(x, y)];
                cell.set_fg(dim_color(cell.fg, t));
                cell.set_bg(dim_color(cell.bg, t));
            }
        }
    }

    /// FadeIn: the inverse — start dark and brighten.
    fn apply_fade_in(&self, frame: &mut Frame) {
        let t = 1.0 - self.progress(); // 1→0 means fully dark→fully bright
        let area = frame.area();
        let buf = frame.buffer_mut();
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let cell = &mut buf[(x, y)];
                cell.set_fg(dim_color(cell.fg, t));
                cell.set_bg(dim_color(cell.bg, t));
            }
        }
    }

    /// WipeLeft: a black column sweeps from left to right.
    fn apply_wipe_left(&self, frame: &mut Frame) {
        let area = frame.area();
        let wipe_col = (self.progress() * area.width as f32) as u16;
        let buf = frame.buffer_mut();
        for y in area.top()..area.bottom() {
            for x in area.left()..area.left() + wipe_col.min(area.width) {
                let cell = &mut buf[(x, y)];
                cell.set_char(' ');
                cell.set_fg(Color::Black);
                cell.set_bg(Color::Black);
            }
        }
    }

    /// Flash: first ~30% bright white bg, then fade back.
    fn apply_flash(&self, frame: &mut Frame) {
        let t = self.progress();
        let area = frame.area();
        let buf = frame.buffer_mut();

        if t < 0.3 {
            // Bright white flash phase
            let intensity = ((1.0 - t / 0.3) * 255.0) as u8;
            let flash_bg = Color::Rgb(intensity, intensity, intensity);
            for y in area.top()..area.bottom() {
                for x in area.left()..area.right() {
                    let cell = &mut buf[(x, y)];
                    cell.set_bg(flash_bg);
                    // Keep fg readable — invert to dark during bright flash
                    if intensity > 128 {
                        cell.set_fg(Color::Black);
                    }
                }
            }
        } else {
            // Fade-back phase — slight residual brightness dims to 0
            let fade = (t - 0.3) / 0.7; // 0→1
            let residual = ((1.0 - fade) * 40.0) as u8;
            if residual > 0 {
                let tint = Color::Rgb(residual, residual, residual);
                for y in area.top()..area.bottom() {
                    for x in area.left()..area.right() {
                        let cell = &mut buf[(x, y)];
                        cell.set_bg(blend_toward(cell.bg, tint, 1.0 - fade));
                    }
                }
            }
        }
    }
}

/// Dim a color toward black by factor `t` (0.0 = unchanged, 1.0 = fully black).
fn dim_color(color: Color, t: f32) -> Color {
    let factor = (1.0 - t).clamp(0.0, 1.0);
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            (r as f32 * factor) as u8,
            (g as f32 * factor) as u8,
            (b as f32 * factor) as u8,
        ),
        Color::White => {
            let v = (255.0 * factor) as u8;
            Color::Rgb(v, v, v)
        }
        Color::Gray => {
            let v = (180.0 * factor) as u8;
            Color::Rgb(v, v, v)
        }
        Color::DarkGray => {
            let v = (100.0 * factor) as u8;
            Color::Rgb(v, v, v)
        }
        Color::Red => Color::Rgb((255.0 * factor) as u8, 0, 0),
        Color::Green => Color::Rgb(0, (255.0 * factor) as u8, 0),
        Color::Blue => Color::Rgb(0, 0, (255.0 * factor) as u8),
        Color::Yellow => Color::Rgb((255.0 * factor) as u8, (255.0 * factor) as u8, 0),
        Color::Cyan => Color::Rgb(0, (255.0 * factor) as u8, (255.0 * factor) as u8),
        Color::Magenta => Color::Rgb((255.0 * factor) as u8, 0, (255.0 * factor) as u8),
        Color::Black => Color::Black,
        // Named/indexed colors we can't decompose — just swap to black past threshold
        other => {
            if factor < 0.2 {
                Color::Black
            } else {
                other
            }
        }
    }
}

/// Blend `base` toward `target` by ratio (0.0 = base, 1.0 = target).
fn blend_toward(base: Color, target: Color, ratio: f32) -> Color {
    let (br, bg_c, bb) = color_to_rgb(base);
    let (tr, tg, tb) = color_to_rgb(target);
    let r = ratio.clamp(0.0, 1.0);
    Color::Rgb(
        (br as f32 + (tr as f32 - br as f32) * r) as u8,
        (bg_c as f32 + (tg as f32 - bg_c as f32) * r) as u8,
        (bb as f32 + (tb as f32 - bb as f32) * r) as u8,
    )
}

/// Best-effort conversion of any Color to (r, g, b).
fn color_to_rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::White => (255, 255, 255),
        Color::Gray => (180, 180, 180),
        Color::DarkGray => (100, 100, 100),
        Color::Red => (255, 0, 0),
        Color::Green => (0, 255, 0),
        Color::Blue => (0, 0, 255),
        Color::Yellow => (255, 255, 0),
        Color::Cyan => (0, 255, 255),
        Color::Magenta => (255, 0, 255),
        _ => (128, 128, 128), // fallback for indexed/etc
    }
}
