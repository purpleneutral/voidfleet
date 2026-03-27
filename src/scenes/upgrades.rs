use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs},
};

use crate::engine::economy;
use crate::engine::ship::{Ship, ShipType};
use crate::rendering::layout::centered_rect;
use crate::state::GameState;

// ── Tab enum ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradeTab {
    Ships,
    Tech,
    Fleet,
}

impl UpgradeTab {
    const ALL: [UpgradeTab; 3] = [UpgradeTab::Ships, UpgradeTab::Tech, UpgradeTab::Fleet];

    fn index(self) -> usize {
        match self {
            Self::Ships => 0,
            Self::Tech => 1,
            Self::Fleet => 2,
        }
    }

    fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    fn prev(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

// ── All ship types in display order ─────────────────────────────────────────

const SHIP_TYPES: [ShipType; 7] = [
    ShipType::Scout,
    ShipType::Fighter,
    ShipType::Bomber,
    ShipType::Frigate,
    ShipType::Destroyer,
    ShipType::Capital,
    ShipType::Carrier,
];

// ── Tech tree constants ─────────────────────────────────────────────────────

const TECH_MAX_LEVEL: u8 = 10;

// ── UpgradeScreen ───────────────────────────────────────────────────────────

pub struct UpgradeScreen {
    pub selected_tab: UpgradeTab,
    pub selected_item: usize,
    pub open: bool,
}

impl UpgradeScreen {
    pub fn new() -> Self {
        Self {
            selected_tab: UpgradeTab::Ships,
            selected_item: 0,
            open: false,
        }
    }

    /// Toggle the upgrade screen open/closed.
    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.selected_item = 0;
        }
    }

    // ── Item counts per tab ─────────────────────────────────────────────

    fn item_count(&self, state: &GameState) -> usize {
        match self.selected_tab {
            // Existing fleet ships + "Build New" entries for each ship type
            UpgradeTab::Ships => state.fleet.len() + SHIP_TYPES.len(),
            UpgradeTab::Tech => 4, // lasers, shields, engines, beams
            UpgradeTab::Fleet => 0, // display-only, no selectable items
        }
    }

    // ── Input ───────────────────────────────────────────────────────────

    pub fn handle_input(&mut self, key: KeyCode, state: &mut GameState) {
        match key {
            KeyCode::Esc => {
                self.open = false;
            }
            KeyCode::Tab | KeyCode::Right => {
                self.selected_tab = self.selected_tab.next();
                self.selected_item = 0;
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.selected_tab = self.selected_tab.prev();
                self.selected_item = 0;
            }
            KeyCode::Up => {
                let count = self.item_count(state);
                if count > 0 {
                    self.selected_item = if self.selected_item == 0 {
                        count - 1
                    } else {
                        self.selected_item - 1
                    };
                }
            }
            KeyCode::Down => {
                let count = self.item_count(state);
                if count > 0 {
                    self.selected_item = (self.selected_item + 1) % count;
                }
            }
            KeyCode::Enter => {
                self.try_purchase(state);
            }
            _ => {}
        }
    }

    fn try_purchase(&mut self, state: &mut GameState) {
        match self.selected_tab {
            UpgradeTab::Ships => self.try_purchase_ships(state),
            UpgradeTab::Tech => self.try_purchase_tech(state),
            UpgradeTab::Fleet => {} // no purchasable items
        }
    }

    fn try_purchase_ships(&self, state: &mut GameState) {
        let fleet_len = state.fleet.len();
        if self.selected_item < fleet_len {
            // Upgrading an existing ship
            let cost = state.fleet[self.selected_item].upgrade_cost();
            let level = state.fleet[self.selected_item].upgrade_level;
            if level < 10 && state.credits >= cost {
                state.credits -= cost;
                state.fleet[self.selected_item].upgrade_level += 1;
                // Heal the HP gained from upgrade
                let ship = &mut state.fleet[self.selected_item];
                ship.current_hp = ship.max_hp();
            }
        } else {
            // Building a new ship
            let type_idx = self.selected_item - fleet_len;
            if type_idx < SHIP_TYPES.len() {
                let stype = SHIP_TYPES[type_idx];
                if state.level >= stype.unlock_level() {
                    let cost = economy::ship_build_cost(stype, state.fleet.len());
                    if state.credits >= cost {
                        state.credits -= cost;
                        state.fleet.push(Ship::new(stype));
                    }
                }
            }
        }
    }

    fn try_purchase_tech(&self, state: &mut GameState) {
        let (level, max) = match self.selected_item {
            0 => (state.tech_lasers, TECH_MAX_LEVEL),
            1 => (state.tech_shields, TECH_MAX_LEVEL),
            2 => (state.tech_engines, TECH_MAX_LEVEL),
            3 => (state.tech_beams, TECH_MAX_LEVEL),
            _ => return,
        };
        if level >= max {
            return;
        }
        let cost = economy::tech_upgrade_cost(level);
        if state.credits >= cost {
            state.credits -= cost;
            match self.selected_item {
                0 => state.tech_lasers += 1,
                1 => state.tech_shields += 1,
                2 => state.tech_engines += 1,
                3 => state.tech_beams += 1,
                _ => {}
            }
        }
    }

    // ── Rendering ───────────────────────────────────────────────────────

    pub fn render(&self, frame: &mut Frame, state: &GameState) {
        if !self.open {
            return;
        }

        let area = centered_rect(80, 80, frame.area());

        // Clear the area behind the overlay
        frame.render_widget(Clear, area);

        let outer_block = Block::default()
            .title(" ◈ UPGRADES ◈ ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));

        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        // Layout: resource bar | tabs | content
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

        match self.selected_tab {
            UpgradeTab::Ships => self.render_ships_tab(frame, chunks[2], state),
            UpgradeTab::Tech => self.render_tech_tab(frame, chunks[2], state),
            UpgradeTab::Fleet => self.render_fleet_tab(frame, chunks[2], state),
        }

        self.render_help(frame, chunks[3]);
    }

    fn render_resources(&self, frame: &mut Frame, area: Rect, state: &GameState) {
        let line = Line::from(vec![
            Span::styled(" ◇ ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{:<8}", state.scrap)),
            Span::styled(" ₿ ", Style::default().fg(Color::Green)),
            Span::raw(format!("{:<8}", state.credits)),
            Span::styled(" BP ", Style::default().fg(Color::Magenta)),
            Span::raw(format!("{:<6}", state.blueprints)),
            Span::styled(" ◈ ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:<6}", state.artifacts)),
            Span::styled(
                format!("  Lv.{}", state.level),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
        ]);
        frame.render_widget(Paragraph::new(line), area);
    }

    fn render_tabs(&self, frame: &mut Frame, area: Rect) {
        let titles: Vec<Line> = UpgradeTab::ALL
            .iter()
            .map(|t| {
                let name = match t {
                    UpgradeTab::Ships => " Ships ",
                    UpgradeTab::Tech => " Tech ",
                    UpgradeTab::Fleet => " Fleet ",
                };
                Line::from(name)
            })
            .collect();

        let tabs = Tabs::new(titles)
            .select(self.selected_tab.index())
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().fg(Color::DarkGray))
            .divider("│");

        frame.render_widget(tabs, area);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help = Line::from(vec![
            Span::styled(" Tab", Style::default().fg(Color::Yellow)),
            Span::styled(" switch  ", Style::default().fg(Color::DarkGray)),
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
            Span::styled(" navigate  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::styled(" buy/upgrade  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::styled(" close", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(help), area);
    }

    // ── Ships tab ───────────────────────────────────────────────────────

    fn render_ships_tab(&self, frame: &mut Frame, area: Rect, state: &GameState) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        // Left: ship list
        let mut items: Vec<ListItem> = Vec::new();
        let fleet_len = state.fleet.len();

        // Existing fleet ships
        for (i, ship) in state.fleet.iter().enumerate() {
            let selected = self.selected_item == i;
            let style = if selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let marker = if selected { "▸ " } else { "  " };
            let line = format!(
                "{}{} Lv.{}  HP:{}/{}  DMG:{}  SPD:{:.0}",
                marker,
                ship.ship_type.name(),
                ship.upgrade_level,
                ship.current_hp,
                ship.max_hp(),
                ship.damage(),
                ship.speed(),
            );
            items.push(ListItem::new(Line::from(line)).style(style));
        }

        // Separator
        items.push(ListItem::new(Line::from(
            "── Build New ──────────────────",
        )).style(Style::default().fg(Color::DarkGray)));

        // Build-new entries (index = fleet_len + type_idx, but the separator is
        // not selectable — we skip it in selection by offsetting)
        for (ti, stype) in SHIP_TYPES.iter().enumerate() {
            let list_idx = fleet_len + ti;
            let selected = self.selected_item == list_idx;
            let unlocked = state.level >= stype.unlock_level();

            let (line, style) = if unlocked {
                let marker = if selected { "▸ " } else { "  " };
                let build_cost = economy::ship_build_cost(*stype, state.fleet.len());
                let cost_str = if build_cost == 0 {
                    "FREE".to_string()
                } else {
                    format!("₿{}", build_cost)
                };
                let affordable = state.credits >= build_cost;
                let cost_color = if affordable { Color::Green } else { Color::Red };
                let line = Line::from(vec![
                    Span::styled(
                        format!("{}+ {} ", marker, stype.name()),
                        if selected {
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::White)
                        },
                    ),
                    Span::styled(cost_str, Style::default().fg(cost_color)),
                ]);
                (
                    line,
                    Style::default(),
                )
            } else {
                let marker = if selected { "▸ " } else { "  " };
                let line = Line::from(format!(
                    "{}🔒 {} — unlock at Lv.{}",
                    marker,
                    stype.name(),
                    stype.unlock_level()
                ));
                (
                    line,
                    Style::default().fg(Color::DarkGray),
                )
            };
            items.push(ListItem::new(line).style(style));
        }

        let list = List::new(items).block(
            Block::default()
                .title(" Fleet ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(list, cols[0]);

        // Right: detail panel for selected item
        self.render_ship_detail(frame, cols[1], state);
    }

    fn render_ship_detail(&self, frame: &mut Frame, area: Rect, state: &GameState) {
        let fleet_len = state.fleet.len();
        let mut lines: Vec<Line> = Vec::new();

        if self.selected_item < fleet_len {
            // Detail for existing ship
            let ship = &state.fleet[self.selected_item];
            let stype = ship.ship_type;

            lines.push(Line::from(""));
            // Sprite
            for sprite_line in stype.sprite() {
                lines.push(Line::from(Span::styled(
                    format!("  {}", sprite_line),
                    Style::default().fg(Color::Cyan),
                )));
            }
            lines.push(Line::from(""));

            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", stype.name()),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("Lv.{}", ship.upgrade_level),
                    Style::default().fg(Color::Yellow),
                ),
            ]));
            lines.push(Line::from(""));

            lines.push(stat_line("  HP", ship.current_hp, ship.max_hp(), Color::Red));
            lines.push(stat_line_single("  DMG", ship.damage(), Color::Magenta));
            lines.push(stat_line_single("  SPD", ship.speed() as u32, Color::Blue));
            lines.push(Line::from(
                format!("  DPS: {:.1}", ship.dps()),
            ));
            lines.push(Line::from(""));

            if ship.upgrade_level < 10 {
                let cost = ship.upgrade_cost();
                let affordable = state.credits >= cost;
                let cost_color = if affordable { Color::Green } else { Color::Red };
                lines.push(Line::from(vec![
                    Span::raw("  Upgrade: "),
                    Span::styled(format!("₿{}", cost), Style::default().fg(cost_color)),
                ]));
            } else {
                lines.push(Line::from(Span::styled(
                    "  ★ MAX LEVEL ★",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )));
            }
        } else {
            // Detail for build-new ship type
            let type_idx = self.selected_item - fleet_len;
            if type_idx < SHIP_TYPES.len() {
                let stype = SHIP_TYPES[type_idx];
                let unlocked = state.level >= stype.unlock_level();

                lines.push(Line::from(""));
                for sprite_line in stype.sprite() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", sprite_line),
                        if unlocked {
                            Style::default().fg(Color::Cyan)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        },
                    )));
                }
                lines.push(Line::from(""));

                lines.push(Line::from(Span::styled(
                    format!("  {}", stype.name()),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));

                lines.push(stat_line_single("  HP", stype.base_hp(), Color::Red));
                lines.push(stat_line_single("  DMG", stype.base_dmg(), Color::Magenta));
                lines.push(stat_line_single(
                    "  SPD",
                    stype.base_speed() as u32,
                    Color::Blue,
                ));
                lines.push(Line::from(""));

                if unlocked {
                    let cost = economy::ship_build_cost(stype, state.fleet.len());
                    let affordable = state.credits >= cost;
                    let cost_color = if affordable { Color::Green } else { Color::Red };
                    let cost_str = if cost == 0 {
                        "FREE".to_string()
                    } else {
                        format!("₿{}", cost)
                    };
                    lines.push(Line::from(vec![
                        Span::raw("  Build: "),
                        Span::styled(cost_str, Style::default().fg(cost_color)),
                    ]));
                } else {
                    lines.push(Line::from(Span::styled(
                        format!("  🔒 Requires Lv.{}", stype.unlock_level()),
                        Style::default().fg(Color::Red),
                    )));
                }
            }
        }

        let detail = Paragraph::new(lines).block(
            Block::default()
                .title(" Details ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(detail, area);
    }

    // ── Tech tab ────────────────────────────────────────────────────────

    fn render_tech_tab(&self, frame: &mut Frame, area: Rect, state: &GameState) {
        let techs: [(& str, u8, Color); 4] = [
            ("Lasers", state.tech_lasers, Color::Red),
            ("Shields", state.tech_shields, Color::Cyan),
            ("Engines", state.tech_engines, Color::Blue),
            ("Beams", state.tech_beams, Color::Magenta),
        ];

        // Each tech gets 3 rows: label, gauge, spacing
        let constraints: Vec<Constraint> = techs
            .iter()
            .flat_map(|_| [Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
            .chain(std::iter::once(Constraint::Min(0)))
            .collect();

        let inner_block = Block::default()
            .title(" Tech Tree ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        let inner_area = inner_block.inner(area);
        frame.render_widget(inner_block, area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner_area);

        for (i, (name, level, color)) in techs.iter().enumerate() {
            let selected = self.selected_item == i;
            let row_label = rows[i * 3];
            let row_gauge = rows[i * 3 + 1];

            let marker = if selected { "▸ " } else { "  " };
            let label_style = if selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let maxed = *level >= TECH_MAX_LEVEL;
            let cost_span = if maxed {
                Span::styled(" ★ MAX", Style::default().fg(Color::Yellow))
            } else {
                let cost = economy::tech_upgrade_cost(*level);
                let affordable = state.credits >= cost;
                let cost_color = if affordable { Color::Green } else { Color::Red };
                Span::styled(format!(" ₿{}", cost), Style::default().fg(cost_color))
            };

            let label_line = Line::from(vec![
                Span::styled(format!("{}{}", marker, name), label_style),
                Span::styled(
                    format!("  Lv {}/{}", level, TECH_MAX_LEVEL),
                    Style::default().fg(Color::DarkGray),
                ),
                cost_span,
            ]);
            frame.render_widget(Paragraph::new(label_line), row_label);

            // Progress gauge
            let filled = (*level as usize).min(TECH_MAX_LEVEL as usize);
            let empty = (TECH_MAX_LEVEL as usize) - filled;
            let bar_str = format!("  {}{}",
                "█".repeat(filled),
                "░".repeat(empty),
            );
            let gauge_line = Line::from(Span::styled(bar_str, Style::default().fg(*color)));
            frame.render_widget(Paragraph::new(gauge_line), row_gauge);
        }
    }

    // ── Fleet tab ───────────────────────────────────────────────────────

    fn render_fleet_tab(&self, frame: &mut Frame, area: Rect, state: &GameState) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        // Left: fleet stats summary
        let total_hp = state.fleet_max_hp();
        let current_hp = state.fleet_total_hp();
        let total_dps = state.fleet_total_dps();
        let ship_count = state.fleet.len();

        // Count by type
        let mut type_counts: Vec<(&str, usize)> = Vec::new();
        for stype in &SHIP_TYPES {
            let count = state.fleet.iter().filter(|s| s.ship_type == *stype).count();
            if count > 0 {
                type_counts.push((stype.name(), count));
            }
        }

        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Fleet Summary",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(format!("  Ships:  {}", ship_count)));
        lines.push(Line::from(vec![
            Span::raw("  HP:     "),
            Span::styled(
                format!("{}/{}", current_hp, total_hp),
                Style::default().fg(Color::Red),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  DPS:    "),
            Span::styled(
                format!("{:.1}", total_dps),
                Style::default().fg(Color::Magenta),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  Sector: "),
            Span::styled(
                format!("{}", state.sector),
                Style::default().fg(Color::Yellow),
            ),
        ]));
        lines.push(Line::from(""));

        lines.push(Line::from(Span::styled(
            "  Composition",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )));
        for (name, count) in &type_counts {
            lines.push(Line::from(format!("    {}x {}", count, name)));
        }

        let stats = Paragraph::new(lines).block(
            Block::default()
                .title(" Stats ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(stats, cols[0]);

        // Right: formation display with ASCII sprites
        let mut formation_lines: Vec<Line> = Vec::new();
        formation_lines.push(Line::from(""));
        formation_lines.push(Line::from(Span::styled(
            "  Formation",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )));
        formation_lines.push(Line::from(""));

        // Render each ship's sprite in a staggered formation
        let max_display = 8; // limit to avoid overflow
        let display_ships = if state.fleet.len() > max_display {
            &state.fleet[..max_display]
        } else {
            &state.fleet
        };

        for (i, ship) in display_ships.iter().enumerate() {
            let sprite = ship.ship_type.sprite();
            // Indent to create staggered formation look
            let indent = if i % 2 == 0 { "    " } else { "        " };
            for sprite_line in sprite {
                let color = if ship.is_alive() {
                    Color::Cyan
                } else {
                    Color::DarkGray
                };
                formation_lines.push(Line::from(Span::styled(
                    format!("{}{}", indent, sprite_line),
                    Style::default().fg(color),
                )));
            }
            // Small gap between ships
            if sprite.len() == 1 {
                formation_lines.push(Line::from(""));
            }
        }

        if state.fleet.len() > max_display {
            formation_lines.push(Line::from(""));
            formation_lines.push(Line::from(Span::styled(
                format!("    ...and {} more", state.fleet.len() - max_display),
                Style::default().fg(Color::DarkGray),
            )));
        }

        let formation = Paragraph::new(formation_lines).block(
            Block::default()
                .title(" Formation ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(formation, cols[1]);
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn stat_line<'a>(label: &'a str, current: u32, max: u32, color: Color) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{}: ", label), Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{}/{}", current, max), Style::default().fg(color)),
    ])
}

fn stat_line_single<'a>(label: &'a str, value: u32, color: Color) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{}: ", label), Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{}", value), Style::default().fg(color)),
    ])
}


