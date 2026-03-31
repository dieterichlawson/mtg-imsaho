# Architecture & Extensibility Report (2026-03-29)

## 1. Architecture Overview

### Core Architecture Pattern
The MTG game engine uses a **clean separation-of-concerns design** with immutable game state and event-driven updates:

- **Game State (`state.rs`)**: Single immutable `GameState` struct containing all game objects, player information, turn structure, and stack. Clone-based state transitions preserve history.
- **Card Registry (`cards/mod.rs`)**: Centralized registry mapping `CardId` to `CardBehavior` trait implementations. Each card is a discrete unit implementing behavior hooks.
- **Action-Driven Engine (`engine.rs`)**: Processes actions, validates legality, generates state transitions via `submit_action`.
- **Event System (`events.rs`)**: Emits typed events (`GameEvent` enum) for triggered abilities, logging, and UI updates.
- **Triggered Ability Pipeline (`triggers.rs`)**: Processes events through card hooks (`on_enter_battlefield`, `on_dies`, `on_any_creature_dies`, `on_leave_battlefield`).
- **State-Based Actions (`sba.rs`)**: Implements rules 704 (death by lethal damage, zero toughness, aura attachment checks, token cleanup, game end).

### Module Dependency Graph
```
engine.rs ────────────┬─────────────────── state.rs
                      ├─── cards/mod.rs (registry)
                      ├─── types.rs (enums)
                      ├─── combat.rs
                      ├─── sba.rs
                      ├─── triggers.rs
                      ├─── destruction.rs
                      ├─── actions.rs
                      ├─── mana.rs
                      └─── stack.rs

cards/*.rs ───────────── Each implements CardBehavior trait
```

### Data Flow for Spell Resolution
1. Player chooses action -> `submit_action(state, action, registry)`
2. Engine validates legality, deducts mana, moves spell to stack
3. Stack resolves: `on_resolve()` hook called on spell
4. SBAs checked: `check_state_based_actions_with_registry()`
5. Triggers processed: `process_triggers()` dispatches events to `CardBehavior` hooks
6. If mid-resolution choice needed: set `state.awaiting_action`, pause
7. Otherwise: return to step 2 (next player's priority)

### Key Type System Choices

**Strengths:**
- **ObjectId/CardId/PlayerId**: Newtype wrappers provide type safety and prevent ID confusion
- **Zone enum**: Exhaustive match-based zone transitions prevent invalid moves
- **Keyword enum**: Compile-time guarantees for supported keywords, easy extension
- **CardType enum**: Type-safe card category checks without string comparisons
- **ManaCost/ManaSymbol**: Structured representation of mana costs with proper algebra (mana_value, colored_requirements)

**Weaknesses:**
- **CardBehavior trait**: Overly coarse-grained; no mechanism for hookable replacement effects or cost reduction
- **Oracle text parsing**: String-based parsing for aura bonuses/anthem effects (`parse_aura_power_bonus`, `parse_anthem_power`) is fragile and unscalable

---

## 2. Extensibility Analysis for Complex MTG Mechanics

### Throne of Eldraine Mechanics

**Adventures (two-faced creature + instant/sorcery)**: MAJOR REFACTORING NEEDED
- Current architecture: One `CardData` per `CardId`
- Problem: Adventures need two distinct faces with different types, costs, and abilities
- Solution: Extend `CardData` with optional `adventure_face: Option<AdventureData>`. Modify casting pipeline to show both faces when casting. When adventure resolves, need to return creature to hand instead of moving to graveyard.
- Difficulty: **HIGH** (affects card representation, casting, resolution pipeline)

**Food Tokens**: SUPPORTED WITH MINOR EXTENSION
- Current: `create_token()` creates creature tokens
- Food tokens: Need artifact tokens with no P/T
- Difficulty: **TRIVIAL**

**Adamant (caring about mana colors spent)**: MAJOR REFACTORING NEEDED
- Current: No tracking of what colors were paid to cast a spell
- Problem: `ManaCost` is static; doesn't track actual payment
- Solution: Add `actual_mana_paid: Vec<ManaType>` to `GameObject` when on stack
- Difficulty: **HIGH** (requires payment tracking infrastructure)

### Modern Horizons / MH2 Mechanics

**Suspend**: MAJOR REFACTORING NEEDED
- No concept of cards in "suspend zone" or time counters
- Difficulty: **HIGH**

**Cascade**: MEDIUM EXTENSION
- Mechanism: Exile cards until one with cost less than spell is found
- Difficulty: **MEDIUM** (card revelation, dynamic choice, exile management)

**Evoke**: MEDIUM EXTENSION
- Mechanism: Alternative casting cost that sacrifices creature when it enters
- Difficulty: **MEDIUM**

**Overload**: MEDIUM-HIGH EXTENSION
- Mechanism: Alternative casting cost that changes "target creature" to "each creature"
- Difficulty: **MEDIUM-HIGH**

**Storm**: MAJOR REFACTORING NEEDED
- No "spells cast this turn" counter in state
- Difficulty: **HIGH** (needs state tracking, copy mechanism)

**Modular**: TRIVIAL
- Just implement `on_dies()` to create tokens. Already supported by Mausoleum Guard pattern.

**Living Weapon**: EASY EXTENSION
- Difficulty: **EASY**

**Affinity**: MEDIUM EXTENSION
- No cost reduction system exists
- Difficulty: **MEDIUM** (cost algebra, validation)

**Tribal Synergies**: TRIVIAL
- Already supports creature subtypes

**Graveyard-Matters**: EASY-MEDIUM EXTENSION
- Can iterate `objects_in_zone(Zone::Graveyard, player)` and count types

### Innistrad Mechanics

**Transform/DFCs**: MAJOR REFACTORING NEEDED
- One `CardData` per card; werewolves need front/back faces
- Difficulty: **VERY HIGH** (affects card registry, object representation, turn tracking)

**Disturb**: EASY EXTENSION (once DFCs exist)

**Flashback**: ALREADY IMPLEMENTED

**Morbid**: IMPLEMENTED

**Investigate/Clue Tokens**: EASY EXTENSION

**Madness**: MAJOR REFACTORING NEEDED
- Needs mid-action resolution choice during discard
- Difficulty: **HIGH**

**Delirium**: EASY-MEDIUM EXTENSION

**Exploit**: EASY EXTENSION

**Skulk**: MEDIUM EXTENSION

**Emerge**: MEDIUM EXTENSION

**Meld**: MAJOR REFACTORING NEEDED - Difficulty: **VERY HIGH**

**Decayed Tokens**: EASY EXTENSION

**Daybound/Nightbound**: REQUIRES DFC ARCHITECTURE - Difficulty: **VERY HIGH**

**Training**: EASY EXTENSION

**Blood Tokens**: TRIVIAL

**Cleave**: MEDIUM EXTENSION

---

## 3. Design Patterns Analysis

### Patterns Used

**1. Strategy Pattern (CardBehavior Trait)**
- Good: Simple, extensible, easy to add new cards
- Gap: No mechanism for replacement effects or state-modifying effects that interact with other cards

**2. Registry Pattern (CardRegistry)**
- Good: Centralized card lookup, supports dynamic registration, name-based lookup
- Gap: No versioning/hot-reloading; card implementations are compiled in

**3. State Machine Pattern (Step/Phase/Priority)**
- Good: Clear turn progression, type-safe step transitions

**4. Event Sourcing (GameEvent Enum)**
- Good: Game log, replay support, triggered ability dispatch, extensible
- Gap: No replacement effect framework to intercept events

**5. Immutable State with Cloning**
- Good: No hidden state mutations, clear causality
- Bad: Performance cost on large games (cloning entire HashMap of objects)

**6. Continuous Effects via Oracle Text Parsing**
- Bad: Fragile, unscalable, no type safety

### Rules Engine Separation

**Good separation:**
- Engine doesn't know about card details; uses `CardBehavior` trait
- Cards don't directly modify state; use provided hooks and functions
- Tests can mock registry for unit testing specific card behavior

**Gaps:**
- No replacement effects framework
- No layer system for continuous effects
- No cost modification system
- No target modification system

### Triggered Abilities

**Good:**
- Simple event-driven model
- Death-watch pattern
- ETB triggers work
- Trigger processing pauses if mid-resolution choice needed

**Gaps:**
- No optional triggered abilities
- No trigger ordering (player chooses order per 116.4)
- No conditional triggers
- No damage/spell-cast triggers processed in trigger loop
- No leaving the battlefield trigger dispatch

---

## 4. Scalability Concerns

### Card Registration & Lookup
- Static registration: All cards compiled in; can't add new cards without recompilation
- Manual: Each new card requires one line in registry + one file
- Performance: HashMap lookup is O(1)
- With 100+ cards, will become a long list; no organization/category

### Card Text and Behavior Association
- 0-100 cards: Current approach is fine
- 100-500 cards: Will start seeing repetitive code; macro-based generation could help
- 500+ cards: Need systematic effect representation (not string parsing)

### Game State Size
- `HashMap<ObjectId, GameObject>` could grow to thousands of objects
- Each `submit_action()` clones entire `GameState`
- State cloning could be optimized via copy-on-write or structural sharing

---

## 5. Type System Usage

### Strong Type System Leverage
1. Newtype pattern for IDs (ObjectId/CardId/PlayerId)
2. Enum-based state machines (Zone, Phase, AwaitingAction)
3. Option/Result for fallible operations
4. Trait objects for extensibility

### Weak or Missing Type System Patterns
1. **Stringly-typed Oracle Text**: `parse_aura_power_bonus()` parses strings at runtime
2. **Loose card data representation**: `subtypes: Vec<String>`, `oracle_text: String` with no validation
3. **Fragile aura attachment**: `attached_to: Option<ObjectId>` used for both auras and equipment
4. **Cast mode not tracked on spell**: `cast_with_flashback: bool` doesn't generalize
5. **Target specification not typed**: `CreatureWithFilter(String)` untyped filter

---

## 6. Critical Gaps for Complex Set Support

| Feature | Difficulty | Impact |
|---------|-----------|--------|
| Adventures | VERY HIGH | Throne of Eldraine |
| Double-Faced Cards | VERY HIGH | Innistrad werewolves, DFCs |
| Replacement Effects | VERY HIGH | "If X would happen, instead Y" |
| Cost Reduction | MEDIUM | Affinity, Blasphemous Act |
| Suspend | HIGH | Modern Horizons |
| Storm | HIGH | Modern Horizons |
| Overload | MEDIUM-HIGH | Modern Horizons |
| Mana-Spent Tracking | HIGH | Adamant (Throne) |
| Optional Triggers | MEDIUM | Many mechanics |
| Trigger Ordering | MEDIUM | Tier 2+ complexity |
| Permanent-Entering Triggers | EASY | Combat damage triggers, Champion |
| Combat Damage Triggers | MEDIUM | Innistrad damage triggers |

---

## 7. Readiness for Complex Sets

- **Throne of Eldraine**: 40% ready (Food yes, Adventures no, Adamant no)
- **Modern Horizons**: 30% ready (Modular yes, Tribal yes, Cascade partial, Suspend no, Storm no)
- **Innistrad**: 50% ready (Flashback yes, Morbid yes, DFCs no, Madness no, Delirium partial)

**Highest ROI addition**: Dual-face card support + transform mechanics (unlocks werewolves, Delver, 20+ Innistrad cards)
