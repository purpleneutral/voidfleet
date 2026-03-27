use rand::Rng;
use ratatui::style::Color;
use ratatui::Frame;

use crate::engine::ship::ShipType;
use crate::rendering::particles::{Particle, ParticleSystem};
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
    /// Tier for projectile type selection (0=pirate, 1=militia, 2=cruiser, 3=warship).
    tier: u32,
    /// Dodge offset applied when evading incoming projectiles.
    dodge_offset: f32,
}

impl EnemyShip {
    fn is_alive(&self) -> bool {
        self.hp > 0
    }

    /// Height of the sprite in rows.
    fn height(&self) -> usize {
        self.sprite.len()
    }

    fn is_big(&self) -> bool {
        self.tier >= 2
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
    let fire_rate = fire_rate_ticks(base_speed, sector);

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
        tier,
        dodge_offset: 0.0,
    }
}

/// Convert speed stat → ticks between shots (faster = fewer ticks = higher fire‐rate).
/// At higher sectors, fire rate increases across the board.
fn fire_rate_ticks(speed: f32, sector: u32) -> u32 {
    // Base: speed 10 → 8 ticks, speed 2 → 30 ticks
    let base = (40.0 / speed) as u32;
    // Sector scaling: reduce cooldown at higher sectors (min 60% of base)
    let sector_mult = (1.0 - (sector as f32 - 1.0) * 0.02).max(0.6);
    ((base as f32 * sector_mult) as u32).max(3)
}

// ---------------------------------------------------------------------------
// Projectile — enhanced with trailing visuals
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum ProjectileKind {
    Laser,    // default: ━━─→ or ←─━━
    Heavy,    // big ships: ══─→
    Missile,  // bomber: ══►
}

struct Projectile {
    x: f32,
    y: f32,
    vx: f32,
    damage: u32,
    friendly: bool, // true = player's projectile
    kind: ProjectileKind,
}

impl Projectile {
    /// Characters for the projectile trail (head first, then trailing chars).
    fn trail_chars(&self) -> &[char] {
        match (&self.kind, self.friendly) {
            (ProjectileKind::Laser, true) => &['→', '─', '━', '━'],
            (ProjectileKind::Laser, false) => &['←', '─', '━', '━'],
            (ProjectileKind::Heavy, true) => &['→', '─', '═', '═'],
            (ProjectileKind::Heavy, false) => &['←', '─', '═', '═'],
            (ProjectileKind::Missile, true) => &['►', '═', '═'],
            (ProjectileKind::Missile, false) => &['◄', '═', '═'],
        }
    }

    fn color(&self) -> Color {
        match (&self.kind, self.friendly) {
            (ProjectileKind::Missile, _) => Color::Yellow,
            (_, true) => Color::Cyan,
            (_, false) => Color::Red,
        }
    }

    fn trail_color(&self) -> Color {
        match (&self.kind, self.friendly) {
            (ProjectileKind::Missile, _) => Color::Rgb(180, 140, 0),
            (_, true) => Color::Rgb(0, 140, 180),
            (_, false) => Color::Rgb(180, 60, 60),
        }
    }
}

// ---------------------------------------------------------------------------
// BattleScene
// ---------------------------------------------------------------------------

/// End-of-battle state machine.
#[derive(Clone, Copy, PartialEq, Eq)]
enum BattleEnd {
    None,
    /// Victory/defeat freeze — counts down frames before transition.
    Freeze(u8),
}

pub struct BattleScene {
    enemies: Vec<EnemyShip>,
    projectiles: Vec<Projectile>,
    width: u16,
    height: u16,
    tick_count: u64,
    sector: u32,
    /// Per‐ship fire cooldowns for the player fleet (indexed same as state.fleet).
    player_cooldowns: Vec<u32>,
    /// Tracks whether battle was won (all enemies dead) or lost (all player ships dead).
    player_won: bool,
    /// Flash timer per player ship (>0 means the ship was just hit — render inverted).
    player_flash: Vec<u8>,
    /// Flash timer per enemy ship.
    enemy_flash: Vec<u8>,
    /// Dodge offsets for player ships.
    player_dodge: Vec<f32>,
    /// End-of-battle state.
    battle_end: BattleEnd,
}

impl BattleScene {
    pub fn new() -> Self {
        Self {
            enemies: Vec::new(),
            projectiles: Vec::new(),
            width: 80,
            height: 24,
            tick_count: 0,
            sector: 1,
            player_cooldowns: Vec::new(),
            player_won: false,
            player_flash: Vec::new(),
            enemy_flash: Vec::new(),
            player_dodge: Vec::new(),
            battle_end: BattleEnd::None,
        }
    }

    // -- helpers --

    fn player_x_start(&self) -> f32 {
        5.0
    }

    fn enemy_x_start(&self) -> f32 {
        (self.width as f32 - 15.0).max(30.0)
    }

    /// Lay out the player fleet vertically, centered, with dodge offsets.
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
                let dodge = if i < self.player_dodge.len() {
                    self.player_dodge[i]
                } else {
                    0.0
                };
                (cx, y + bob + dodge)
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

    // -- projectile kind from ship type --

    fn player_projectile_kind(ship_type: ShipType) -> ProjectileKind {
        match ship_type {
            ShipType::Bomber => ProjectileKind::Missile,
            ShipType::Destroyer | ShipType::Capital | ShipType::Carrier => ProjectileKind::Heavy,
            _ => ProjectileKind::Laser,
        }
    }

    fn enemy_projectile_kind(tier: u32) -> ProjectileKind {
        match tier {
            3 => ProjectileKind::Heavy,
            2 => ProjectileKind::Heavy,
            _ => ProjectileKind::Laser,
        }
    }

    // -- muzzle flash particles --

    fn emit_muzzle_flash(particles: &mut ParticleSystem, x: f32, y: f32, facing_right: bool) {
        let mut rng = rand::thread_rng();
        let dir = if facing_right { 1.0 } else { -1.0 };
        for _ in 0..3 {
            particles.emit(Particle::new(
                x,
                y,
                dir * rng.gen_range(0.3..0.8),
                rng.gen_range(-0.2..0.2),
                3,
                if rng.gen_bool(0.5) { '✦' } else { '*' },
                if rng.gen_bool(0.6) {
                    Color::White
                } else {
                    Color::Yellow
                },
            ));
        }
    }

    // -- chain explosion for destroyed ships --

    fn chain_explosion(particles: &mut ParticleSystem, x: f32, y: f32, big: bool) {
        let mut rng = rand::thread_rng();

        // Phase 1: Bright white core burst
        let core_count = if big { 8 } else { 4 };
        for _ in 0..core_count {
            let angle: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
            let speed: f32 = rng.gen_range(0.2..0.6);
            particles.emit(Particle::new(
                x,
                y,
                angle.cos() * speed,
                angle.sin() * speed * 0.5,
                4,
                '█',
                Color::White,
            ));
        }

        // Phase 2: Expanding ring of orange/red
        let ring_count = if big { 16 } else { 8 };
        for _ in 0..ring_count {
            let angle: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
            let speed: f32 = rng.gen_range(0.5..1.8);
            let life: u8 = rng.gen_range(8..16);
            let color = if rng.gen_bool(0.5) {
                Color::Rgb(255, 140, 0) // orange
            } else {
                Color::LightRed
            };
            particles.emit(Particle::new(
                x,
                y,
                angle.cos() * speed,
                angle.sin() * speed * 0.5,
                life,
                if rng.gen_bool(0.4) { '✦' } else { '◆' },
                color,
            ));
        }

        // Phase 3: Debris chars that linger and drift slowly
        let debris_chars = ['▪', '▫', '◦', '∙', '·'];
        let debris_count = if big { 12 } else { 5 };
        for _ in 0..debris_count {
            let angle: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
            let speed: f32 = rng.gen_range(0.1..0.5);
            let life: u8 = rng.gen_range(15..30);
            let ch = debris_chars[rng.gen_range(0..debris_chars.len())];
            particles.emit(Particle::new(
                x,
                y,
                angle.cos() * speed,
                angle.sin() * speed * 0.3,
                life,
                ch,
                Color::DarkGray,
            ));
        }
    }

    // -- dodge logic: compute dodge offsets for each ship --

    fn compute_dodge_offsets(
        projectiles: &[Projectile],
        positions: &[(f32, f32)],
        ships_alive: &[bool],
        current_dodge: &mut [f32],
        dodge_toward_enemy: bool, // false = friendly ships dodge enemy projectiles
    ) {
        let dodge_range = 3.0_f32;
        let dodge_strength = 0.6_f32;
        let decay = 0.8_f32;

        for (i, (_, sy)) in positions.iter().enumerate() {
            if i >= current_dodge.len() || i >= ships_alive.len() || !ships_alive[i] {
                continue;
            }

            // Find nearest threatening projectile
            let mut nearest_dy: Option<f32> = None;
            for p in projectiles.iter() {
                // Only dodge projectiles heading toward us
                if dodge_toward_enemy && p.friendly {
                    continue;
                }
                if !dodge_toward_enemy && !p.friendly {
                    continue;
                }
                let dy = p.y - (*sy - current_dodge[i]); // compare against base position
                if dy.abs() < dodge_range {
                    match nearest_dy {
                        None => nearest_dy = Some(dy),
                        Some(prev) => {
                            if dy.abs() < prev.abs() {
                                nearest_dy = Some(dy);
                            }
                        }
                    }
                }
            }

            if let Some(dy) = nearest_dy {
                // Move away from the projectile
                let dodge_dir = if dy > 0.0 { -dodge_strength } else { dodge_strength };
                current_dodge[i] = (current_dodge[i] + dodge_dir).clamp(-2.0, 2.0);
            } else {
                // Decay back to center
                current_dodge[i] *= decay;
                if current_dodge[i].abs() < 0.05 {
                    current_dodge[i] = 0.0;
                }
            }
        }
    }

    // -- convert all remaining projectiles into particles (victory/defeat moment) --

    fn projectiles_to_particles(
        projectiles: &mut Vec<Projectile>,
        particles: &mut ParticleSystem,
    ) {
        let mut rng = rand::thread_rng();
        for p in projectiles.drain(..) {
            let color = if p.friendly {
                Color::Cyan
            } else {
                Color::Red
            };
            particles.emit(Particle::new(
                p.x,
                p.y,
                p.vx * 0.3 + rng.gen_range(-0.3..0.3),
                rng.gen_range(-0.3..0.3),
                rng.gen_range(4..8),
                '✦',
                color,
            ));
        }
    }
}

impl Scene for BattleScene {
    fn enter(&mut self, state: &GameState, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.tick_count = 0;
        self.sector = state.sector;
        self.projectiles.clear();
        self.player_won = false;
        self.battle_end = BattleEnd::None;

        // Generate enemy fleet based on sector
        let mut rng = rand::thread_rng();
        let count = (2 + state.sector / 3).min(8) as usize;
        self.enemies.clear();
        for _ in 0..count {
            self.enemies.push(enemy_template(state.sector, &mut rng));
        }
        self.layout_enemies();

        // Fire cooldowns for player ships (sector-scaled)
        self.player_cooldowns = state
            .fleet
            .iter()
            .map(|s| {
                let rate = fire_rate_ticks(s.speed(), state.sector);
                rng.gen_range(0..rate) // stagger
            })
            .collect();

        self.player_flash = vec![0u8; state.fleet.len()];
        self.enemy_flash = vec![0u8; self.enemies.len()];
        self.player_dodge = vec![0.0f32; state.fleet.len()];
    }

    fn tick(&mut self, state: &mut GameState, particles: &mut ParticleSystem) -> SceneAction {
        self.tick_count += 1;

        // ── Handle end-of-battle freeze ────────────────────────────────
        if let BattleEnd::Freeze(ref mut frames) = self.battle_end {
            if *frames == 0 {
                state.total_battles += 1;
                return SceneAction::TransitionTo(GamePhase::Loot);
            }
            *frames -= 1;
            // During freeze, only tick particles (no new firing/movement)
            return SceneAction::Continue;
        }

        // ── Enemy bobbing + dodge ──────────────────────────────────────
        let cx = self.enemy_x_start();
        let cy = self.height as f32 / 2.0;
        let spacing = 3.0_f32;
        let total = self.enemies.len() as f32 * spacing;

        // Compute enemy dodge offsets against friendly projectiles
        {
            let enemy_positions: Vec<(f32, f32)> = self
                .enemies
                .iter()
                .map(|e| (e.x, e.y))
                .collect();
            let alive: Vec<bool> = self.enemies.iter().map(|e| e.is_alive()).collect();
            let mut dodge_offsets: Vec<f32> =
                self.enemies.iter().map(|e| e.dodge_offset).collect();
            Self::compute_dodge_offsets(
                &self.projectiles,
                &enemy_positions,
                &alive,
                &mut dodge_offsets,
                true, // dodge friendly (player) projectiles
            );
            for (i, e) in self.enemies.iter_mut().enumerate() {
                if i < dodge_offsets.len() {
                    e.dodge_offset = dodge_offsets[i];
                }
            }
        }

        for (i, e) in self.enemies.iter_mut().enumerate() {
            if e.is_alive() {
                let base_y = cy - total / 2.0 + i as f32 * spacing;
                let bob = (self.tick_count as f32 * 0.05 + i as f32 * 1.3).sin() * 0.4;
                e.x = cx;
                e.y = base_y + bob + e.dodge_offset;
            }
        }

        // ── Compute player dodge offsets ───────────────────────────────
        {
            let positions = self.player_positions(&state.fleet);
            let alive: Vec<bool> = state.fleet.iter().map(|s| s.is_alive()).collect();
            Self::compute_dodge_offsets(
                &self.projectiles,
                &positions,
                &alive,
                &mut self.player_dodge,
                false, // dodge enemy projectiles
            );
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
                let muzzle_x = px + sprite_w + 1.0;
                let kind = Self::player_projectile_kind(ship.ship_type);
                let speed = match kind {
                    ProjectileKind::Missile => 0.8 + ship.speed() * 0.05,
                    _ => 1.2 + ship.speed() * 0.08,
                };
                self.projectiles.push(Projectile {
                    x: muzzle_x,
                    y: py,
                    vx: speed,
                    damage: ship.damage(),
                    friendly: true,
                    kind,
                });

                // Muzzle flash
                Self::emit_muzzle_flash(particles, muzzle_x, py, true);

                self.player_cooldowns[i] = fire_rate_ticks(ship.speed(), self.sector);
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
                let kind = Self::enemy_projectile_kind(e.tier);
                let speed = match kind {
                    ProjectileKind::Heavy => 0.6 + e.speed * 0.04,
                    _ => 0.8 + e.speed * 0.06,
                };
                let muzzle_x = e.x - 1.0;
                self.projectiles.push(Projectile {
                    x: muzzle_x,
                    y: e.y,
                    vx: -speed,
                    damage: e.damage,
                    friendly: false,
                    kind,
                });

                // Muzzle flash
                Self::emit_muzzle_flash(particles, muzzle_x, e.y, false);

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
        self.projectiles.retain(|p| p.x >= -4.0 && p.x <= w + 4.0);

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
                        self.enemy_flash[ei] = 3;
                    }

                    // Hit spark
                    particles.emit(Particle::new(
                        proj.x,
                        proj.y,
                        0.0,
                        0.0,
                        2,
                        '✦',
                        Color::White,
                    ));

                    // Death?
                    if !enemy.is_alive() {
                        let center_x = enemy.x + sprite_w / 2.0;
                        Self::chain_explosion(particles, center_x, enemy.y, enemy.is_big());
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
                        self.player_flash[si] = 3;
                    }

                    // Hit spark
                    particles.emit(Particle::new(
                        proj.x,
                        proj.y,
                        0.0,
                        0.0,
                        2,
                        '✦',
                        Color::White,
                    ));

                    // Death?
                    if !ship.is_alive() {
                        let is_big = matches!(
                            ship.ship_type,
                            ShipType::Destroyer | ShipType::Capital | ShipType::Carrier | ShipType::Frigate
                        );
                        Self::chain_explosion(
                            particles,
                            sx + sprite_w / 2.0,
                            sy,
                            is_big,
                        );
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

        // ── Win / lose check — enter freeze instead of instant transition
        if self.enemy_alive_count() == 0 {
            self.player_won = true;
            // Convert remaining projectiles to particles for victory moment
            Self::projectiles_to_particles(&mut self.projectiles, particles);
            self.battle_end = BattleEnd::Freeze(10);
            return SceneAction::Continue;
        }
        if Self::player_alive_count(&state.fleet) == 0 {
            self.player_won = false;
            Self::projectiles_to_particles(&mut self.projectiles, particles);
            self.battle_end = BattleEnd::Freeze(10);
            return SceneAction::Continue;
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
            let damaged = ship.current_hp < ship.max_hp();

            // Color coding: cyan base, flash white on hit, dim when damaged
            let fg = if flashing {
                Color::White
            } else if damaged && self.tick_count % 6 < 2 {
                Color::DarkGray // damaged flash
            } else {
                Color::Cyan
            };
            let bg = if flashing {
                Color::Red
            } else {
                Color::Reset
            };

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
            let damaged = enemy.hp < enemy.max_hp;

            // Color coding: red base, flash white on hit, dim when damaged
            let fg = if flashing {
                Color::White
            } else if damaged && self.tick_count % 6 < 2 {
                Color::DarkGray
            } else {
                Color::LightRed
            };
            let bg = if flashing {
                Color::Yellow
            } else {
                Color::Reset
            };

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

        // ── Projectiles with trailing chars ───────────────────────────
        for p in &self.projectiles {
            let py = p.y as u16;
            if py >= area.height {
                continue;
            }

            let trail = p.trail_chars();
            let head_color = p.color();
            let trail_color = p.trail_color();

            for (idx, &ch) in trail.iter().enumerate() {
                let offset = if p.vx > 0.0 {
                    // Moving right: head is rightmost, trail extends left
                    -(idx as f32)
                } else {
                    // Moving left: head is leftmost, trail extends right
                    idx as f32
                };
                let tx = (p.x + offset) as i32;
                if tx >= 0 && (tx as u16) < area.width {
                    let cell = &mut buf[(area.x + tx as u16, area.y + py)];
                    cell.set_char(ch);
                    cell.set_fg(if idx == 0 { head_color } else { trail_color });
                }
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
