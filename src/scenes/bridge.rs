use crossterm::event::KeyCode;
use ratatui::style::Color;
use ratatui::Frame;

use crate::engine::events::GameEvent;
use crate::state::GameState;

// ── Pip's emotional states ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mood {
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
    pub gift_shop_open: bool,
    gift_shop_cursor: u8,

    // Pip state (animation only — stats live in GameState)
    pip_x: f32,
    pip_target_x: f32,
    pip_mood: Mood,
    pip_mood_timer: u16,

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
            gift_shop_open: false,
            gift_shop_cursor: 0,
            pip_x: 20.0,
            pip_target_x: 20.0,
            pip_mood: Mood::Idle,
            pip_mood_timer: 0,
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

    pub fn toggle(&mut self, state: &mut GameState) {
        self.open = !self.open;
        self.gift_shop_open = false;
        if self.open {
            // Pip reacts to being visited
            if self.idle_ticks > 2000 {
                self.pip_mood = Mood::Lonely;
                self.pip_mood_timer = 40;
                self.say("...you came back!");
            } else {
                self.pip_mood = Mood::Happy;
                self.pip_mood_timer = 30;
                self.say("Hi!");
            }
            self.idle_ticks = 0;
            state.pip_bond = state.pip_bond.saturating_add(1).min(1000);
        }
    }

    fn say(&mut self, text: &str) {
        self.speech = Some(text.to_string());
        self.speech_timer = 40;
    }

    /// Notify Pip of game events (called from main loop)
    pub fn notify_battle_win(&mut self, state: &mut GameState) {
        state.pip_happiness = state.pip_happiness.saturating_add(10).min(100);
        state.add_pip_xp(10);
        self.pip_mood = Mood::Excited;
        self.pip_mood_timer = 40;
    }

    pub fn notify_battle_loss(&mut self, state: &mut GameState) {
        state.pip_happiness = state.pip_happiness.saturating_sub(15);
        self.pip_mood = Mood::Sad;
        self.pip_mood_timer = 60;
    }

    pub fn notify_loot(&mut self) {
        if self.pip_mood != Mood::Sad {
            self.pip_mood = Mood::Happy;
            self.pip_mood_timer = 20;
        }
    }

    pub fn notify_achievement(&mut self, state: &mut GameState) {
        state.add_pip_xp(20);
        self.pip_mood = Mood::Dancing;
        self.pip_mood_timer = 60;
    }

    /// React to game events with deeper mood changes and Pip stat effects.
    /// Called from process_events for richer emotional responses.
    pub fn react_to_event(&mut self, event: &GameEvent, state: &mut GameState) {
        match event {
            GameEvent::BattleWon { was_boss, fleet_hp_pct, .. } => {
                state.pip_happiness = state.pip_happiness.saturating_add(10).min(100);
                state.add_pip_xp(10);
                if *was_boss {
                    self.pip_mood = Mood::Dancing;
                    self.pip_mood_timer = 200;
                    self.say("BOSS DOWN!!");
                } else if *fleet_hp_pct > 0.9 {
                    self.pip_mood = Mood::Excited;
                    self.pip_mood_timer = 100;
                } else if *fleet_hp_pct < 0.3 {
                    // Close call — happy but shaken
                    self.pip_mood = Mood::Happy;
                    self.pip_mood_timer = 60;
                } else {
                    self.pip_mood = Mood::Excited;
                    self.pip_mood_timer = 100;
                }
            }
            GameEvent::BattleLost { .. } => {
                state.pip_happiness = state.pip_happiness.saturating_sub(15);
                self.pip_mood = Mood::Sad;
                self.pip_mood_timer = 300;
            }
            GameEvent::EquipmentDropped { rarity, .. } => {
                match rarity.as_str() {
                    "Legendary" => {
                        self.pip_mood = Mood::Excited;
                        self.pip_mood_timer = 150;
                        self.say("LEGENDARY?!");
                    }
                    "Epic" => {
                        self.pip_mood = Mood::Excited;
                        self.pip_mood_timer = 80;
                    }
                    _ => {
                        if self.pip_mood != Mood::Sad {
                            self.pip_mood = Mood::Happy;
                            self.pip_mood_timer = 20;
                        }
                    }
                }
            }
            GameEvent::AchievementUnlocked { .. } => {
                state.add_pip_xp(20);
                self.pip_mood = Mood::Dancing;
                self.pip_mood_timer = 60;
            }
            GameEvent::CrewGrief { fallen, .. } => {
                self.pip_mood = Mood::Sad;
                self.pip_mood_timer = 400;
                self.say(&format!("{}...", fallen));
            }
            GameEvent::PrestigeCompleted { .. } => {
                self.pip_mood = Mood::Dancing;
                self.pip_mood_timer = 200;
                self.say("A new beginning!");
            }
            _ => {}
        }
    }

    /// Check resource-based mood — called each tick for ambient reactions.
    fn check_resource_mood(&mut self, state: &GameState) {
        // Only trigger if no active mood override
        if self.pip_mood_timer > 0 {
            return;
        }
        if state.scrap < 50 && self.pip_mood == Mood::Idle {
            self.say("We're running low...");
            self.pip_mood = Mood::Sad;
            self.pip_mood_timer = 60;
        }
    }

    pub fn handle_input(&mut self, key: KeyCode, state: &mut GameState) {
        // Gift shop sub-menu
        if self.gift_shop_open {
            match key {
                KeyCode::Up => {
                    self.gift_shop_cursor = self.gift_shop_cursor.saturating_sub(1);
                }
                KeyCode::Down => {
                    self.gift_shop_cursor = (self.gift_shop_cursor + 1).min(3);
                }
                KeyCode::Enter => {
                    self.try_buy_gift(state);
                }
                KeyCode::Esc | KeyCode::Char('g') | KeyCode::Char('G') => {
                    self.gift_shop_open = false;
                }
                _ => {}
            }
            return;
        }

        match key {
            KeyCode::Char('f') | KeyCode::Char('F') => {
                // Feed Pip — costs 10 scrap
                if state.scrap >= 10 && state.pip_hunger < 90 {
                    state.scrap -= 10;
                    state.pip_hunger = state.pip_hunger.saturating_add(25).min(100);
                    state.pip_happiness = state.pip_happiness.saturating_add(5).min(100);
                    state.pip_bond = state.pip_bond.saturating_add(2).min(1000);
                    state.add_pip_xp(5);
                    self.pip_mood = Mood::Eating;
                    self.pip_mood_timer = 30;
                    self.say("Yum!");
                } else if state.pip_hunger >= 90 {
                    self.say("I'm full!");
                } else {
                    self.say("No scrap...");
                    self.pip_mood = Mood::Sad;
                    self.pip_mood_timer = 20;
                }
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                // Pet/play with Pip
                state.pip_happiness = state.pip_happiness.saturating_add(10).min(100);
                state.pip_bond = state.pip_bond.saturating_add(3).min(1000);
                state.add_pip_xp(3);
                self.pip_mood = Mood::Happy;
                self.pip_mood_timer = 40;
                let reactions = ["Hehe!", ":D", "Wheee!", "♥", "Yay!"];
                let idx = (self.tick_count as usize) % reactions.len();
                self.say(reactions[idx]);
            }
            KeyCode::Char('g') | KeyCode::Char('G') => {
                self.gift_shop_open = true;
                self.gift_shop_cursor = 0;
            }
            KeyCode::Esc => self.toggle(state),
            _ => {}
        }
    }

    fn try_buy_gift(&mut self, state: &mut GameState) {
        let (required_level, cost, appearance, name): (u8, u64, u8, &str) =
            match self.gift_shop_cursor {
                0 => (3, 200, 1, "Antenna"),
                1 => (5, 500, 2, "Visor"),
                2 => (7, 1000, 3, "Wings"),
                3 => (9, 2500, 4, "Crown"),
                _ => return,
            };

        if state.pip_appearance >= appearance {
            self.say("Already have that!");
            return;
        }
        if state.pip_level < required_level {
            self.say(&format!("Need Lv.{}!", required_level));
            return;
        }
        if state.credits < cost {
            self.say("Not enough ₿!");
            return;
        }

        state.credits -= cost;
        state.pip_appearance = appearance;
        self.pip_mood = Mood::Excited;
        self.pip_mood_timer = 60;
        self.say(&format!("{} equipped!", name));
    }

    pub fn tick(&mut self, state: &mut GameState) {
        if !self.open {
            self.idle_ticks += 1;
            // Pip's needs still decay even when not viewing
            if self.tick_count.is_multiple_of(200) {
                state.pip_hunger = state.pip_hunger.saturating_sub(1);
                state.pip_energy = state.pip_energy.saturating_sub(1);
            }
            self.tick_count += 1;
            return;
        }

        self.tick_count += 1;

        // Passive XP: 1 XP per 100 ticks when happy
        if self.tick_count.is_multiple_of(100) && state.pip_happiness > 50 {
            state.add_pip_xp(1);
        }

        // Stat decay
        if self.tick_count.is_multiple_of(100) {
            state.pip_hunger = state.pip_hunger.saturating_sub(1);
        }
        if self.tick_count.is_multiple_of(150) {
            state.pip_energy = state.pip_energy.saturating_sub(1);
        }

        // Energy recovery when sleeping
        if self.pip_mood == Mood::Sleeping
            && self.tick_count.is_multiple_of(20) {
                state.pip_energy = state.pip_energy.saturating_add(2).min(100);
            }

        // Auto-mood based on stats
        if self.pip_mood_timer > 0 {
            self.pip_mood_timer -= 1;
        } else {
            self.pip_mood = if state.pip_energy < 20 {
                Mood::Sleeping
            } else if state.pip_hunger < 20 {
                Mood::Sad
            } else if state.pip_happiness > 80 {
                Mood::Happy
            } else {
                Mood::Idle
            };
        }

        // Check resource-based mood (every 200 ticks)
        if self.tick_count.is_multiple_of(200) {
            self.check_resource_mood(state);
        }

        // Blink every ~80 ticks
        if self.blink_timer > 0 {
            self.blink_timer -= 1;
        } else if self.tick_count.is_multiple_of(80) && self.pip_mood != Mood::Sleeping {
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

    /// Get Pip's face based on level evolution and current mood
    fn evolved_face(&self, state: &GameState, blink: bool) -> String {
        if blink {
            return match state.pip_level {
                1..=2 => "(- -)".into(),
                3..=4 => "(- -)^".into(),
                5..=6 => "[- -]".into(),
                7..=8 => "{- -}~".into(),
                _ => "{- -}♛".into(),
            };
        }
        let base = self.pip_mood.face(false);
        match state.pip_level {
            1..=2 => base.into(),
            3..=4 => format!("{}^", base),
            5..=6 => {
                // Replace parens with square brackets
                let inner = base.trim_start_matches('(').trim_end_matches(')');
                format!("[{}]", inner)
            }
            7..=8 => {
                let inner = base.trim_start_matches('(').trim_end_matches(')');
                format!("{{{}}}~", inner)
            }
            _ => {
                let inner = base.trim_start_matches('(').trim_end_matches(')');
                format!("{{{}}}♛", inner)
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
        let bowl = if _state.pip_hunger < 50 { "◇·" } else { "◇◇" };
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
        let face = self.evolved_face(_state, is_blinking);
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

        // Pip level indicator
        let level_str = format!("Lv.{} ({}/{}XP)", _state.pip_level, _state.pip_xp, _state.pip_xp_to_next());
        let level_x = bx + bw - 2 - level_str.len() as u16;
        let level_y = by + 1;
        for (i, ch) in level_str.chars().enumerate() {
            let x = level_x + i as u16;
            if x < bx + bw - 1 && level_y < area.height {
                buf[(x, level_y)].set_char(ch);
                buf[(x, level_y)].set_fg(Color::Magenta);
            }
        }

        // Stat bars at bottom
        let stat_y = by + bh - 2;
        let stats_str = format!(
            " {}  │  Hunger: {}  │  Energy: {}  │  Bond: {}  │  [F]eed [P]et [G]ifts [Esc] ",
            self.mood_label(),
            Self::bar(_state.pip_hunger),
            Self::bar(_state.pip_energy),
            Self::bond_label(_state.pip_bond),
        );
        for (i, ch) in stats_str.chars().enumerate() {
            let x = bx + 1 + i as u16;
            if x < bx + bw - 1 && stat_y < area.height {
                buf[(x, stat_y)].set_char(ch);
                buf[(x, stat_y)].set_fg(Color::DarkGray);
                buf[(x, stat_y)].set_bg(Color::Rgb(20, 20, 30));
            }
        }

        // Gift shop overlay
        if self.gift_shop_open {
            self.render_gift_shop(frame, _state, bx, by, bw, bh);
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

    fn bond_label(bond: u16) -> &'static str {
        match bond {
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

    fn render_gift_shop(&self, frame: &mut Frame, state: &GameState, bx: u16, by: u16, bw: u16, bh: u16) {
        let buf = frame.buffer_mut();
        let area = ratatui::layout::Rect::new(0, 0, buf.area.width, buf.area.height);

        // Gift shop panel (centered overlay)
        let sw: u16 = 30;
        let sh: u16 = 14;
        let sx = bx + (bw.saturating_sub(sw)) / 2;
        let sy = by + (bh.saturating_sub(sh)) / 2;

        // Clear shop area
        for y in sy..sy + sh {
            for x in sx..sx + sw {
                if x < area.width && y < area.height {
                    let cell = &mut buf[(x, y)];
                    cell.set_char(' ');
                    cell.set_bg(Color::Rgb(10, 10, 20));
                    cell.set_fg(Color::White);
                }
            }
        }

        // Border
        let border_fg = Color::Rgb(120, 80, 200);
        for x in sx..sx + sw {
            if sy < area.height { buf[(x, sy)].set_char('═'); buf[(x, sy)].set_fg(border_fg); }
            if sy + sh - 1 < area.height { buf[(x, sy + sh - 1)].set_char('═'); buf[(x, sy + sh - 1)].set_fg(border_fg); }
        }
        for y in sy..sy + sh {
            if sx < area.width { buf[(sx, y)].set_char('║'); buf[(sx, y)].set_fg(border_fg); }
            if sx + sw - 1 < area.width { buf[(sx + sw - 1, y)].set_char('║'); buf[(sx + sw - 1, y)].set_fg(border_fg); }
        }
        if sx < area.width && sy < area.height { buf[(sx, sy)].set_char('╔'); buf[(sx, sy)].set_fg(border_fg); }
        if sx + sw - 1 < area.width && sy < area.height { buf[(sx + sw - 1, sy)].set_char('╗'); buf[(sx + sw - 1, sy)].set_fg(border_fg); }
        if sx < area.width && sy + sh - 1 < area.height { buf[(sx, sy + sh - 1)].set_char('╚'); buf[(sx, sy + sh - 1)].set_fg(border_fg); }
        if sx + sw - 1 < area.width && sy + sh - 1 < area.height { buf[(sx + sw - 1, sy + sh - 1)].set_char('╝'); buf[(sx + sw - 1, sy + sh - 1)].set_fg(border_fg); }

        // Title
        let title = "═══ PIP GIFTS ═══";
        let tx = sx + (sw - title.len() as u16) / 2;
        for (i, ch) in title.chars().enumerate() {
            let x = tx + i as u16;
            if x < area.width && sy < area.height {
                buf[(x, sy)].set_char(ch);
                buf[(x, sy)].set_fg(Color::Magenta);
            }
        }

        // Gift items
        struct GiftItem {
            name: &'static str,
            level: u8,
            cost: u64,
            bonus: &'static str,
            appearance: u8,
        }
        let gifts = [
            GiftItem { name: "Antenna", level: 3, cost: 200, bonus: "Ambush warnings", appearance: 1 },
            GiftItem { name: "Visor", level: 5, cost: 500, bonus: "+5% battle loot", appearance: 2 },
            GiftItem { name: "Wings", level: 7, cost: 1000, bonus: "+10% travel speed", appearance: 3 },
            GiftItem { name: "Crown", level: 9, cost: 2500, bonus: "+15% all bonuses", appearance: 4 },
        ];

        for (i, gift) in gifts.iter().enumerate() {
            let row_y = sy + 2 + (i as u16 * 3);
            let selected = i as u8 == self.gift_shop_cursor;
            let owned = state.pip_appearance >= gift.appearance;
            let can_buy = state.pip_level >= gift.level && state.credits >= gift.cost && !owned;

            let prefix = if selected { "▸ " } else { "  " };
            let status = if owned {
                " ✓".to_string()
            } else if state.pip_level < gift.level {
                format!(" (Lv.{})", gift.level)
            } else {
                String::new()
            };
            let line1 = format!("{}{}{}", prefix, gift.name, status);
            let line2 = format!("{}  {}₿ — {}", prefix, gift.cost, gift.bonus);

            let fg1 = if owned {
                Color::DarkGray
            } else if selected {
                Color::White
            } else {
                Color::Gray
            };
            let fg2 = if can_buy && selected {
                Color::Green
            } else {
                Color::Rgb(80, 80, 100)
            };

            for (ci, ch) in line1.chars().enumerate() {
                let x = sx + 2 + ci as u16;
                if x < sx + sw - 1 && row_y < area.height {
                    buf[(x, row_y)].set_char(ch);
                    buf[(x, row_y)].set_fg(fg1);
                }
            }
            if row_y + 1 < area.height {
                for (ci, ch) in line2.chars().enumerate() {
                    let x = sx + 2 + ci as u16;
                    if x < sx + sw - 1 && row_y + 1 < area.height {
                        buf[(x, row_y + 1)].set_char(ch);
                        buf[(x, row_y + 1)].set_fg(fg2);
                    }
                }
            }
        }
    }
}
