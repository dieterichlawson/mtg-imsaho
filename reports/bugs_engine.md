# Engine Bugs Report (2026-03-29)

## Critical

### 1. No Legend Rule (CR 704.5k)
**File:** `sba.rs`
If a player controls multiple legendary permanents with the same name, they should all go to the graveyard as an SBA. Not implemented -- a player can currently control duplicate legendaries.

### 2. No +1/+1 and -1/-1 Counter Annihilation (CR 704.5q)
**File:** `sba.rs`
If a permanent has both +1/+1 and -1/-1 counters, they should annihilate in pairs as an SBA. Not implemented.

## High

### 3. Spells Don't Fizzle When Targets Become Illegal (CR 608.2b)
**Files:** All card `on_resolve` methods
When a spell's only target becomes illegal before resolution, the spell should be countered by game rules ("fizzle"). Currently spells silently do nothing. Multi-target spells should resolve with remaining legal targets, only fizzling if ALL targets are illegal (CR 608.2c).

### 4. Combat Steps Skipped When No Attackers (CR 507-510)
**File:** `engine.rs:1447-1450`
When no attackers are declared, the code jumps directly from DeclareAttackers to EndCombat, skipping DeclareBlockers and CombatDamage steps entirely. All steps should execute in sequence.

## Medium

### 5. No APNAP Trigger Ordering (CR 603.3b)
**File:** `triggers.rs`
When multiple triggered abilities trigger simultaneously, they should stack in Active Player, Non-Active Player order. Currently triggers stack in detection order.

### 6. Cleanup Step Doesn't Loop on SBAs/Triggers (CR 514.3a)
**File:** `engine.rs`
If SBAs or triggers happen during cleanup, there should be another cleanup step. Engine just clears and moves on.

### 7. Mana May Not Empty at Step Boundaries (CR 106.4)
**File:** `engine.rs`
Mana should empty at end of each step AND phase, not just phases.

### 8. Sorcery-Speed May Not Check Empty Stack
**File:** `engine.rs`
Players shouldn't be able to cast sorceries while something is on the stack, even during their main phase.

### 9. Zone Changes May Not Create New Object Identity (CR 400.7)
A card leaving and re-entering the battlefield is a new object. If the permanent ID is reused, Auras/equipment may incorrectly stay attached.

## Low

### 10. Redundant resolve_top_of_stack Call
**File:** `engine.rs:1427`
`resolve_top_of_stack` called on a temporary clone then discarded, then called again on a fresh clone. Wasteful but harmless.

### 11. No Planeswalker Uniqueness Rule (CR 704.5j)
**File:** `sba.rs`
No planeswalkers implemented yet, but rule is missing.

### 12. Missing Upkeep/Beginning of Combat Triggers
**File:** `triggers.rs`
No `on_upkeep` or `on_begin_combat` callbacks. Cards with these triggers won't work.
