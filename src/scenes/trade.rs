use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::engine::factions::Faction;
use crate::engine::trade::{SectorMarket, TradeGood, generate_market};
use crate::rendering::layout::centered_rect;
use crate::state::GameState;

// ── Trade mode ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TradeMode {
    Buy,
    Sell,
}

// ── TradeScreen ─────────────────────────────────────────────────────────────

pub struct TradeScreen {
    pub open: bool,
    selected: usize,
    quantity: u32,
    mode: TradeMode,
    market: Option<SectorMarket>,
    faction: Faction,
    last_sector: u32,
}

impl TradeScreen {
    pub fn new() -> Self {
        Self {
            open: false,
            selected: 0,
            quantity: 1,
            mode: TradeMode::Buy,
            market: None,
            faction: Faction::Independent,
            last_sector: 0,
        }
    }

    pub fn toggle(&mut self, state: &GameState) {
        self.open = !self.open;
        if self.open {
            self.selected = 0;
            self.quantity = 1;
            self.mode = TradeMode::Buy;
            if self.market.is_none() || self.last_sector != state.sector {
                let faction = state.sector_faction(state.sector);
                self.market = Some(generate_market(state.sector, &faction));
                self.faction = faction;
                self.last_sector = state.sector;
            }
        }
    }

    // ── Input ───────────────────────────────────────────────────────

    pub fn handle_input(&mut self, key: KeyCode, state: &mut GameState) {
        let count = TradeGood::ALL.len();

        match key {
            KeyCode::Esc => { self.open = false; }
            KeyCode::Up => {
                if count > 0 {
                    self.selected = if self.selected == 0 { count - 1 } else { self.selected - 1 };
                    self.quantity = 1;
                }
            }
            KeyCode::Down => {
                if count > 0 {
                    self.selected = (self.selected + 1) % count;
                    self.quantity = 1;
                }
            }
            KeyCode::Right => { self.quantity = self.quantity.saturating_add(1).min(99); }
            KeyCode::Left => { self.quantity = self.quantity.saturating_sub(1).max(1); }
            KeyCode::Char('s') | KeyCode::Char('S') => { self.mode = TradeMode::Sell; }
            KeyCode::Enter => {
                match self.mode {
                    TradeMode::Buy => self.execute_buy(state),
                    TradeMode::Sell => {
                        self.execute_sell(state);
                        self.mode = TradeMode::Buy;
                    }
                }
            }
            _ => {}
        }
    }

    fn execute_buy(&mut self, state: &mut GameState) {
        let market = match self.market.as_ref() {
            Some(m) => m,
            None => return,
        };
        if self.selected >= TradeGood::ALL.len() { return; }
        let good = TradeGood::ALL[self.selected];

        let buy_price = match market.buy_price(good) {
            Some(p) => p,
            None => return,
        };

        let qty = self.quantity.min(state.cargo_space_remaining());
        if qty == 0 { return; }

        state.buy_goods(good, qty, buy_price);
        self.quantity = 1;
    }

    fn execute_sell(&mut self, state: &mut GameState) {
        let market = match self.market.as_ref() {
            Some(m) => m,
            None => return,
        };
        if self.selected >= TradeGood::ALL.len() { return; }
        let good = TradeGood::ALL[self.selected];

        let sell_price = match market.sell_price(good) {
            Some(p) => p,
            None => return,
        };

        let held = state.cargo.get(good.key()).copied().unwrap_or(0);
        let qty = self.quantity.min(held);
        if qty == 0 { return; }

        state.sell_goods(good, qty, sell_price);
        self.quantity = 1;
    }

    // ── Rendering ───────────────────────────────────────────────────

    pub fn render(&self, frame: &mut Frame, state: &GameState) {
        if !self.open { return; }

        let area = centered_rect(70, 80, frame.area());
        frame.render_widget(Clear, area);

        let outer = Block::default()
            .title(" \u{20bf} MARKET \u{20bf} ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .style(Style::default().bg(Color::Black));

        let inner = outer.inner(area);
        frame.render_widget(outer, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // header
                Constraint::Length(2), // column headers
                Constraint::Min(5),   // listings
                Constraint::Length(3), // footer
                Constraint::Length(1), // help
            ])
            .split(inner);

        let market = match &self.market {
            Some(m) => m,
            None => {
                let msg = Paragraph::new(Line::from(Span::styled(
                    "  No market data available",
                    Style::default().fg(Color::DarkGray),
                )));
                frame.render_widget(msg, inner);
                return;
            }
        };

        self.render_header(frame, chunks[0], state);
        self.render_col_headers(frame, chunks[1]);
        self.render_listings(frame, chunks[2], market, state);
        self.render_footer(frame, chunks[3], state, market);
        self.render_help(frame, chunks[4]);
    }

    fn render_header(&self, frame: &mut Frame, area: Rect, state: &GameState) {
        let faction_name = self.faction.info().name;
        let cargo_used = state.cargo_total();
        let cargo_cap = state.cargo_capacity;

        let bar_width = 10u16;
        let filled = if cargo_cap > 0 {
            ((cargo_used as u64 * bar_width as u64) / cargo_cap as u64).min(bar_width as u64) as u16
        } else { 0 };
        let bar: String = format!(
            "{}{}",
            "\u{2588}".repeat(filled as usize),
            "\u{2591}".repeat((bar_width - filled) as usize),
        );
        let bar_color = if cargo_used >= cargo_cap { Color::Red }
            else if cargo_used as f32 >= cargo_cap as f32 * 0.8 { Color::Yellow }
            else { Color::Green };

        let lines = vec![
            Line::from(vec![
                Span::styled(
                    format!("  Sector {} ", self.last_sector),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("\u{2014} {} ", faction_name),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Cargo: ", Style::default().fg(Color::DarkGray)),
                Span::styled(bar, Style::default().fg(bar_color)),
                Span::styled(format!(" {}/{}", cargo_used, cargo_cap), Style::default().fg(Color::White)),
            ]),
        ];
        frame.render_widget(Paragraph::new(lines), area);
    }

    fn render_col_headers(&self, frame: &mut Frame, area: Rect) {
        let mode_indicator = match self.mode {
            TradeMode::Buy => Span::styled(
                " [BUY] ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
            TradeMode::Sell => Span::styled(
                " [SELL] ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
        };

        let header = Line::from(vec![
            Span::styled("  Good        ", Style::default().fg(Color::DarkGray)),
            Span::styled("Buy   ", Style::default().fg(Color::DarkGray)),
            Span::styled("Sell  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Supply ", Style::default().fg(Color::DarkGray)),
            Span::styled("Held  ", Style::default().fg(Color::DarkGray)),
            mode_indicator,
        ]);
        let sep = Line::from(Span::styled(
            "  \u{2500}".repeat(22),
            Style::default().fg(Color::DarkGray),
        ));
        frame.render_widget(Paragraph::new(vec![header, sep]), area);
    }

    fn render_listings(&self, frame: &mut Frame, area: Rect, market: &SectorMarket, state: &GameState) {
        let mut lines: Vec<Line> = Vec::new();

        for (i, &good) in TradeGood::ALL.iter().enumerate() {
            let is_selected = self.selected == i;
            let marker = if is_selected { "\u{25b8} " } else { "  " };
            let held = state.cargo.get(good.key()).copied().unwrap_or(0);
            let available = market.is_available(good);

            let (buy_str, sell_str, supply_label, price_color) = if available {
                let mp = market.prices.get(&good).unwrap();
                let supply_label = match mp.supply {
                    crate::engine::trade::Supply::Surplus => "Surplus",
                    crate::engine::trade::Supply::Normal => "Normal",
                    crate::engine::trade::Supply::Scarce => "Scarce",
                };
                let color = match mp.supply {
                    crate::engine::trade::Supply::Surplus => Color::Green,
                    crate::engine::trade::Supply::Normal => Color::White,
                    crate::engine::trade::Supply::Scarce => Color::Red,
                };
                (format!("{:<5}", mp.buy_price), format!("{:<5}", mp.sell_price), supply_label, color)
            } else {
                (" --  ".to_string(), " --  ".to_string(), "N/A", Color::DarkGray)
            };

            let name_style = if is_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let line = Line::from(vec![
                Span::styled(marker, Style::default().fg(Color::Yellow)),
                Span::styled(format!("{} ", good.icon()), Style::default().fg(price_color)),
                Span::styled(format!("{:<10}", good.name()), name_style),
                Span::styled(buy_str, if available { Style::default().fg(price_color) } else { Style::default().fg(Color::DarkGray) }),
                Span::raw("  "),
                Span::styled(sell_str, Style::default().fg(Color::White)),
                Span::raw(" "),
                Span::styled(format!("{:<7}", supply_label), Style::default().fg(price_color)),
                Span::styled(
                    format!("{}", held),
                    if held > 0 { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::DarkGray) },
                ),
            ]);
            lines.push(line);

            if !available && is_selected {
                lines.push(Line::from(Span::styled(
                    "      \u{26a0} Not traded here",
                    Style::default().fg(Color::Red).add_modifier(Modifier::DIM),
                )));
            }
        }

        frame.render_widget(Paragraph::new(lines), area);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect, state: &GameState, market: &SectorMarket) {
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(vec![
            Span::styled("  \u{20bf} Credits: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", state.credits),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]));

        if self.selected < TradeGood::ALL.len() {
            let good = TradeGood::ALL[self.selected];
            let held = state.cargo.get(good.key()).copied().unwrap_or(0);

            if held > 0 {
                if let (Some(buy), Some(sell)) = (market.buy_price(good), market.sell_price(good)) {
                    let sell_total = sell * held as u64;
                    let buy_total = buy * held as u64;
                    if sell_total >= buy_total {
                        lines.push(Line::from(vec![
                            Span::styled("  Profit: ", Style::default().fg(Color::DarkGray)),
                            Span::styled(
                                format!("+{}\u{20bf}", sell_total.saturating_sub(buy_total)),
                                Style::default().fg(Color::Green),
                            ),
                            Span::styled(
                                format!(" ({}x @ {})", held, sell),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ]));
                    } else {
                        lines.push(Line::from(vec![
                            Span::styled("  Loss: ", Style::default().fg(Color::DarkGray)),
                            Span::styled(
                                format!("-{}\u{20bf}", buy_total.saturating_sub(sell_total)),
                                Style::default().fg(Color::Red),
                            ),
                            Span::styled(
                                format!(" ({}x @ {})", held, sell),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ]));
                    }
                }
            } else if let Some(buy) = market.buy_price(good) {
                let qty = self.quantity;
                let cost = buy * qty as u64;
                let affordable = state.credits >= cost;
                lines.push(Line::from(vec![
                    Span::styled(format!("  Buy {}x: ", qty), Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{}\u{20bf}", cost),
                        Style::default().fg(if affordable { Color::Green } else { Color::Red }),
                    ),
                ]));
            }
        }

        frame.render_widget(Paragraph::new(lines), area);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help = Line::from(vec![
            Span::styled(" \u{2191}\u{2193}", Style::default().fg(Color::Yellow)),
            Span::styled("Select ", Style::default().fg(Color::DarkGray)),
            Span::styled("\u{2190}\u{2192}", Style::default().fg(Color::Yellow)),
            Span::styled("Qty ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::styled(" Buy ", Style::default().fg(Color::DarkGray)),
            Span::styled("S", Style::default().fg(Color::Yellow)),
            Span::styled("ell ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::styled(" Close", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(help), area);
    }
}
