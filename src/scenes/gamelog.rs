use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::rendering::layout::centered_rect;

// ── Log Entry ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub sector: u32,
    pub icon: char,
    pub title: String,
    pub details: Vec<String>,
    pub color: Color,
}

// ── GameLog Screen ──────────────────────────────────────────────────────────

pub struct GameLogScreen {
    pub open: bool,
    pub entries: Vec<LogEntry>,
    pub scroll: usize,
    pub max_entries: usize,
    /// Whether the user has scrolled up (disables auto-scroll)
    user_scrolled: bool,
}

impl GameLogScreen {
    pub fn new() -> Self {
        Self {
            open: false,
            entries: Vec::new(),
            scroll: 0,
            max_entries: 100,
            user_scrolled: false,
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            // Auto-scroll to bottom when opening
            self.scroll_to_bottom();
            self.user_scrolled = false;
        }
    }

    pub fn add_entries(&mut self, entries: Vec<LogEntry>) {
        if entries.is_empty() {
            return;
        }
        self.entries.extend(entries);
        while self.entries.len() > self.max_entries {
            self.entries.remove(0);
            // Adjust scroll position when old entries are removed
            self.scroll = self.scroll.saturating_sub(1);
        }
        if !self.user_scrolled {
            self.scroll_to_bottom();
        }
    }

    pub fn handle_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
                self.user_scrolled = true;
            }
            KeyCode::Down => {
                self.scroll = self.scroll.saturating_add(1);
                // If we scrolled to the bottom, re-enable auto-scroll
                let max = self.max_scroll();
                if self.scroll >= max {
                    self.scroll = max;
                    self.user_scrolled = false;
                }
            }
            KeyCode::PageUp => {
                self.scroll = self.scroll.saturating_sub(10);
                self.user_scrolled = true;
            }
            KeyCode::PageDown => {
                self.scroll = self.scroll.saturating_add(10);
                let max = self.max_scroll();
                if self.scroll >= max {
                    self.scroll = max;
                    self.user_scrolled = false;
                }
            }
            KeyCode::Home => {
                self.scroll = 0;
                self.user_scrolled = true;
            }
            KeyCode::End => {
                self.scroll_to_bottom();
                self.user_scrolled = false;
            }
            _ => {}
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        if !self.open {
            return;
        }

        let area = centered_rect(50, 85, frame.area());
        frame.render_widget(Clear, area);

        let outer_block = Block::default()
            .title(" ◈ SHIP LOG ◈ ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .style(Style::default().bg(Color::Black));

        let inner = outer_block.inner(area);

        // Build lines from entries
        let mut lines: Vec<Line> = Vec::new();

        if self.entries.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  No events recorded yet...",
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Events will appear here as you play.",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            let mut last_sector: Option<u32> = None;

            for entry in &self.entries {
                // Sector divider
                if last_sector != Some(entry.sector) {
                    if last_sector.is_some() {
                        lines.push(Line::from(""));
                    }
                    lines.push(Line::from(Span::styled(
                        format!("  ── Sector {} ──", entry.sector),
                        Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
                    )));
                    lines.push(Line::from(""));
                    last_sector = Some(entry.sector);
                }

                // Entry title with icon
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {} ", entry.icon),
                        Style::default().fg(entry.color),
                    ),
                    Span::styled(
                        entry.title.clone(),
                        Style::default().fg(entry.color).add_modifier(Modifier::BOLD),
                    ),
                ]));

                // Detail lines (indented)
                for detail in &entry.details {
                    lines.push(Line::from(Span::styled(
                        format!("    {}", detail),
                        Style::default().fg(Color::Gray),
                    )));
                }
            }
        }

        // Calculate scroll bounds
        let visible_height = inner.height as usize;
        let total_lines = lines.len();
        let max_scroll = total_lines.saturating_sub(visible_height);
        let scroll = self.scroll.min(max_scroll);

        let paragraph = Paragraph::new(lines)
            .scroll((scroll as u16, 0))
            .block(outer_block);

        frame.render_widget(paragraph, area);

        // Scrollbar
        if total_lines > visible_height {
            let scrollbar_area = ratatui::layout::Rect {
                x: area.x + area.width - 1,
                y: area.y + 1,
                width: 1,
                height: area.height.saturating_sub(2),
            };
            let mut scrollbar_state = ScrollbarState::new(max_scroll)
                .position(scroll);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("▲"))
                    .end_symbol(Some("▼"))
                    .track_symbol(Some("│"))
                    .thumb_symbol("█"),
                scrollbar_area,
                &mut scrollbar_state,
            );
        }

        // Entry count at bottom
        let count_text = format!(" {} entries ", self.entries.len());
        let count_x = area.x + area.width.saturating_sub(count_text.len() as u16 + 2);
        let count_y = area.y + area.height.saturating_sub(1);
        let frame_width = frame.area().width;
        let frame_height = frame.area().height;
        if count_y < frame_height && count_x < frame_width {
            let buf = frame.buffer_mut();
            for (i, ch) in count_text.chars().enumerate() {
                let x = count_x + i as u16;
                if x < frame_width {
                    let cell = &mut buf[(x, count_y)];
                    cell.set_char(ch);
                    cell.set_fg(Color::DarkGray);
                    cell.set_bg(Color::Black);
                }
            }
        }
    }

    fn scroll_to_bottom(&mut self) {
        // This will be clamped during render
        self.scroll = self.entries.len().saturating_mul(4); // generous overestimate
    }

    fn max_scroll(&self) -> usize {
        // Rough estimate: each entry ~2-4 lines + sector headers
        // Actual clamping happens in render, but this is for input logic
        self.entries.len().saturating_mul(4)
    }
}
