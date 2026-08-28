## Audit — 2026-08-27 (Tier D — card data and static abilities only)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/42/voiceless-spirit?utm_source=api
**Type line**: `Creature — Spirit` — {2}{W}, 2/1
**Oracle text**:
```
Flying, first strike
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


## Audit — 2026-08-28 20:28

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying, first strike
**Type line**: Creature — Spirit
**P/T**: 2/1
**Status**: PASS

### Code issues
No issues found. `mtg-engine/src/cards/isd/voiceless_spirit.rs` matches: {2}{W}, Spirit, 2/1, Flying + First strike. (The file's doc comment says only "Flying." — comment-only nit, the data itself is complete.)

One coverage gap closed: its flying was never behavior-checked (dropping it passed the whole keywords suite) — the first-strike test now asserts it.

### Tricky interactions checked
- First strike splits the combat damage step: as a 2/1 it kills a 3/2 blocker before taking damage back — tested with this card. PASS
- Both keywords through the shared `has_keyword` path (zone-gated, grant-aware). PASS
- Spirit: lord-buffed via shared paths. PASS

### Test coverage
- First strike + flying: `mtg-engine/tests/keywords.rs` `first_strike_kills_before_normal_damage` (flying assertion added this audit)
- Trigger-dispatch fixture: `trigger_dispatch.rs`
- No rulings on Scryfall for this card.

Mutation checks: dropping FirstStrike fails the test; dropping Flying previously passed the suite (gap recorded above) and now fails the new assertion. Both bite.
