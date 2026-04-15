---
id: merged-token-copy-inconsistent-01
status: new
card: multiple
created: 2026-04-15T02:45:29Z
kind: consolidated
source_tickets: cackling_counterpart-01, cackling_counterpart-02, back_from_the_brink-04
---

# `create_token_copy` has inconsistent copiable-value sources (CR 707.2)

## Description
Per CR 707.2, a copy effect copies the "copiable values" — a consistent set of characteristics from one source. `create_token_copy` (state.rs:479-527) reads some fields from the runtime `GameObject` (power, toughness) and other fields from the registry via `card_data(card_id).unwrap_or_default()` (keywords, subtypes, card_types, colors). This splits the copy source inconsistently:
- Copying a generic token (card_id = CardId(0)): registry returns None → token_copy loses subtypes, card_types, and colors.
- Copying a transformed DFC: registry returns front-face data but obj has back-face name → token gets contradictory mix.
- Copying a creature with runtime-modified P/T (e.g., Tree of Redemption with swapped toughness): copy inherits modified P/T instead of printed values.

Additionally, a token copy of a transforming DFC should not itself transform (CR 712.12), but `apply_transform` has no `is_token` guard.

## Engine path
- state.rs:479-527 (create_token_copy — mixed data sources)
- state.rs:486-505 (copiable-value reads)
- state.rs:487 (power/toughness read from GameObject)
- cards/helpers.rs:262-293 (apply_transform — no is_token check)

## Tests

### test_copy_of_generic_token_preserves_characteristics
Source ticket: cackling_counterpart-01
Implementation: (not yet written)
Scenario: A generic 2/2 black Zombie token is on the battlefield. Cast Cackling Counterpart copying it. Verify the copy token is also 2/2, black, and has the Zombie subtype.

### test_copy_of_creature_uses_printed_pt
Source ticket: cackling_counterpart-02
Implementation: (not yet written)
Scenario: Tree of Redemption with exchanged toughness (e.g., 0/20 after swapping with life total 20). Cast Cackling Counterpart targeting Tree. Verify the copy is 0/13 (printed P/T), not 0/20.

### test_dfc_token_does_not_transform
Source ticket: back_from_the_brink-04
Implementation: (not yet written)
Scenario: Exile Mayor of Avabruck from graveyard via Back from the Brink; a token copy of the front face is created. Pass through an upkeep where no spells were cast last turn. Verify the token copy does NOT transform into Howlpack Alpha.

