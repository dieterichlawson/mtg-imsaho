## Audit — 2026-08-27 (Tier D — card data and static abilities only)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/1/abbey-griffin?utm_source=api
**Type line**: `Creature — Griffin` — {3}{W}, 2/2
**Oracle text**:
```
Flying, vigilance
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


## Audit — 2026-08-28 20:22

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying, vigilance
**Type line**: Creature — Griffin
**P/T**: 2/2
**Status**: PASS

### Code issues
No issues found. `mtg-engine/src/cards/isd/abbey_griffin.rs` matches: {3}{W}, Griffin, 2/2, Flying + Vigilance. French vanilla, no behavior hooks.

### Tricky interactions checked
- None card-specific; both keywords are engine-generic and behavior-tested with this very card.

### Test coverage
- Flying (blocker table rows): `mtg-engine/tests/keywords.rs` `flying_restricts_who_can_block`
- Vigilance (attacks untapped): `keywords.rs` `vigilance_does_not_tap_on_attack`
- Fixture duty in `hexproof_filter.rs` (as the flying target) and `cards_complex_creatures.rs`.
- No rulings on Scryfall for this card.

Mutation checks: dropping Flying fails the blocker table; dropping Vigilance fails the vigilance test. Both bite.
