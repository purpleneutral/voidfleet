# Changelog

All notable changes to Void Fleet will be documented here.

## [0.1.0] — 2026-03-26

### 🎮 Initial Release

**Scenes**
- Travel scene with parallax starfield, nebula clouds, asteroid fields, and warp transitions
- Battle scene with projectile combat, ship AI (flanking, focus fire, dodging, retreating), and chain explosions
- Raid scene with 5 planet types, tractor beams, panicking surface entities, and turret defenses
- Loot scene with animated item reveal, XP bar, and performance-based rewards
- Title screen with ASCII art logo and animated starfield

**Ships & Combat**
- 7 ship types: Scout, Fighter, Bomber, Frigate, Destroyer, Capital Ship, Carrier
- Ship special abilities: AOE blasts, shield bubbles, broadsides, beam weapons, fighter launches
- Tech upgrades affect combat: lasers boost damage, shields boost HP, engines boost fire rate
- Adaptive difficulty with rubber banding — game adjusts to your skill level
- Boss encounters every 10 sectors with escalating difficulty

**Companion**
- Pip — a tiny robot companion living on your bridge
- 8 moods: idle, happy, sleeping, eating, sad, excited, dancing, lonely
- Tamagotchi mechanics: hunger, energy, happiness, bond level
- Visual evolution through 5 stages (basic → antenna → visor → wings → crown)
- Gift shop with gameplay bonuses (combat damage, travel speed, loot multiplier)
- Reacts to game events — celebrates wins, mourns losses, dances on achievements

**Progression**
- 13 achievements from "First Blood" to "Admiral"
- Prestige system — reset at sector 30+ for permanent XP/credit/scrap bonuses
- Sector map with branching route choices (5 route types with different risk/reward)
- 7 random travel events with meaningful choices

**Systems**
- Dynamic economy with catch-up mechanics for struggling players
- Fleet death penalty: lose resources and get pushed back sectors
- Save/load to `~/.voidfleet/save.json` with auto-save every 60 seconds
- Context-aware HUD showing relevant controls per screen
- 20fps animation with particle system (explosions, exhaust, sparkles, beams)
