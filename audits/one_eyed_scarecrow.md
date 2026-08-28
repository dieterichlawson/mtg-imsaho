## Audit — 2026-08-27 (Tier D — card data and static abilities only)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/230/one-eyed-scarecrow?utm_source=api
**Type line**: `Artifact Creature — Scarecrow` — {3}, 2/3
**Oracle text**:
```
Defender
Creatures with flying your opponents control get -1/-0.
```
**Status**: PASS

### Code issues
No issues found.

### What was checked
This card declares no behaviour hooks — no `on_resolve`, no triggered abilities,
no activated abilities. Everything it does is card data plus declarative `continuous_effects`,
so the audit is steps 1, 2, 6 and 9 in full; step 3 is skipped, which the
procedure directs for vanilla creatures and basic spells.

- Mana cost, card types, supertypes, subtypes, power/toughness and oracle text
  compared character-for-character against the cached Scryfall entry: exact.
- Keywords checked against the oracle text of this face: complete, with no
  keyword declared that the text does not grant.
- Flashback cost: none, and the oracle names none.
- Trigger kinds: none declared, and the oracle text contains no triggered-ability
  phrasing that would need one.
- `continuous_effects` compared clause by clause against the static abilities in the oracle text, including the scope distinction between "creatures you control" (`Global`) and "**other** creatures you control" (`GlobalOther`).
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
Static-ability behaviour is exercised through the shared continuous-effects tests in `continuous_effects.rs` and `snapshot_anthems.rs`.


## Audit — 2026-08-28 20:07

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Defender
Creatures with flying your opponents control get -1/-0.
**Type line**: Artifact Creature — Scarecrow
**P/T**: 2/3
**Status**: PASS

### Code issues
No issues found. `mtg-engine/src/cards/isd/one_eyed_scarecrow.rs` matches: {3}, Artifact + Creature, Scarecrow, 2/3, Defender, `ModifyPT { -1, 0, Global(ControlledByOpponent AND HasKeyword(Flying)) }`.

One test gap closed: the generic defender-can't-attack test only used Grave Bramble, so the Scarecrow's own Defender declaration was unguarded. The test now tables both defenders in the set.

### Tricky interactions checked
- The debuff reads `HasKeyword(Flying)` continuously — a creature granted flying mid-turn shrinks, one losing it recovers. PASS
- Only opponents' flyers; your own and their ground creatures untouched. Tested. PASS
- -1/-0 can make a 1-power flyer 0-power (not dead — power, not toughness). Consistent with `ModifyPT` semantics. PASS
- It is colourless ({3} artifact): used as the colourless fixture in `gruesome_deformity_on_a_colourless_creature_leaves_only_artifacts` (CR 105.1). PASS
- Artifact: vulnerable to artifact removal and Stony-Silence-type gates read `HasCardType(Artifact)` from its card_types. PASS

### Test coverage
- Debuff (flyer shrunk, toughness unchanged, ground and own flyer untouched): `mtg-engine/tests/cards_morbid_and_ltb.rs` `one_eyed_scarecrow_debuffs_opponent_flyers`
- Defender can't attack (now includes the Scarecrow): `mtg-engine/tests/keywords.rs` `defender_cannot_attack`
- No rulings on Scryfall for this card.

Mutation checks:
- Emptying `keywords` (Defender): `defender_cannot_attack` FAILS on the Scarecrow row. Bites (new coverage).
- Dropping the Flying conjunct from the filter: debuff test FAILS ("ground creature should not be affected"). Bites.
