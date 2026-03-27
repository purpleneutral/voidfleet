use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Tabs},
};

use crate::engine::missions::{Mission, MissionStatus};
use crate::rendering::layout::centered_rect;
use crate::state::GameState;

// ── Tab enum ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MissionTab {
    Available,
    Active,
    Log,
}

impl MissionTab {
    const ALL: [MissionTab; 3] = [MissionTab::Available, MissionTab::Active, MissionTab::Log];

    fn index(self) -> usize {
        match self {
            Self::Available => 0,
            Self::Active => 1,
            Self::Log => 2,
        }
    }

    fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }
}

// ── MissionScreen ───────────────────────────────────────────────────────────

pub struct MissionScreen {
    pub open: bool,
    tab: MissionTab,
    selected: usize,
    log: Vec<Mission>,
}

impl MissionScreen {
    pub fn new() -> Self {
        Self {
            open: false,
            tab: MissionTab::Available,
            selected: 0,
            log: Vec::new(),
        }
    }

    pub fn toggle(&mut self, state: &mut GameState) {
        self.open = !self.open;
        if self.open {
            self.tab = MissionTab::Available;
            self.selected = 0;
            if state.available_missions.is_empty() {
                state.refresh_available_missions(state.sector);
            }
        }
    }

    /// Add a mission to the display log (called after completion/failure).
    pub fn log_mission(&mut self, mission: Mission) {
        if self.log.len() >= 50 {
            self.log.remove(0);
        }
        self.log.push(mission);
    }

    // ── Input ───────────────────────────────────────────────────────

    pub fn handle_input(&mut self, key: KeyCode, state: &mut GameState) {
        match key {
            KeyCode::Esc => {
                self.open = false;
            }
            KeyCode::Tab => {
                self.tab = self.tab.next();
                self.selected = 0;
            }
            KeyCode::Up => {
                let count = self.current_list_len(state);
                if count > 0 {
                    self.selected = if self.selected == 0 {
                        count - 1
                    } else {
                        self.selected - 1
                    };
                }
            }
            KeyCode::Down => {
                let count = self.current_list_len(state);
                if count > 0 {
                    self.selected = (self.selected + 1) % count;
                }
            }
            KeyCode::Enter => {
                if self.tab == MissionTab::Available {
                    self.accept_selected(state);
                }
            }
            _ => {}
        }
    }

    fn current_list_len(&self, state: &GameState) -> usize {
        match self.tab {
            MissionTab::Available => state.available_missions.len(),
            MissionTab::Active => state.active_missions.len(),
            MissionTab::Log => self.log.len(),
        }
    }

    fn accept_selected(&mut self, state: &mut GameState) {
        if self.selected >= state.available_missions.len() {
            return;
        }
        let mission_id = state.available_missions[self.selected].id;
        if state.accept_mission(mission_id) {
            if self.selected > 0 && self.selected >= state.available_missions.len() {
                self.selected = state.available_missions.len().saturating_sub(1);
            }
        }
    }

    // ── Rendering ───────────────────────────────────────────────────

    pub fn render(&self, frame: &mut Frame, state: &GameState) {
        if !self.open {
            return;
        }

        let area = centered_rect(75, 85, frame.area());
        frame.render_widget(Clear, area);

        let outer = Block::default()
            .title(" \u{2605} MISSIONS \u{2605} ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));

        let inner = outer.inner(area);
        frame.render_widget(outer, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Min(5),
                Constraint::Length(1),
            ])
            .split(inner);

        self.render_tabs(frame, chunks[0], state);

        match self.tab {
            MissionTab::Available => self.render_list(frame, chunks[1], &state.available_missions, false, state.sector),
            MissionTab::Active => self.render_list(frame, chunks[1], &state.active_missions, true, state.sector),
            MissionTab::Log => self.render_log(frame, chunks[1]),
        }

        self.render_help(frame, chunks[2]);
    }

    fn render_tabs(&self, frame: &mut Frame, area: Rect, state: &GameState) {
        let titles: Vec<Line> = vec![
            Line::from(format!(" Available ({}) ", state.available_missions.len())),
            Line::from(format!(" Active ({}) ", state.active_missions.len())),
            Line::from(format!(" Log ({}) ", self.log.len())),
        ];

        let tabs = Tabs::new(titles)
            .select(self.tab.index())
            .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .style(Style::default().fg(Color::DarkGray))
            .divider("\u{2502}");

        frame.render_widget(tabs, area);
    }

    fn render_list(&self, frame: &mut Frame, area: Rect, missions: &[Mission], show_progress: bool, current_sector: u32) {
        let mut lines: Vec<Line> = Vec::new();

        if missions.is_empty() {
            lines.push(Line::from(""));
            let msg = if show_progress {
                "  No active missions — accept some from the Available tab"
            } else {
                "  No missions available — travel to new sectors to find missions"
            };
            lines.push(Line::from(Span::styled(msg, Style::default().fg(Color::DarkGray))));
        } else {
            for (i, mission) in missions.iter().enumerate() {
                lines.push(Line::from(""));
                self.render_card(&mut lines, mission, self.selected == i, show_progress, current_sector);
            }
        }

        frame.render_widget(Paragraph::new(lines), area);
    }

    fn render_log(&self, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();

        if self.log.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  No completed missions yet",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for (i, mission) in self.log.iter().rev().enumerate() {
                let is_sel = self.selected == i;
                lines.push(Line::from(""));

                let status_icon = match mission.status {
                    MissionStatus::Completed => Span::styled(" \u{2714} ", Style::default().fg(Color::Green)),
                    MissionStatus::Failed | MissionStatus::Expired => Span::styled(" \u{2718} ", Style::default().fg(Color::Red)),
                    _ => Span::raw("   "),
                };

                let marker = if is_sel { "\u{25b8}" } else { " " };
                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", marker), Style::default().fg(Color::Yellow)),
                    status_icon,
                    Span::styled(
                        mission.title.clone(),
                        Style::default().fg(Color::DarkGray).add_modifier(
                            if is_sel { Modifier::BOLD } else { Modifier::DIM },
                        ),
                    ),
                    Span::styled(
                        format!("  +{}\u{20bf}", mission.reward_credits),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }
        }

        frame.render_widget(Paragraph::new(lines), area);
    }

    fn render_card(
        &self,
        lines: &mut Vec<Line<'_>>,
        mission: &Mission,
        is_selected: bool,
        show_progress: bool,
        current_sector: u32,
    ) {
        let marker = if is_selected { "\u{25b8}" } else { " " };

        let stars: String = (0..5)
            .map(|i| if i < mission.difficulty { '\u{2605}' } else { '\u{2606}' })
            .collect();

        let title_style = if is_selected {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let type_icon = mission.mission_type.icon();

        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", marker), Style::default().fg(Color::Yellow)),
            Span::styled(stars, Style::default().fg(Color::Yellow)),
            Span::styled(format!(" {} ", type_icon), Style::default().fg(Color::Cyan)),
            Span::styled(mission.title.clone(), title_style),
        ]));

        lines.push(Line::from(Span::styled(
            format!("      {}", mission.description),
            Style::default().fg(Color::DarkGray),
        )));

        let mut reward_spans = vec![
            Span::styled("      Sector: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", mission.target_sector), Style::default().fg(Color::White)),
            Span::styled(" \u{2502} Reward: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}\u{20bf}", mission.reward_credits), Style::default().fg(Color::Green)),
        ];

        if mission.reward_equipment {
            reward_spans.push(Span::styled(" + Equipment", Style::default().fg(Color::Yellow)));
        }

        lines.push(Line::from(reward_spans));

        let faction_name = mission.faction.name();
        lines.push(Line::from(vec![
            Span::styled("      Faction: ", Style::default().fg(Color::DarkGray)),
            Span::styled(faction_name.to_string(), Style::default().fg(Color::Cyan)),
            Span::styled(format!(" +{} rep", mission.reward_rep), Style::default().fg(Color::Green)),
        ]));

        if show_progress {
            if mission.sectors_remaining > 0 {
                let pct = mission.progress_pct(current_sector);
                let bar_width = 20u16;
                let filled = (pct * bar_width as f32) as u16;
                let bar_str = format!(
                    "{}{}",
                    "\u{2588}".repeat(filled as usize),
                    "\u{2591}".repeat((bar_width - filled) as usize),
                );

                let bar_color = if pct >= 1.0 {
                    Color::Green
                } else if pct >= 0.5 {
                    Color::Yellow
                } else {
                    Color::White
                };

                lines.push(Line::from(vec![
                    Span::styled("      \u{23f3} ", Style::default().fg(Color::DarkGray)),
                    Span::styled(bar_str, Style::default().fg(bar_color)),
                    Span::styled(
                        format!(" {} sectors left", mission.sectors_remaining),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("      \u{25b6} ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("Reach sector {}", mission.target_sector),
                        Style::default().fg(Color::White),
                    ),
                ]));
            }
        }

        if is_selected && mission.status == MissionStatus::Available {
            lines.push(Line::from(Span::styled(
                "      [Enter] Accept",
                Style::default().fg(Color::Yellow),
            )));
        }
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help = Line::from(vec![
            Span::styled(" Tab", Style::default().fg(Color::Cyan)),
            Span::styled(" Tabs ", Style::default().fg(Color::DarkGray)),
            Span::styled("\u{2191}\u{2193}", Style::default().fg(Color::Cyan)),
            Span::styled(" Browse ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::styled(" Accept ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::styled(" Close", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(help), area);
    }
}
