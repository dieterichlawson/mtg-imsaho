# Audit: Hanweir Watchkeep / Bane of Hanweir

## Oracle Reference (Scryfall)
**Front Face: Hanweir Watchkeep**
- Cost: {2}{R}
- Type: Creature -- Human Warrior Werewolf
- P/T: 1/5
- Oracle: "Defender
  At the beginning of each upkeep, if no spells were cast last turn, transform Hanweir Watchkeep."

**Back Face: Bane of Hanweir**
- Type: Creature -- Werewolf
- P/T: 5/5
- Oracle: "Bane of Hanweir attacks each combat if able.
  At the beginning of each upkeep, if a player cast two or more spells last turn, transform Bane of Hanweir."

## Implementation: hanweir_watchkeep.rs

## Issues Found

No issues found. Front and back face names, types, subtypes (Human Warrior Werewolf / Werewolf), P/T (1/5 / 5/5), defender keyword, forced attack on back face (ContinuousEffect::ForceAttack), and werewolf transform logic all match.

## Verdict: PASS

---

## Re-audit: 2026-04-02

### Oracle Text (Scryfall, 2026-04-01 cache)
**Front:**
```
Defender
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back (Bane of Hanweir):**
```
This creature attacks each combat if able.
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```

### Findings
- Name, cost ({2}{R}), type (Creature -- Human Warrior Werewolf), P/T (1/5 / 5/5) all match.
- Front face: Defender keyword present -- correct.
- Back face: Bane of Hanweir, Creature -- Werewolf, 5/5, ForceAttack effect -- correct.
- Transform logic matches standard werewolf pattern -- correct.

### ISSUE: Back face oracle text mismatch
- **Oracle (Scryfall)**: "This creature attacks each combat if able."
- **Code oracle_text**: "Bane of Hanweir attacks each combat if able."

The current Scryfall oracle uses "This creature" instead of the card name. Behavior is functionally correct regardless.

### Verdict: ISSUE
Back face oracle text in code uses "Bane of Hanweir" where current oracle uses "This creature".

## Re-audit — 2026-04-02
**Status**: PASS
Oracle text updated to match Scryfall for both faces: "transform this creature" (was "transform Hanweir Watchkeep"/"transform Bane of Hanweir"), and "This creature attacks each combat if able" (was "Bane of Hanweir attacks each combat if able"). Behavior unchanged.

## Audit — 2026-04-02 21:12
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**:
Front (Hanweir Watchkeep):
```
Defender
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
Back (Bane of Hanweir):
```
This creature attacks each combat if able.
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```
**Type line**:
Front: Creature — Human Warrior Werewolf
Back: Creature — Werewolf
**Status**: PASS

### Code issues
None. All card data fields match oracle exactly:
- Name: "Hanweir Watchkeep" / "Bane of Hanweir" -- correct.
- Cost: {2}{R} -- `Generic(2), Colored(Color::Red)` -- correct.
- Types: `Creature` with subtypes `["Human", "Warrior", "Werewolf"]` / `["Werewolf"]` -- matches type lines.
- P/T: 1/5 / 5/5 -- matches. `dynamic_pt` returns `Some((5, 5))` when transformed, `None` otherwise (falls back to object base 1/5).
- Oracle text strings match Scryfall verbatim on both faces.
- Front face keywords: `[Keyword::Defender]` -- correct.
- Back face keywords: `[]` (no keyword abilities) -- correct.
- Back face continuous effects: `ForceAttack { scope: EffectScope::OnSelf }` -- correctly models "attacks each combat if able".
- Transform logic: standard werewolf pattern (front: no spells + not first turn; back: any player cast 2+). Correct.
- `on_upkeep` checks zone == Battlefield before transforming. Correct.
- Name is updated on transform. Correct.

### Tricky interactions checked (min 3)
1. **Defender on front face only**: Engine's `has_keyword` reads front face keywords when not transformed, back face when transformed. Back face has no `Keyword::Defender`, so Bane of Hanweir can attack. Verified in test and engine code.
2. **ForceAttack on back face only**: Engine's `has_continuous_effect` reads back face effects when transformed. Only back face declares `ForceAttack`, so Hanweir Watchkeep (front) is not forced to attack (and couldn't anyway due to Defender). Bane of Hanweir (back) is forced to attack. Correct.
3. **First turn suppression**: `werewolf_should_transform` returns false on first turn (`!state.is_first_turn`), preventing transform when there is no "last turn" to check. Test `reckless_waif_stays_human_on_first_turn` covers this for the shared werewolf logic.
4. **Transform back condition checks any player**: `state.spells_cast_last_turn.values().any(|&count| count >= 2)` correctly checks if ANY single player cast 2+, not total across all players. This matches "if a player cast two or more spells last turn".

### Test coverage
- `hanweir_watchkeep_loses_defender_gains_force_attack` (werewolf_cards.rs:174): Verifies front face has Defender and 1 power, transforms on upkeep, back face is 5/5 without Defender, has ForceAttack continuous effect.
- Generic werewolf tests cover the shared transform logic (no spells -> transform, first turn suppression, spells cast -> stay, 2+ spells -> transform back, multiple werewolves transform together).
