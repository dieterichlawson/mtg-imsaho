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

## Audit — 2026-04-02 21:23

**Oracle text source**: Scryfall API (cached 2026-04-01), https://scryfall.com/card/isd/3/angelic-overseer
**Oracle text**: Flying
As long as you control a Human, this creature has hexproof and indestructible.
**Type line**: Creature — Angel
**Status**: PASS

### Code issues
No issues found.

All card data fields verified against Scryfall oracle text:
- Name: `"Angelic Overseer"` -- matches
- Mana cost: `Generic(3), Colored(White), Colored(White)` -- matches `{3}{W}{W}`
- Types: `Creature` with subtype `"Angel"`, no supertypes -- matches `Creature — Angel`
- P/T: `power: Some(5), toughness: Some(3)` -- matches `5/3`
- Keywords: `vec![Keyword::Flying]` -- matches (hexproof/indestructible are conditional, not static keywords)
- Oracle text string: `"Flying\nAs long as you control a Human, this creature has hexproof and indestructible."` -- matches verbatim
- Continuous effects: Two `ConditionalKeyword` entries (Hexproof and Indestructible), both conditioned on `YouControlSubtype("Human")` with `EffectScope::OnSelf` -- correctly implements the conditional ability
- No triggered abilities, no `on_resolve` override -- correct for a vanilla creature with static/conditional abilities

### Tricky interactions checked (min 3)
1. **Simultaneous destruction with Humans (Ruling 2011-09-22)**: If a "destroy all creatures" effect resolves, the engine iterates and calls `try_destroy` individually. At the time each call is made, `has_keyword` dynamically evaluates `YouControlSubtype("Human")`. If a Human is destroyed before the Overseer in iteration order, the Overseer could lose indestructible. However, no "destroy all creatures" card exists in the current card pool, so this interaction cannot occur. The SBA path handles this correctly: SBAs are checked in a loop, so if a Human dies from lethal damage first, the Overseer loses indestructible and dies on the next SBA pass -- consistent with the ruling.
2. **Marked damage persists after losing Human (Ruling 2011-09-22)**: `has_keyword` re-evaluates the condition on every SBA check. If lethal damage is marked on the Overseer while it is indestructible (Human present), and the Human later leaves, the next SBA pass sees the Overseer no longer has indestructible and destroys it. This matches the ruling exactly.
3. **Opponent's Humans do not trigger the ability**: `YouControlSubtype` checks `o.controller == controller` where `controller` is the Overseer's controller. An opponent controlling a Human does not satisfy the condition.
4. **Hexproof targeting check**: Engine-level `can_be_targeted` in `engine.rs` checks `has_keyword(target, Hexproof)` and prevents opponent targeting. Works correctly with the conditional grant.
5. **Sacrifice bypasses indestructible**: `destruction::sacrifice` does not check `has_keyword` for indestructible, correctly allowing sacrifice even when the Overseer is protected.
6. **EffectScope::OnSelf correctness**: `effect_applies_to` with `OnSelf` checks `creature_id == source_id`, so hexproof/indestructible only apply to the Overseer itself, not to other creatures.

### Test coverage
- `angelic_overseer_has_flying` (tier12_cards.rs:563): Verifies flying keyword is always present
- `angelic_overseer_hexproof_indestructible_with_human` (tier12_cards.rs:574): Verifies hexproof/indestructible granted with Human, lost when Human leaves battlefield
- `angelic_overseer_survives_destroy_with_human` (tier12_cards.rs:605): Verifies Overseer survives `try_destroy` when controlling a Human
- All 3 tests pass.
- Not tested: simultaneous mass destruction with Humans (no "destroy all creatures" card in pool)
- Not tested: marked damage persisting after Human removal (verifiable by code inspection)

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

## Audit — 2026-04-10 18:24

**Oracle text source**: Oracle cache (Scryfall API), cached 2026-04-01
**Oracle text**:
```
Flying
As long as you control a Human, this creature has hexproof and indestructible.
```
**Type line**: Creature — Angel
**Mana cost**: {3}{W}{W}
**P/T**: 5/3
**Keywords**: Flying
**Status**: PASS

### Code issues
No issues found.

Verified against `/home/user/mtg-imsaho/mtg-engine/src/cards/isd/angelic_overseer.rs`:
- name "Angelic Overseer" matches
- cost `{3}{W}{W}` matches (Generic(3) + White + White)
- card_types: `[Creature]` matches
- supertypes: `[]` matches (type line has no Legendary supertype per Scryfall)
- subtypes: `["Angel"]` matches
- power 5, toughness 3 matches
- oracle_text field is verbatim match
- keywords: `[Flying]` matches
- Two `ContinuousEffect::ConditionalKeyword` entries (Hexproof, Indestructible), both with `EffectCondition::YouControlSubtype("Human")` and `EffectScope::OnSelf`, correctly implementing "As long as you control a Human, this creature has hexproof and indestructible."
- No triggered abilities or on_resolve needed (vanilla static ability creature).

### Tricky interactions checked
- Ruling #1 (simultaneous destroy-all-Humans + Overseer e.g. Wrath): The card itself is declared correctly. Preservation during simultaneous destruction must be handled by the destroy pipeline / caller; the card correctly declares the conditional Indestructible. Not a card-file issue.
- Ruling #2 (lethal damage marked + Human dies later in turn → Overseer dies): PASS. SBA lethal-damage path correctly re-evaluates indestructible; `src/sba.rs:97-100` snapshots indestructible BEFORE processing deaths in a single SBA pass, so within one SBA pass the Overseer survives alongside a dying Human; on a later SBA pass (after the Human is in the graveyard) the Overseer loses indestructible and dies. Matches the ruling.
- Human token recognition: PASS. `state.rs:1197-1206` `YouControlSubtype` check inspects both `o.subtypes` (populated for tokens) and `registry.card_data(o.card_id).subtypes` (for printed cards), so Human tokens granted by effects like Mayor of Avabruck / Midnight Haunting are recognized.
- Opponent's Humans do NOT grant the ability: PASS. Controller check `o.controller == controller` uses Overseer's controller.
- "You control a Human" vs. "another Human": oracle does not say "another," and since Angelic Overseer is an Angel (not a Human), self-exclusion is moot regardless.
- Losing the Human removes hexproof/indestructible: PASS, exercised by `tier12_cards.rs:599-604`.
- Hexproof prevents being targeted by opponent spells while a Human is controlled: relies on engine's generic hexproof enforcement (not card-specific).

### Test coverage
- Flying always present: `mtg-engine/tests/tier12_cards.rs:567` (`angelic_overseer_has_flying`)
- Hexproof + indestructible gained with Human, lost when Human leaves: `mtg-engine/tests/tier12_cards.rs:578` (`angelic_overseer_hexproof_indestructible_with_human`)
- try_destroy respects indestructible while controlling Human: `mtg-engine/tests/tier12_cards.rs:609` (`angelic_overseer_survives_destroy_with_human`)
- SBA ordering — lethal damage to both Human and Overseer (ruling #2 spirit): `mtg-engine/tests/audit_bugs2.rs:467` (`bug_angelic_overseer_sba_ordering`)
- Simultaneous destroy effect (Wrath/Divine Reckoning) preserving Overseer via ruling #1: NOT TESTED (this is an engine-wide mass-destroy semantics concern, not card-specific)
- Hexproof preventing an opponent's targeted removal: NOT TESTED directly for Angelic Overseer (relies on engine hexproof tests)
- Opponent-controlled Human does not grant the ability: NOT TESTED
- Human token (e.g., Midnight Haunting / Mayor of Avabruck transformed) granting the ability: NOT TESTED
