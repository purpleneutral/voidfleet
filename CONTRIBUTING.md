# Contributing to Void Fleet

Thanks for your interest in contributing! Void Fleet is a terminal-based space game built in Rust with ratatui.

## Getting Started

```bash
git clone https://github.com/purpleneutral/voidfleet.git
cd voidfleet
cargo build
cargo run
```

Requires Rust 1.85+ (2024 edition).

## Project Structure

```
src/
  main.rs              — game loop, input handling, HUD rendering
  state.rs             — game state, save/load, prestige/death mechanics
  engine/              — game logic (no rendering)
    ship.rs            — ship types, stats, sprites, abilities
    combat.rs          — tech-aware damage/HP/fire-rate calculations
    economy.rs         — loot scaling, costs, catch-up mechanics
    procedural.rs      — enemy generation, adaptive difficulty, bosses
    achievements.rs    — achievement definitions and state checking
  scenes/              — each scene handles its own tick + render
    travel.rs          — starfield cruise, collectibles, random events
    battle.rs          — fleet combat, AI, projectiles, abilities
    raid.rs            — planet raids, tractor beams, turrets
    loot.rs            — reward display with animations
    title.rs           — title screen
    bridge.rs          — Pip companion (tamagotchi)
    upgrades.rs        — ship/tech upgrade shop
    stats.rs           — lifetime statistics overlay
    map.rs             — sector route selection
  rendering/           — shared visual systems
    particles.rs       — particle system (explosions, exhaust, sparkles)
    starfield.rs       — parallax star background with shooting stars
    effects.rs         — screen transition effects
    layout.rs          — shared UI utilities
```

## How Scenes Work

Each scene implements the `Scene` trait:

```rust
pub trait Scene {
    fn enter(&mut self, state: &GameState, width: u16, height: u16);
    fn tick(&mut self, state: &mut GameState, particles: &mut ParticleSystem) -> SceneAction;
    fn render(&self, frame: &mut Frame, state: &GameState, particles: &ParticleSystem);
}
```

- `enter()` — called once when transitioning to this scene
- `tick()` — called every frame (~50ms), returns `Continue` or `TransitionTo(phase)`
- `render()` — draws to the ratatui frame buffer

## Adding a New Feature

**New ship type**: Add variant to `ShipType` in `ship.rs`, define stats + sprite, add to unlock table.

**New achievement**: Add entry to `ACHIEVEMENTS` in `achievements.rs` with condition closure.

**New travel event**: Add variant to `EventType` in `travel.rs`, add to `generate_event()` and `event_weights()`.

**New scene**: Create `src/scenes/yourscene.rs`, implement `Scene` trait, register in `scenes/mod.rs`, wire into `main.rs`.

## Code Style

- Run `cargo clippy` — should produce zero warnings
- Run `cargo test` — all tests must pass
- Keep rendering code separated from game logic (engine/ vs scenes/)
- Bounds-check all buffer writes: `if x < area.width && y < area.height`
- Use `saturating_sub`/`saturating_add` for game math to prevent overflow
- No `unwrap()` in production code paths — use `if let` or pattern matching

## Testing

```bash
cargo test
```

Tests live alongside the code they test (inline `#[cfg(test)]` modules). Priority areas for new tests:
- Game state transitions (prestige, death, level-up)
- Economy calculations (loot, costs, scaling)
- Save/load roundtrips

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
