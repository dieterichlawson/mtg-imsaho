## Audit — 2026-08-27 (Tier D — card data and static abilities only)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/104/heartless-summoning?utm_source=api
**Type line**: `Enchantment` — {1}{B}
**Oracle text**:
```
Creature spells you cast cost {2} less to cast.
Creatures you control get -1/-1.
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


## Audit — 2026-08-28 19:55

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Creature spells you cast cost {2} less to cast.
Creatures you control get -1/-1.
**Type line**: Enchantment
**Status**: PASS

### Code issues
No issues found. `mtg-engine/src/cards/isd/heartless_summoning.rs` matches: {1}{B} Enchantment, `ReduceCost { reduction: 2, filter: CreatureSpells }` and `ModifyPT { -1, -1, Global(ControlledByYou) }`.

### Tricky interactions checked
- Ruling: "can only reduce the generic mana portion": `reduce()` in `engine/costs.rs` takes the reduction off Generic symbols only (CR 601.2f), colored requirements untouched. Tested directly. PASS
- "you cast": `total_reduction` walks `effects_controlled_by(caster)` — an opponent's Summoning doesn't discount your spells. Tested. PASS
- Reduction reaches graveyard/flashback casts and doesn't change mana value. Tested. PASS
- "Creatures you control" get -1/-1 — an opponent's creatures don't. Assertion added this audit.
- The classic: a 1-toughness creature you control has toughness 0 and dies to SBAs (CR 704.5f) — no damage, no regeneration. Assertion added this audit (Doomed Traveler enters, SBA pass, graveyard).
- The two halves are independent effects: the shrink applies to all your creatures, cheapened or not.

### Test coverage
- Generic-only reduction: `mtg-engine/tests/spell_costs.rs` `a_reduction_only_comes_off_the_generic_portion`
- Controller scoping: `spell_costs.rs` `a_reduction_only_helps_the_player_who_controls_it`
- From-hand / from-graveyard / offer-time / filter / source-leaves / MV: `spell_costs.rs` (six more tests)
- Which spells the discount reaches (creature yes, instant no): `cards_rule_modifiers.rs` `a_cost_modifier_reaches_the_spells_its_text_names`
- -1/-1 (yours shrunk, theirs not, 1/1 dies to SBA): `cards_rule_modifiers.rs` `heartless_summoning_shrinks_the_creatures_it_cheapens` (extended this audit)

Mutation checks:
- Widening the shrink to all creatures (`HasCardType(Creature)` instead of `ControlledByYou`): extended test FAILS. Bites. (A first attempt used a nonexistent `CreatureFilter::Any` and did not compile — discarded as proving nothing.)
