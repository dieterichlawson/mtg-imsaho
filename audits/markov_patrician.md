## Audit — 2026-08-27 (Tier D — card data and static abilities only)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/107/markov-patrician?utm_source=api
**Type line**: `Creature — Vampire` — {2}{B}, 3/1
**Oracle text**:
```
Lifelink (Damage dealt by this creature also causes you to gain that much life.)
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
**Oracle text**: Lifelink (Damage dealt by this creature also causes you to gain that much life.)
**Type line**: Creature — Vampire
**P/T**: 3/1
**Status**: PASS

### Code issues
No issues found. `mtg-engine/src/cards/isd/markov_patrician.rs` matches: {2}{B}, Vampire, 3/1, Lifelink. No behavior hooks.

### Tricky interactions checked
- Lifelink applies to combat damage to players AND to creatures — both tested with this card. The grant path (until-end-of-turn lifelink) is tested separately. PASS
- Lifelink is not a trigger: the gain is simultaneous with the damage (`LifeChanged` from the damage pipeline). PASS
- Vampire subtype read by Elite Inquisitor's protection (this card was the vacuous attacker there until the rewrite — its 3/1 dies to first strike, which is exactly why the rewritten test replaced it). PASS

### Test coverage
- Combat damage to player: `mtg-engine/tests/keywords.rs` `lifelink_gains_life_on_combat_damage`
- Combat damage to a blocking creature: `keywords.rs` `lifelink_gains_life_from_creature_damage`
- Fixture duty in vampire-subtype and removal tests.
- No rulings on Scryfall for this card.

Mutation check: emptying `keywords` (Lifelink) fails both lifelink tests. Bites.
