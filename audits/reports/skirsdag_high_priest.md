# Audit: Skirsdag High Priest

## Oracle (Scryfall)
- **Name:** Skirsdag High Priest
- **Cost:** {1}{B}
- **Type:** Creature -- Human Cleric
- **Oracle:** Morbid -- {T}, Tap two untapped creatures you control: Create a 5/5 black Demon creature token with flying. Activate only if a creature died this turn.
- **P/T:** 1/2

## Implementation: `mtg-engine/src/cards/skirsdag_high_priest.rs`
- **Name:** Skirsdag High Priest ✅
- **Cost:** {1}{B} ✅
- **Type:** Creature ✅
- **Subtypes:** Human, Cleric ✅
- **P/T:** 1/2 ✅
- **Morbid check:** checks `state.creature_died_this_turn` ✅
- **Tap cost:** requires_tap: true, plus taps 2 other untapped creatures ✅
- **Sorcery speed:** sorcery_speed_only: true ✅
- **Token:** 5/5 black Demon with flying, subtypes ["Demon"] ✅
- **Pre-checks:** verifies not tapped, not summoning sick, at least 2 other untapped creatures ✅

### Note
- The two creatures to tap are auto-selected (first two found) rather than player-chosen. This is a simplification but functionally acceptable in most cases.

## Verdict: PASS -- no issues found

## Audit — 2026-04-02

**Oracle Text:**
> Morbid — {T}, Tap two untapped creatures you control: Create a 5/5 black Demon creature token with flying. Activate only if a creature died this turn.

**Card Data:**
- Name: Skirsdag High Priest — correct
- Cost: {1}{B} — correct
- Type: Creature — Human Cleric — correct
- P/T: 1/2 — correct
- ISSUE: oracle_text in code says "Activate only as a sorcery" but the actual oracle text says "Activate only if a creature died this turn."
  - Code: `"Morbid — {T}, Tap two untapped creatures you control: Create a 5/5 black Demon creature token with flying. Activate only as a sorcery."`
  - Oracle: `"Morbid — {T}, Tap two untapped creatures you control: Create a 5/5 black Demon creature token with flying. Activate only if a creature died this turn."`

**Behavior:**
- Morbid check via `state.creature_died_this_turn` — correct
- requires_tap: true — correct
- Taps two other untapped creatures — correct
- Creates 5/5 black Demon token with flying — correct
- sorcery_speed_only: true — this is NOT in the oracle text. The oracle restriction is the morbid condition ("Activate only if a creature died this turn"), not sorcery speed. The morbid check is already enforced by the `creature_died_this_turn` guard. The `sorcery_speed_only` flag adds an extra restriction not present in the oracle.

**Result: ISSUE** — (1) oracle_text string says "Activate only as a sorcery" but oracle says "Activate only if a creature died this turn." (2) `sorcery_speed_only: true` imposes a restriction not present in the oracle text — the ability can be activated at instant speed as long as morbid is satisfied.

## Re-audit — 2026-04-02
**Status**: PASS
Previously fixed bug re-verified: activated ability correctly requires morbid condition, tapping self + two other creatures, and creates 5/5 Demon token with flying. Oracle text matches Scryfall verbatim. Behavior unchanged.
