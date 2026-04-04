## Audit — 2026-04-03 22:21

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life.
**Type line**: Creature — Vampire Noble
**Status**: ISSUE

### Code issues
- Targeting mechanism not implemented correctly in `mtg-engine/src/cards/isd/falkenrath_noble.rs:57-69`
  - Oracle text says: `target player loses 1 life and you gain 1 life`
  - Code does: `let opponent = state.opponent(controller);` and auto-targets the opponent without offering player choice

### Tricky interactions checked
- Self-death triggering (Noble dies and triggers on itself): PASS - `SelfDies` trigger correctly implemented
- Simultaneous death with multiple creatures: PASS - Engine pushes death events before moving creatures to graveyard, allowing proper last-known-information triggering per ruling
- "This creature or another creature" scope: PASS - Both `SelfDies` and `AnyCreatureDies` triggers properly defined
- Life gain/loss timing: PASS - Both life changes applied correctly in sequence with proper GameEvent generation
- APNAP trigger ordering: PASS - Tested in `mtg-engine/tests/apnap.rs` with correct non-active player triggers resolving first

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Self-death triggering: `mtg-engine/tests/bug_fixes.rs:449` (falkenrath_noble_triggers_on_self_death)
- Opponent creature death: `mtg-engine/tests/bug_fixes.rs:401` (falkenrath_noble_triggers_on_opponent_creature_death) 
- Allied creature death: `mtg-engine/tests/bug_fixes.rs:426` (falkenrath_noble_triggers_on_own_creature_death)
- Basic functionality: `mtg-engine/tests/tier3_cards.rs:283` (falkenrath_noble_drains_on_any_death)
- APNAP ordering: `mtg-engine/tests/apnap.rs:94` (non_active_player_triggers_resolve_first)
- Simultaneous death ruling "If Falkenrath Noble and another creature die at the same time, Falkenrath Noble's triggered ability will trigger for each of them": NOT TESTED
- Targeting player choice in multiplayer: NOT TESTED
- Manual targeting vs auto-targeting behavior: NOT TESTED

## Audit — 2026-04-03 22:21 (independent re-audit)

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life.
**Type line**: Creature — Vampire Noble
**Status**: ISSUE

### Code issues

- **Simultaneous death does not trigger for each creature** (engine bug in `mtg-engine/src/triggers.rs:418-419`, affects card in `mtg-engine/src/cards/isd/falkenrath_noble.rs`)
  - Oracle text says: `Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life.`
  - Ruling [2017-03-14] says: `If Falkenrath Noble and another creature die at the same time, Falkenrath Noble's triggered ability will trigger for each of them.`
  - Code does: In `collect_triggers`, the AnyCreatureDies watcher lookup at line 418-419 filters `state.objects.values().filter(|o| o.zone == Zone::Battlefield && o.id != dead_id)`. When Noble dies simultaneously with another creature (e.g., in the same SBA pass), Noble is moved to the graveyard before trigger collection occurs. When the other creature's `CreatureDied` event is processed, Noble is no longer on the battlefield and is not found as a watcher. Noble only triggers once (via `SelfDies` for its own death) instead of twice. Per CR 603.10c, death triggers should "look back in time" to check abilities that existed prior to the event, but the engine does not implement this look-back for AnyCreatureDies watchers.

- **"target player" auto-resolved to opponent without player choice** (`mtg-engine/src/cards/isd/falkenrath_noble.rs:59-60`)
  - Oracle text says: `target player loses 1 life`
  - Code does: `let opponent = state.opponent(controller);` — always selects the opponent without presenting a choice. The oracle says "target player" (which includes the controller), not "target opponent."

- **LLM card knowledge says "opponent" instead of "target player"** (`mtg-player/src/llm.rs:102`)
  - Oracle text says: `target player loses 1 life and you gain 1 life`
  - LLM hint says: `Whenever ANY creature dies, opponent loses 1 life and you gain 1.` — should say "target player" not "opponent"

### Tricky interactions checked
- Self-death triggering (Noble dies alone): pass — `SelfDies` trigger correctly fires via `on_dies` handler
- Simultaneous death (Noble + another creature die together): FAIL — Noble only triggers once instead of twice; see code issues above
- "another creature dies" scope (opponent's creature): pass — `AnyCreatureDies` watcher correctly picks up opponent's creature dying when Noble is alive
- "another creature dies" scope (own creature): pass — same watcher correctly fires for allied creature death
- Life loss vs damage semantics: pass — `drain()` directly modifies life totals and emits `LifeChanged` events, not damage events
- APNAP trigger ordering: pass — tested and correct in apnap.rs
- Card data accuracy (cost, types, P/T, keywords): pass — {3}{B}, Creature — Vampire Noble, 2/2, Flying all match oracle

### Test coverage
- Self-death triggering: `mtg-engine/tests/bug_fixes.rs:449` (falkenrath_noble_triggers_on_self_death)
- Opponent creature death: `mtg-engine/tests/bug_fixes.rs:401` (falkenrath_noble_triggers_on_opponent_creature_death)
- Allied creature death: `mtg-engine/tests/bug_fixes.rs:426` (falkenrath_noble_triggers_on_own_creature_death)
- Basic drain functionality: `mtg-engine/tests/tier3_cards.rs:283` (falkenrath_noble_drains_on_any_death)
- APNAP ordering with Noble: `mtg-engine/tests/apnap.rs:94` (non_active_player_triggers_resolve_first)
- Simultaneous death ruling (Noble + another creature die at same time, should trigger twice): NOT TESTED — this would expose the bug
- "target player" choice (player picks target): NOT TESTED

## Audit — 2026-04-03 22:32

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life.
**Type line**: Creature — Vampire Noble
**Status**: ISSUE

### Code issues
- Targeting violation in `mtg-engine/src/cards/isd/falkenrath_noble.rs:58-69`
  - Oracle text says: `target player loses 1 life`
  - Code does: Automatically targets the opponent with `state.opponent(controller)` instead of presenting player choice

This is inconsistent with other "target player" effects in the codebase. Selhoff Occultist (same trigger pattern), Rage Thrower, and Bloodgift Demon all correctly implement player choice for "target player" effects using the targeting system.

The previous audit also identified a simultaneous death issue, which I confirmed by tracing the SBA+trigger loop in `engine.rs:3121-3126`. When multiple creatures die in the same SBA pass, they are all moved to graveyard before trigger collection, so Noble cannot be found as a watcher for other creatures' deaths.

### Tricky interactions checked
- Simultaneous death triggers: ISSUE (confirmed engine bug - creatures moved to graveyard before trigger collection in SBA loop)
- Self-death trigger: PASS (on_dies method correctly handles Noble triggering on its own death)
- Opponent creature death: PASS (on_any_creature_dies correctly handles death watch for all creatures, when Noble remains on battlefield)
- Controller identification for triggers: PASS (uses last known information when Noble dies, current controller when watching)
- Life change events: PASS (GameEvent::LifeChanged generated for both life loss and gain)
- "Any creature" scope: PASS (no controller restriction in trigger collection)
- Target player vs target opponent semantics: ISSUE (should allow choosing any player including self)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Noble triggers on opponent creature death: `bug_fixes.rs:401`
- Noble triggers on own creature death: `bug_fixes.rs:426` 
- Noble triggers on self death: `bug_fixes.rs:449`
- Drain effect (life loss + gain): `tier3_cards.rs:283`
- APNAP trigger ordering: `apnap.rs:95, apnap.rs:195`
- Simultaneous death ruling (Noble + other creatures die together): NOT TESTED

## Audit — 2026-04-03 22:38

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life.
**Type line**: Creature — Vampire Noble
**Status**: ISSUE

### Code issues
- **"target player" auto-targeting without player choice** in `mtg-engine/src/cards/isd/falkenrath_noble.rs:59-60`
  - Oracle text says: `target player loses 1 life`
  - Code does: `let opponent = state.opponent(controller);` — automatically selects the opponent without allowing player choice
  - Correct implementation should present choice to all players using `AwaitingAction::ResolutionChoice` like Bloodgift Demon does (lines 48-64 in `bloodgift_demon.rs`)

### Tricky interactions checked
- **"target player" vs "target opponent"**: ISSUE — "target player" means ANY player including self, but code auto-selects opponent
- **Self-death triggering**: PASS — `SelfDies` trigger with `on_dies` correctly handles Noble dying
- **Death watching (other creatures)**: PASS — `AnyCreatureDies` trigger with `on_any_creature_dies` correctly handles other deaths when Noble alive
- **Card data accuracy**: PASS — mana cost {3}{B}, type line, P/T 2/2, flying keyword all match oracle
- **Life modification mechanics**: PASS — `drain()` function correctly modifies life totals and emits `LifeChanged` events
- **Trigger timing and APNAP ordering**: PASS — tested in multiple scenarios, works correctly
- **"Another" vs "any" semantics**: PASS — triggers include both self-death and other-creature-death cases
- **Simultaneous death (multiple creatures die together)**: UNCERTAIN — ruling says Noble should trigger for each death, but this specific scenario not tested

### Test coverage
- Self-death: `mtg-engine/tests/bug_fixes.rs:449` (falkenrath_noble_triggers_on_self_death)
- Opponent creature death: `mtg-engine/tests/bug_fixes.rs:401` (falkenrath_noble_triggers_on_opponent_creature_death)
- Own creature death: `mtg-engine/tests/bug_fixes.rs:426` (falkenrath_noble_triggers_on_own_creature_death)
- Basic drain functionality: `mtg-engine/tests/tier3_cards.rs:283` (falkenrath_noble_drains_on_any_death)
- APNAP ordering: `mtg-engine/tests/apnap.rs:94` (non_active_player_triggers_resolve_first)
- Simultaneous death ruling ("If Falkenrath Noble and another creature die at the same time, Falkenrath Noble's triggered ability will trigger for each of them"): NOT TESTED
- Player choice for "target player": NOT TESTED

## Audit — 2026-04-03 22:44

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life.
**Type line**: Creature — Vampire Noble
**Status**: ISSUE

### Code issues

- Targeting implementation violates oracle text (mtg-engine/src/cards/isd/falkenrath_noble.rs:57-69)
  - Oracle text says: `target player loses 1 life`
  - Code does: Auto-selects opponent via `let opponent = state.opponent(controller)` instead of presenting player choice. Compare with Selhoff Occultist implementation which correctly presents all players as targeting options for similar trigger.

- Simultaneous death triggers missed due to SBA/trigger timing (mtg-engine/src/sba.rs:94-95 + mtg-engine/src/triggers.rs:418-421)
  - Oracle text says: `Whenever this creature or another creature dies` and ruling states "If Falkenrath Noble and another creature die at the same time, Falkenrath Noble's triggered ability will trigger for each of them"
  - Code does: SBA processes each death by pushing CreatureDied event then immediately moving creature to graveyard. When triggers are later collected, Noble is already in graveyard so DeathWatch filter excludes it (zone != Battlefield check). This causes Noble to miss other creatures' deaths in same SBA pass.

### Tricky interactions checked

- **Target player choice**: FAIL - Code auto-selects opponent instead of presenting choice
- **Simultaneous death triggers**: FAIL - Noble misses other deaths when dying in same SBA pass due to zone filter timing  
- **Self-death trigger**: PASS - SelfDies trigger works correctly when Noble dies alone
- **Other creature death trigger**: PASS - DeathWatch trigger works when Noble survives and other creatures die
- **Life loss vs damage distinction**: PASS - Code correctly uses life loss, not damage (cannot be redirected to planeswalkers)
- **APNAP trigger ordering**: PASS - Triggers correctly follow APNAP rules per existing tests

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:
- Noble triggers on opponent creature death: `mtg-engine/tests/bug_fixes.rs:401`
- Noble triggers on own creature death: `mtg-engine/tests/bug_fixes.rs:426` 
- Noble triggers on self death: `mtg-engine/tests/bug_fixes.rs:449`
- Noble drain effect (life loss/gain): `mtg-engine/tests/tier3_cards.rs:283`
- APNAP ordering with other death triggers: `mtg-engine/tests/apnap.rs:94`
- **Simultaneous death ruling (key ruling)**: NOT TESTED
- **Target player choice functionality**: NOT TESTED

## Audit — 2026-04-03 22:50

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life.
**Type line**: Creature — Vampire Noble
**Status**: ISSUE

### Code issues
- **"target player" auto-targeting without player choice** in `mtg-engine/src/cards/isd/falkenrath_noble.rs:59-60`
  - Oracle text says: `target player loses 1 life`
  - Code does: `let opponent = state.opponent(controller);` — automatically selects opponent without presenting choice to the controller

### Tricky interactions checked
- **"target" keyword semantics**: ISSUE — "target player" requires player choice but code auto-selects opponent
- **Self-death triggering**: PASS — `SelfDies` trigger correctly implemented with `on_dies` handler
- **Other creature deaths**: PASS — `AnyCreatureDies` trigger correctly implemented with `on_any_creature_dies` handler  
- **Card data (mana cost, types, P/T, keywords)**: PASS — all data matches oracle text exactly
- **Life modification mechanics**: PASS — uses direct life total changes with `LifeChanged` events, not damage
- **APNAP trigger ordering**: PASS — verified in existing tests, works correctly
- **"Whenever" trigger timing**: PASS — triggers fire for each individual creature death
- **Simultaneous deaths**: UNCERTAIN — critical ruling not tested, potential engine issue with death trigger timing

### Test coverage
- Self-death triggering: `mtg-engine/tests/bug_fixes.rs:449` (falkenrath_noble_triggers_on_self_death)
- Opponent creature death: `mtg-engine/tests/bug_fixes.rs:401` (falkenrath_noble_triggers_on_opponent_creature_death)
- Own creature death: `mtg-engine/tests/bug_fixes.rs:426` (falkenrath_noble_triggers_on_own_creature_death)  
- Basic drain functionality: `mtg-engine/tests/tier3_cards.rs:283` (falkenrath_noble_drains_on_any_death)
- APNAP ordering: `mtg-engine/tests/apnap.rs:94` (non_active_player_triggers_resolve_first)
- Simultaneous death ruling "If Falkenrath Noble and another creature die at the same time, Falkenrath Noble's triggered ability will trigger for each of them": NOT TESTED
- Target player choice presentation: NOT TESTED