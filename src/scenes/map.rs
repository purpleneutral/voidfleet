use crossterm::event::KeyCode;
use rand::Rng;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::state::GameState;

// ── Route types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RouteBranch {
    pub name: &'static str,
    pub difficulty: f32, // multiplier: 0.7–1.5
    pub loot: f32,       // multiplier: 0.5–2.0
    pub flavor: &'static str,
    pub color: Color,
}

const BRANCH_TYPES: &[RouteBranch] = &[
    RouteBranch {
        name: "Nebula",
        difficulty: 1.4,
        loot: 1.8,
        flavor: "Dense nebula clouds — harder enemies, better loot",
        color: Color::Magenta,
    },
    RouteBranch {
        name: "Asteroid Belt",
        difficulty: 1.0,
        loot: 1.2,
        flavor: "Moderate danger, rich salvage opportunities",
        color: Color::Yellow,
    },
    RouteBranch {
        name: "Void Corridor",
        difficulty: 0.7,
        loot: 0.6,
        flavor: "Empty space — easy sailing, sparse pickings",
        color: Color::DarkGray,
    },
    RouteBranch {
        name: "Pirate Territory",
        difficulty: 1.5,
        loot: 1.5,
        flavor: "Combat-heavy zone, good bounties",
        color: Color::Red,
    },
    RouteBranch {
        name: "Trading Lane",
        difficulty: 0.8,
        loot: 1.4,
        flavor: "Busy trade route — more events, more credits",
        color: Color::Green,
    },
];

// ── Sector node types ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum SectorNode {
    /// A normal sector on the current path.
    Linear { sector: u32 },
    /// A branching choice point offering 2-3 routes.
    Branch {
        sector: u32,
        choices: Vec<RouteBranch>,
    },
}

// ── MapScreen ───────────────────────────────────────────────────────────

pub struct MapScreen {
    pub open: bool,
    /// The generated path of upcoming sector nodes.
    path: Vec<SectorNode>,
    /// Index of the active branch point in `path` (if any).
    active_branch: Option<usize>,
    /// Which choice is highlighted (0-indexed).
    cursor: usize,
}

impl MapScreen {
    pub fn new() -> Self {
        Self {
            open: false,
            path: Vec::new(),
            active_branch: None,
            cursor: 0,
        }
    }

    pub fn toggle(&mut self, state: &GameState) {
        self.open = !self.open;
        if self.open {
            self.generate_path(state);
        }
    }

    /// Generate upcoming sector path based on current sector.
    /// Branch points appear every 5 sectors.
    fn generate_path(&mut self, state: &GameState) {
        let mut rng = rand::thread_rng();
        self.path.clear();
        self.active_branch = None;
        self.cursor = 0;

        let current = state.sector;
        // Show ~12 sectors ahead
        for i in 0..12 {
            let sec = current + i;
            // Every 5 sectors (relative to sector 1) is a branch point
            if sec > current && (sec % 5) == 0 {
                let num_choices = rng.gen_range(2..=3_usize);
                let mut choices: Vec<RouteBranch> = Vec::new();
                let mut used: Vec<usize> = Vec::new();
                for _ in 0..num_choices {
                    loop {
                        let idx = rng.gen_range(0..BRANCH_TYPES.len());
                        if !used.contains(&idx) {
                            used.push(idx);
                            choices.push(BRANCH_TYPES[idx].clone());
                            break;
                        }
                    }
                }
                let node_idx = self.path.len();
                self.path.push(SectorNode::Branch {
                    sector: sec,
                    choices,
                });
                // First branch point is the active one
                if self.active_branch.is_none() {
                    self.active_branch = Some(node_idx);
                }
            } else {
                self.path.push(SectorNode::Linear { sector: sec });
            }
        }
    }

    pub fn handle_input(&mut self, key: KeyCode, state: &mut GameState) -> bool {
        if !self.open {
            return false;
        }

        match key {
            KeyCode::Esc | KeyCode::Char('m') | KeyCode::Char('M') => {
                self.open = false;
                true
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(idx) = self.active_branch {
                    if let SectorNode::Branch { choices, .. } = &self.path[idx] {
                        if self.cursor + 1 < choices.len() {
                            self.cursor += 1;
                        }
                    }
                }
                true
            }
            KeyCode::Enter => {
                // Apply chosen route
                if let Some(idx) = self.active_branch {
                    if let SectorNode::Branch { choices, .. } = &self.path[idx] {
                        if self.cursor < choices.len() {
                            let chosen = &choices[self.cursor];
                            // Store the average of difficulty and loot as route modifier
                            // Higher difficulty + higher loot = higher modifier
                            state.current_route_modifier = chosen.difficulty * chosen.loot;
                        }
                    }
                }
                self.open = false;
                true
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.select_branch(0, state);
                true
            }
            KeyCode::Char('b') | KeyCode::Char('B') => {
                self.select_branch(1, state);
                true
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.select_branch(2, state);
                true
            }
            _ => false,
        }
    }

    fn select_branch(&mut self, index: usize, state: &mut GameState) {
        if let Some(idx) = self.active_branch {
            if let SectorNode::Branch { choices, .. } = &self.path[idx] {
                if index < choices.len() {
                    let chosen = &choices[index];
                    state.current_route_modifier = chosen.difficulty * chosen.loot;
                    self.open = false;
                }
            }
        }
    }

    pub fn render(&self, frame: &mut Frame, state: &GameState) {
        if !self.open {
            return;
        }

        let area = centered_rect(70, 70, frame.area());
        frame.render_widget(Clear, area);

        let title = if state.prestige_level > 0 {
            format!(
                " ◈ SECTOR MAP ◈  [Prestige {}] ",
                state.prestige_level
            )
        } else {
            " ◈ SECTOR MAP ◈ ".to_string()
        };

        let outer_block = Block::default()
            .title(title)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));

        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        let mut lines: Vec<Line> = Vec::new();

        // ── Path visualization ──────────────────────────────────────
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  ── Upcoming Sectors ──",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        // Draw the horizontal path
        let mut path_spans: Vec<Span> = Vec::new();
        path_spans.push(Span::raw("  "));

        for (i, node) in self.path.iter().enumerate() {
            match node {
                SectorNode::Linear { sector } => {
                    let is_current = *sector == state.sector;
                    let style = if is_current {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    path_spans.push(Span::styled(
                        format!("[{:>2}]", sector),
                        style,
                    ));
                    // Connector to next
                    if i + 1 < self.path.len() {
                        if matches!(self.path[i + 1], SectorNode::Branch { .. }) {
                            path_spans.push(Span::styled("═══╦", Style::default().fg(Color::DarkGray)));
                        } else {
                            path_spans.push(Span::styled("═══", Style::default().fg(Color::DarkGray)));
                        }
                    }
                }
                SectorNode::Branch { sector, choices } => {
                    let is_active = self.active_branch.map_or(false, |a| a == i);
                    let marker = if is_active { "»" } else { " " };
                    path_spans.push(Span::styled(
                        format!("{}[{:>2}]", marker, sector),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ));
                    // Show branch labels inline
                    let labels: Vec<String> = choices
                        .iter()
                        .map(|c| c.name.chars().next().unwrap_or('?').to_string())
                        .collect();
                    path_spans.push(Span::styled(
                        format!("({})", labels.join("/")),
                        Style::default().fg(Color::Yellow),
                    ));
                    if i + 1 < self.path.len() {
                        path_spans.push(Span::styled("═══", Style::default().fg(Color::DarkGray)));
                    }
                }
            }
        }

        lines.push(Line::from(path_spans));

        // ── Branch details (if active branch exists) ────────────────
        if let Some(idx) = self.active_branch {
            if let SectorNode::Branch { sector, choices } = &self.path[idx] {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!("  ── Choose Route at Sector {} ──", sector),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));

                let labels = ['A', 'B', 'C'];
                for (i, choice) in choices.iter().enumerate() {
                    let is_selected = i == self.cursor;
                    let marker = if is_selected { "▸" } else { " " };
                    let label = labels.get(i).unwrap_or(&'?');

                    // Route name line
                    let name_style = if is_selected {
                        Style::default()
                            .fg(choice.color)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(choice.color)
                    };

                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("  {} [{}] ", marker, label),
                            Style::default().fg(if is_selected {
                                Color::White
                            } else {
                                Color::DarkGray
                            }),
                        ),
                        Span::styled(choice.name.to_string(), name_style),
                    ]));

                    // Stats line
                    let diff_color = if choice.difficulty > 1.2 {
                        Color::Red
                    } else if choice.difficulty > 0.9 {
                        Color::Yellow
                    } else {
                        Color::Green
                    };
                    let loot_color = if choice.loot > 1.5 {
                        Color::Green
                    } else if choice.loot > 0.9 {
                        Color::Yellow
                    } else {
                        Color::Red
                    };

                    lines.push(Line::from(vec![
                        Span::raw("        "),
                        Span::styled(
                            format!("Danger: {:.0}%", choice.difficulty * 100.0),
                            Style::default().fg(diff_color),
                        ),
                        Span::styled("  │  ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            format!("Loot: {:.0}%", choice.loot * 100.0),
                            Style::default().fg(loot_color),
                        ),
                    ]));

                    // Flavor text
                    lines.push(Line::from(Span::styled(
                        format!("        {}", choice.flavor),
                        Style::default().fg(Color::DarkGray),
                    )));
                    lines.push(Line::from(""));
                }
            }
        } else {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  No branch points ahead — keep flying!",
                Style::default().fg(Color::DarkGray),
            )));
        }

        // ── Current route info ──────────────────────────────────────
        lines.push(Line::from(""));
        let modifier = state.current_route_modifier;
        let route_desc = if (modifier - 1.0).abs() < 0.01 {
            "Standard".to_string()
        } else if modifier > 1.5 {
            format!("Intense ({:.0}%)", modifier * 100.0)
        } else if modifier > 1.0 {
            format!("Moderate ({:.0}%)", modifier * 100.0)
        } else {
            format!("Calm ({:.0}%)", modifier * 100.0)
        };

        lines.push(Line::from(vec![
            Span::styled(
                "  Current Route: ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                route_desc,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        // ── Footer ──────────────────────────────────────────────────
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  [", Style::default().fg(Color::DarkGray)),
            Span::styled("A/B/C", Style::default().fg(Color::Yellow)),
            Span::styled("] Choose  [", Style::default().fg(Color::DarkGray)),
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
            Span::styled("] Navigate  [", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::styled("] Confirm  [", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::styled("] Close", Style::default().fg(Color::DarkGray)),
        ]));

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }
}

/// Create a centered rectangle that occupies `percent_x`% width and `percent_y`% height.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
