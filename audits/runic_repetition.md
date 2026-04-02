# Audit: Runic Repetition

## Official Oracle
- **Name:** Runic Repetition
- **Cost:** {2}{U}
- **Type:** Sorcery
- **Oracle Text:** Return target exiled card with flashback you own to your hand.
- **P/T:** N/A

## Implementation Review
- **Name:** OK
- **Cost:** {2}{U} — OK
- **Type:** Sorcery — OK
- **Oracle Text:** "Return target exiled card you own with flashback to your hand." — minor wording order difference from official "Return target exiled card with flashback you own to your hand." — functionally identical — OK
- **P/T:** N/A — OK
- **on_resolve:** Auto-selects a card in exile owned by controller with flashback, moves to hand — functionally correct but no targeting

## Issues
1. **ISSUE: No targeting — auto-selects**: The card says "target exiled card with flashback you own" but the implementation auto-selects the first matching card rather than letting the player choose which exiled flashback card to return. The player should have a choice when multiple candidates exist.

## Verdict: FAIL
- **Targeting is bypassed** — player should choose which exiled flashback card to return

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: Return target exiled card with flashback you own to your hand.
**Scryfall type line**: Sorcery
**Status**: PASS

Previous targeting issue has been fixed. The implementation now uses `TargetRequirement::ExileCard` and implements `is_valid_target` to check that the target is in exile, owned by the caster, and has flashback. The player can now choose which exiled flashback card to return.

Verified correct:
- Mana cost: {2}{U} -- matches
- Type: Sorcery -- matches
- Oracle text: minor word order difference ("card you own with flashback" vs "card with flashback you own") but functionally identical
- `on_resolve`: moves target from exile to hand, then calls `move_spell_after_resolve(object_id)` -- correct
- No anti-patterns detected
- Tests found in `mtg-engine/tests/innistrad_simple_cards.rs`

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch
**Oracle text**: Return target exiled card with flashback you own to your hand.
**Type line**: Sorcery
**Status**: PASS

Card data correct: name, mana cost ({2}{U}), type (Sorcery).

Targeting uses TargetRequirement::ExileCard with custom is_valid_target that checks: card in exile, owned by caster, and has flashback (flashback_cost.is_some()). This correctly implements the oracle text restriction.

on_resolve moves target from exile to hand and calls move_spell_after_resolve(object_id).

Tests in innistrad_simple_cards.rs cover card data and returning a flashback card from exile. No anti-patterns found.

## Audit — 2026-04-02

### Oracle Text (Scryfall)
"Return target exiled card with flashback you own to your hand."

### Implementation Oracle Text
"Return target exiled card you own with flashback to your hand."

### Card Data
- **Name**: Runic Repetition — correct
- **Mana cost**: {2}{U} — correct
- **Type**: Sorcery — correct
- **No subtypes/supertypes**: correct

### Findings

#### Issue 1: Oracle text word order mismatch (cosmetic)
- **Oracle**: "Return target exiled card with flashback you own to your hand."
- **Implementation**: "Return target exiled card you own with flashback to your hand."
- The words "with flashback" and "you own" are swapped. This is cosmetic only and does not affect gameplay logic, but the oracle_text string should match Scryfall verbatim.

#### Targeting — CORRECT
- `target_requirement` returns `ExileCard`, which is appropriate for targeting cards in exile.
- `is_valid_target` checks:
  1. Target is an `Object` — correct
  2. Object is in `Zone::Exile` — correct (targets exiled cards)
  3. `o.owner == caster` — correct ("you own")
  4. Card has `flashback_cost.is_some()` via registry lookup — correct ("with flashback")
- Per ruling 2011-09-22: cards that were *granted* flashback in the graveyard lose it once exiled; the implementation checks the card's innate `flashback_cost` from the registry, which correctly excludes temporarily-granted flashback. This matches the ruling.
- Face-down exiled cards: the implementation does not explicitly handle face-down cards (ruling says they can't be targeted). However, face-down cards presumably wouldn't have a `card_id` that resolves to flashback data, so this is likely handled implicitly.

#### Zone Movement — CORRECT
- `on_resolve` calls `state.move_object(*target_id, Zone::Hand)` — moves target from exile to hand.
- Since `is_valid_target` already ensures `o.owner == caster`, the card returns to its owner's hand. Correct.
- `move_spell_after_resolve` is called to handle the spell itself post-resolution. Correct.

#### Spell Cleanup — CORRECT
- Sorcery correctly goes to graveyard after resolution via `move_spell_after_resolve`.

### Test Coverage
- `runic_repetition_card_data`: verifies Sorcery type and CMC 3. PASS.
- `runic_repetition_returns_flashback_card_from_exile`: puts Think Twice (has flashback) in exile, casts Runic Repetition targeting it, verifies it moves to Hand. PASS.

#### Missing Test Coverage
- No test for rejecting a non-flashback exiled card as a target.
- No test for rejecting an opponent's exiled flashback card as a target.
- No test for rejecting a card not in exile (e.g., in graveyard) as a target.

### Verdict
**PASS with cosmetic fix needed.** The gameplay logic is correct. The oracle text string has a word order discrepancy that should be fixed to match Scryfall verbatim.

---

## Audit — 2026-04-02

**Oracle text (Scryfall):** "Return target exiled card with flashback you own to your hand."

**Implementation oracle_text:** "Return target exiled card with flashback you own to your hand."

### Card Data — PASS
- Name: "Runic Repetition" — matches
- Mana cost: {2}{U} (Generic(2), Colored(Blue)) — matches
- Type: Sorcery — matches
- No supertypes, subtypes, keywords, flashback — correct for this card

### Targeting — PASS
- `target_requirement()` returns `TargetRequirement::ExileCard`, which filters candidates from exile owned by the caster.
- `is_valid_target` checks all three required conditions:
  1. `o.zone == Zone::Exile` — correct, targets exiled cards only
  2. `o.owner == caster` — correct, "you own"
  3. `registry.card_data(o.card_id).flashback_cost.is_some()` — correct, "with flashback"
- Per ruling (2011-09-22): cards that only temporarily had flashback (e.g., granted by another effect while in graveyard) lose it once they leave the graveyard/stack. The implementation checks the card's innate `flashback_cost` from `CardData`, which correctly excludes temporarily-granted flashback since `CardData` represents the card's printed characteristics.
- Face-down exile (ruling 2011-09-22): the engine does not support face-down exile, so this ruling is not applicable.

### Zone Movement — PASS
- `on_resolve` calls `state.move_object(*target_id, Zone::Hand)`, moving the target from exile to hand.
- Since `is_valid_target` already ensures `o.owner == caster`, the card goes to the correct player's hand (the owner/caster).
- `state.move_spell_after_resolve(object_id)` correctly handles the sorcery going to graveyard after resolution.

### Tests
- `runic_repetition_card_data`: verifies Sorcery type and mana value 3 — PASS
- `runic_repetition_returns_flashback_card_from_exile`: creates a Think Twice (has flashback) in exile, casts Runic Repetition targeting it, verifies Think Twice moves to hand — PASS
- Missing test coverage:
  - No test for rejecting a non-flashback exiled card as a target.
  - No test for rejecting an opponent's exiled flashback card.
  - No test verifying the spell cannot be cast if no valid targets exist.

### Verdict
**PASS.** The implementation correctly implements all aspects of the oracle text: targeting (exiled cards with flashback owned by caster), zone movement (exile to hand), and spell type (sorcery). The oracle_text string now matches Scryfall verbatim.
