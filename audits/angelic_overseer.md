## Audit — 2026-08-27 (Tier D — card data and static abilities only)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/3/angelic-overseer?utm_source=api
**Type line**: `Creature — Angel` — {3}{W}{W}, 5/3
**Oracle text**:
```
Flying
As long as you control a Human, this creature has hexproof and indestructible.
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


## Audit — 2026-08-28 19:51

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying
As long as you control a Human, this creature has hexproof and indestructible.
**Type line**: Creature — Angel
**P/T**: 5/3
**Status**: PASS

### Code issues
No issues found. `mtg-engine/src/cards/isd/angelic_overseer.rs` matches: {3}{W}{W}, Angel, 5/3, Flying printed, and two conditional `ContinuousEffect::when(EffectCondition::YouControlSubtype("Human"), GrantKeyword { Hexproof/Indestructible, OnSelf })`.

### Tricky interactions checked
- Ruling: "destroy each Human you control and Angelic Overseer simultaneously — Overseer won't be destroyed": no destroy-based sweeper exists in the ISD pool, so the reachable form is the per-destroy check — `try_destroy` consults `has_keyword(Indestructible)` at the moment of destruction, while the Humans still stand. PASS (engine-level)
- Ruling: "lethal damage stays marked; if later in the turn you no longer control a Human, it will be destroyed": reachable via Blasphemous Act killing your only Human while marking 13 on the Overseer. The SBA loop runs to quiescence (CR 704.3): pass 1 kills the Human (Overseer still indestructible — the Human is on the battlefield during that pass), pass 2 finds the Overseer with lethal marked damage and no Human, and destroys it. The engine already did this correctly; a test now pins it.
- `YouControlSubtype` counts tokens too (`has_subtype` unions object subtypes with the registry face) and is scoped to the controller only. PASS
- Flying is unconditional and survives the Human leaving (asserted explicitly). PASS
- Hexproof gates opponent targeting via the shared `is_target_legal` path. PASS

### Test coverage
- Conditional keywords on/off + Flying unconditional: `mtg-engine/tests/cards_spells_and_enchantments.rs` `angelic_overseer_hexproof_indestructible_with_human`
- Survives try_destroy with a Human: `cards_spells_and_enchantments.rs` `angelic_overseer_survives_destroy_with_human`
- Ruling 2 (marked damage kills it once the Human dies): `cards_spells_and_enchantments.rs` `angelic_overseer_dies_to_marked_damage_once_its_human_is_gone` — NEW this audit
- Ruling 1: NOT TESTED directly (no mass-destroy card in pool); the per-destroy indestructible check is the tested mechanism.

Mutation check: collapsing the SBA loop to a single pass fails the new test (Overseer survives with 13 marked and no Human). Bites — the test pins the loop-to-quiescence behavior the ruling depends on.
