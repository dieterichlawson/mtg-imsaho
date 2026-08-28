## Audit — 2026-08-27 (Tier D — card data and static abilities only)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/69/moon-heron?utm_source=api
**Type line**: `Creature — Spirit Bird` — {3}{U}, 3/2
**Oracle text**:
```
Flying
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


## Audit — 2026-08-28 20:26

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying
**Type line**: Creature — Spirit Bird
**P/T**: 3/2
**Status**: PASS

### Code issues
No issues found. `mtg-engine/src/cards/isd/moon_heron.rs` matches: {3}{U}, Spirit Bird (both subtypes), 3/2, Flying. French vanilla.

### Tricky interactions checked
- None of its own. It is a Spirit (lord-buffed via shared paths) and the flyer-blocks-flyer / reach-blocks-flyer fixture.

### Test coverage
- Flying, both directions of the block table (blocks a flyer; blocked by reach): `mtg-engine/tests/keywords.rs` `flying_restricts_who_can_block`
- Debuff fixture for One-Eyed Scarecrow: `cards_morbid_and_ltb.rs`
- No rulings on Scryfall for this card.

Mutation check: emptying `keywords` (Flying) fails the block table ("Moon Heron should be able to block Abbey Griffin"). Bites.
