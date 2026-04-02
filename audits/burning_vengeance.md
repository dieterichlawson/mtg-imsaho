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
