# Survivors Prototype - Design Document

> **Last Updated:** 2026-09-08 (dependency migration; game design remains provisional)
> **Branch:** `magick_circle_pattern_select` (dependency upgrade merged)
> **Engine:** Bevy 0.19.1 (Rust 1.95+, tested with 1.96)

---

## Table of Contents

1. [Project Vision](#1-project-vision)
2. [Current Implementation Status](#2-current-implementation-status)
3. [Architecture Overview](#3-architecture-overview)
4. [Core Systems (Implemented)](#4-core-systems-implemented)
5. [Magick Circle Weapon (Implemented)](#5-magick-circle-weapon-implemented)
6. [Upgrade System (Implemented)](#6-upgrade-system-implemented)
7. [UI and Menus (Implemented)](#7-ui-and-menus-implemented)
8. [Planned Systems (Not Yet Implemented)](#8-planned-systems-not-yet-implemented)
9. [Balance Reference](#9-balance-reference)
10. [Technical Conventions](#10-technical-conventions)
11. [Open Questions and TODOs](#11-open-questions-and-todos)

---

## 1. Project Vision

An occult-themed survivors-like game inspired by Vampire Survivors and HoloCure, set in a world where various traditions of magick are treated as authentic systems rather than superficial fantasy tropes. The game blends accessible survivors-style gameplay with deep, discoverable mechanics rooted in real occult traditions.

### Core Inspirations

- **Gameplay:** Vampire Survivors, HoloCure: Save the Fans
- **Depth/Discovery:** Morrowind, Bloodborne, Noita, Dwarf Fortress, Factorio
- **Thematic:** Solomonic Magick, Kabbalah, Enochian Magick, Alchemy, Hermeticism, and many more

### Long-Term Vision

- **Hub World:** A small village (Stardew Valley-like) with NPCs, shops, a player-owned estate, ritual spaces, and resource management
- **Expedition System:** Map room for planning expeditions; expeditions play out as survivors-like runs
- **Multiple Reality Layers:** Hidden areas and alternate planes (inspired by Noita, Bloodborne insight)
- **Magnum Opus Progression:** Four-stage alchemical progression (Nigredo, Albedo, Citrinitas, Rubedo) serving as the game's meta-progression arc
- **Post-Completion Content:** Revelation about reality's nature, the Demiurge, opportunity to "unspeak the First Word"

### Design Philosophy

- Authentic treatment of occult traditions
- Deep, discoverable systems that reward exploration and mastery
- Accessible core gameplay loop with optional depth
- "As Above, So Below" principle reflected in world and system design

---

## 2. Current Implementation Status

### What Exists (Playable)

| System | Status | Notes |
|--------|--------|-------|
| Player movement (WASD) | Done | 150 base speed |
| Enemy spawning (waves) | Done | 20 per wave, 2s interval |
| Magick Circle weapon | Done | 5 pattern types, orbiting sigils |
| Pattern selection menu | Done | Choose pattern on circle add |
| Level-up upgrade menu | Done | 3-4 choices, keyboard + mouse nav |
| Pause menu | Done | Resume / Quit |
| Experience / leveling | Done | Orb pickups, exponential scaling |
| Health / damage | Done | Cooldown-based, protection buff |
| Physics (Rapier2D) | Done | Collision groups, sensors |
| HUD (health, timer, kills) | Done | Health bar, game timer, kill count |
| Circle overlap detection | In Progress | Tracking overlapping patterns per enemy |
| Weapon upgrade progression | Done | 8 levels + limit breaks |

### What Does Not Exist Yet

- Main menu / title screen
- Settings menu
- Sound / music
- Multiple weapon types (only Magick Circle)
- Equipment / item system
- Stat upgrades (framework exists, no implementations)
- Hub world / village
- Expedition system
- Multiple enemy types (only basic chasers)
- Boss encounters
- Persistent progression / save system
- Any magical tradition beyond Circle Magick

---

## 3. Architecture Overview

### Tech Stack

- **Language:** Rust (Edition 2021)
- **Engine:** Bevy 0.19.1 (ECS architecture, 2D/UI features)
- **Physics:** bevy_rapier2d 0.36.0 (published crate)
- **Shape Rendering:** bevy_prototype_lyon 0.17.0
- **RNG:** rand 0.10.2
- **Enum Utilities:** strum 0.28 with the derive feature

### Source File Map

```
src/
  main.rs             Entry point, plugin registration, system scheduling
  components.rs       ECS components (Player, Enemy, Health, multipliers)
  resources.rs        Game state enum, textures, wave config, game timer
  types.rs            Enums: Rarity, EquipmentType, StatType
  events.rs           EntityDeathEvent
  systems.rs          Core gameplay (spawning, movement, vacuum, victory)
  physics.rs          Rapier setup, collision groups, player-enemy collision
  combat.rs           Damage handling with cooldowns
  experience.rs       XP orbs, collection, level-up check
  death.rs            Death marking, cleanup, experience spawning
  ui.rs               HUD elements (health bar, timer, kill counter)
  menu.rs             All menus (pause, level-up, pattern select)
  upgrade.rs          Upgrade generation and application
  weapons/
    mod.rs            Weapon plugin, weapon systems, upgrade application
    magick_circle.rs  Magick Circle implementation (patterns, spawning, effects)
    weapon_upgrade.rs Upgrade config and spec definitions
    overlap.rs        Circle overlap detection system
```

### Game State Machine

```
MainMenu  -->  Playing  <-->  LevelUp
                  |              |
                  v              v
               Paused      PatternSelect (sub-state within LevelUp)
                  |
                  v
              GameOver
```

States are defined in `resources.rs` as `GameState`:
- `MainMenu` - Not implemented yet
- `Playing` - Active gameplay; all core systems run
- `LevelUp` - Upgrade selection menu; gameplay paused
- `Paused` - Pause menu shown
- `GameOver` - Player death
- `Settings` - Not implemented yet
- `Quit` - Exit

### ECS Organization Patterns

- Systems are grouped into ordered sets using `.chain()` for execution dependencies
- State-gated execution via `run_if(in_state(GameState::Playing))`
- `ParamSet` used to avoid query conflicts in complex systems
- Frame-scoped resources (`EnemyCircleHits`, `ActiveOverlaps`) cleared each frame
- Buffered messages for decoupled communication: `DamageEvent`, `EntityDeathEvent`, `WeaponUpgradeConfirmedEvent`, `AddWeaponEvent`, etc. (the existing type names are retained).

### Collision Groups (Rapier)

| Group | Entity Type | Behavior |
|-------|-------------|----------|
| GROUP_1 | Player | Kinematic rigid body |
| GROUP_2 | Enemies | Dynamic with damping |
| GROUP_3 | Projectiles / Attacks | Sensors |
| GROUP_4 | Experience orbs | Sensors |

---

## 4. Core Systems (Implemented)

### 4.1 Player

**Spawning** (`systems.rs: spawn_player`):
- Sprite: `sprites/player.png` (32x32)
- Health: 100 HP
- Speed: 150
- Magnet strength: 150 range, 1.0 speed
- Starts with: 1 Magick Circle (Banishment pattern)
- Multiplier components initialized: CooldownReduction(0), DamageMultiplier(1.0), AreaMultiplier(1.0), Luck(20)

**Movement** (`systems.rs: gameplay_movement_system`):
- WASD input, normalized diagonal movement
- Speed comes from `Player::speed`, initialized to 150

### 4.2 Enemies

**Spawning** (`systems.rs: spawn_enemies`):
- Spawn in waves at 400px distance from player
- 20 enemies max per wave, 2-second spawn interval
- Random position on circle around player
- Chase AI: move directly toward player

**Movement** (`systems.rs: enemy_movement`):
- Base speed toward player
- Affected by Binding (slowed) and Invocation (pulled toward circle)
- Dynamic rigid body with linear damping

### 4.3 Experience and Leveling

**Orb Spawning** (`experience.rs: spawn_experience_orbs`):
- On enemy death, drop XP orbs
- XP value: `enemy_exp * 66` per orb

**Collection** (`experience.rs: collect_experience_orbs`):
- Player magnet pulls orbs within range (150px)
- Contact with player collects orb

**Level-Up Check** (`experience.rs: check_level_up`):
- XP threshold scales exponentially: `base * 1.25^level`
- Triggers `GameState::LevelUp` when threshold reached

### 4.4 Combat

**Damage System** (`combat.rs: handle_damage`):
- Processes `DamageEvent` with source, target, amount
- Cooldown protection: 0.25s between hits on an entity with `DamageCooldown`, shared across sources
- `ProtectionBuff` applies 50% damage reduction
- Marks the entity for death when health reaches 0; `death_system` emits `EntityDeathEvent`

### 4.5 Victory Condition

- A threshold of 200 kills is configured but not consumed by active code; victory handling and its screen remain unimplemented

---

## 5. Magick Circle Weapon (Implemented)

The Magick Circle is the only weapon currently in the game. It spawns orbiting sigils around the player that deal area-of-effect damage or apply status effects to enemies.

### 5.1 Pattern Types

Each pattern has distinct visual identity, behavior, and gameplay role:

| Pattern | Color | Sigil Shape | Sigil Count | Orbit Speed | Effect |
|---------|-------|-------------|-------------|-------------|--------|
| **Banishment** | Red-orange | Triangle (3-sided) | 4 | 1.5 (fast) | Direct damage |
| **Binding** | Purple | Hexagon (6-sided) | 6 | 0.5 (slow) | Roots enemies; expires 1s after the last effect tick |
| **Invocation** | Teal | Pentagram (5-sided) | 5 | -1.0 (reverse) | Pulls enemies toward center |
| **Manifestation** | Green | Star (6-pointed) | 3 | 0.8 (moderate) | 1.5x damage bursts |
| **Protection** | Silver-blue | Diamond (4-sided) | 4 | 0.7 (steady) | 50% damage reduction to player |

### 5.2 Circle Mechanics

- **Spawning:** Circles spawn on cooldown (base 3.5s), positioned around the player
- **Duration:** Each circle lasts 3 seconds before despawning
- **Area Effect Tick:** Damage and Binding refresh every 0.5 seconds. Invocation pulls continuously while inside; Protection is checked every frame.
- **Orbiting Sigils:** Visual sigils orbit the circle center at pattern-specific speeds
- **Base Damage:** 10 (modified by upgrades and multipliers)
- **Base Radius:** 64 pixels

### 5.3 Key Components

- `MagickCircle` - Holds the list of patterns and sigil count
- `Sigil` / `Orbits` - Individual sigil positioning and animation
- `Attack` - Marker for spawned circle attacks
- `Lifetime` - Despawn timer
- `AreaEffect` - Tick rate for periodic damage
- `BindingEffect` - Movement root with expiry (external Invocation pulls can still move enemies)
- `InvocationPull` - Pull-toward-center force
- `ProtectionBuff` - Damage reduction applied to player

### 5.4 Overlap System (In Progress)

Located in `weapons/overlap.rs`. Detects when enemies are inside multiple circles simultaneously:

- `EnemyCircleHits` - Maps each enemy to the set of circles affecting it
- `ActiveOverlaps` - Tracks physical circle pairs and their shared enemies; lookup helpers return unique, unordered pattern combinations
- `OverlappingPatterns` - Component attached to enemies in 2+ circles

This is the foundation for combo/synergy effects between different pattern types (not yet implemented).

Pattern combinations have stable ordering and include same-pattern overlaps. Multiple physical circle pairs remain available separately, so a future combo design can choose its own stacking rules. Weapon effects run before enemy movement and combat; damage is handled before death processing.

---

## 6. Upgrade System (Implemented)

### 6.1 Level-Up Flow

1. Player collects enough XP -> `GameState::LevelUp`
2. System generates 3 upgrade choices (4 if Luck roll succeeds: 2% per Luck point)
3. Player selects an upgrade via keyboard or mouse
4. If upgrade is `AddCircle` with no preset pattern -> `PatternSelect` sub-menu appears
5. Upgrade is applied, game resumes

### 6.2 Upgrade Types

**Weapon Upgrades** (`UpgradeType::Weapon`):
- Tied to a specific weapon type
- Defined by `WeaponUpgradeSpec` containing one or more `WeaponUpgradeChange` entries

**Generic Upgrades** (`UpgradeType::Generic`):
- `HealthPickup(20)` - "Philosopher's Elixir" - restores 20 HP
- `ResourcePickup(100)` - "Void Shards" - placeholder for future resource system

### 6.3 Magick Circle Upgrade Progression

| Level | Changes |
|-------|---------|
| 2 | +2 Damage, +1 Area |
| 3 | Add circle (pattern select) |
| 4 | Add circle + +1 Damage, +1 Area |
| 5 | Add circle |
| 6 | Add circle + +2 Damage, +1 Area |
| 7 | Add circle + +1 Damage, +1 Area |
| 8 | Add circle + +3 Damage, +2 Area |

**Limit Breaks (post level 8):**
- +2 Damage
- +2 Area
- +2 Duration
- -2 Cooldown

### 6.4 Rarity System

Upgrades have rarity tiers with associated colors (used in menu borders):

| Rarity | Color |
|--------|-------|
| Common | Gray |
| Uncommon | Green |
| Rare | Blue |
| Epic | Purple |
| Legendary | Gold/Orange |

The Luck stat influences the chance of receiving a 4th upgrade option.

---

## 7. UI and Menus (Implemented)

### 7.1 HUD (`ui.rs`)

Spawned when entering `GameState::Playing`:
- **Health Bar:** Red bar (200x30px) with text overlay showing "current/max"
- **Game Timer:** Center-top, "MM:SS" format
- **Kill Counter:** Top-right, "Kills: X"

### 7.2 Level-Up Menu (`menu.rs`)

- Shows 3-4 upgrade cards with icon, name, and description
- Rarity-colored borders
- First item auto-selected
- Navigation: Arrow keys, WASD, or mouse hover/click
- Confirmation: Enter, Space, or mouse click
- Scrolls text to prevent overflow/overlap

### 7.3 Pattern Selection Menu (`menu.rs`)

- Triggered when a circle-add upgrade has no preset pattern
- Shows all 5 pattern types with names and descriptions
- Same navigation as level-up menu
- Frame-skip guard prevents accidental double-confirmation from held keys

### 7.4 Pause Menu (`menu.rs`)

- Triggered by Escape key
- Resume / Quit options
- Keyboard and mouse navigation

---

## 8. Planned Systems (Not Yet Implemented)

This section documents the broader game vision. These systems exist only in design documents, not in code. See `design/project-summary.md` and `design/magic-dashboard-refined.tsx` for the full vision.

### 8.1 Hub World / Estate

A village hub between expedition runs:
- NPC shops and services
- Player-owned estate with ritual spaces
- Resource management and crafting
- Excavation / discovery mechanics
- Run preparation and loadout customization

### 8.2 Magical Traditions

The full game envisions 12+ magical traditions, each as a distinct weapon/ability system:

- **Solomonic Magick** - Divine authority, angel summoning, binding circles
- **Alchemical Magick** - Four-stage transmutation (Nigredo/Albedo/Citrinitas/Rubedo)
- **Enochian Magick** - Angelic alphabet weapons, Watchtower zones, 30 Aethyrs
- **Kabbalah** - Tree of Life paths, Sephiroth weapons, sacred geometry
- **Hermetic** - "As Above, So Below" mirroring, planetary forces, correspondences
- **Vedic/Hindu** - Yantra patterns, mantra sound weapons, chakra system
- **Chinese** - Wu Xing (Five Elements), Feng Shui, Bagua, Qi channeling
- **Voudon** - Loa channeling, Serviteur allies, Veve pattern zones
- **Egyptian** - Heka (name-based power), Book spells, Ka/Ba manipulation
- **Norse** - Seidr fate-weaving, Galdr rune songs, Bind-runes, World-tree
- **Tibetan** - Tantric energy, Mandala effects, Prayer wheels
- **Gnostic** - Archon binding, Pleroma energy, Aeon manifestation

### 8.3 Additional Weapon Systems (Planned)

Beyond Circle Magick, the design envisions:
- **Magick Square Weapons** - Numerical grid patterns that charge and trigger
- **Sigil Magick** - Auto-drawing floating sigils with intent-based effects
- **Mirror Weapons** - Cross-reality damage and reflection mechanics
- **Quantum Weapons** - Probability manipulation and superposition
- **Non-Euclidean Weapons** - Space-warping geometry attacks

### 8.4 Item System (Planned)

- **Magick Square Talismans** - Passive planetary effects, estate-charged
- **Sacred Stones** - Gem-based passives with crystal grid combinations
- **Ritual Components** - Herbs, oils, candles, woods for temporary buffs and zones
- **Prayer Wheels** - Movement-powered energy accumulation

### 8.5 Progression Systems (Planned)

**Magnum Opus (meta-progression):**
1. Nigredo (Blackening) - Basic reality manipulation, initial unlocks
2. Albedo (Whitening) - Purification, reality layering
3. Citrinitas (Yellowing) - Solar energies, consciousness expansion
4. Rubedo (Reddening) - Final transformation, reality mastery

**Tradition Mastery** - Per-tradition skill trees with cross-synergy bonuses

**Reality Understanding** - Discovering hidden layers, uncovering truths about the world

### 8.6 Unique Mechanics (Planned)

- **Time Dilation** - Adjustable game speed (0.0-4.0x), unlocked through rituals
- **Reality Inversion** - Mirrored controls/visuals for alternate reality layers
- **Loadout System** - Point-based starting weapon selection
- **Circle Overlap Combos** - Synergy effects when different patterns affect the same enemy

### 8.7 Thematic Elements (Planned)

- Cosmic horror (distinct from Lovecraftian tropes)
- Liminal horror
- Layered / mirrored / glitched realities
- Temporal anomalies
- Post-completion revelation about the Demiurge and the nature of reality

---

## 9. Balance Reference

### Current Values

| Parameter | Value |
|-----------|-------|
| Player HP | 100 |
| Player Speed | 150 |
| Magnet Range | 150 px |
| Magnet Speed | 1.0 |
| Enemy Spawn Distance | 400 px |
| Enemies Per Wave | 20 |
| Wave Spawn Interval | 2.0 s |
| Victory Kill Threshold | 200 |
| Damage Cooldown | 0.25 s |
| XP per Orb | enemy_exp * 66 |
| XP Scaling | base * 1.25^level |
| Luck (4th option chance) | 2% per point (default 20 = 40%) |
| Circle Base Damage | 10 |
| Circle Base Radius | 64 px |
| Circle Base Cooldown | 3.5 s |
| Circle Duration | 3.0 s |
| Area Effect Tick | 0.5 s |
| Protection Reduction | 50% |
| Binding Duration | 1.0 s |

### Rarity Colors (UI)

| Rarity | RGB |
|--------|-----|
| Common | Gray |
| Uncommon | Green |
| Rare | Blue |
| Epic | Purple |
| Legendary | Gold |

---

## 10. Technical Conventions

### Naming

- Components: PascalCase structs with `#[derive(Component)]`
- Resources: PascalCase structs with `#[derive(Resource)]`
- Systems: snake_case functions
- Events: PascalCase structs ending in `Event`
- Enums: PascalCase with strum derives for iteration

### Patterns

- New weapons should follow the pattern established by `weapons/magick_circle.rs`: self-contained module with its own components, spawn logic, and effect systems
- Upgrade specs go in `weapons/weapon_upgrade.rs`
- All weapons register through `WeaponPlugin` in `weapons/mod.rs`
- State transitions handle menu/gameplay boundaries
- Collision detection uses Rapier sensor colliders for overlap detection without physics response

### Assets

```
assets/
  sprites/
    player.png         32x32, 1 frame
    enemies.png        32x32, 2 types
    projectiles.png    16x16, 2 types
```

---

## 11. Open Questions and TODOs

### Near-Term

- [ ] Circle overlap combo effects - what happens when Banishment + Binding overlap? Define combo table
- [ ] Enemy variety - different enemy types with distinct behavior
- [ ] Main menu implementation
- [ ] Sound design - what audio style fits the occult theme?
- [ ] Victory screen / game over screen

### Medium-Term

- [ ] Second weapon type - which tradition should be next?
- [ ] Equipment/item slots - how many? What types?
- [ ] Stat upgrade system - which stats beyond the current multipliers?
- [ ] Boss encounters - when do they appear? What triggers them?
- [ ] Visual effects polish - particles, screen shake, impact feedback

### Long-Term

- [ ] Hub world scope and layout
- [ ] Expedition system design (map room, run modifiers)
- [ ] Reality layer implementation - how do alternate planes work mechanically?
- [ ] Save/persistence system
- [ ] Magnum Opus progression - how does this gate content?
- [ ] Multi-tradition unlock order - linear? player-chosen? discovery-based?

### Design Decisions Needed

- How should circle overlap combos work? Additive effects? Unique combo effects per pair?
- Should traditions be unlocked sequentially or chosen by the player?
- How does the hub world connect to the survivors gameplay loop?
- What is the target run length for an expedition?
- How does difficulty scale - enemy stats, wave size, enemy types, or all of the above?

---

*This document reflects the state of the codebase as of the last update date. The "Planned Systems" section is aspirational and subject to change as development progresses.*
