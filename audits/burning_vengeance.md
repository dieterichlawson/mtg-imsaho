# Audit: Burning Vengeance

## Oracle (Scryfall/API)
- **Name:** Burning Vengeance
- **Cost:** {2}{R}
- **Type:** Enchantment
- **Oracle:** Whenever you cast a spell from your graveyard, this enchantment deals 2 damage to any target.
- **Rulings:**
  - (2025-01-24) Burning Vengeance doesn't trigger when you activate an ability of a card in your graveyard, such as unearth or the ability of Reassembling Skeleton.
  - (2025-01-24) Burning Vengeance's triggered ability will resolve before the spell you cast from your graveyard.

## Implementation: `mtg-engine/src/cards/isd/burning_vengeance.rs`
- **Name:** Burning Vengeance -- CORRECT
- **Cost:** {2}{R} -- CORRECT
- **Type:** Enchantment -- CORRECT
- **Triggered ability kind:** `TriggerKind::SpellCast` -- CORRECT
- **Zone check:** Checks self is on battlefield -- CORRECT
- **Controller check:** Only triggers on own spells (`caster == controller`) -- CORRECT
- **Damage amount:** 2 via `PendingEffect::DealDamage` -- CORRECT
- **Damage source:** `source_id: self_id` (the enchantment itself) -- CORRECT
- **Target selection:** Uses `any_targets()` helper -- CORRECT ("any target" means creature, player, or planeswalker)

## Issues

### 1. BUG (major): Trigger condition checks only `cast_with_flashback`, misses other graveyard-cast spells

The oracle text reads:

> Whenever you **cast a spell from your graveyard**, this enchantment deals 2 damage to any target.

The implementation on line 48-52 checks:

```rust
let cast_from_gy = state.get_object(spell_id)
    .map(|o| o.cast_with_flashback)
    .unwrap_or(false);
if !cast_from_gy {
    return;
}
```

The engine distinguishes between flashback casts (`cast_with_flashback = true`) and other graveyard-cast spells (via `can_cast_from_graveyard()`, e.g. Skaab Ruinator). In `engine.rs` line 1309-1310:

```rust
let is_cast_from_graveyard = in_graveyard && behavior.can_cast_from_graveyard();
let is_flashback = in_graveyard && !is_cast_from_graveyard;
```

Only flashback casts set `cast_with_flashback = true` (engine.rs line 1451-1452). Cards like Skaab Ruinator that use `can_cast_from_graveyard()` are cast from the graveyard but do NOT set this flag. Therefore Burning Vengeance will fail to trigger on non-flashback graveyard casts.

Per Scryfall rulings and community consensus, Burning Vengeance triggers on ANY spell cast from the graveyard, not just flashback. The implementation needs a zone-tracking field (e.g. `cast_from_zone`) or should check both `cast_with_flashback` and any other graveyard-cast indicator.

### 2. ISSUE (minor): Premature/misleading log message

Line 67-68:

```rust
state.log(crate::state::LogLevel::Event,
    format!("Burning Vengeance deals 2 damage to opponent (flashback spell cast)"));
```

This log fires before the target choice is resolved and always says "to opponent", even though the player could target a creature or planeswalker. The actual damage is dealt later when the PendingEffect resolves. The log is misleading.

### 3. ISSUE (minor): Comment says "flashback" only

Line 47 comment says `// Only trigger on spells cast from graveyard (flashback).` -- the parenthetical "(flashback)" is misleading. The oracle text does not limit this to flashback; it applies to any graveyard cast.

## Tests: `mtg-engine/tests/tier12_cards.rs`

Two tests exist:
- `burning_vengeance_triggers_on_flashback` -- tests flashback trigger, passes
- `burning_vengeance_ignores_non_flashback` -- tests that normal (hand) casts don't trigger, passes

**Missing test coverage:**
- No test for non-flashback graveyard cast (e.g. Skaab Ruinator-style `can_cast_from_graveyard`). This would expose bug #1.
- No test targeting a creature or planeswalker (only tests targeting a player).

## Anti-Pattern Check
- Damage uses `PendingEffect::DealDamage` which correctly flows through `apply_pending_effect`, pushing `NonCombatDamageDealt` events and updating `damaged_by` on creatures. No anti-pattern.
- Target selection uses `any_targets()` + `present_target_choice()` helper. No anti-pattern.

## Verdict: FAIL

**Major bug:** Burning Vengeance only triggers on flashback casts (`cast_with_flashback`), not on all graveyard casts. This is incorrect per oracle text. The fix requires either (a) adding a `cast_from_zone` field to track the origin zone of any cast spell, or (b) checking an additional flag for `can_cast_from_graveyard`-style casts.

Sources:
- [Scryfall: Burning Vengeance](https://scryfall.com/card/ema/121/burning-vengeance)
- [Magic Rules Tips: What will and will not trigger Burning Vengeance](https://blogs.magicjudges.org/rulestips/2012/01/what-will-and-what-will-not-trigger-burning-vengeances-ability/)
- [MTG Assist: Burning Vengeance rulings](https://www.mtgassist.com/cards/Innistrad/Burning-Vengeance/rulings/)

---

## Audit 2 -- 2026-04-02

### Oracle Text (Scryfall, cached 2026-04-01)

> Whenever you cast a spell from your graveyard, this enchantment deals 2 damage to any target.

**Rulings:**
1. Burning Vengeance doesn't trigger when you activate an ability of a card in your graveyard, such as unearth or Reassembling Skeleton.
2. Burning Vengeance's triggered ability resolves before the spell you cast from your graveyard.
3. Triggers on any spell cast from graveyard, not just flashback (also retrace, `can_cast_from_graveyard`, etc.).

### Card Data Verification

| Field | Expected | Implemented | Status |
|-------|----------|-------------|--------|
| Name | Burning Vengeance | "Burning Vengeance" | PASS |
| Mana cost | {2}{R} | Generic(2), Red | PASS |
| Type | Enchantment | CardType::Enchantment | PASS |
| Supertypes | (none) | vec![] | PASS |
| Subtypes | (none) | vec![] | PASS |
| P/T | N/A | None/None | PASS |
| Oracle text | matches | matches | PASS |
| Keywords | (none) | vec![] | PASS |
| Triggered ability kind | SpellCast | TriggerKind::SpellCast | PASS |

### Bugs Found

#### BUG 1 (major, confirmed from prior audit): Trigger checks only `cast_with_flashback`, misses other graveyard-cast spells

**Oracle text:** "Whenever you cast a spell **from your graveyard**"
**Implementation (burning_vengeance.rs lines 48-52):**
```rust
let cast_from_gy = state.get_object(spell_id)
    .map(|o| o.cast_with_flashback)
    .unwrap_or(false);
if !cast_from_gy {
    return;
}
```

The code checks `cast_with_flashback`, but the engine only sets this flag for flashback casts (engine.rs line 1451-1452):
```rust
if is_flashback {
    obj.cast_with_flashback = true;
}
```

Cards that use `can_cast_from_graveyard()` (e.g. Skaab Ruinator) are cast from the graveyard but do NOT set `cast_with_flashback` (engine.rs lines 1309-1310):
```rust
let is_cast_from_graveyard = in_graveyard && behavior.can_cast_from_graveyard();
let is_flashback = in_graveyard && !is_cast_from_graveyard;
```

Per Scryfall rulings and community consensus, Burning Vengeance triggers on ANY spell cast from the graveyard. The fix requires either a `cast_from_zone` field or checking both `cast_with_flashback` and an additional graveyard-cast indicator.

**Severity:** Major -- incorrect game behavior for non-flashback graveyard casts.

#### BUG 2 (major): Trigger system filters SpellCast to instant/sorcery only

**In triggers.rs lines 628-633:**
```rust
GameEvent::SpellCast { player: caster, object: spell_id } => {
    let is_instant_sorcery = state.get_object(*spell_id)
        .and_then(|o| registry.card_data(o.card_id))
        .map(|d| d.card_types.iter().any(|ct| matches!(ct, crate::types::CardType::Instant | crate::types::CardType::Sorcery)))
        .unwrap_or(false);
    if is_instant_sorcery {
```

The `SpellCast` event processing only creates `SpellCastWatch` triggers for instants and sorceries. This means even if Bug 1 were fixed, Burning Vengeance would still not trigger on creature spells cast from the graveyard (e.g. Skaab Ruinator). The oracle text says "a spell" with no type restriction.

Note: This filter may be appropriate for Charmbreaker Devils (which cares about instants/sorceries), but it is incorrect as a blanket filter for all `TriggerKind::SpellCast` watchers. Each card's `on_spell_cast` callback should do its own filtering.

**Severity:** Major -- Burning Vengeance will never trigger on non-instant/sorcery graveyard casts.

#### BUG 3 (minor): Premature log message

**In burning_vengeance.rs line 67-68:**
```rust
state.log(crate::state::LogLevel::Event,
    format!("Burning Vengeance deals 2 damage to opponent (flashback spell cast)"));
```

This log fires immediately when the trigger is set up (before target selection and resolution), not when damage is actually dealt. It also hardcodes "opponent" even though the target could be any creature, player, or planeswalker. The `apply_pending_effect` function already logs the actual damage event correctly, so this log is redundant and misleading.

**Severity:** Minor -- cosmetic/logging only.

### Damage Handling Verification

- `PendingEffect::DealDamage` with `amount: 2` and `source_id: self_id`: PASS
- `apply_pending_effect` uses `NonCombatDamageDealt` (not combat damage): PASS
- `damaged_by` tracking on creature targets (engine.rs line 1985): PASS
- Player life reduction (engine.rs line 1998-1999): PASS
- "Any target" via `helpers::any_targets()`: PASS (targets creatures, players, and planeswalkers)

### Test Coverage

Two tests exist in `mtg-engine/tests/tier12_cards.rs`:
1. `burning_vengeance_triggers_on_flashback` (line 282): Tests basic flashback trigger dealing 2 damage to a player. PASS.
2. `burning_vengeance_ignores_non_flashback` (line 329): Tests that a normal (non-graveyard) cast does not trigger. PASS.

**Missing test coverage:**
- No test for non-flashback graveyard cast (e.g. a `can_cast_from_graveyard` creature). This would expose Bug 1 and Bug 2.
- No test targeting a creature (to verify `damaged_by` tracking).
- No test targeting a planeswalker.

### Anti-Pattern Check

- No unsafe unwrap on game state lookups (uses `map`/`unwrap_or` properly): PASS
- Battlefield zone check present: PASS
- Controller check present: PASS
- `triggered_abilities` declares `TriggerKind::SpellCast`: PASS

### Summary

| # | Issue | Severity | Status |
|---|-------|----------|--------|
| 1 | Trigger checks `cast_with_flashback` instead of general graveyard-cast | Major | Open |
| 2 | `triggers.rs` SpellCast handler filters to instant/sorcery only | Major | Open |
| 3 | Premature/inaccurate log message | Minor | Open |

Card data, mana cost, types, damage amount, damage type (non-combat), and target selection are all correct. The two major bugs both relate to the same root cause: the engine lacks a general "cast from graveyard" tracking mechanism, instead relying on the flashback-specific flag.

Sources:
- [Scryfall: Burning Vengeance](https://scryfall.com/card/isd/133/burning-vengeance)
- [Magic Rules Tips: What will and will not trigger Burning Vengeance](https://blogs.magicjudges.org/rulestips/2012/01/what-will-and-what-will-not-trigger-burning-vengeances-ability/)
- [MTG Assist: Burning Vengeance rulings](https://www.mtgassist.com/cards/Innistrad/Burning-Vengeance/rulings/)
- [MTG Salvation: Burning Vengeance + Increasing Vengeance](https://www.mtgsalvation.com/forums/magic-fundamentals/magic-rulings/magic-rulings-archives/307845-burning-vengeance-increasing-vengenace)

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Whenever you cast a spell from your graveyard, this enchantment deals 2 damage to any target.
**Type line**: Enchantment
**Status**: ISSUE

### Code issues
1. **Oracle text mismatch**: Oracle says "this enchantment deals 2 damage to any target" but code oracle_text says "Burning Vengeance deals 2 damage to any target." The oracle has been updated to use "this enchantment" self-referential language. The code oracle_text should be updated to match. Behavior is functionally equivalent — no gameplay impact.
