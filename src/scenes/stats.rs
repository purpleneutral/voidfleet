use ratatui::{
    Frame,
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::engine::achievements::ACHIEVEMENTS;
use crate::rendering::layout::centered_rect;
use crate::state::GameState;

// ── StatsScreen ─────────────────────────────────────────────────────────────

pub struct StatsScreen {
    pub open: bool,
}

impl StatsScreen {
    pub fn new() -> Self {
        Self { open: false }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn render(&self, frame: &mut Frame, state: &GameState) {
        if !self.open {
            return;
        }

        let area = centered_rect(50, 90, frame.area());

        // Clear background behind overlay
        frame.render_widget(Clear, area);

        let outer_block = Block::default()
            .title(" ◈ FLEET STATISTICS ◈ ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));

        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        let mut lines: Vec<Line> = Vec::new();

        // ── Progress ────────────────────────────────────────────────────
        lines.push(Line::from(""));
        lines.push(section_header("Progress"));
        lines.push(stat_row("Current Sector", &format_num(state.sector as u64)));
        lines.push(stat_row("Highest Sector", &format_num(state.highest_sector as u64)));
        lines.push(stat_row("Player Level", &format_num(state.level as u64)));
        lines.push(stat_row(
            "XP",
            &format!("{} / {}", format_num(state.xp), format_num(state.xp_to_next)),
        ));
        lines.push(stat_row("Time Played", &format_duration(state.time_played_secs)));

        // ── Resources ───────────────────────────────────────────────────
        lines.push(Line::from(""));
        lines.push(section_header("Resources"));
        lines.push(resource_row("Scrap", state.scrap));
        lines.push(resource_row("Credits", state.credits));
        lines.push(resource_row("Blueprints", state.blueprints));
        lines.push(resource_row("Artifacts", state.artifacts));

        // ── Combat ──────────────────────────────────────────────────────
        lines.push(Line::from(""));
        lines.push(section_header("Combat"));
        lines.push(stat_row("Battles Won", &format_num(state.total_battles)));
        lines.push(stat_row("Raids Completed", &format_num(state.total_raids)));
        lines.push(stat_row("Enemies Destroyed", &format_num(state.enemies_destroyed)));
        lines.push(stat_row("Deaths", &format_num(state.deaths)));

        // ── Fleet ───────────────────────────────────────────────────────
        lines.push(Line::from(""));
        lines.push(section_header("Fleet"));
        lines.push(stat_row("Ships", &format_num(state.fleet.len() as u64)));
        lines.push(stat_row(
            "Fleet HP",
            &format!(
                "{} / {}",
                format_num(state.fleet_total_hp() as u64),
                format_num(state.fleet_max_hp() as u64),
            ),
        ));
        lines.push(stat_row(
            "Fleet DPS",
            &format!("{:.1}", state.fleet_total_dps()),
        ));

        // ── Tech ────────────────────────────────────────────────────────
        lines.push(Line::from(""));
        lines.push(section_header("Tech"));
        lines.push(tech_bar("Lasers", state.tech_lasers));
        lines.push(tech_bar("Shields", state.tech_shields));
        lines.push(tech_bar("Engines", state.tech_engines));
        lines.push(tech_bar("Beams", state.tech_beams));

        // ── Achievements ────────────────────────────────────────────────
        lines.push(Line::from(""));
        lines.push(section_header("Achievements"));
        render_achievements(&mut lines, state);

        // ── Footer ──────────────────────────────────────────────────────
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("          [", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::styled("] Close", Style::default().fg(Color::DarkGray)),
        ]));

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Format a number with comma separators: 12340 → "12,340"
fn format_num(n: u64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result
}

/// Format seconds into "Xh Ym" or "Ym Zs"
fn format_duration(secs: u64) -> String {
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let s = secs % 60;
    if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else if mins > 0 {
        format!("{}m {}s", mins, s)
    } else {
        format!("{}s", s)
    }
}

/// Section header line: "  ── Header ──"
fn section_header(name: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!("  ── {} ──", name),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ))
}

/// Stat row with white value — all owned data to avoid lifetime issues
fn stat_row(label: &str, value: &str) -> Line<'static> {
    let padded_label = format!("  {:<20}", format!("{}:", label));
    Line::from(vec![
        Span::styled(padded_label, Style::default().fg(Color::DarkGray)),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ])
}

/// Resource row with yellow value
fn resource_row(label: &str, value: u64) -> Line<'static> {
    let padded_label = format!("  {:<20}", format!("{}:", label));
    Line::from(vec![
        Span::styled(padded_label, Style::default().fg(Color::DarkGray)),
        Span::styled(format_num(value), Style::default().fg(Color::Yellow)),
    ])
}

/// Tech progress bar: "  Lasers:     ████░░░░░░  Lv.4"
fn tech_bar(name: &str, level: u8) -> Line<'static> {
    let max_level: u8 = 10;
    let filled = level.min(max_level) as usize;
    let empty = (max_level as usize).saturating_sub(filled);

    let padded_label = format!("  {:<12}", format!("{}:", name));
    let bar_filled = "█".repeat(filled);
    let bar_empty = "░".repeat(empty);

    Line::from(vec![
        Span::styled(padded_label, Style::default().fg(Color::DarkGray)),
        Span::styled(bar_filled, Style::default().fg(Color::Green)),
        Span::styled(bar_empty, Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("  Lv.{}", level),
            Style::default().fg(Color::White),
        ),
    ])
}

/// Render achievements: unlocked bright with icon, locked dim
fn render_achievements(lines: &mut Vec<Line<'static>>, state: &GameState) {
    // Build pairs of achievements per line for compact display
    let mut spans_row: Vec<Span<'static>> = Vec::new();
    spans_row.push(Span::raw("  ".to_string()));

    let mut col = 0;
    for achievement in ACHIEVEMENTS.iter() {
        let unlocked = state.achievements_unlocked.contains(&achievement.id.to_string());

        let entry = if unlocked {
            Span::styled(
                format!("{} {}  ", achievement.icon, achievement.name),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                format!("{} {}  ", achievement.icon, achievement.name),
                Style::default().fg(Color::DarkGray),
            )
        };

        spans_row.push(entry);
        col += 1;

        // Two achievements per line
        if col >= 2 {
            lines.push(Line::from(std::mem::take(&mut spans_row)));
            spans_row.push(Span::raw("  ".to_string()));
            col = 0;
        }
    }
    // Flush remaining
    if col > 0 {
        lines.push(Line::from(spans_row));
    }

    if ACHIEVEMENTS.is_empty() {
        lines.push(Line::from(Span::styled(
            "  None yet",
            Style::default().fg(Color::DarkGray),
        )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_num() {
        assert_eq!(format_num(0), "0");
        assert_eq!(format_num(42), "42");
        assert_eq!(format_num(999), "999");
        assert_eq!(format_num(1_000), "1,000");
        assert_eq!(format_num(12_340), "12,340");
        assert_eq!(format_num(1_234_567), "1,234,567");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0), "0s");
        assert_eq!(format_duration(45), "45s");
        assert_eq!(format_duration(125), "2m 5s");
        assert_eq!(format_duration(3661), "1h 1m");
    }
}
