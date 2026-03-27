use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::engine::factions::{Faction, sector_faction};
use crate::rendering::layout::centered_rect;
use crate::state::GameState;

// ── DiplomacyScreen ─────────────────────────────────────────

pub struct DiplomacyScreen {
    pub open: bool,
    selected: usize,
}

impl DiplomacyScreen {
    pub fn new() -> Self {
        Self {
            open: false,
            selected: 0,
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.selected = 0;
        }
    }

    pub fn handle_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            KeyCode::Down => {
                let max = Faction::TRACKABLE.len().saturating_sub(1);
                if self.selected < max {
                    self.selected += 1;
                }
            }
            KeyCode::Esc => {
                self.open = false;
            }
            _ => {}
        }
    }

    pub fn render(&self, frame: &mut Frame, state: &GameState) {
        if !self.open {
            return;
        }

        let area = centered_rect(55, 85, frame.area());
        frame.render_widget(Clear, area);

        let current_faction = sector_faction(state.sector);

        let outer_block = Block::default()
            .title(" ◈ FACTIONS ◈ ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightBlue))
            .style(Style::default().bg(Color::Black));

        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        let mut lines: Vec<Line> = Vec::new();

        // Sector dominant faction header
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  Sector ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", state.sector),
                Style::default().fg(Color::White),
            ),
            Span::styled(" — Controlled by ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                current_faction.name(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));

        // Active mission banner
        if let Some(ref mission) = state.pending_faction_mission {
            let progress = format!("{} sectors remaining", mission.sectors_remaining);
            lines.push(Line::from(vec![
                Span::styled("  ⚡ Active Mission: ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    &mission.description,
                    Style::default().fg(Color::White),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled(
                    format!(
                        "    {} │ Reward: {}₿ │ +{} rep",
                        progress, mission.reward_credits, mission.rep_reward
                    ),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
            lines.push(Line::from(""));
        }

        // Each faction (TRACKABLE excludes Independent)
        for (i, &faction) in Faction::TRACKABLE.iter().enumerate() {
            let rep = state.faction_reputation.get(faction);
            let tier = state.faction_reputation.tier(faction);
            let is_selected = i == self.selected;
            let is_dominant = faction == current_faction;

            // Header line: icon + name + code
            let highlight = if is_selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Rgb(180, 180, 200))
            };

            let mut header_spans = vec![
                Span::styled(
                    if is_selected { "▶ " } else { "  " },
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    format!("{} ", faction.icon()),
                    Style::default().fg(tier.color()),
                ),
                Span::styled(faction.name().to_string(), highlight),
            ];
            if is_dominant {
                header_spans.push(Span::styled(
                    " ◀",
                    Style::default().fg(Color::Yellow),
                ));
            }
            // Right-align code
            let used_len = 2 + faction.icon().len() + 1 + faction.name().len()
                + if is_dominant { 2 } else { 0 };
            let inner_width = inner.width as usize;
            let pad = inner_width.saturating_sub(used_len + faction.code().len() + 2);
            header_spans.push(Span::raw(" ".repeat(pad)));
            header_spans.push(Span::styled(
                faction.code().to_string(),
                Style::default().fg(Color::DarkGray),
            ));

            lines.push(Line::from(header_spans));

            // Reputation bar
            let bar = reputation_bar(rep, tier.color());
            lines.push(Line::from(vec![
                Span::raw("  Reputation: "),
                bar,
                Span::styled(
                    format!(" {:+}", rep),
                    Style::default().fg(tier.color()),
                ),
            ]));

            // Status
            lines.push(Line::from(vec![
                Span::styled("  Status: ", Style::default().fg(Color::DarkGray)),
                Span::styled(tier.name().to_string(), Style::default().fg(tier.color())),
            ]));

            // Description (only for selected)
            if is_selected {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {}", faction.description()),
                        Style::default().fg(Color::Rgb(120, 120, 140)),
                    ),
                ]));

                // Rivals
                let rivals = faction.rivals();
                if !rivals.is_empty() {
                    let rival_names: Vec<&str> = rivals.iter().map(|r| r.name()).collect();
                    lines.push(Line::from(vec![
                        Span::styled("  Rivals: ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            rival_names.join(", "),
                            Style::default().fg(Color::Rgb(200, 100, 80)),
                        ),
                    ]));
                }
            }

            lines.push(Line::from(""));
        }

        // Footer
        lines.push(Line::from(vec![
            Span::styled("        [", Style::default().fg(Color::DarkGray)),
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
            Span::styled("] Browse  [", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::styled("] Close", Style::default().fg(Color::DarkGray)),
        ]));

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }
}

// ── Helpers ─────────────────────────────────────────────────

/// Build a reputation bar: 10 chars, filled based on 0-100 mapped from -100..+100.
fn reputation_bar(rep: i32, color: Color) -> Span<'static> {
    // Map -100..+100 to 0..10
    let normalized = ((rep + 100) as f32 / 200.0 * 10.0).round() as usize;
    let filled = normalized.min(10);
    let empty = 10 - filled;

    let bar_filled = "█".repeat(filled);
    let bar_empty = "░".repeat(empty);
    let bar = format!("{}{}", bar_filled, bar_empty);

    Span::styled(bar, Style::default().fg(color))
}
