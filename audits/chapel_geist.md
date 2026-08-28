## Audit — 2026-08-27 (Tier D — card data and static abilities only)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/7/chapel-geist?utm_source=api
**Type line**: `Creature — Spirit` — {1}{W}{W}, 2/3
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


## Audit — 2026-08-28 20:18

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying
**Type line**: Creature — Spirit
**P/T**: 2/3
**Status**: PASS

### Code issues
No issues found. `mtg-engine/src/cards/isd/chapel_geist.rs` matches: {1}{W}{W}, Spirit, 2/3, Flying. French vanilla — no behavior hooks, correctly.

### Tricky interactions checked
- None specific to the card; flying evasion and Spirit-lord interactions are engine-generic. It serves as a fixture in several rule tests, which is where its data is load-bearing.

### Test coverage
- Flying (as the flying-blocker row): `mtg-engine/tests/cards_evasion_and_graveyard_pt.rs` `orchard_spirit_is_blocked_only_by_flying_or_reach`
- Spirit + printed 2/3 (buffed by both lords): same file, `a_spirit_lord_buffs_other_spirits_you_control_and_nothing_else`
- No rulings on Scryfall for this card.

Mutation check: emptying `keywords` (Flying) fails the Orchard Spirit blocker table ("Chapel Geist (flying)"). Bites.
