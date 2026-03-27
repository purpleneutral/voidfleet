# Void Fleet — Space Conquest Idle TUI

A terminal-based idle space conquest game where animated "screensaver" scenes show your fleet traveling, battling, and raiding planets. Dip into upgrades when you want, watch your fleet grow.

## Core Loop

```
TRAVEL (30-60s) → ENCOUNTER → BATTLE or RAID (15-30s) → LOOT → TRAVEL
```

Each phase is a full-screen animated scene running at ~20fps. The game cycles automatically. Player input is optional — press `U` for upgrades anytime, or just watch.

## Scenes

### 1. Travel
- Stars scroll right-to-left (parallax: far stars slow, near stars fast)
- Fleet flies in formation on the left third of screen
- Floating objects drift by: scrap `◇`, cargo pods `□`, beacons `⊕`
- Ships auto-collect nearby items
- Sector counter ticks up
- Occasionally: nebula clouds (colored background regions), asteroid fields

### 2. Battle
- Your fleet on the left, enemy fleet on the right
- Projectiles streak between them: `─→` lasers, `═→` missiles, `←─` enemy fire
- Ships flash when hit (color invert for 2 frames)
- Destroyed ships burst into particle explosion (characters scatter outward with velocity, fade over ~10 frames)
- Health bars for both fleets at bottom
- Ships dodge slightly (vertical oscillation)
- Battle ends when one fleet is destroyed

### 3. Planet Raid
- Planet surface along bottom (terrain varies: urban `🏢`, desert `▒`, ice `░`)
- Fleet hovers above, tractor beams `↓` stream down
- People/vehicles on surface scatter away from beams
- Resources float upward as particles `·` `°` `◦`
- Defense turrets fire back (projectiles upward)
- Buildings crumble over time (multi-frame animation)
- Surface degrades as resources are extracted

### 4. Loot
- Brief results overlay (3-5 seconds)
- Resources gained, ships lost, blueprints found
- Auto-transitions to travel

### 5. Upgrades (press U anytime)
- Full screen overlay with tabs
- SHIPS: buy new ships, upgrade individual ships (HP, DMG, speed)
- TECH: weapons (lasers, missiles, plasma), shields, engines, tractor beams
- FLEET: formation presets, auto-behavior, fleet size cap
- Shows current resources, unlock requirements

## Ship Types

| Type | ASCII | HP | DMG | Speed | Cost | Unlock |
|------|-------|----|-----|-------|------|--------|
| Scout | `=>` | 10 | 3 | 10 | Free | Start |
| Fighter | `═╝►` | 20 | 8 | 8 | 100 | Lv.3 |
| Bomber | `═══►` | 30 | 20 | 4 | 300 | Lv.7 |
| Frigate | `═══╗═╝►` | 80 | 15 | 5 | 500 | Lv.10 |
| Destroyer | (2-line) | 150 | 35 | 3 | 1500 | Lv.18 |
| Capital | (3-line) | 500 | 80 | 2 | 5000 | Lv.30 |
| Carrier | (3-line, spawns fighters) | 300 | 10 | 2 | 8000 | Boss #5 |

## Resources

- **Scrap** `◇` — collected during travel, used for ship repairs and basic upgrades
- **Credits** `₿` — earned from battles and raids, used for buying ships and tech
- **Blueprints** `📋` — rare drops, unlock new ship types and tech tiers
- **Artifacts** `◈` — very rare, prestige currency for permanent bonuses

## Progression

### Visual Progression (what you SEE changing)
- Fleet grows from 1 scout to 15+ ship armada
- Ships get visually larger/more detailed with upgrades
- Enemies scale: pirates → militia → military → alien → boss encounters
- Planets scale: outpost → colony → city → metropolis → megastructure
- Space gets busier: more debris, more nebulae, more traffic
- Battle effects intensify: more projectiles, bigger explosions, beam weapons

### Mechanical Progression
- Sectors increase in difficulty and reward
- New enemy types with different behaviors
- Boss encounters every 10 sectors (unique multi-phase battles)
- Planet types with different resources and defenses
- Tech tree unlocks new weapon types, passive abilities
- Fleet formation options change combat effectiveness

## Technical Design

### Architecture
```
src/
  main.rs              — app setup, main loop, input handling
  state.rs             — GameState, save/load JSON
  scenes/
    mod.rs             — Scene trait, scene transitions
    travel.rs          — star field, fleet cruise, collection
    battle.rs          — combat simulation, projectiles
    raid.rs            — planet surface, tractor beams
    loot.rs            — results display
    upgrades.rs        — shop/upgrade UI
  engine/
    mod.rs
    ship.rs            — Ship struct, ShipType, stats
    fleet.rs           — Fleet management, formation
    combat.rs          — damage calc, targeting AI, battle sim
    economy.rs         — resources, costs, unlock checks
    procedural.rs      — sector gen, enemy gen, planet gen
  rendering/
    mod.rs
    particles.rs       — particle system (explosions, beams, debris)
    sprites.rs         — ASCII sprite definitions for ships
    starfield.rs       — parallax star background
    effects.rs         — screen flash, transitions, color cycling
```

### Game Loop
- 20fps tick rate (50ms per frame)
- Each tick: update game state → update scene → render
- Input checked each tick (non-blocking)
- Scene transitions: fade out (5 frames) → switch → fade in (5 frames)

### Rendering
- ratatui with crossterm backend
- Full terminal canvas — no panels or borders in scene mode
- Upgrade screen uses ratatui widgets (List, Gauge, Table)
- Color: 256-color for effects, fallback to 16 for basic terminals
- Particle system: Vec<Particle> with position, velocity, lifetime, character, color

### Particle System
```rust
struct Particle {
    x: f32, y: f32,        // position (fractional for smooth movement)
    vx: f32, vy: f32,      // velocity
    life: u8,              // remaining frames
    max_life: u8,          // for fade calculation
    ch: char,              // render character
    color: Color,          // ratatui color
}
```
Used for: explosions, beam trails, collection sparkles, engine exhaust, debris

### State Persistence
- Save to `~/.voidfleet/save.json` on scene transitions and every 60 seconds
- Load on startup, resume from last sector/phase
- Save includes: fleet composition, resources, upgrades, current sector, stats

### Dependencies
- ratatui + crossterm (TUI)
- serde + serde_json (save/load)
- rand (procedural generation)
- No async needed — single-threaded game loop
