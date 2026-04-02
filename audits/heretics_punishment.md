# Audit: Heretic's Punishment

## Oracle Reference (Scryfall, cached 2026-04-01)
- **Cost:** {4}{R}
- **Type:** Enchantment
- **Keywords:** Mill
- **Oracle Text:** "{3}{R}: Choose any target, then mill three cards. This enchantment deals damage to that permanent or player equal to the greatest mana value among the milled cards."

### Key Rulings (2011-09-22)
1. If the targeted permanent or player is an illegal target by the time the ability resolves, the entire ability won't resolve. No cards will be put into your graveyard, and no damage will be dealt.
2. If you have two or fewer cards in your library when the ability resolves, all of them will be put into your graveyard. Heretic's Punishment will still deal damage equal to the highest mana value among those cards.
3. The mana value of a double-faced card in your graveyard is the mana value of the front face.
4. If all three cards have a mana value of 0, no damage will be dealt.

## Implementation: `mtg-engine/src/cards/isd/heretics_punishment.rs`

## Issues Found

### 1. CRITICAL — Oracle text in `card_data()` is outdated (wrong mechanic)

The `oracle_text` field in the code uses the original printed text, not the current oracle text. This describes a fundamentally different mechanic (reveal + bottom of library vs. mill to graveyard).

**Code (line 26):**
```
oracle_text: "{3}{R}: Choose target creature or player. Reveal the top 3 cards of your
library. Heretic's Punishment deals damage to that target equal to the greatest mana value
among the revealed cards. Put the revealed cards on the bottom of your library."
```

**Oracle (Scryfall):**
```
{3}{R}: Choose any target, then mill three cards. This enchantment deals damage to that
permanent or player equal to the greatest mana value among the milled cards.
```

The code's `oracle_text` says "target creature or player" (missing planeswalkers) and "reveal... put on bottom of library" (should be "mill" i.e. to graveyard).

### 2. CRITICAL — Order of operations is wrong

Oracle says: "Choose any target, **then mill three cards**. This enchantment deals damage..."
The mill happens first, then damage is calculated from the milled cards and dealt.

The implementation (lines 60-117) does:
1. Reads top 3 cards from library
2. Calculates greatest mana value
3. Deals damage
4. **Then** moves cards to graveyard (lines 113-117)

The damage calculation happens to read from the same cards that get milled, so the result is the same in most cases, but the sequencing matters for replacement effects and triggers that care about cards entering the graveyard. The mill should occur before damage is dealt.

### 3. BUG — Missing `damaged_by` tracking for creature targets

Lines 82-85 mark damage on creature targets (`obj.damage_marked += max_mv`) but do not push to `obj.damaged_by`. Effects that track damage sources (e.g. "whenever a source deals damage to this creature") will not function correctly.

**Code (lines 82-85):**
```rust
if let Some(obj) = state.get_object_mut(*target_id) {
    if obj.zone == Zone::Battlefield {
        obj.damage_marked += max_mv;
    }
}
```

### 4. BUG — No target legality check before milling

Per ruling #1: "If the targeted permanent or player is an illegal target by the time the ability resolves, the entire ability won't resolve. No cards will be put into your graveyard, and no damage will be dealt."

The `on_activate_ability` function does not verify that the target is still legal before proceeding. It should abort entirely (no mill, no damage) if the target has become illegal.

### 5. MINOR — Keywords field is empty

Oracle lists `Mill` as a keyword. The code has:
```rust
keywords: vec![],
```
Should be:
```rust
keywords: vec![Keyword::Mill],
```

### 6. MINOR — `oracle_text` says "creature or player" but `target_requirement` says `AnyTarget`

The `target_requirement` field correctly uses `AnyTarget` (line 46), matching the current oracle ("any target" includes creatures, players, and planeswalkers). However, the `oracle_text` string says "target creature or player," which is inconsistent. The `oracle_text` should be updated to match the current oracle.

## Test Coverage

One test exists in `mtg-engine/tests/tier15_cards.rs`:
- `heretics_punishment_deals_damage_from_revealed_cards` — Verifies damage dealt to a player equals the greatest mana value among top 3 library cards. Passes because sequencing difference is not observable in this simple scenario.

Missing test coverage:
- Targeting a creature (damage_marked + damaged_by)
- Targeting a planeswalker
- Fewer than 3 cards in library
- All milled cards with MV 0 (no damage dealt)
- Target becoming illegal before resolution
- Mill keyword / cards going to graveyard zone

## Verdict: ISSUES FOUND

- 2 critical issues (outdated oracle text describing wrong mechanic; wrong order of operations)
- 2 bugs (missing damaged_by tracking; no target legality check)
- 2 minor issues (missing Mill keyword; inconsistent oracle_text vs target_requirement)
