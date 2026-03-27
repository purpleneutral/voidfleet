# Changelog

All notable changes to Void Fleet will be documented here.

## [0.2.0] — 2026-03-27

### 🚀 Voyage System
- Themed multi-sector voyages with escalating prestige
- 5 unique voyage themes; infinite scaling beyond that
- Permanent bonuses on completion: damage, hull HP, speed, crit
- Voyage completion cinematic scene with stats recap

### 🏪 Trade Economy
- 8 trade goods: Ore, Food, Tech, Weapons, Luxuries, Med Supplies, Contraband, Artifacts
- Dynamic sector markets with supply/demand pricing
- Faction price modifiers — Trade Guild gets better deals
- Contraband risk system with detection and penalties
- Cargo hold management

### 📋 Mission Contracts
- Procedurally generated missions issued by factions
- 7 mission types: Bounty Hunt, Delivery, Escort, Exploration, Sabotage, Rescue, Trade Run
- Mission journal with Available / Active / Log tabs
- Rewards: credits, faction reputation, equipment drops
- Difficulty ratings (1-5 stars)

### ⚔️ Faction Diplomacy
- 5 factions: Trade Guild, Pirate Clan, Military Corp, Alien Collective, Rebel Alliance
- Reputation tracking with each faction
- Faction rivalries and alliances
- Sector control influenced by faction presence
- Diplomacy screen showing standings and relations

### 👥 Crew System
- 6 crew classes: Pilot, Gunner, Engineer, Medic, Captain, Navigator
- Procedurally generated crew with names, stats, and personalities
- Crew abilities with varied triggers (battle start, HP threshold, periodic, passive)
- Ability effects: Evasive Maneuvers, Afterburner, Lock On, Barrage, Homing Shots, Passive Regen, Emergency Repair, Shield Overclock, Triage, and more
- Crew bonds system — relationships between crew members unlock synergy bonuses
- Ship crew assignments
- Recruitment screen

### 🎒 Equipment & Inventory
- Loot drops with 5 rarity tiers: Common, Uncommon, Rare, Epic, Legendary
- Equipment slots per ship
- Equipment set bonuses for collecting matching pieces
- Salvage system — break down equipment for scrap
- Inventory management with fleet loadout view

### 📜 Game Log
- Scrollable event history with colored, categorized entries
- Sector markers, combat events, economy, crew, and progression tracking
- Auto-scroll with manual override

### 🛠️ Quality of Life
- Help screen (`?`) with full controls reference
- Game event system powering log, Pip reactions, and achievement tracking
- Context-aware HUD updates for all new screens

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
