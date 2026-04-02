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

---

## Re-Audit (2026-04-02)

Previous critical and bug issues have been fixed. Re-auditing the current implementation against oracle text.

### Oracle Text (Scryfall, cached 2026-04-01)
```
{3}{R}: Choose any target, then mill three cards. This enchantment deals damage to that permanent or player equal to the greatest mana value among the milled cards.
```

### Checks Passed

1. **Mill-then-damage order:** Code (lines 79-101) mills cards to graveyard first, then (lines 103-136) deals damage equal to the greatest mana value. Correct.
2. **Target legality check:** Lines 60-77 verify the target is still legal before any milling occurs. If illegal, the entire ability fizzles (no mill, no damage). Matches ruling #1.
3. **damaged_by tracking:** Line 111 pushes `object_id` to `obj.damaged_by` when dealing damage to a creature. Correct.
4. **Fewer than 3 cards in library:** Line 81 uses `min(3, library.len())`. Matches ruling #2.
5. **MV 0 case:** Line 104 gates damage behind `if max_mv > 0`. Matches ruling #4.
6. **Mana cost:** `{4}{R}` — correct.
7. **Card type:** `Enchantment` — correct.
8. **Activation cost:** `{3}{R}`, no tap required — correct.
9. **Target requirement:** `AnyTarget` — correct.
10. **Cards moved to graveyard:** Lines 97-101 drain from library and call `move_object(card_id, Zone::Graveyard)` — correct (mill).

### Issues Still Present

#### 1. MINOR — `oracle_text` field does not match current oracle text

**Code (line 25):**
```
{3}{R}: Mill three cards, then Heretic's Punishment deals damage to any target equal to the highest mana value among the milled cards.
```

**Oracle (Scryfall):**
```
{3}{R}: Choose any target, then mill three cards. This enchantment deals damage to that permanent or player equal to the greatest mana value among the milled cards.
```

Differences:
- Code says "Mill three cards, then ... deals damage to any target"; oracle says "Choose any target, then mill three cards. This enchantment deals damage to that permanent or player".
- Code says "highest"; oracle says "greatest".
- Code says "Heretic's Punishment"; oracle says "This enchantment".

The functional behavior is implemented correctly (target chosen at activation, mill then damage on resolution), but the `oracle_text` string itself is a paraphrase rather than the verbatim oracle text.

#### 2. MINOR — `keywords` field is empty; oracle lists Mill

**Code (line 28):**
```rust
keywords: vec![],
```

Should include the Mill keyword per Scryfall data.

### Test Coverage

Three tests exist in `mtg-engine/tests/tier15_cards.rs`:

| Test | What it covers | Status |
|------|---------------|--------|
| `heretics_punishment_mills_then_deals_damage` | Mills 3 cards, deals damage to player equal to greatest MV, cards end up in graveyard | PASS (correct) |
| `heretics_punishment_tracks_damaged_by_on_creature` | Deals damage to creature, tracks `damaged_by` source | PASS (correct) |
| `heretics_punishment_fizzles_when_target_illegal` | Target moved off battlefield before resolution; ability fizzles, no mill occurs | PASS (correct) |

Still missing:
- Fewer than 3 cards in library
- All milled cards with MV 0 (no damage dealt)
- Targeting a planeswalker

### Verdict: MINOR ISSUES ONLY

All critical and bug-level issues from the previous audit have been fixed. Two minor issues remain:
- `oracle_text` string is a paraphrase, not the verbatim oracle text
- `keywords` field missing Mill

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: {3}{R}: Choose any target, then mill three cards. This enchantment deals damage to that permanent or player equal to the greatest mana value among the milled cards.
**Type line**: Enchantment
**Status**: ISSUE

### Code issues
1. **Oracle text mismatch**: The stored oracle_text in the code reads "{3}{R}: Mill three cards, then Heretic's Punishment deals damage to any target equal to the highest mana value among the milled cards." The current Scryfall oracle text reads "{3}{R}: Choose any target, then mill three cards. This enchantment deals damage to that permanent or player equal to the greatest mana value among the milled cards." The code uses outdated oracle wording. The functional behavior is correct (target is chosen at activation, mill happens on resolution before damage), but the oracle_text string should be updated.

No other issues. Card data correct: name, mana cost {4}{R}, Enchantment. Activated ability: {3}{R}, no tap, AnyTarget. Mills up to 3 cards, computes max mana value via registry, deals damage to target (creature via damage_marked or player via life reduction). Handles fewer than 3 cards in library. Target legality check on resolution (fizzle if illegal). NonCombatDamageDealt and LifeChanged events emitted. No anti-patterns.
