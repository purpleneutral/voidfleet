use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs},
};

use crate::engine::equipment::{Equipment, SET_BONUSES, Slot};
use crate::engine::ship::Ship;
use crate::rendering::layout::centered_rect;
use crate::state::GameState;

// ── Tab enum ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InvTab {
    Fleet,
    Inventory,
    Salvage,
}

impl InvTab {
    const ALL: [InvTab; 3] = [InvTab::Fleet, InvTab::Inventory, InvTab::Salvage];

    fn index(self) -> usize {
        match self {
            Self::Fleet => 0,
            Self::Inventory => 1,
            Self::Salvage => 2,
        }
    }

    fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    fn prev(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

// ── Equip picker sub-state ──────────────────────────────────────────────────

/// When the player selects a slot on the Fleet tab, we show a picker list
/// of matching inventory items.
#[derive(Debug, Clone)]
struct EquipPicker {
    ship_idx: usize,
    slot: Slot,
    selected: usize,
    /// Cached list of inventory item IDs that match this slot.
    matching_ids: Vec<u64>,
}

// ── InventoryScreen ─────────────────────────────────────────────────────────

pub struct InventoryScreen {
    pub open: bool,
    tab: InvTab,
    selected_ship: usize,
    /// Cursor within the currently displayed list (slot index for Fleet,
    /// item index for Inventory/Salvage).
    selected_item: usize,
    detail_open: bool,
    /// Active equip-picker overlay (Fleet tab only).
    equip_picker: Option<EquipPicker>,
    /// Salvage tab: set of item IDs marked for salvage.
    salvage_selected: Vec<u64>,
}

impl InventoryScreen {
    pub fn new() -> Self {
        Self {
            open: false,
            tab: InvTab::Fleet,
            selected_ship: 0,
            selected_item: 0,
            detail_open: false,
            equip_picker: None,
            salvage_selected: Vec::new(),
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.tab = InvTab::Fleet;
            self.selected_ship = 0;
            self.selected_item = 0;
            self.detail_open = false;
            self.equip_picker = None;
            self.salvage_selected.clear();
        }
    }

    // ── Item counts per tab ─────────────────────────────────────────

    fn fleet_slot_count() -> usize {
        Slot::ALL.len() // 4 slots per ship
    }

    fn inventory_count(state: &GameState) -> usize {
        state.inventory.len()
    }

    // ── Input handling ──────────────────────────────────────────────

    pub fn handle_input(&mut self, key: KeyCode, state: &mut GameState) {
        // If equip picker is open, route input there
        if let Some(ref mut picker) = self.equip_picker {
            match key {
                KeyCode::Esc => {
                    self.equip_picker = None;
                }
                KeyCode::Up => {
                    let count = picker.matching_ids.len() + 1; // +1 for "Unequip" option
                    picker.selected = if picker.selected == 0 {
                        count - 1
                    } else {
                        picker.selected - 1
                    };
                }
                KeyCode::Down => {
                    let count = picker.matching_ids.len() + 1;
                    picker.selected = (picker.selected + 1) % count;
                }
                KeyCode::Enter => {
                    self.confirm_equip(state);
                }
                _ => {}
            }
            return;
        }

        match key {
            KeyCode::Esc => {
                self.open = false;
            }
            KeyCode::Tab => {
                self.tab = self.tab.next();
                self.selected_item = 0;
                self.salvage_selected.clear();
            }
            KeyCode::BackTab => {
                self.tab = self.tab.prev();
                self.selected_item = 0;
                self.salvage_selected.clear();
            }
            KeyCode::Up => {
                let count = self.current_list_len(state);
                if count > 0 {
                    self.selected_item = if self.selected_item == 0 {
                        count - 1
                    } else {
                        self.selected_item - 1
                    };
                }
            }
            KeyCode::Down => {
                let count = self.current_list_len(state);
                if count > 0 {
                    self.selected_item = (self.selected_item + 1) % count;
                }
            }
            KeyCode::Left => {
                if self.tab == InvTab::Fleet && !state.fleet.is_empty() {
                    self.selected_ship = if self.selected_ship == 0 {
                        state.fleet.len() - 1
                    } else {
                        self.selected_ship - 1
                    };
                    self.selected_item = 0;
                }
            }
            KeyCode::Right => {
                if self.tab == InvTab::Fleet && !state.fleet.is_empty() {
                    self.selected_ship = (self.selected_ship + 1) % state.fleet.len();
                    self.selected_item = 0;
                }
            }
            KeyCode::Enter => match self.tab {
                InvTab::Fleet => self.open_equip_picker(state),
                InvTab::Inventory => {} // detail view only
                InvTab::Salvage => self.confirm_salvage(state),
            },
            KeyCode::Char(' ') => {
                if self.tab == InvTab::Salvage {
                    self.toggle_salvage_selection(state);
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.quick_salvage(state);
            }
            _ => {}
        }
    }

    fn current_list_len(&self, state: &GameState) -> usize {
        match self.tab {
            InvTab::Fleet => Self::fleet_slot_count(),
            InvTab::Inventory => Self::inventory_count(state),
            InvTab::Salvage => Self::inventory_count(state),
        }
    }

    // ── Fleet tab: equip picker ─────────────────────────────────────

    fn open_equip_picker(&mut self, state: &GameState) {
        if state.fleet.is_empty() {
            return;
        }
        let ship_idx = self.selected_ship.min(state.fleet.len() - 1);
        let slot = Slot::ALL[self.selected_item.min(Slot::ALL.len() - 1)];

        // Gather inventory items matching this slot
        let matching_ids: Vec<u64> = state
            .inventory
            .iter()
            .filter(|e| e.slot == slot)
            .map(|e| e.id)
            .collect();

        self.equip_picker = Some(EquipPicker {
            ship_idx,
            slot,
            selected: 0,
            matching_ids,
        });
    }

    fn confirm_equip(&mut self, state: &mut GameState) {
        let picker = match self.equip_picker.take() {
            Some(p) => p,
            None => return,
        };

        if picker.ship_idx >= state.fleet.len() {
            return;
        }

        if picker.selected == 0 {
            // "Unequip" option — remove item from slot, put back in inventory
            let ship = &mut state.fleet[picker.ship_idx];
            let slot_ref = match picker.slot {
                Slot::Weapon => &mut ship.weapon,
                Slot::Shield => &mut ship.shield,
                Slot::Engine => &mut ship.engine_mod,
                Slot::Special => &mut ship.special,
            };
            if let Some(old_item) = slot_ref.take() {
                state.inventory.push(old_item);
            }
        } else {
            // Equip selected item (picker.selected - 1 indexes into matching_ids)
            let pick_idx = picker.selected - 1;
            if pick_idx >= picker.matching_ids.len() {
                return;
            }
            let item_id = picker.matching_ids[pick_idx];

            // Remove item from inventory
            if let Some(pos) = state.inventory.iter().position(|i| i.id == item_id) {
                let item = state.inventory.remove(pos);

                // Equip on ship (returns old item if any)
                let old = state.fleet[picker.ship_idx].equip(item);

                // Put old item back into inventory
                if let Some(old_item) = old {
                    state.inventory.push(old_item);
                }
            }
        }
    }

    // ── Salvage tab ─────────────────────────────────────────────────

    fn toggle_salvage_selection(&mut self, state: &GameState) {
        let sorted = sorted_inventory(state);
        if self.selected_item >= sorted.len() {
            return;
        }
        let id = sorted[self.selected_item].id;
        if let Some(pos) = self.salvage_selected.iter().position(|&sid| sid == id) {
            self.salvage_selected.remove(pos);
        } else {
            self.salvage_selected.push(id);
        }
    }

    fn confirm_salvage(&mut self, state: &mut GameState) {
        if self.salvage_selected.is_empty() {
            return;
        }
        let ids: Vec<u64> = self.salvage_selected.drain(..).collect();
        for id in ids {
            state.salvage_item(id);
        }
        // Clamp cursor
        if self.selected_item > 0 && self.selected_item >= state.inventory.len() {
            self.selected_item = state.inventory.len().saturating_sub(1);
        }
    }

    fn quick_salvage(&mut self, state: &mut GameState) {
        if self.tab != InvTab::Inventory && self.tab != InvTab::Salvage {
            return;
        }
        let sorted = sorted_inventory(state);
        if self.selected_item >= sorted.len() {
            return;
        }
        let id = sorted[self.selected_item].id;
        state.salvage_item(id);
        // Clamp cursor
        if self.selected_item > 0 && self.selected_item >= state.inventory.len() {
            self.selected_item = state.inventory.len().saturating_sub(1);
        }
    }

    // ── Rendering ───────────────────────────────────────────────────

    pub fn render(&self, frame: &mut Frame, state: &GameState) {
        if !self.open {
            return;
        }

        let area = centered_rect(85, 85, frame.area());
        frame.render_widget(Clear, area);

        let outer_block = Block::default()
            .title(" \u{2694} EQUIPMENT \u{2694} ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
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
            InvTab::Fleet => self.render_fleet_tab(frame, chunks[2], state),
            InvTab::Inventory => self.render_inventory_tab(frame, chunks[2], state),
            InvTab::Salvage => self.render_salvage_tab(frame, chunks[2], state),
        }

        self.render_help(frame, chunks[3]);

        // Equip picker overlay
        if let Some(ref picker) = self.equip_picker {
            self.render_equip_picker(frame, area, picker, state);
        }
    }

    fn render_resources(&self, frame: &mut Frame, area: Rect, state: &GameState) {
        let line = Line::from(vec![
            Span::styled(" \u{25c7} ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{:<8}", state.scrap)),
            Span::styled(" \u{20bf} ", Style::default().fg(Color::Green)),
            Span::raw(format!("{:<8}", state.credits)),
            Span::styled(
                format!("  Inv: {}/{}", state.inventory.len(), state.inventory_capacity),
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
        let titles: Vec<Line> = InvTab::ALL
            .iter()
            .map(|t| {
                let name = match t {
                    InvTab::Fleet => " Fleet ",
                    InvTab::Inventory => " Inventory ",
                    InvTab::Salvage => " Salvage ",
                };
                Line::from(name)
            })
            .collect();

        let tabs = Tabs::new(titles)
            .select(self.tab.index())
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().fg(Color::DarkGray))
            .divider("\u{2502}"); // │

        frame.render_widget(tabs, area);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help_spans = match self.tab {
            InvTab::Fleet => vec![
                Span::styled(" \u{2190}\u{2192}", Style::default().fg(Color::Yellow)),
                Span::styled(" ship  ", Style::default().fg(Color::DarkGray)),
                Span::styled("\u{2191}\u{2193}", Style::default().fg(Color::Yellow)),
                Span::styled(" slot  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::styled(" equip  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Tab", Style::default().fg(Color::Yellow)),
                Span::styled(" switch  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Esc", Style::default().fg(Color::Yellow)),
                Span::styled(" close", Style::default().fg(Color::DarkGray)),
            ],
            InvTab::Inventory => vec![
                Span::styled(" \u{2191}\u{2193}", Style::default().fg(Color::Yellow)),
                Span::styled(" browse  ", Style::default().fg(Color::DarkGray)),
                Span::styled("D", Style::default().fg(Color::Yellow)),
                Span::styled(" salvage  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Tab", Style::default().fg(Color::Yellow)),
                Span::styled(" switch  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Esc", Style::default().fg(Color::Yellow)),
                Span::styled(" close", Style::default().fg(Color::DarkGray)),
            ],
            InvTab::Salvage => vec![
                Span::styled(" \u{2191}\u{2193}", Style::default().fg(Color::Yellow)),
                Span::styled(" browse  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Space", Style::default().fg(Color::Yellow)),
                Span::styled(" select  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::styled(" confirm  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Esc", Style::default().fg(Color::Yellow)),
                Span::styled(" close", Style::default().fg(Color::DarkGray)),
            ],
        };
        frame.render_widget(Paragraph::new(Line::from(help_spans)), area);
    }

    // ── Fleet tab ───────────────────────────────────────────────────

    fn render_fleet_tab(&self, frame: &mut Frame, area: Rect, state: &GameState) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);

        // Left: ship + slots
        self.render_fleet_ship(frame, cols[0], state);

        // Right: detail panel for highlighted slot
        self.render_slot_detail(frame, cols[1], state);
    }

    fn render_fleet_ship(&self, frame: &mut Frame, area: Rect, state: &GameState) {
        let mut lines: Vec<Line> = Vec::new();

        if state.fleet.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No ships in fleet!",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            let ship_idx = self.selected_ship.min(state.fleet.len() - 1);
            let ship = &state.fleet[ship_idx];

            // Ship navigation header
            let nav = format!(
                "  \u{25c0} Ship {}/{} \u{25b6}",
                ship_idx + 1,
                state.fleet.len()
            );
            lines.push(Line::from(Span::styled(
                nav,
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(""));

            // Ship sprite + name
            for sprite_line in ship.ship_type.sprite() {
                lines.push(Line::from(Span::styled(
                    format!("  {}", sprite_line),
                    Style::default().fg(Color::Cyan),
                )));
            }
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", ship.ship_type.name()),
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

            // Equipment slots
            for (slot_idx, slot) in Slot::ALL.iter().enumerate() {
                let is_selected = self.selected_item == slot_idx;
                let equipped = get_ship_slot(ship, *slot);

                let marker = if is_selected { "\u{25b8} " } else { "  " };

                let slot_line = match equipped {
                    Some(item) => {
                        let rarity_color = item.rarity.color();
                        let style = if is_selected {
                            Style::default()
                                .fg(rarity_color)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(rarity_color)
                        };
                        Line::from(vec![
                            Span::styled(
                                format!("  {}{} ", marker, slot.icon()),
                                if is_selected {
                                    Style::default()
                                        .fg(Color::Yellow)
                                        .add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default().fg(Color::White)
                                },
                            ),
                            Span::styled(&item.name, style),
                        ])
                    }
                    None => {
                        let style = if is_selected {
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        };
                        Line::from(Span::styled(
                            format!("  {}{} [Empty {} Slot]", marker, slot.icon(), slot.name()),
                            style,
                        ))
                    }
                };
                lines.push(slot_line);

                // Sub-line: stat summary for equipped item
                if let Some(item) = equipped {
                    let summary = item.summary();
                    lines.push(Line::from(Span::styled(
                        format!("      {} \u{2502} {}", item.rarity.name(), summary),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }
        }

        let block = Block::default()
            .title(format!(" {} ", if state.fleet.is_empty() { "Fleet" } else { state.fleet[self.selected_ship.min(state.fleet.len().saturating_sub(1))].ship_type.name() }))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(Paragraph::new(lines).block(block), area);
    }

    fn render_slot_detail(&self, frame: &mut Frame, area: Rect, state: &GameState) {
        let mut lines: Vec<Line> = Vec::new();

        if !state.fleet.is_empty() {
            let ship_idx = self.selected_ship.min(state.fleet.len() - 1);
            let ship = &state.fleet[ship_idx];
            let slot = Slot::ALL[self.selected_item.min(Slot::ALL.len() - 1)];
            let equipped = get_ship_slot(ship, slot);

            if let Some(item) = equipped {
                lines.extend(render_item_detail(item));
            } else {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!("  Empty {} Slot", slot.name()),
                    Style::default().fg(Color::DarkGray),
                )));
                lines.push(Line::from(""));

                // Show how many matching items are in inventory
                let matching = state
                    .inventory
                    .iter()
                    .filter(|e| e.slot == slot)
                    .count();
                lines.push(Line::from(Span::styled(
                    format!("  {} matching items in inventory", matching),
                    Style::default().fg(Color::DarkGray),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Press Enter to equip",
                    Style::default().fg(Color::Yellow),
                )));
            }
        }

        let block = Block::default()
            .title(" Details ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(Paragraph::new(lines).block(block), area);
    }

    // ── Inventory tab ───────────────────────────────────────────────

    fn render_inventory_tab(&self, frame: &mut Frame, area: Rect, state: &GameState) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Left: item list
        let sorted = sorted_inventory(state);
        let mut items: Vec<ListItem> = Vec::new();

        for (i, item) in sorted.iter().enumerate() {
            let is_selected = self.selected_item == i;
            let rarity_color = item.rarity.color();

            let marker = if is_selected { "\u{25b8} " } else { "  " };
            let name_style = if is_selected {
                Style::default()
                    .fg(rarity_color)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(rarity_color)
            };

            let line = Line::from(vec![
                Span::styled(marker, Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{} ", item.rarity.icon()),
                    Style::default().fg(rarity_color),
                ),
                Span::styled(&item.name, name_style),
            ]);
            items.push(ListItem::new(vec![
                line,
                Line::from(Span::styled(
                    format!(
                        "    {} \u{2502} Lv.{} \u{2502} {}",
                        item.slot.name(),
                        item.level,
                        item.summary()
                    ),
                    Style::default().fg(Color::DarkGray),
                )),
            ]));
        }

        let title = format!(
            " Inventory ({}/{}) ",
            state.inventory.len(),
            state.inventory_capacity
        );
        let list = List::new(items).block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(list, cols[0]);

        // Right: detail panel
        let sorted = sorted_inventory(state);
        let mut detail_lines: Vec<Line> = Vec::new();
        if self.selected_item < sorted.len() {
            detail_lines = render_item_detail(sorted[self.selected_item]);
        } else {
            detail_lines.push(Line::from(""));
            detail_lines.push(Line::from(Span::styled(
                "  No items",
                Style::default().fg(Color::DarkGray),
            )));
        }

        let detail_block = Block::default()
            .title(" Item Detail ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(Paragraph::new(detail_lines).block(detail_block), cols[1]);
    }

    // ── Salvage tab ─────────────────────────────────────────────────

    fn render_salvage_tab(&self, frame: &mut Frame, area: Rect, state: &GameState) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        let sorted = sorted_inventory(state);
        let mut items: Vec<ListItem> = Vec::new();

        for (i, item) in sorted.iter().enumerate() {
            let is_selected = self.selected_item == i;
            let is_marked = self.salvage_selected.contains(&item.id);
            let rarity_color = item.rarity.color();

            let checkbox = if is_marked { "[x]" } else { "[ ]" };
            let marker = if is_selected { "\u{25b8}" } else { " " };

            let name_style = if is_selected {
                Style::default()
                    .fg(rarity_color)
                    .add_modifier(Modifier::BOLD)
            } else if is_marked {
                Style::default()
                    .fg(Color::Red)
                    .add_modifier(Modifier::DIM)
            } else {
                Style::default().fg(rarity_color)
            };

            let salvage_val = item.salvage_value();

            let line = Line::from(vec![
                Span::styled(
                    format!("{} {} ", marker, checkbox),
                    if is_marked {
                        Style::default().fg(Color::Red)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
                Span::styled(
                    format!("{} ", item.rarity.icon()),
                    Style::default().fg(rarity_color),
                ),
                Span::styled(&item.name, name_style),
                Span::styled(
                    format!("  \u{2192} {}\u{25c7}", salvage_val),
                    Style::default().fg(Color::Yellow),
                ),
            ]);
            items.push(ListItem::new(line));
        }

        // Total salvage value
        let total_value: u64 = sorted
            .iter()
            .filter(|item| self.salvage_selected.contains(&item.id))
            .map(|item| item.salvage_value())
            .sum();

        let title = " Select items to salvage ".to_string();
        let list = List::new(items).block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(list, cols[0]);

        // Right: salvage summary
        let mut summary_lines: Vec<Line> = Vec::new();
        summary_lines.push(Line::from(""));
        summary_lines.push(Line::from(Span::styled(
            "  Salvage Summary",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )));
        summary_lines.push(Line::from(""));
        summary_lines.push(Line::from(vec![
            Span::styled("  Items: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", self.salvage_selected.len()),
                Style::default().fg(Color::White),
            ),
        ]));
        summary_lines.push(Line::from(vec![
            Span::styled("  Total: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}\u{25c7}", total_value),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        summary_lines.push(Line::from(""));

        if self.salvage_selected.is_empty() {
            summary_lines.push(Line::from(Span::styled(
                "  Use Space to select items",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            summary_lines.push(Line::from(Span::styled(
                "  Press Enter to confirm",
                Style::default().fg(Color::Green),
            )));
        }

        // List marked items
        summary_lines.push(Line::from(""));
        for id in &self.salvage_selected {
            if let Some(item) = sorted.iter().find(|e| e.id == *id) {
                summary_lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {} ", item.rarity.icon()),
                        Style::default().fg(item.rarity.color()),
                    ),
                    Span::styled(
                        format!("{} ", item.name),
                        Style::default().fg(item.rarity.color()),
                    ),
                    Span::styled(
                        format!("{}\u{25c7}", item.salvage_value()),
                        Style::default().fg(Color::Yellow),
                    ),
                ]));
            }
        }

        let summary_block = Block::default()
            .title(" Scrap Yield ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(
            Paragraph::new(summary_lines).block(summary_block),
            cols[1],
        );
    }

    // ── Equip picker overlay ────────────────────────────────────────

    fn render_equip_picker(
        &self,
        frame: &mut Frame,
        parent_area: Rect,
        picker: &EquipPicker,
        state: &GameState,
    ) {
        // Small centered overlay
        let popup = centered_rect(60, 60, parent_area);
        frame.render_widget(Clear, popup);

        let title = format!(" Equip {} ", picker.slot.name());
        let block = Block::default()
            .title(title)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .style(Style::default().bg(Color::Black));
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(""));

        // Option 0: Unequip
        let unequip_selected = picker.selected == 0;
        let unequip_style = if unequip_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let marker = if unequip_selected { "\u{25b8} " } else { "  " };
        lines.push(Line::from(Span::styled(
            format!("{}[Remove / Unequip]", marker),
            unequip_style,
        )));
        lines.push(Line::from(""));

        // Options 1..N: matching inventory items
        for (i, &item_id) in picker.matching_ids.iter().enumerate() {
            let pick_idx = i + 1;
            let is_selected = picker.selected == pick_idx;

            if let Some(item) = state.inventory.iter().find(|e| e.id == item_id) {
                let rarity_color = item.rarity.color();
                let marker = if is_selected { "\u{25b8} " } else { "  " };
                let name_style = if is_selected {
                    Style::default()
                        .fg(rarity_color)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(rarity_color)
                };

                lines.push(Line::from(vec![
                    Span::styled(marker, Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("{} ", item.rarity.icon()),
                        Style::default().fg(rarity_color),
                    ),
                    Span::styled(&item.name, name_style),
                ]));
                lines.push(Line::from(Span::styled(
                    format!("    Lv.{} \u{2502} {}", item.level, item.summary()),
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }

        if picker.matching_ids.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No matching items in inventory",
                Style::default().fg(Color::DarkGray),
            )));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(" Enter", Style::default().fg(Color::Yellow)),
            Span::styled(" select  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::styled(" cancel", Style::default().fg(Color::DarkGray)),
        ]));

        frame.render_widget(Paragraph::new(lines), inner);
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Get the equipment in a specific slot on a ship.
fn get_ship_slot(ship: &Ship, slot: Slot) -> Option<&Equipment> {
    match slot {
        Slot::Weapon => ship.weapon.as_ref(),
        Slot::Shield => ship.shield.as_ref(),
        Slot::Engine => ship.engine_mod.as_ref(),
        Slot::Special => ship.special.as_ref(),
    }
}

/// Return inventory items sorted by rarity (Legendary first), then level descending.
fn sorted_inventory(state: &GameState) -> Vec<&Equipment> {
    let mut sorted: Vec<&Equipment> = state.inventory.iter().collect();
    sorted.sort_by(|a, b| {
        b.rarity
            .cmp(&a.rarity)
            .then(b.level.cmp(&a.level))
            .then(a.name.cmp(&b.name))
    });
    sorted
}

/// Render detailed item info lines for the detail panel.
fn render_item_detail(item: &Equipment) -> Vec<Line<'_>> {
    let mut lines: Vec<Line> = Vec::new();
    let rarity_color = item.rarity.color();

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("  {}", item.name),
        Style::default()
            .fg(rarity_color)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        format!(
            "  {} {} \u{2502} Level {}",
            item.rarity.name(),
            item.slot.name(),
            item.level
        ),
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));

    // Stat lines
    for stat in item.detail_lines() {
        lines.push(Line::from(Span::styled(
            format!("  {}", stat),
            Style::default().fg(Color::White),
        )));
    }

    // Special effect
    if let Some(ref effect) = item.special_effect {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  \u{26a1} {}", effect.description()),
            Style::default().fg(Color::Cyan),
        )));
    }

    // Set bonus
    if let Some(ref set_id) = item.set_id
        && let Some(set) = SET_BONUSES.iter().find(|s| s.set_id == set_id) {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  Set: {} ({} pc)", set.set_name, set.pieces_required),
                Style::default().fg(Color::Green),
            )));
            lines.push(Line::from(Span::styled(
                format!("  {}", set.bonus_description),
                Style::default().fg(Color::DarkGray),
            )));
        }

    // Salvage value
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Salvage: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{}\u{25c7}", item.salvage_value()),
            Style::default().fg(Color::Yellow),
        ),
    ]));

    lines
}
