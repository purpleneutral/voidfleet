use rand::Rng;
use ratatui::style::Color;
use ratatui::Frame;

use crate::rendering::particles::ParticleSystem;
use crate::state::{GamePhase, GameState};

use super::{Scene, SceneAction};

// ---------------------------------------------------------------------------
// Enemy ship — local struct mirroring player Ship but simpler
// ---------------------------------------------------------------------------

struct EnemyShip {
    x: f32,
    y: f32,
    hp: u32,
    max_hp: u32,
    damage: u32,
    speed: f32,
    #[allow(dead_code)]
    name: &'static str,
    sprite: &'static [&'static str],
    /// Tick counter for fire‐rate cadence (fires when countdown reaches 0).
    fire_cooldown: u32,
    fire_rate: u32,
}

impl EnemyShip {
    fn is_alive(&self) -> bool {
        self.hp > 0
    }

    /// Height of the sprite in rows.
    fn height(&self) -> usize {
        self.sprite.len()
    }
}

// Enemy templates — sprites face left (mirrored player ships).
const ENEMY_PIRATE: &[&str] = &["◄="];
const ENEMY_MILITIA: &[&str] = &["◄╚═"];
const ENEMY_CRUISER: &[&str] = &["◄══╗", "◄══╝"];
const ENEMY_WARSHIP: &[&str] = &["╔═══►", "╣███◄", "╚═══►"];

fn enemy_template(sector: u32, rng: &mut impl Rng) -> EnemyShip {
    // Difficulty scales with sector — higher sectors get tougher ships + stat boost
    let scale = 1.0 + (sector as f32 - 1.0) * 0.15;
    let tier = rng.gen_range(0..=(sector.min(20) / 5).min(3));

    let (name, sprite, base_hp, base_dmg, base_speed) = match tier {
        0 => ("Pirate Scout", ENEMY_PIRATE, 8u32, 2u32, 9.0f32),
        1 => ("Militia Fighter", ENEMY_MILITIA, 18, 6, 7.0),
        2 => ("Cruiser", ENEMY_CRUISER, 50, 12, 4.0),
        _ => ("Warship", ENEMY_WARSHIP, 120, 28, 3.0),
    };

    let hp = (base_hp as f32 * scale) as u32;
    let fire_rate = fire_rate_ticks(base_speed);

    EnemyShip {
        x: 0.0, // set by enter()
        y: 0.0,
        hp,
        max_hp: hp,
        damage: (base_dmg as f32 * scale) as u32,
        speed: base_speed,
        name,
        sprite,
        fire_cooldown: rng.gen_range(0..fire_rate), // stagger first volley
        fire_rate,
    }
}

/// Convert speed stat → ticks between shots (faster = fewer ticks = higher fire‐rate).
fn fire_rate_ticks(speed: f32) -> u32 {
    // speed 10 → 8 ticks (0.4s), speed 2 → 30 ticks (1.5s)
    ((40.0 / speed) as u32).max(4)
}

// ---------------------------------------------------------------------------
// Projectile
// ---------------------------------------------------------------------------

struct Projectile {
    x: f32,
    y: f32,
    vx: f32,
    ch: char,
    color: Color,
    damage: u32,
    friendly: bool, // true = player's projectile
}

// ---------------------------------------------------------------------------
// BattleScene
// ---------------------------------------------------------------------------

pub struct BattleScene {
    enemies: Vec<EnemyShip>,
    projectiles: Vec<Projectile>,
    width: u16,
    height: u16,
    tick_count: u64,
    /// Per‐ship fire cooldowns for the player fleet (indexed same as state.fleet).
    player_cooldowns: Vec<u32>,
    /// Tracks whether battle was won (all enemies dead) or lost (all player ships dead).
    player_won: bool,
    /// Flash timer per player ship (>0 means the ship was just hit — render inverted).
    player_flash: Vec<u8>,
    /// Flash timer per enemy ship.
    enemy_flash: Vec<u8>,
}

impl BattleScene {
    pub fn new() -> Self {
        Self {
            enemies: Vec::new(),
            projectiles: Vec::new(),
            width: 80,
            height: 24,
            tick_count: 0,
            player_cooldowns: Vec::new(),
            player_won: false,
            player_flash: Vec::new(),
            enemy_flash: Vec::new(),
        }
    }

    // -- helpers --

    fn player_x_start(&self) -> f32 {
        5.0
    }

    fn enemy_x_start(&self) -> f32 {
        (self.width as f32 - 15.0).max(30.0)
    }

    /// Lay out the player fleet vertically, centered.
    fn player_positions(&self, fleet: &[crate::engine::ship::Ship]) -> Vec<(f32, f32)> {
        let cx = self.player_x_start();
        let cy = self.height as f32 / 2.0;
        let spacing = 3.0_f32;
        let total = fleet.len() as f32 * spacing;

        fleet
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let y = cy - total / 2.0 + i as f32 * spacing;
                let bob = (self.tick_count as f32 * 0.06 + i as f32 * 1.1).sin() * 0.4;
                (cx, y + bob)
            })
            .collect()
    }

    fn layout_enemies(&mut self) {
        let cx = self.enemy_x_start();
        let cy = self.height as f32 / 2.0;
        let spacing = 3.0_f32;
        let total = self.enemies.len() as f32 * spacing;

        for (i, e) in self.enemies.iter_mut().enumerate() {
            e.x = cx;
            e.y = cy - total / 2.0 + i as f32 * spacing;
        }
    }

    fn enemy_alive_count(&self) -> usize {
        self.enemies.iter().filter(|e| e.is_alive()).count()
    }

    fn player_alive_count(fleet: &[crate::engine::ship::Ship]) -> usize {
        fleet.iter().filter(|s| s.is_alive()).count()
    }

    fn enemy_total_hp(&self) -> u32 {
        self.enemies.iter().map(|e| e.hp).sum()
    }

    fn enemy_max_hp(&self) -> u32 {
        self.enemies.iter().map(|e| e.max_hp).sum()
    }
}

impl Scene for BattleScene {
    fn enter(&mut self, state: &GameState, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.tick_count = 0;
        self.projectiles.clear();
        self.player_won = false;

        // Generate enemy fleet based on sector
        let mut rng = rand::thread_rng();
        let count = (2 + state.sector / 3).min(8) as usize;
        self.enemies.clear();
        for _ in 0..count {
            self.enemies.push(enemy_template(state.sector, &mut rng));
        }
        self.layout_enemies();

        // Fire cooldowns for player ships
        self.player_cooldowns = state
            .fleet
            .iter()
            .map(|s| {
                let rate = fire_rate_ticks(s.speed());
                rng.gen_range(0..rate) // stagger
            })
            .collect();

        self.player_flash = vec![0u8; state.fleet.len()];
        self.enemy_flash = vec![0u8; self.enemies.len()];
    }

    fn tick(&mut self, state: &mut GameState, particles: &mut ParticleSystem) -> SceneAction {
        self.tick_count += 1;

        // ── Enemy bobbing ──────────────────────────────────────────────
        let cx = self.enemy_x_start();
        let cy = self.height as f32 / 2.0;
        let spacing = 3.0_f32;
        let total = self.enemies.len() as f32 * spacing;
        for (i, e) in self.enemies.iter_mut().enumerate() {
            if e.is_alive() {
                let base_y = cy - total / 2.0 + i as f32 * spacing;
                let bob = (self.tick_count as f32 * 0.05 + i as f32 * 1.3).sin() * 0.4;
                e.x = cx;
                e.y = base_y + bob;
            }
        }

        // ── Player fleet fires ─────────────────────────────────────────
        let positions = self.player_positions(&state.fleet);
        for (i, ship) in state.fleet.iter().enumerate() {
            if !ship.is_alive() {
                continue;
            }
            if i >= self.player_cooldowns.len() {
                continue;
            }
            if self.player_cooldowns[i] == 0 {
                let (px, py) = positions[i];
                let sprite_w = ship.ship_type.sprite()[0].chars().count() as f32;
                self.projectiles.push(Projectile {
                    x: px + sprite_w + 1.0,
                    y: py,
                    vx: 1.2 + ship.speed() * 0.08,
                    ch: if ship.damage() >= 15 { '═' } else { '─' },
                    color: if ship.damage() >= 15 {
                        Color::Yellow
                    } else {
                        Color::Cyan
                    },
                    damage: ship.damage(),
                    friendly: true,
                });
                self.player_cooldowns[i] = fire_rate_ticks(ship.speed());
            } else {
                self.player_cooldowns[i] -= 1;
            }
        }

        // ── Enemy fleet fires ──────────────────────────────────────────
        for e in self.enemies.iter_mut() {
            if !e.is_alive() {
                continue;
            }
            if e.fire_cooldown == 0 {
                self.projectiles.push(Projectile {
                    x: e.x - 1.0,
                    y: e.y,
                    vx: -(0.8 + e.speed * 0.06),
                    ch: '─',
                    color: Color::Red,
                    damage: e.damage,
                    friendly: false,
                });
                e.fire_cooldown = e.fire_rate;
            } else {
                e.fire_cooldown -= 1;
            }
        }

        // ── Move projectiles ───────────────────────────────────────────
        for p in self.projectiles.iter_mut() {
            p.x += p.vx;
        }

        // Remove off-screen
        let w = self.width as f32;
        self.projectiles.retain(|p| p.x >= -2.0 && p.x <= w + 2.0);

        // ── Hit detection: friendly projectiles → enemies ──────────────
        let mut to_remove_proj: Vec<usize> = Vec::new();
        for (pi, proj) in self.projectiles.iter().enumerate() {
            if !proj.friendly {
                continue;
            }
            for (ei, enemy) in self.enemies.iter_mut().enumerate() {
                if !enemy.is_alive() {
                    continue;
                }
                let sprite_w = enemy.sprite[0].chars().count() as f32;
                let hit_x = proj.x >= enemy.x && proj.x <= enemy.x + sprite_w;
                let hit_y = (proj.y - enemy.y).abs() < (enemy.height() as f32 * 0.5 + 0.5);
                if hit_x && hit_y {
                    let dmg = proj.damage.min(enemy.hp);
                    enemy.hp -= dmg;
                    to_remove_proj.push(pi);

                    // Flash
                    if ei < self.enemy_flash.len() {
                        self.enemy_flash[ei] = 2;
                    }

                    // Death?
                    if !enemy.is_alive() {
                        particles.explode(
                            enemy.x + sprite_w / 2.0,
                            enemy.y,
                            12,
                            Color::LightRed,
                        );
                        state.enemies_destroyed += 1;
                    }
                    break;
                }
            }
        }

        // ── Hit detection: enemy projectiles → player ships ────────────
        let positions = self.player_positions(&state.fleet);
        for (pi, proj) in self.projectiles.iter().enumerate() {
            if proj.friendly {
                continue;
            }
            for (si, ship) in state.fleet.iter_mut().enumerate() {
                if !ship.is_alive() {
                    continue;
                }
                if si >= positions.len() {
                    continue;
                }
                let (sx, sy) = positions[si];
                let sprite = ship.ship_type.sprite();
                let sprite_w = sprite[0].chars().count() as f32;
                let sprite_h = sprite.len() as f32;
                let hit_x = proj.x >= sx && proj.x <= sx + sprite_w;
                let hit_y = (proj.y - sy).abs() < (sprite_h * 0.5 + 0.5);
                if hit_x && hit_y {
                    let dmg = proj.damage.min(ship.current_hp);
                    ship.current_hp -= dmg;
                    if !to_remove_proj.contains(&pi) {
                        to_remove_proj.push(pi);
                    }

                    // Flash
                    if si < self.player_flash.len() {
                        self.player_flash[si] = 2;
                    }

                    // Death?
                    if !ship.is_alive() {
                        particles.explode(sx + sprite_w / 2.0, sy, 10, Color::Cyan);
                    }
                    break;
                }
            }
        }

        // Remove hit projectiles (reverse order)
        to_remove_proj.sort_unstable();
        to_remove_proj.dedup();
        for i in to_remove_proj.into_iter().rev() {
            if i < self.projectiles.len() {
                self.projectiles.swap_remove(i);
            }
        }

        // ── Decrement flash timers ─────────────────────────────────────
        for f in self.player_flash.iter_mut() {
            *f = f.saturating_sub(1);
        }
        for f in self.enemy_flash.iter_mut() {
            *f = f.saturating_sub(1);
        }

        // ── Win / lose check ───────────────────────────────────────────
        if self.enemy_alive_count() == 0 {
            self.player_won = true;
            state.total_battles += 1;
            return SceneAction::TransitionTo(GamePhase::Loot);
        }
        if Self::player_alive_count(&state.fleet) == 0 {
            self.player_won = false;
            state.total_battles += 1;
            return SceneAction::TransitionTo(GamePhase::Loot);
        }

        // Fallback timeout — force end after phase_timer expires
        state.phase_timer -= 0.05;
        if state.phase_timer <= 0.0 {
            self.player_won = true;
            state.total_battles += 1;
            return SceneAction::TransitionTo(GamePhase::Loot);
        }

        SceneAction::Continue
    }

    fn render(&self, frame: &mut Frame, state: &GameState, particles: &ParticleSystem) {
        let area = frame.area();
        let buf = frame.buffer_mut();

        // ── Player ships (left side) ───────────────────────────────────
        let positions = self.player_positions(&state.fleet);
        for (i, ship) in state.fleet.iter().enumerate() {
            if !ship.is_alive() || i >= positions.len() {
                continue;
            }
            let (fx, fy) = positions[i];
            let sprite = ship.ship_type.sprite();
            let flashing = i < self.player_flash.len() && self.player_flash[i] > 0;
            let fg = if flashing { Color::White } else { Color::Cyan };
            let bg = if flashing { Color::Red } else { Color::Black };

            for (row, line) in sprite.iter().enumerate() {
                let sy = (fy + row as f32) as u16;
                for (col, ch) in line.chars().enumerate() {
                    let sx = (fx + col as f32) as u16;
                    if sx < area.width && sy < area.height {
                        let cell = &mut buf[(area.x + sx, area.y + sy)];
                        cell.set_char(ch);
                        cell.set_fg(fg);
                        cell.set_bg(bg);
                    }
                }
            }
        }

        // ── Enemy ships (right side) ──────────────────────────────────
        for (i, enemy) in self.enemies.iter().enumerate() {
            if !enemy.is_alive() {
                continue;
            }
            let flashing = i < self.enemy_flash.len() && self.enemy_flash[i] > 0;
            let fg = if flashing { Color::White } else { Color::LightRed };
            let bg = if flashing { Color::Yellow } else { Color::Black };

            for (row, line) in enemy.sprite.iter().enumerate() {
                let sy = (enemy.y + row as f32) as u16;
                for (col, ch) in line.chars().enumerate() {
                    let sx = (enemy.x + col as f32) as u16;
                    if sx < area.width && sy < area.height {
                        let cell = &mut buf[(area.x + sx, area.y + sy)];
                        cell.set_char(ch);
                        cell.set_fg(fg);
                        cell.set_bg(bg);
                    }
                }
            }
        }

        // ── Projectiles ───────────────────────────────────────────────
        for p in &self.projectiles {
            let px = p.x as u16;
            let py = p.y as u16;
            if px < area.width && py < area.height {
                let cell = &mut buf[(area.x + px, area.y + py)];
                cell.set_char(p.ch);
                cell.set_fg(p.color);
            }
            // Draw arrow head / trail character
            let head_x = if p.vx > 0.0 {
                (p.x + 1.0) as u16
            } else {
                p.x as u16
            };
            // Arrow head
            if head_x < area.width && py < area.height {
                let head_ch = if p.vx > 0.0 { '→' } else { '←' };
                let cell = &mut buf[(area.x + head_x, area.y + py)];
                cell.set_char(head_ch);
                cell.set_fg(p.color);
            }
        }

        // ── Particles ─────────────────────────────────────────────────
        for p in &particles.particles {
            let px = p.x as u16;
            let py = p.y as u16;
            if px < area.width && py < area.height {
                let cell = &mut buf[(area.x + px, area.y + py)];
                cell.set_char(p.render_char());
                cell.set_fg(p.color);
            }
        }

        // ── Health bars at bottom ──────────────────────────────────────
        let bar_y = area.height.saturating_sub(2);
        let bar_width = (area.width / 2).saturating_sub(4) as usize;

        // Player fleet HP
        let player_hp: u32 = state.fleet.iter().map(|s| s.current_hp).sum();
        let player_max: u32 = state.fleet.iter().map(|s| s.max_hp()).sum();
        let player_ratio = if player_max > 0 {
            player_hp as f32 / player_max as f32
        } else {
            0.0
        };
        let player_filled = (player_ratio * bar_width as f32) as usize;

        // Label
        let label = "FLEET ";
        for (i, ch) in label.chars().enumerate() {
            let x = 1 + i as u16;
            if x < area.width && bar_y < area.height {
                let cell = &mut buf[(area.x + x, area.y + bar_y)];
                cell.set_char(ch);
                cell.set_fg(Color::Green);
            }
        }
        // Bar
        let bar_start = 1 + label.len() as u16;
        for i in 0..bar_width {
            let x = bar_start + i as u16;
            if x < area.width && bar_y < area.height {
                let cell = &mut buf[(area.x + x, area.y + bar_y)];
                if i < player_filled {
                    cell.set_char('█');
                    cell.set_fg(Color::Green);
                } else {
                    cell.set_char('░');
                    cell.set_fg(Color::DarkGray);
                }
            }
        }
        // HP text
        let hp_text = format!(" {}/{}", player_hp, player_max);
        let hp_x = bar_start + bar_width as u16;
        for (i, ch) in hp_text.chars().enumerate() {
            let x = hp_x + i as u16;
            if x < area.width && bar_y < area.height {
                let cell = &mut buf[(area.x + x, area.y + bar_y)];
                cell.set_char(ch);
                cell.set_fg(Color::Green);
            }
        }

        // Enemy fleet HP
        let enemy_hp = self.enemy_total_hp();
        let enemy_max = self.enemy_max_hp();
        let enemy_ratio = if enemy_max > 0 {
            enemy_hp as f32 / enemy_max as f32
        } else {
            0.0
        };
        let enemy_filled = (enemy_ratio * bar_width as f32) as usize;

        let right_start = area.width / 2 + 2;
        let elabel = "ENEMY ";
        for (i, ch) in elabel.chars().enumerate() {
            let x = right_start + i as u16;
            if x < area.width && bar_y < area.height {
                let cell = &mut buf[(area.x + x, area.y + bar_y)];
                cell.set_char(ch);
                cell.set_fg(Color::Red);
            }
        }
        let ebar_start = right_start + elabel.len() as u16;
        for i in 0..bar_width {
            let x = ebar_start + i as u16;
            if x < area.width && bar_y < area.height {
                let cell = &mut buf[(area.x + x, area.y + bar_y)];
                if i < enemy_filled {
                    cell.set_char('█');
                    cell.set_fg(Color::Red);
                } else {
                    cell.set_char('░');
                    cell.set_fg(Color::DarkGray);
                }
            }
        }
        let ehp_text = format!(" {}/{}", enemy_hp, enemy_max);
        let ehp_x = ebar_start + bar_width as u16;
        for (i, ch) in ehp_text.chars().enumerate() {
            let x = ehp_x + i as u16;
            if x < area.width && bar_y < area.height {
                let cell = &mut buf[(area.x + x, area.y + bar_y)];
                cell.set_char(ch);
                cell.set_fg(Color::Red);
            }
        }

        // ── Battle header ──────────────────────────────────────────────
        let header = format!(
            "⚔ BATTLE — Sector {} — {:.0}s",
            state.sector, state.phase_timer
        );
        let hx = (area.width / 2).saturating_sub(header.len() as u16 / 2);
        for (i, ch) in header.chars().enumerate() {
            let x = hx + i as u16;
            if x < area.width && area.height > 0 {
                let cell = &mut buf[(area.x + x, area.y)];
                cell.set_char(ch);
                cell.set_fg(Color::Yellow);
            }
        }
    }
}
