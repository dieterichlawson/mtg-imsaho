## Audit — 2026-08-27 (Tier D — card data and static abilities only)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/190/kindercatch?utm_source=api
**Type line**: `Creature — Spirit` — {3}{G}{G}{G}, 6/6
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


## Audit — 2026-08-28 20:25

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: (none — vanilla creature)
**Type line**: Creature — Spirit
**P/T**: 6/6
**Status**: PASS

### Code issues
No issues found. `mtg-engine/src/cards/isd/kindercatch.rs` matches: {3}{G}{G}{G}, Spirit, 6/6, no text, no behavior hooks. The empty `oracle_text` is allowed by `card_data_invariants.rs` for exactly this shape (vanilla creature).

### Tricky interactions checked
- None of its own. It is a Spirit, so both Spirit lords buff it (shared `HasSubtype` path), and its mana value 6 is what Mindshrieker's tests read.

### Test coverage
- P/T pinned as the Heartless Summoning shrink fixture (6/6 -> 5/5): `cards_rule_modifiers.rs` `heartless_summoning_shrinks_the_creatures_it_cheapens`
- Mana value 6 pinned by Mindshrieker's mill tests: `cards_activated_abilities.rs`
- No rulings on Scryfall for this card.

Mutation check: `power: 6 -> 5` fails the Heartless Summoning test. Bites.
