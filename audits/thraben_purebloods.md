## Audit — 2026-08-27 (Tier D — card data and static abilities only)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/37/thraben-purebloods?utm_source=api
**Type line**: `Creature — Dog` — {4}{W}, 3/5
**Oracle text**:
```

```
**Status**: PASS

### Code issues
No issues found.

### What was checked
This card declares no behaviour hooks — no `on_resolve`, no triggered abilities,
no activated abilities. Everything it does is card data,
so the audit is steps 1, 2, 6 and 9 in full; step 3 is skipped, which the
procedure directs for vanilla creatures and basic spells.

- Mana cost, card types, supertypes, subtypes, power/toughness and oracle text
  compared character-for-character against the cached Scryfall entry: exact.
- Keywords checked against the oracle text of this face: complete, with no
  keyword declared that the text does not grant.
- Flashback cost: none, and the oracle names none.
- Trigger kinds: none declared, and the oracle text contains no triggered-ability
  phrasing that would need one.

- Step 9 anti-patterns: clean. No self spell-cleanup, no `obj.power` used as a
  creature test, no `CombatDamageDealt` for non-combat damage, no token created
  without its subtypes, no hook left undeclared.

### Tricky interactions checked
None apply: with no triggered or activated ability there is no stack entry to
outlive its source, no target to re-check on resolution, and no choice to
present.

### Test coverage
Registry-wide invariants in `card_data_invariants.rs` cover this card's data
consistency (P/T exactly on creatures, subtypes implying their card type, every
declared keyword printed on the card, no field declared twice).
A vanilla creature has no behaviour to exercise beyond that.


## Audit — 2026-08-28 20:31

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: (none — vanilla creature)
**Type line**: Creature — Dog
**P/T**: 3/5
**Status**: PASS

### Code issues
No issues found. `mtg-engine/src/cards/isd/thraben_purebloods.rs` matches: {4}{W}, **Dog** (the current type line — this card was printed as a Hound and errata'd, exactly the drift the audit procedure's no-training-data rule exists for; the code is on the right side of it), 3/5, no text, no hooks.

### Tricky interactions checked
- True vanilla; nothing in the ISD pool reads "Dog" or "Hound", so the subtype is future-proofing rather than live — but it is now pinned to the current oracle so a well-meaning "fix" back to Hound fails a test.

### Test coverage
- All printed characteristics: `mtg-engine/tests/cards_vanilla_and_keywords.rs` `vanilla_creatures_have_their_printed_characteristics`
- No rulings on Scryfall for this card.

Mutation check: subtype "Dog" -> "Hound" (the pre-errata type) fails the table ("Thraben Purebloods is a Dog"). Bites.
