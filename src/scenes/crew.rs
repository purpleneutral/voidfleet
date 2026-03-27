use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Tabs},
};

use crate::engine::crew::{generate_crew, BondType, CrewClass, CrewMember};
use crate::rendering::layout::centered_rect;
use crate::state::GameState;

// ── Tab enum ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CrewTab {
    Roster,
    Assignments,
    Recruit,
}

impl CrewTab {
    const ALL: [CrewTab; 3] = [CrewTab::Roster, CrewTab::Assignments, CrewTab::Recruit];

    fn index(self) -> usize {
        match self {
            Self::Roster => 0,
            Self::Assignments => 1,
            Self::Recruit => 2,
        }
    }

    fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    fn prev(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

// ── Assignment sub-state ────────────────────────────────────────────────────

/// When assigning a crew member, we first select the crew, then pick a ship.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssignPhase {
    SelectCrew,
    SelectShip,
}

// ── CrewScreen ──────────────────────────────────────────────────────────────

pub struct CrewScreen {
    pub open: bool,
    tab: CrewTab,
    selected: usize,
    /// Assignment flow state (Assignments tab).
    assign_phase: AssignPhase,
    assign_crew_id: Option<u64>,
    ship_cursor: usize,
    /// Recruits available for hire (regenerated on open).
    recruits: Vec<(CrewMember, u64)>, // (crew, cost)
}

impl CrewScreen {
    pub fn new() -> Self {
        Self {
            open: false,
            tab: CrewTab::Roster,
            selected: 0,
            assign_phase: AssignPhase::SelectCrew,
            assign_crew_id: None,
            ship_cursor: 0,
            recruits: Vec::new(),
        }
    }

    pub fn toggle(&mut self, state: &GameState) {
        self.open = !self.open;
        if self.open {
            self.tab = CrewTab::Roster;
            self.selected = 0;
            self.assign_phase = AssignPhase::SelectCrew;
            self.assign_crew_id = None;
            self.ship_cursor = 0;
            self.regenerate_recruits(state);
        }
    }

    /// Regenerate the recruit pool (called on open and after battles).
    pub fn regenerate_recruits(&mut self, state: &GameState) {
        self.recruits.clear();
        for _ in 0..3 {
            let crew = generate_crew(state.sector);
            let cost = recruit_cost(&crew);
            self.recruits.push((crew, cost));
        }
    }

    // ── Input handling ──────────────────────────────────────────

    pub fn handle_input(&mut self, key: KeyCode, state: &mut GameState) {
        match key {
            KeyCode::Esc => {
                // If in ship selection, go back to crew selection
                if self.tab == CrewTab::Assignments
                    && self.assign_phase == AssignPhase::SelectShip
                {
                    self.assign_phase = AssignPhase::SelectCrew;
                    self.assign_crew_id = None;
                    return;
                }
                self.open = false;
            }
            KeyCode::Tab => {
                self.tab = self.tab.next();
                self.selected = 0;
                self.assign_phase = AssignPhase::SelectCrew;
                self.assign_crew_id = None;
                self.ship_cursor = 0;
            }
            KeyCode::BackTab => {
                self.tab = self.tab.prev();
                self.selected = 0;
                self.assign_phase = AssignPhase::SelectCrew;
                self.assign_crew_id = None;
                self.ship_cursor = 0;
            }
            KeyCode::Up => {
                let count = self.current_list_len(state);
                if count > 0 {
                    match self.tab {
                        CrewTab::Assignments if self.assign_phase == AssignPhase::SelectShip => {
                            if state.fleet.is_empty() {
                                return;
                            }
                            self.ship_cursor = if self.ship_cursor == 0 {
                                state.fleet.len() - 1
                            } else {
                                self.ship_cursor - 1
                            };
                        }
                        _ => {
                            self.selected = if self.selected == 0 {
                                count - 1
                            } else {
                                self.selected - 1
                            };
                        }
                    }
                }
            }
            KeyCode::Down => {
                let count = self.current_list_len(state);
                if count > 0 {
                    match self.tab {
                        CrewTab::Assignments if self.assign_phase == AssignPhase::SelectShip => {
                            if state.fleet.is_empty() {
                                return;
                            }
                            self.ship_cursor = (self.ship_cursor + 1) % state.fleet.len();
                        }
                        _ => {
                            self.selected = (self.selected + 1) % count;
                        }
                    }
                }
            }
            KeyCode::Enter => match self.tab {
                CrewTab::Roster => {
                    // Enter on roster: jump to assignments tab with this crew selected
                    if self.selected < state.crew_roster.len() {
                        let crew_id = state.crew_roster[self.selected].id;
                        self.tab = CrewTab::Assignments;
                        self.assign_phase = AssignPhase::SelectShip;
                        self.assign_crew_id = Some(crew_id);
                        self.ship_cursor = 0;
                    }
                }
                CrewTab::Assignments => {
                    match self.assign_phase {
                        AssignPhase::SelectCrew => {
                            if self.selected < state.crew_roster.len() {
                                let crew_id = state.crew_roster[self.selected].id;
                                self.assign_phase = AssignPhase::SelectShip;
                                self.assign_crew_id = Some(crew_id);
                                self.ship_cursor = 0;
                            }
                        }
                        AssignPhase::SelectShip => {
                            if let Some(crew_id) = self.assign_crew_id {
                                let ship_idx = self.ship_cursor;
                                // If ship already has crew, swap them
                                if let Some(existing_crew_id) =
                                    state.fleet.get(ship_idx).and_then(|s| s.crew_id)
                                {
                                    if existing_crew_id != crew_id {
                                        // Unassign existing crew first
                                        state.unassign_crew(existing_crew_id);
                                    }
                                }
                                // Unassign from old ship if any, then assign
                                state.unassign_crew(crew_id);
                                state.assign_crew(crew_id, ship_idx);
                                self.assign_phase = AssignPhase::SelectCrew;
                                self.assign_crew_id = None;
                            }
                        }
                    }
                }
                CrewTab::Recruit => {
                    self.try_recruit(state);
                }
            },
            KeyCode::Char('d') | KeyCode::Char('D') => match self.tab {
                CrewTab::Roster => {
                    // Dismiss crew member
                    if self.selected < state.crew_roster.len() {
                        let crew_id = state.crew_roster[self.selected].id;
                        state.dismiss_crew(crew_id);
                        if self.selected > 0 && self.selected >= state.crew_roster.len() {
                            self.selected = state.crew_roster.len().saturating_sub(1);
                        }
                    }
                }
                CrewTab::Assignments => {
                    // Unassign selected crew
                    if self.assign_phase == AssignPhase::SelectCrew
                        && self.selected < state.crew_roster.len()
                    {
                        let crew_id = state.crew_roster[self.selected].id;
                        state.unassign_crew(crew_id);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn current_list_len(&self, state: &GameState) -> usize {
        match self.tab {
            CrewTab::Roster | CrewTab::Assignments => state.crew_roster.len(),
            CrewTab::Recruit => self.recruits.len(),
        }
    }

    fn try_recruit(&mut self, state: &mut GameState) {
        if self.selected >= self.recruits.len() {
            return;
        }
        let (_, cost) = &self.recruits[self.selected];
        let cost = *cost;
        if state.credits < cost {
            return;
        }
        if state.crew_roster.len() >= state.crew_capacity {
            return;
        }
        state.credits -= cost;
        let (crew, _) = self.recruits.remove(self.selected);
        state.add_crew(crew);
        if self.selected > 0 && self.selected >= self.recruits.len() {
            self.selected = self.recruits.len().saturating_sub(1);
        }
    }

    // ── Rendering ───────────────────────────────────────────────

    pub fn render(&self, frame: &mut Frame, state: &GameState) {
        if !self.open {
            return;
        }

        let area = centered_rect(80, 85, frame.area());
        frame.render_widget(Clear, area);

        let outer_block = Block::default()
            .title(" \u{2694} CREW ROSTER \u{2694} ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta))
            .style(Style::default().bg(Color::Black));

        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // resource bar
                Constraint::Length(2), // tabs
                Constraint::Min(5),   // content
                Constraint::Length(1), // help bar
            ])
            .split(inner);

        self.render_resources(frame, chunks[0], state);
        self.render_tabs(frame, chunks[1]);

        match self.tab {
            CrewTab::Roster => self.render_roster_tab(frame, chunks[2], state),
            CrewTab::Assignments => self.render_assignments_tab(frame, chunks[2], state),
            CrewTab::Recruit => self.render_recruit_tab(frame, chunks[2], state),
        }

        self.render_help(frame, chunks[3]);
    }

    fn render_resources(&self, frame: &mut Frame, area: Rect, state: &GameState) {
        let line = Line::from(vec![
            Span::styled(" \u{20bf} ", Style::default().fg(Color::Green)),
            Span::raw(format!("{:<8}", state.credits)),
            Span::styled(
                format!(
                    "  Crew: {}/{}",
                    state.crew_roster.len(),
                    state.crew_capacity
                ),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("  Ships: {}", state.fleet.len()),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("  Lv.{}", state.level),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        frame.render_widget(Paragraph::new(line), area);
    }

    fn render_tabs(&self, frame: &mut Frame, area: Rect) {
        let titles: Vec<Line> = CrewTab::ALL
            .iter()
            .map(|t| {
                let name = match t {
                    CrewTab::Roster => " Roster ",
                    CrewTab::Assignments => " Assignments ",
                    CrewTab::Recruit => " Recruit ",
                };
                Line::from(name)
            })
            .collect();

        let tabs = Tabs::new(titles)
            .select(self.tab.index())
            .highlight_style(
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().fg(Color::DarkGray))
            .divider("\u{2502}");

        frame.render_widget(tabs, area);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help_spans = match self.tab {
            CrewTab::Roster => vec![
                Span::styled(" \u{2191}\u{2193}", Style::default().fg(Color::Yellow)),
                Span::styled(" browse  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::styled(" assign  ", Style::default().fg(Color::DarkGray)),
                Span::styled("D", Style::default().fg(Color::Yellow)),
                Span::styled(" dismiss  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Tab", Style::default().fg(Color::Yellow)),
                Span::styled(" switch  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Esc", Style::default().fg(Color::Yellow)),
                Span::styled(" close", Style::default().fg(Color::DarkGray)),
            ],
            CrewTab::Assignments => {
                if self.assign_phase == AssignPhase::SelectShip {
                    vec![
                        Span::styled(" \u{2191}\u{2193}", Style::default().fg(Color::Yellow)),
                        Span::styled(" ship  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("Enter", Style::default().fg(Color::Yellow)),
                        Span::styled(" assign  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("Esc", Style::default().fg(Color::Yellow)),
                        Span::styled(" back", Style::default().fg(Color::DarkGray)),
                    ]
                } else {
                    vec![
                        Span::styled(" \u{2191}\u{2193}", Style::default().fg(Color::Yellow)),
                        Span::styled(" browse  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("Enter", Style::default().fg(Color::Yellow)),
                        Span::styled(" select  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("D", Style::default().fg(Color::Yellow)),
                        Span::styled(" unassign  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("Tab", Style::default().fg(Color::Yellow)),
                        Span::styled(" switch  ", Style::default().fg(Color::DarkGray)),
                        Span::styled("Esc", Style::default().fg(Color::Yellow)),
                        Span::styled(" close", Style::default().fg(Color::DarkGray)),
                    ]
                }
            }
            CrewTab::Recruit => vec![
                Span::styled(" \u{2191}\u{2193}", Style::default().fg(Color::Yellow)),
                Span::styled(" browse  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::styled(" hire  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Tab", Style::default().fg(Color::Yellow)),
                Span::styled(" switch  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Esc", Style::default().fg(Color::Yellow)),
                Span::styled(" close", Style::default().fg(Color::DarkGray)),
            ],
        };
        frame.render_widget(Paragraph::new(Line::from(help_spans)), area);
    }

    // ── Roster tab ──────────────────────────────────────────────

    fn render_roster_tab(&self, frame: &mut Frame, area: Rect, state: &GameState) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Left: crew list
        let mut lines: Vec<Line> = Vec::new();

        if state.crew_roster.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  No crew members yet.",
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(Span::styled(
                "  Visit the Recruit tab to hire crew.",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for (i, crew) in state.crew_roster.iter().enumerate() {
                let is_selected = self.selected == i;
                let marker = if is_selected { "\u{25b8} " } else { "  " };

                let class_color = class_color(crew.class);
                let name_style = if is_selected {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                // Name + class line
                lines.push(Line::from(vec![
                    Span::styled(marker, Style::default().fg(Color::Yellow)),
                    Span::styled(&crew.name, name_style),
                    Span::styled(
                        format!(" \u{2014} {} Lv.{}", crew.class.name(), crew.level),
                        Style::default().fg(class_color),
                    ),
                ]));

                // Assignment line
                let assignment = if let Some(ship_idx) = crew.assigned_ship {
                    if ship_idx < state.fleet.len() {
                        format!(
                            "    {} #{} ",
                            state.fleet[ship_idx].ship_type.name(),
                            ship_idx + 1
                        )
                    } else {
                        "    [Invalid]".to_string()
                    }
                } else {
                    "    [Unassigned]".to_string()
                };
                let assign_color = if crew.assigned_ship.is_some() {
                    Color::Green
                } else {
                    Color::DarkGray
                };
                lines.push(Line::from(Span::styled(assignment, Style::default().fg(assign_color))));

                // Separator
                if i + 1 < state.crew_roster.len() {
                    lines.push(Line::from(""));
                }
            }
        }

        let list_block = Block::default()
            .title(format!(
                " Crew ({}/{}) ",
                state.crew_roster.len(),
                state.crew_capacity
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(Paragraph::new(lines).block(list_block), cols[0]);

        // Right: detail panel
        let detail_lines = if self.selected < state.crew_roster.len() {
            render_crew_detail(&state.crew_roster[self.selected], state)
        } else {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  No crew selected",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        };

        let detail_block = Block::default()
            .title(" Details ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(Paragraph::new(detail_lines).block(detail_block), cols[1]);
    }

    // ── Assignments tab ─────────────────────────────────────────

    fn render_assignments_tab(&self, frame: &mut Frame, area: Rect, state: &GameState) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(area);

        // Left: crew list (select crew to assign)
        let mut lines: Vec<Line> = Vec::new();
        let in_ship_select = self.assign_phase == AssignPhase::SelectShip;

        if state.crew_roster.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  No crew to assign.",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for (i, crew) in state.crew_roster.iter().enumerate() {
                let is_selected = self.selected == i && !in_ship_select;
                let is_active =
                    in_ship_select && self.assign_crew_id == Some(crew.id);
                let marker = if is_active {
                    "\u{2605} " // ★
                } else if is_selected {
                    "\u{25b8} "
                } else {
                    "  "
                };

                let name_style = if is_active {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else if is_selected {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let assignment = if let Some(ship_idx) = crew.assigned_ship {
                    if ship_idx < state.fleet.len() {
                        format!(
                            " \u{2192} {} #{}",
                            state.fleet[ship_idx].ship_type.name(),
                            ship_idx + 1
                        )
                    } else {
                        " \u{2192} ?".to_string()
                    }
                } else {
                    String::new()
                };
                let assign_color = if crew.assigned_ship.is_some() {
                    Color::Green
                } else {
                    Color::DarkGray
                };

                lines.push(Line::from(vec![
                    Span::styled(marker, Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("{} ({})", crew.name, crew.class.name()),
                        name_style,
                    ),
                    Span::styled(assignment, Style::default().fg(assign_color)),
                ]));
            }
        }

        let crew_title = if in_ship_select {
            " Select Ship \u{2192} "
        } else {
            " Select Crew "
        };
        let crew_block = Block::default()
            .title(crew_title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if in_ship_select {
                Color::Yellow
            } else {
                Color::DarkGray
            }));
        frame.render_widget(Paragraph::new(lines).block(crew_block), cols[0]);

        // Right: ship list
        let mut ship_lines: Vec<Line> = Vec::new();

        if state.fleet.is_empty() {
            ship_lines.push(Line::from(""));
            ship_lines.push(Line::from(Span::styled(
                "  No ships in fleet.",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for (i, ship) in state.fleet.iter().enumerate() {
                let is_selected = in_ship_select && self.ship_cursor == i;
                let marker = if is_selected { "\u{25b8} " } else { "  " };

                let name_style = if is_selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Cyan)
                };

                ship_lines.push(Line::from(vec![
                    Span::styled(marker, Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("{} #{}", ship.ship_type.name(), i + 1),
                        name_style,
                    ),
                    Span::styled(
                        format!("  Lv.{}", ship.upgrade_level),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));

                // Show assigned crew for this ship
                let crew_name = state
                    .get_ship_crew(i)
                    .map(|c| format!("    {} {} ({})", c.class.icon(), c.name, c.class.name()))
                    .unwrap_or_else(|| "    [No Crew]".to_string());
                let crew_color = if state.get_ship_crew(i).is_some() {
                    Color::Green
                } else {
                    Color::DarkGray
                };
                ship_lines.push(Line::from(Span::styled(
                    crew_name,
                    Style::default().fg(crew_color),
                )));

                if i + 1 < state.fleet.len() {
                    ship_lines.push(Line::from(""));
                }
            }
        }

        let ship_block = Block::default()
            .title(" Fleet Ships ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(Paragraph::new(ship_lines).block(ship_block), cols[1]);
    }

    // ── Recruit tab ─────────────────────────────────────────────

    fn render_recruit_tab(&self, frame: &mut Frame, area: Rect, state: &GameState) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Left: recruit list
        let mut lines: Vec<Line> = Vec::new();
        let at_capacity = state.crew_roster.len() >= state.crew_capacity;

        if self.recruits.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  No recruits available.",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for (i, (crew, cost)) in self.recruits.iter().enumerate() {
                let is_selected = self.selected == i;
                let can_afford = state.credits >= *cost && !at_capacity;
                let marker = if is_selected { "\u{25b8} " } else { "  " };

                let class_color = class_color(crew.class);
                let name_style = if is_selected {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                lines.push(Line::from(vec![
                    Span::styled(marker, Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("{} ", crew.class.icon()),
                        Style::default().fg(class_color),
                    ),
                    Span::styled(&crew.name, name_style),
                    Span::styled(
                        format!(" \u{2014} {} Lv.{}", crew.class.name(), crew.level),
                        Style::default().fg(class_color),
                    ),
                ]));

                // Cost line
                let cost_color = if can_afford { Color::Green } else { Color::Red };
                lines.push(Line::from(Span::styled(
                    format!("    Cost: {}\u{20bf}", cost),
                    Style::default().fg(cost_color),
                )));

                // Personality
                lines.push(Line::from(Span::styled(
                    format!("    {}: {}", crew.personality.name(), crew.personality.description()),
                    Style::default().fg(Color::DarkGray),
                )));

                if i + 1 < self.recruits.len() {
                    lines.push(Line::from(""));
                }
            }
        }

        if at_capacity {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Crew at capacity! Dismiss someone first.",
                Style::default().fg(Color::Red),
            )));
        }

        let list_block = Block::default()
            .title(" Available Recruits ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(Paragraph::new(lines).block(list_block), cols[0]);

        // Right: detail panel for selected recruit
        let detail_lines = if self.selected < self.recruits.len() {
            render_recruit_detail(&self.recruits[self.selected].0, self.recruits[self.selected].1, state)
        } else {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  No recruit selected",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        };

        let detail_block = Block::default()
            .title(" Recruit Details ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(Paragraph::new(detail_lines).block(detail_block), cols[1]);
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn class_color(class: CrewClass) -> Color {
    match class {
        CrewClass::Pilot => Color::Cyan,
        CrewClass::Gunner => Color::Red,
        CrewClass::Engineer => Color::Green,
        CrewClass::Medic => Color::Yellow,
        CrewClass::Captain => Color::Magenta,
        CrewClass::Navigator => Color::Blue,
    }
}

/// Calculate hire cost for a crew member.
fn recruit_cost(crew: &CrewMember) -> u64 {
    let base = 200 + (crew.level as u64 * 80);
    let class_mult = if crew.class == CrewClass::Captain {
        3
    } else {
        1
    };
    (base * class_mult).min(3000)
}

/// Render a morale bar.
fn morale_bar(morale: u8) -> String {
    let filled = (morale as usize) / 10;
    let empty = 10_usize.saturating_sub(filled);
    format!(
        "{}{}",
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(empty),
    )
}

fn morale_color(morale: u8) -> Color {
    if morale > 70 {
        Color::Green
    } else if morale >= 40 {
        Color::Yellow
    } else {
        Color::Red
    }
}

fn render_crew_detail<'a>(crew: &CrewMember, state: &GameState) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(""));

    // Name + class
    lines.push(Line::from(vec![
        Span::styled(
            format!("  {} ", crew.class.icon()),
            Style::default().fg(class_color(crew.class)),
        ),
        Span::styled(
            crew.name.clone(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(Span::styled(
        format!("  {} Lv.{}", crew.class.name(), crew.level),
        Style::default().fg(class_color(crew.class)),
    )));

    // XP bar
    let xp_pct = if crew.level >= 20 {
        100
    } else {
        let needed = crew.xp_to_next();
        if needed > 0 {
            (crew.xp * 100 / needed).min(100) as u32
        } else {
            0
        }
    };
    lines.push(Line::from(Span::styled(
        format!("  XP: {}/{} ({}%)", crew.xp, crew.xp_to_next(), xp_pct),
        Style::default().fg(Color::DarkGray),
    )));

    lines.push(Line::from(""));

    // Stats
    lines.push(Line::from(vec![
        Span::styled("  Piloting:    ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{:>3}", crew.piloting),
            Style::default().fg(Color::Cyan),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Gunnery:     ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{:>3}", crew.gunnery),
            Style::default().fg(Color::Red),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Engineering: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{:>3}", crew.engineering),
            Style::default().fg(Color::Yellow),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Leadership:  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{:>3}", crew.leadership),
            Style::default().fg(Color::Magenta),
        ),
    ]));

    lines.push(Line::from(""));

    // Personality
    lines.push(Line::from(vec![
        Span::styled("  Personality: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            crew.personality.name().to_string(),
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(Span::styled(
        format!("  {}", crew.personality.description()),
        Style::default().fg(Color::DarkGray),
    )));

    lines.push(Line::from(""));

    // Morale
    let m_color = morale_color(crew.morale);
    lines.push(Line::from(vec![
        Span::styled("  Morale: ", Style::default().fg(Color::DarkGray)),
        Span::styled(morale_bar(crew.morale), Style::default().fg(m_color)),
        Span::styled(
            format!(" {}%", crew.morale),
            Style::default().fg(m_color),
        ),
    ]));

    lines.push(Line::from(""));

    // Assignment
    let assignment = if let Some(ship_idx) = crew.assigned_ship {
        if ship_idx < state.fleet.len() {
            format!(
                "  Assigned: {} #{}",
                state.fleet[ship_idx].ship_type.name(),
                ship_idx + 1
            )
        } else {
            "  Assigned: [Invalid]".to_string()
        }
    } else {
        "  Assigned: [None]".to_string()
    };
    let assign_color = if crew.assigned_ship.is_some() {
        Color::Green
    } else {
        Color::DarkGray
    };
    lines.push(Line::from(Span::styled(
        assignment,
        Style::default().fg(assign_color),
    )));

    // Combat stats
    lines.push(Line::from(Span::styled(
        format!(
            "  Kills: {} \u{2502} Battles: {}",
            crew.kills, crew.battles_survived
        ),
        Style::default().fg(Color::DarkGray),
    )));

    lines.push(Line::from(""));

    // ── Abilities ──────────────────────────────────────────
    lines.push(Line::from(Span::styled(
        "  Abilities:",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )));

    let all_abilities = crew.class.abilities();
    if all_abilities.is_empty() {
        lines.push(Line::from(Span::styled(
            "    None",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for ability in &all_abilities {
            let unlocked = crew.level >= ability.level_required;
            if unlocked {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("    {} ", ability.icon),
                        Style::default().fg(Color::Green),
                    ),
                    Span::styled(
                        format!("{} (Lv.{})", ability.name, ability.level_required),
                        Style::default().fg(Color::Green),
                    ),
                    Span::styled(
                        format!(" \u{2014} {}", ability.description),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("    {} ", ability.icon),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(
                        format!("{} (Lv.{})", ability.name, ability.level_required),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(
                        format!(" \u{2014} \u{1f512} Requires Lv.{}", ability.level_required),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }
        }
    }

    // ── Bonds ──────────────────────────────────────────────
    let mut has_bonds = false;
    for bond in &state.crew_bonds {
        let (other_id, is_member) = if bond.crew_a_id == crew.id {
            (bond.crew_b_id, true)
        } else if bond.crew_b_id == crew.id {
            (bond.crew_a_id, true)
        } else {
            (0, false)
        };
        if !is_member || bond.bond_type == BondType::None {
            continue;
        }
        if !has_bonds {
            lines.push(Line::from(""));
            has_bonds = true;
        }
        let other_name = state.crew_roster.iter()
            .find(|c| c.id == other_id)
            .map(|c| c.name.as_str())
            .unwrap_or("Unknown");
        let bond_color = match bond.bond_type {
            BondType::BattleBrothers => Color::Green,
            BondType::Respect => Color::Cyan,
            BondType::Rivals => Color::Red,
            BondType::Acquaintance => Color::Yellow,
            BondType::None => Color::DarkGray,
        };
        lines.push(Line::from(vec![
            Span::styled("  Bond: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", bond.bond_type.description()),
                Style::default().fg(bond_color),
            ),
            Span::styled(
                format!(" with {}", other_name),
                Style::default().fg(Color::White),
            ),
        ]));
    }

    // Grief/vengeance status
    if crew.grief_battles_remaining > 0 {
        lines.push(Line::from(Span::styled(
            format!("  \u{1f494} Grieving ({} battles left)", crew.grief_battles_remaining),
            Style::default().fg(Color::Magenta),
        )));
    }
    if crew.vengeance_battles_remaining > 0 {
        lines.push(Line::from(Span::styled(
            format!("  \u{1f525} Vengeance ({} battles left)", crew.vengeance_battles_remaining),
            Style::default().fg(Color::Red),
        )));
    }

    lines
}

fn render_recruit_detail<'a>(crew: &CrewMember, cost: u64, state: &GameState) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(""));

    // Name + class
    lines.push(Line::from(vec![
        Span::styled(
            format!("  {} ", crew.class.icon()),
            Style::default().fg(class_color(crew.class)),
        ),
        Span::styled(
            crew.name.clone(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(Span::styled(
        format!("  {} Lv.{}", crew.class.name(), crew.level),
        Style::default().fg(class_color(crew.class)),
    )));

    lines.push(Line::from(""));

    // Stats
    lines.push(Line::from(vec![
        Span::styled("  Piloting:    ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{:>3}", crew.piloting),
            Style::default().fg(Color::Cyan),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Gunnery:     ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{:>3}", crew.gunnery),
            Style::default().fg(Color::Red),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Engineering: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{:>3}", crew.engineering),
            Style::default().fg(Color::Yellow),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Leadership:  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{:>3}", crew.leadership),
            Style::default().fg(Color::Magenta),
        ),
    ]));

    lines.push(Line::from(""));

    // Personality
    lines.push(Line::from(vec![
        Span::styled("  Personality: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            crew.personality.name().to_string(),
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(Span::styled(
        format!("  {}", crew.personality.description()),
        Style::default().fg(Color::DarkGray),
    )));

    lines.push(Line::from(""));

    // Cost
    let can_afford = state.credits >= cost;
    let cost_color = if can_afford { Color::Green } else { Color::Red };
    lines.push(Line::from(vec![
        Span::styled("  Hire Cost: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}\u{20bf}", cost),
            Style::default()
                .fg(cost_color)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    if !can_afford {
        lines.push(Line::from(Span::styled(
            "  Not enough credits!",
            Style::default().fg(Color::Red),
        )));
    } else if state.crew_roster.len() >= state.crew_capacity {
        lines.push(Line::from(Span::styled(
            "  Crew roster is full!",
            Style::default().fg(Color::Red),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "  Press Enter to hire",
            Style::default().fg(Color::Green),
        )));
    }

    lines
}
