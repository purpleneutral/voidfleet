use crossterm::event::KeyCode;
use ratatui::style::Color;
use ratatui::Frame;

use crate::state::GameState;

// ── Pip's emotional states ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mood {
    Idle,
    Happy,
    Sleeping,
    Eating,
    Sad,
    Excited,
    Dancing,
    Lonely,
}

// ── Pip sprites ─────────────────────────────────────────────────────────
// Tiny 2-line sprites: face line + feet line

impl Mood {
    fn face(&self, blink: bool) -> &'static str {
        if blink {
            return "(- -)";
        }
        match self {
            Mood::Idle => "(◕◕)",
            Mood::Happy => "(◕‿◕)",
            Mood::Sleeping => "(- -)",
            Mood::Eating => "(◕◕)",
            Mood::Sad => "(;_;)",
            Mood::Excited => "(★★)",
            Mood::Dancing => "(◕‿◕)",
            Mood::Lonely => "(◕._◕)",
        }
    }

    fn decoration(&self) -> &'static str {
        match self {
            Mood::Happy => "♪",
            Mood::Sleeping => "z",
            Mood::Excited => "!",
            Mood::Eating => "◇",
            Mood::Dancing => "♫",
            Mood::Sad => "",
            Mood::Lonely => "...",
            Mood::Idle => "",
        }
    }

    fn color(&self) -> Color {
        match self {
            Mood::Happy | Mood::Dancing => Color::Green,
            Mood::Sad | Mood::Lonely => Color::Blue,
            Mood::Excited => Color::Yellow,
            Mood::Eating => Color::Cyan,
            Mood::Sleeping => Color::DarkGray,
            Mood::Idle => Color::White,
        }
    }
}

// ── Bridge furniture ────────────────────────────────────────────────────

struct BridgeObject {
    x: u16,
    label: &'static str,
    art: &'static [&'static str], // multi-line ASCII art
}

const BRIDGE_OBJECTS: &[BridgeObject] = &[
    BridgeObject {
        x: 3,
        label: "HELM",
        art: &["┌───┐", "│≡≡≡│", "└───┘"],
    },
    BridgeObject {
        x: 14,
        label: "NAV",
        art: &["╔═══╗", "║·*·║", "╚═══╝"],
    },
    BridgeObject {
        x: 25,
        label: "COMMS",
        art: &["┌───┐", "│)))│", "└───┘"],
    },
];

// ── Pip state ───────────────────────────────────────────────────────────

pub struct BridgeScene {
    pub open: bool,

    // Pip state
    pip_x: f32,
    pip_target_x: f32,
    pip_mood: Mood,
    pip_mood_timer: u16,

    // Tamagotchi stats (0-100)
    hunger: u8,
    energy: u8,
    happiness: u8,
    bond: u16, // 0-1000, long-term relationship

    // Animation
    tick_count: u64,
    blink_timer: u8,
    bounce_frame: u8,
    walk_frame: u8,
    dance_frame: u8,

    // Speech bubble
    speech: Option<String>,
    speech_timer: u8,

    // Scene dimensions
    width: u16,
    height: u16,

    // Idle timer — ticks since last player interaction
    idle_ticks: u64,
}

impl BridgeScene {
    pub fn new() -> Self {
        Self {
            open: false,
            pip_x: 20.0,
            pip_target_x: 20.0,
            pip_mood: Mood::Idle,
            pip_mood_timer: 0,
            hunger: 80,
            energy: 80,
            happiness: 70,
            bond: 0,
            tick_count: 0,
            blink_timer: 0,
            bounce_frame: 0,
            walk_frame: 0,
            dance_frame: 0,
            speech: None,
            speech_timer: 0,
            width: 60,
            height: 20,
        idle_ticks: 0,
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            // Pip reacts to being visited
            if self.idle_ticks > 2000 {
                // Haven't visited in a while
                self.pip_mood = Mood::Lonely;
                self.pip_mood_timer = 40;
                self.say("...you came back!");
            } else {
                self.pip_mood = Mood::Happy;
                self.pip_mood_timer = 30;
                self.say("Hi!");
            }
            self.idle_ticks = 0;
            self.bond = self.bond.saturating_add(1).min(1000);
        }
    }

    fn say(&mut self, text: &str) {
        self.speech = Some(text.to_string());
        self.speech_timer = 40;
    }

    /// Notify Pip of game events (called from main loop)
    pub fn notify_battle_win(&mut self) {
        self.happiness = self.happiness.saturating_add(10).min(100);
        self.pip_mood = Mood::Excited;
        self.pip_mood_timer = 40;
    }

    pub fn notify_battle_loss(&mut self) {
        self.happiness = self.happiness.saturating_sub(15);
        self.pip_mood = Mood::Sad;
        self.pip_mood_timer = 60;
    }

    pub fn notify_loot(&mut self) {
        if self.pip_mood != Mood::Sad {
            self.pip_mood = Mood::Happy;
            self.pip_mood_timer = 20;
        }
    }

    pub fn notify_achievement(&mut self) {
        self.pip_mood = Mood::Dancing;
        self.pip_mood_timer = 60;
    }

    pub fn handle_input(&mut self, key: KeyCode, state: &mut GameState) {
        match key {
            KeyCode::Char('f') | KeyCode::Char('F') => {
                // Feed Pip — costs 10 scrap
                if state.scrap >= 10 && self.hunger < 90 {
                    state.scrap -= 10;
                    self.hunger = self.hunger.saturating_add(25).min(100);
                    self.happiness = self.happiness.saturating_add(5).min(100);
                    self.pip_mood = Mood::Eating;
                    self.pip_mood_timer = 30;
                    self.say("Yum!");
                    self.bond = self.bond.saturating_add(2).min(1000);
                } else if self.hunger >= 90 {
                    self.say("I'm full!");
                } else {
                    self.say("No scrap...");
                    self.pip_mood = Mood::Sad;
                    self.pip_mood_timer = 20;
                }
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                // Pet/play with Pip
                self.happiness = self.happiness.saturating_add(10).min(100);
                self.pip_mood = Mood::Happy;
                self.pip_mood_timer = 40;
                self.bond = self.bond.saturating_add(3).min(1000);
                let reactions = ["Hehe!", ":D", "Wheee!", "♥", "Yay!"];
                let idx = (self.tick_count as usize) % reactions.len();
                self.say(reactions[idx]);
            }
            KeyCode::Esc => self.toggle(),
            _ => {}
        }
    }

    pub fn tick(&mut self) {
        if !self.open {
            self.idle_ticks += 1;
            // Pip's needs still decay even when not viewing
            if self.tick_count % 200 == 0 {
                self.hunger = self.hunger.saturating_sub(1);
                self.energy = self.energy.saturating_sub(1);
            }
            self.tick_count += 1;
            return;
        }

        self.tick_count += 1;

        // Stat decay
        if self.tick_count % 100 == 0 {
            self.hunger = self.hunger.saturating_sub(1);
        }
        if self.tick_count % 150 == 0 {
            self.energy = self.energy.saturating_sub(1);
        }

        // Energy recovery when sleeping
        if self.pip_mood == Mood::Sleeping {
            if self.tick_count % 20 == 0 {
                self.energy = self.energy.saturating_add(2).min(100);
            }
        }

        // Auto-mood based on stats
        if self.pip_mood_timer > 0 {
            self.pip_mood_timer -= 1;
        } else {
            // Determine mood from stats
            self.pip_mood = if self.energy < 20 {
                Mood::Sleeping
            } else if self.hunger < 20 {
                Mood::Sad
            } else if self.happiness > 80 {
                Mood::Happy
            } else {
                Mood::Idle
            };
        }

        // Blink every ~80 ticks
        if self.blink_timer > 0 {
            self.blink_timer -= 1;
        } else if self.tick_count % 80 == 0 && self.pip_mood != Mood::Sleeping {
            self.blink_timer = 3;
        }

        // Bounce animation (gentle bob)
        self.bounce_frame = ((self.tick_count / 8) % 2) as u8;

        // Walk frame
        self.walk_frame = ((self.tick_count / 6) % 4) as u8;

        // Dance frame
        if self.pip_mood == Mood::Dancing {
            self.dance_frame = ((self.tick_count / 4) % 4) as u8;
        }

        // Movement — pick new target when reached current one
        let dx = self.pip_target_x - self.pip_x;
        if dx.abs() < 0.5 {
            // Pick new target
            if self.pip_mood != Mood::Sleeping {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                self.pip_target_x = rng.gen_range(5.0..45.0);
            }
        } else {
            // Move toward target
            let speed = if self.pip_mood == Mood::Dancing { 0.3 } else { 0.15 };
            self.pip_x += dx.signum() * speed;
        }

        // Speech timer
        if self.speech_timer > 0 {
            self.speech_timer -= 1;
            if self.speech_timer == 0 {
                self.speech = None;
            }
        }
    }

    pub fn render(&self, frame: &mut Frame, _state: &GameState) {
        let area = frame.area();
        let buf = frame.buffer_mut();

        // Calculate bridge area (centered box)
        let bw = 56.min(area.width.saturating_sub(4));
        let bh = 16.min(area.height.saturating_sub(4));
        let bx = (area.width - bw) / 2;
        let by = (area.height - bh) / 2;

        // Clear bridge area
        for y in by..by + bh {
            for x in bx..bx + bw {
                if x < area.width && y < area.height {
                    let cell = &mut buf[(x, y)];
                    cell.set_char(' ');
                    cell.set_bg(Color::Rgb(15, 15, 25));
                    cell.set_fg(Color::White);
                }
            }
        }

        // Border
        let border_color = Color::Rgb(60, 60, 80);
        for x in bx..bx + bw {
            if by < area.height {
                buf[(x, by)].set_char('═');
                buf[(x, by)].set_fg(border_color);
            }
            if by + bh - 1 < area.height {
                buf[(x, by + bh - 1)].set_char('═');
                buf[(x, by + bh - 1)].set_fg(border_color);
            }
        }
        for y in by..by + bh {
            if bx < area.width {
                buf[(bx, y)].set_char('║');
                buf[(bx, y)].set_fg(border_color);
            }
            if bx + bw - 1 < area.width {
                buf[(bx + bw - 1, y)].set_char('║');
                buf[(bx + bw - 1, y)].set_fg(border_color);
            }
        }
        // Corners
        if bx < area.width && by < area.height {
            buf[(bx, by)].set_char('╔');
            buf[(bx, by)].set_fg(border_color);
        }
        if bx + bw - 1 < area.width && by < area.height {
            buf[(bx + bw - 1, by)].set_char('╗');
            buf[(bx + bw - 1, by)].set_fg(border_color);
        }
        if bx < area.width && by + bh - 1 < area.height {
            buf[(bx, by + bh - 1)].set_char('╚');
            buf[(bx, by + bh - 1)].set_fg(border_color);
        }
        if bx + bw - 1 < area.width && by + bh - 1 < area.height {
            buf[(bx + bw - 1, by + bh - 1)].set_char('╝');
            buf[(bx + bw - 1, by + bh - 1)].set_fg(border_color);
        }

        // Title
        let title = "═══ BRIDGE ═══";
        let tx = bx + (bw - title.len() as u16) / 2;
        for (i, ch) in title.chars().enumerate() {
            let x = tx + i as u16;
            if x < area.width && by < area.height {
                buf[(x, by)].set_char(ch);
                buf[(x, by)].set_fg(Color::Cyan);
            }
        }

        // Floor
        let floor_y = by + bh - 3;
        for x in bx + 1..bx + bw - 1 {
            if x < area.width && floor_y < area.height {
                buf[(x, floor_y)].set_char('▓');
                buf[(x, floor_y)].set_fg(Color::Rgb(40, 40, 50));
            }
        }

        // Bridge objects
        for obj in BRIDGE_OBJECTS {
            let ox = bx + 2 + obj.x;
            let oy = floor_y - obj.art.len() as u16;
            for (row, line) in obj.art.iter().enumerate() {
                for (col, ch) in line.chars().enumerate() {
                    let x = ox + col as u16;
                    let y = oy + row as u16;
                    if x < bx + bw - 1 && y < area.height && y > by {
                        buf[(x, y)].set_char(ch);
                        buf[(x, y)].set_fg(Color::Rgb(80, 80, 100));
                    }
                }
            }
        }

        // Feeding bowl
        let bowl_x = bx + 8;
        let bowl_y = floor_y - 1;
        let bowl = if self.hunger < 50 { "◇·" } else { "◇◇" };
        for (i, ch) in bowl.chars().enumerate() {
            let x = bowl_x + i as u16;
            if x < bx + bw - 1 && bowl_y < area.height && bowl_y > by {
                buf[(x, bowl_y)].set_char(ch);
                buf[(x, bowl_y)].set_fg(Color::Yellow);
            }
        }

        // ── Render Pip ──
        let px = bx + 2 + self.pip_x as u16;
        let py = floor_y - 1 - self.bounce_frame as u16;

        let is_blinking = self.blink_timer > 0;
        let face = self.pip_mood.face(is_blinking);
        let deco = self.pip_mood.decoration();
        let color = self.pip_mood.color();

        // Face
        for (i, ch) in face.chars().enumerate() {
            let x = px + i as u16;
            if x < bx + bw - 1 && py < area.height && py > by {
                buf[(x, py)].set_char(ch);
                buf[(x, py)].set_fg(color);
            }
        }

        // Decoration (after face)
        let deco_x = px + face.chars().count() as u16;
        for (i, ch) in deco.chars().enumerate() {
            let x = deco_x + i as u16;
            if x < bx + bw - 1 && py < area.height && py > by {
                buf[(x, py)].set_char(ch);
                buf[(x, py)].set_fg(Color::Yellow);
            }
        }

        // Feet
        let feet_y = py + 1;
        let feet = match self.pip_mood {
            Mood::Sleeping => "__",
            Mood::Dancing => match self.dance_frame {
                0 => " ╵╵",
                1 => "╵ ╵",
                2 => "╵╵ ",
                _ => " ╵╵",
            },
            _ => match self.walk_frame {
                0 if (self.pip_target_x - self.pip_x).abs() > 1.0 => "╵ ╵",
                2 if (self.pip_target_x - self.pip_x).abs() > 1.0 => " ╵╵",
                _ => " ╵╵",
            },
        };
        let feet_x = px + 1; // center under face
        for (i, ch) in feet.chars().enumerate() {
            let x = feet_x + i as u16;
            if x < bx + bw - 1 && feet_y < area.height && feet_y > by {
                buf[(x, feet_y)].set_char(ch);
                buf[(x, feet_y)].set_fg(color);
            }
        }

        // Speech bubble
        if let Some(ref text) = self.speech {
            let bubble_y = py.saturating_sub(2);
            let bubble = format!("「{}」", text);
            let bubble_x = px;
            for (i, ch) in bubble.chars().enumerate() {
                let x = bubble_x + i as u16;
                if x < bx + bw - 1 && bubble_y < area.height && bubble_y > by {
                    buf[(x, bubble_y)].set_char(ch);
                    buf[(x, bubble_y)].set_fg(Color::White);
                }
            }
        }

        // Stat bars at bottom
        let stat_y = by + bh - 2;
        let stats_str = format!(
            " Mood: {}  │  Hunger: {}  │  Energy: {}  │  Bond: {}  │  [F]eed [P]et [Esc]Close ",
            self.mood_label(),
            Self::bar(self.hunger),
            Self::bar(self.energy),
            self.bond_label(),
        );
        for (i, ch) in stats_str.chars().enumerate() {
            let x = bx + 1 + i as u16;
            if x < bx + bw - 1 && stat_y < area.height {
                buf[(x, stat_y)].set_char(ch);
                buf[(x, stat_y)].set_fg(Color::DarkGray);
                buf[(x, stat_y)].set_bg(Color::Rgb(20, 20, 30));
            }
        }
    }

    fn mood_label(&self) -> &'static str {
        match self.pip_mood {
            Mood::Idle => "Chill",
            Mood::Happy => "Happy",
            Mood::Sleeping => "Zzz",
            Mood::Eating => "Nom",
            Mood::Sad => "Sad",
            Mood::Excited => "Wow!",
            Mood::Dancing => "Dance",
            Mood::Lonely => "Lonely",
        }
    }

    fn bond_label(&self) -> &'static str {
        match self.bond {
            0..=50 => "Stranger",
            51..=150 => "Acquaintance",
            151..=350 => "Friend",
            351..=600 => "Best Friend",
            601..=850 => "Soulmate",
            _ => "Bonded ♥",
        }
    }

    fn bar(value: u8) -> String {
        let filled = (value as usize) / 10;
        let empty = 10 - filled;
        let color = if value > 60 {
            "█"
        } else if value > 30 {
            "▓"
        } else {
            "░"
        };
        format!("{}{}", color.repeat(filled), "·".repeat(empty))
    }
}
