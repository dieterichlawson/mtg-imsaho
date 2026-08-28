## Audit — 2026-08-27 (Tier D — card data and static abilities only)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/71/rooftop-storm?utm_source=api
**Type line**: `Enchantment` — {5}{U}
**Oracle text**:
```
You may pay {0} rather than pay the mana cost for Zombie creature spells you cast.
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


## Audit — 2026-08-28 20:13

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: You may pay {0} rather than pay the mana cost for Zombie creature spells you cast.
**Type line**: Enchantment
**Status**: ISSUE

### Code issues
The card itself matches oracle: `mtg-engine/src/cards/isd/rooftop_storm.rs` is {5}{U} Enchantment with `AlternativeCost { cost: {0}, filter: CreatureWithSubtype("Zombie") }`. Writing the ruling-1 test exposed an adjacent engine hole:

- **A submitted cast or activation whose funding could not pay PANICKED** (`engine/actions/cast.rs`, `engine/actions/abilities.rs`).
  - Oracle/CR says: CR 601.2h — a cast whose costs cannot be paid is rewound; it never happens.
  - Code did: `mana::auto_pay(...).expect("legal_actions should have verified mana availability")` at both payment sites — a client-assembled action with an unpayable mana cost crashed the engine (and `auto_pay` drains the pool as it goes, so the failure could not be unwound either).
  - Fix: the cast path rehearses the submitted tap plan on a scratch clone (its production can depend on state it changes) and checks `can_pay` before touching real state; the ability path checks `can_pay` before deducting. Refusal leaves the state untouched. Committed separately.

### Tricky interactions checked
- Ruling: "You must still pay any mandatory additional costs" (Makeshift Mauler's exile): a {0} cast with no creature card in the graveyard is not offered and is refused if submitted; a legal {0} cast really exiles. NEW test this audit.
- Ruling: "The mana cost and mana value of the spell are unchanged": MV always reads `card_data().cost`; `spell_costs.rs` pins this pattern (`a_cost_reduction_does_not_change_a_cards_mana_value`, plus the walk in `spell_costs_are_determined_in_one_place`). PASS
- "you cast": `alternative_costs` walks `effects_controlled_by(caster)`. PASS
- Reaches every cast zone: Skaab Ruinator's graveyard cast castable for {0}. Tested. PASS
- Optional ("may"): the normal cost remains offered alongside {0} (cost-dedup machinery from CR 702.33 work). PASS
- Does NOT apply to non-cast mana payments: Back from the Brink's activated-ability copy cost ignores it. Tested. PASS

### Test coverage
- Filter reach (Zombie yes, non-Zombie no): `cards_rule_modifiers.rs` `a_cost_modifier_reaches_the_spells_its_text_names`
- Graveyard cast for {0}: `spell_costs.rs` `rooftop_storms_free_cast_reaches_a_zombie_cast_from_the_graveyard`
- Ruling 1 (additional cost still owed + actually paid): `spell_costs.rs` `rooftop_storms_zero_does_not_waive_a_mandatory_additional_cost` (NEW)
- Not-casting boundary: `cards_complex_creatures.rs` `back_from_the_brink_ignores_an_alternative_cost_for_casting`
- Engine hole regression: `submitted_targets.rs` `a_cast_submitted_without_funding_is_refused`, `an_activation_submitted_without_funding_is_refused` (NEW)

Mutation checks: disabling the cast-path guard fails `a_cast_submitted_without_funding_is_refused` (panics); disabling the ability-path guard fails `an_activation_submitted_without_funding_is_refused` (panics). Both bite.
