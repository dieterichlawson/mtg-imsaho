# Audit: Angelic Overseer

## Reference (Scryfall/API)
- **Name:** Angelic Overseer
- **Mana Cost:** {3}{W}{W}
- **Type:** Creature — Angel
- **Oracle:** Flying. As long as you control a Human, Angelic Overseer has hexproof and indestructible.
- **P/T:** 5/3

## Implementation: `angelic_overseer.rs`
- **Name:** Angelic Overseer -- CORRECT
- **Mana Cost:** {3}{W}{W} -- CORRECT
- **Type:** Creature — Angel -- CORRECT
- **P/T:** 5/3 -- CORRECT
- **Keywords:** Flying -- CORRECT
- **Continuous effects:** ConditionalKeyword Hexproof (YouControlSubtype Human) + ConditionalKeyword Indestructible (YouControlSubtype Human) -- CORRECT

## Verdict: PASS -- No issues found

## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying\nAs long as you control a Human, this creature has hexproof and indestructible.
**Type line**: Creature — Angel
**Status**: ISSUE
### Code issues
1. **Oracle text wording mismatch (cosmetic)**: Oracle says `"As long as you control a Human, this creature has hexproof and indestructible."` but code oracle_text field says `"As long as you control a Human, Angelic Overseer has hexproof and is indestructible."` The code uses the old self-referential template instead of the updated "this creature" template.
   - Code: `"As long as you control a Human, Angelic Overseer has hexproof and is indestructible."`
   - Oracle: `"As long as you control a Human, this creature has hexproof and indestructible."`

Behavior is otherwise correct: two ConditionalKeyword continuous effects (Hexproof, Indestructible) both conditioned on YouControlSubtype("Human") with OnSelf scope. Stats, cost, types, and keywords all match.

## Re-audit — 2026-04-02
**Status**: PASS
Oracle text updated to match Scryfall: "this creature has hexproof and indestructible" (was "Angelic Overseer has hexproof and is indestructible"). Doc comment updated. Behavior unchanged.

## Audit — 2026-04-02 (full-reaudit)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-01

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying
As long as you control a Human, this creature has hexproof and indestructible.
**Type line**: Creature — Angel
**Mana cost**: {3}{W}{W}
**P/T**: 5/3
**Status**: PASS

### Code issues
No issues found.

All card data fields match the oracle text exactly:
- Mana cost: `Generic(3), Colored(White), Colored(White)` matches `{3}{W}{W}`
- Types: `Creature` with subtype `Angel`, no supertypes -- matches `Creature — Angel`
- P/T: `5/3` matches
- Keywords: `Flying` matches
- Oracle text string matches verbatim
- Continuous effects: Two `ConditionalKeyword` entries (Hexproof and Indestructible), both conditioned on `YouControlSubtype("Human")` with `EffectScope::OnSelf` -- correctly implements "As long as you control a Human, this creature has hexproof and indestructible."
- No triggered abilities declared, none needed
- No `on_resolve` override needed (vanilla creature with static abilities)

### Tricky interactions checked
- Simultaneous destruction (Ruling 1: destroy all Humans + Overseer at once): PASS -- `YouControlSubtype` checks current battlefield state, so indestructible is evaluated while Humans still exist during mass destruction
- Marked damage + losing Human (Ruling 2): PASS -- `has_keyword` dynamically evaluates the condition each time it is checked, so state-based actions will correctly see indestructible is gone once the Human leaves
- "You control" vs "another" distinction: PASS -- oracle says "you control a Human" (not "another"), and `YouControlSubtype` correctly checks all permanents (no self-exclusion needed; Angelic Overseer is an Angel anyway)
- `EffectScope::OnSelf` correctness: PASS -- the keywords should only apply to Angelic Overseer itself, not other creatures
- Hexproof prevents targeting by opponents: Handled by engine-level `has_keyword` check (not card-specific)
- Indestructible prevents destroy effects: Handled by engine-level `try_destroy` pipeline (not card-specific)

### Test coverage
- Flying keyword: `tier12_cards.rs:563` (`angelic_overseer_has_flying`)
- Hexproof/Indestructible with Human present: `tier12_cards.rs:574` (`angelic_overseer_hexproof_indestructible_with_human`)
- Hexproof/Indestructible lost when Human leaves: `tier12_cards.rs:574` (same test, lines 596-600)
- Survives destroy when indestructible: `tier12_cards.rs:605` (`angelic_overseer_survives_destroy_with_human`)
- Ruling 1 (simultaneous destruction with Humans): NOT TESTED
- Ruling 2 (marked damage persists, then Human removed): NOT TESTED
- LLM card knowledge entry: No card knowledge system found in this codebase

## Audit — 2026-04-02 20:28

**Oracle text source**: Oracle cache (Scryfall API, cached 2026-04-01)
**Oracle text**: Flying
As long as you control a Human, this creature has hexproof and indestructible.
**Type line**: Creature — Angel
**Status**: PASS

### Code issues
No issues found.

All card data fields verified against oracle:
- Name: "Angelic Overseer" -- matches
- Mana cost: `Generic(3), Colored(White), Colored(White)` -- matches `{3}{W}{W}`
- Types: `Creature` with subtype `Angel`, no supertypes -- matches
- P/T: `power: Some(5), toughness: Some(3)` -- matches `5/3`
- Keywords: `vec![Keyword::Flying]` -- matches (hexproof/indestructible are conditional, not static)
- Oracle text string: matches verbatim
- Continuous effects: Two `ConditionalKeyword` entries (Hexproof, Indestructible) conditioned on `YouControlSubtype("Human")` with `EffectScope::OnSelf` -- correctly implements the conditional ability

### Tricky interactions checked
- Simultaneous destruction (Ruling: destroy all Humans + Overseer at once): PASS -- `try_destroy` checks each permanent individually; at check time, Humans are still on the battlefield so `YouControlSubtype` returns true and Overseer is indestructible
- Marked damage persists after losing Human (Ruling: lethal damage stays marked; if Human leaves, Overseer dies): PASS -- `has_keyword` dynamically re-evaluates the condition on each SBA check, so once the Human is gone, indestructible is no longer granted and SBA destroys the creature via `try_destroy`
- Scope correctness (OnSelf): PASS -- `effect_applies_to` with `EffectScope::OnSelf` checks `creature_id == source_id`, so hexproof/indestructible only applies to the Overseer itself
- Opponent's Humans do not trigger the ability: PASS -- `YouControlSubtype` checks `o.controller == controller` where controller is the Overseer's controller, not the opponent's
- Sacrifice bypasses indestructible: PASS -- `destruction::sacrifice` does not call `has_keyword` for indestructible, correctly bypassing the protection

### Test coverage
- Flying keyword always present: `tier12_cards.rs:563` (`angelic_overseer_has_flying`)
- Hexproof/Indestructible granted with Human, lost without: `tier12_cards.rs:574` (`angelic_overseer_hexproof_indestructible_with_human`)
- Survives destroy effect when indestructible: `tier12_cards.rs:605` (`angelic_overseer_survives_destroy_with_human`)
- Ruling 1 (simultaneous destruction with Humans): NOT TESTED (but engine behavior verified via code inspection)
- Ruling 2 (marked damage persists, then Human removed): NOT TESTED (but engine behavior verified via code inspection)
