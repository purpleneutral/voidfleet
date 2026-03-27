use crossterm::event::KeyCode;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::rendering::layout::centered_rect;

pub struct HelpScreen {
    pub open: bool,
    scroll: u16,
}

impl HelpScreen {
    pub fn new() -> Self {
        Self {
            open: false,
            scroll: 0,
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
        self.scroll = 0;
    }

    pub fn handle_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc | KeyCode::Char('?') => self.toggle(),
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll = self.scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll += 1;
            }
            KeyCode::PageUp => {
                self.scroll = self.scroll.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.scroll += 10;
            }
            KeyCode::Home => {
                self.scroll = 0;
            }
            _ => {}
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = centered_rect(70, 85, frame.area());

        // Clear background
        frame.render_widget(Clear, area);

        let header = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let key_style = Style::default().fg(Color::Yellow);
        let desc = Style::default().fg(Color::Gray);
        let section = Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD);

        let lines = vec![
            Line::from(vec![
                Span::styled("  ── ", desc),
                Span::styled("GAMEPLAY", section),
                Span::styled(" ──", desc),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Space    ", key_style),
                Span::styled("Skip current phase", desc),
            ]),
            Line::from(vec![
                Span::styled("  S        ", key_style),
                Span::styled("Manual save", desc),
            ]),
            Line::from(vec![
                Span::styled("  Q / Esc  ", key_style),
                Span::styled("Quit game (auto-saves)", desc),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ── ", desc),
                Span::styled("SCREENS", section),
                Span::styled(" ──", desc),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  U        ", key_style),
                Span::styled("Upgrades — buy ships, upgrade tech", desc),
            ]),
            Line::from(vec![
                Span::styled("  I        ", key_style),
                Span::styled("Inventory — equipment, equip, salvage", desc),
            ]),
            Line::from(vec![
                Span::styled("  C        ", key_style),
                Span::styled("Crew — roster, assign to ships, recruit", desc),
            ]),
            Line::from(vec![
                Span::styled("  B        ", key_style),
                Span::styled("Bridge — visit Pip, feed, pet, gifts", desc),
            ]),
            Line::from(vec![
                Span::styled("  M        ", key_style),
                Span::styled("Sector Map — choose routes", desc),
            ]),
            Line::from(vec![
                Span::styled("  F        ", key_style),
                Span::styled("Factions — diplomacy, reputation", desc),
            ]),
            Line::from(vec![
                Span::styled("  T        ", key_style),
                Span::styled("Trade — buy/sell goods at market", desc),
            ]),
            Line::from(vec![
                Span::styled("  J        ", key_style),
                Span::styled("Journal — missions, contracts", desc),
            ]),
            Line::from(vec![
                Span::styled("  L        ", key_style),
                Span::styled("Log — scrollable event history", desc),
            ]),
            Line::from(vec![
                Span::styled("  Tab      ", key_style),
                Span::styled("Stats — lifetime statistics", desc),
            ]),
            Line::from(vec![
                Span::styled("  ?        ", key_style),
                Span::styled("This help screen", desc),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ── ", desc),
                Span::styled("UPGRADES SCREEN", section),
                Span::styled(" ──", desc),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ↑ ↓      ", key_style),
                Span::styled("Navigate items", desc),
            ]),
            Line::from(vec![
                Span::styled("  ← → Tab  ", key_style),
                Span::styled("Switch tabs (Ships / Tech / Fleet)", desc),
            ]),
            Line::from(vec![
                Span::styled("  Enter    ", key_style),
                Span::styled("Buy / upgrade selected item", desc),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ── ", desc),
                Span::styled("INVENTORY", section),
                Span::styled(" ──", desc),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ↑ ↓      ", key_style),
                Span::styled("Navigate items", desc),
            ]),
            Line::from(vec![
                Span::styled("  ← → Tab  ", key_style),
                Span::styled("Switch tabs (Fleet / Inventory / Salvage)", desc),
            ]),
            Line::from(vec![
                Span::styled("  Enter    ", key_style),
                Span::styled("Equip item to ship", desc),
            ]),
            Line::from(vec![
                Span::styled("  D        ", key_style),
                Span::styled("Salvage item for scrap", desc),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ── ", desc),
                Span::styled("CREW", section),
                Span::styled(" ──", desc),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ↑ ↓      ", key_style),
                Span::styled("Navigate crew members", desc),
            ]),
            Line::from(vec![
                Span::styled("  Tab      ", key_style),
                Span::styled("Switch tabs (Roster / Assign / Recruit)", desc),
            ]),
            Line::from(vec![
                Span::styled("  Enter    ", key_style),
                Span::styled("Assign crew / recruit", desc),
            ]),
            Line::from(vec![
                Span::styled("  D        ", key_style),
                Span::styled("Dismiss / unassign crew", desc),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ── ", desc),
                Span::styled("BRIDGE (Pip)", section),
                Span::styled(" ──", desc),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  F        ", key_style),
                Span::styled("Feed Pip (costs 10 scrap)", desc),
            ]),
            Line::from(vec![
                Span::styled("  P        ", key_style),
                Span::styled("Pet Pip (builds bond)", desc),
            ]),
            Line::from(vec![
                Span::styled("  G        ", key_style),
                Span::styled("Gift shop (appearance upgrades)", desc),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ── ", desc),
                Span::styled("TRADE", section),
                Span::styled(" ──", desc),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ↑ ↓      ", key_style),
                Span::styled("Select trade good", desc),
            ]),
            Line::from(vec![
                Span::styled("  ← →      ", key_style),
                Span::styled("Adjust quantity", desc),
            ]),
            Line::from(vec![
                Span::styled("  Enter    ", key_style),
                Span::styled("Buy selected goods", desc),
            ]),
            Line::from(vec![
                Span::styled("  S        ", key_style),
                Span::styled("Sell selected goods", desc),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ── ", desc),
                Span::styled("TRAVEL EVENTS", section),
                Span::styled(" ──", desc),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ↑ ↓      ", key_style),
                Span::styled("Select option", desc),
            ]),
            Line::from(vec![
                Span::styled("  Enter    ", key_style),
                Span::styled("Confirm choice", desc),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ── ", desc),
                Span::styled("GAME LOOP", section),
                Span::styled(" ──", desc),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Travel   ", header),
                Span::styled("Cruise through space, collect scrap, events", desc),
            ]),
            Line::from(vec![
                Span::styled("  Battle   ", header),
                Span::styled("Fight enemy fleets, earn loot", desc),
            ]),
            Line::from(vec![
                Span::styled("  Raid     ", header),
                Span::styled("Harvest planet resources", desc),
            ]),
            Line::from(vec![
                Span::styled("  Loot     ", header),
                Span::styled("Collect rewards, advance sector", desc),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Reach sector 100 to complete a Voyage", desc),
            ]),
            Line::from(vec![
                Span::styled("  Each Voyage grants permanent bonuses", desc),
            ]),
            Line::from(""),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" HELP ")
            .title_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
            .border_style(Style::default().fg(Color::Rgb(60, 60, 80)));

        let paragraph = Paragraph::new(lines)
            .block(block)
            .scroll((self.scroll, 0))
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }
}
