## Audit — 2026-08-27 (Tier D — card data and static abilities only)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/123/vampire-interloper?utm_source=api
**Type line**: `Creature — Vampire Scout` — {1}{B}, 2/1
**Oracle text**:
```
Flying
This creature can't block.
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


## Audit — 2026-08-28 20:16

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying
This creature can't block.
**Type line**: Creature — Vampire Scout
**P/T**: 2/1
**Status**: PASS

### Code issues
No issues found. `mtg-engine/src/cards/isd/vampire_interloper.rs` matches: {1}{B}, Vampire Scout (both subtypes), 2/1, Flying, `PreventBlock { OnSelf }`.

### Tricky interactions checked
- "Can't block" enforced in `eligible_blockers` via the shared `can_block` walk (`state.rs:1699`), same effect Pacifism uses with `Attached` scope — no special casing. PASS
- Can't block anything, flyers included — the restriction is absolute, unlike Orchard Spirit's conditional one. PASS
- It can still ATTACK freely (no defender); flying evasion is engine-generic. PASS
- Vampire subtype: Elite Inquisitor's protection, Rakish Heir triggers, etc. read it. PASS

### Test coverage
- Can't block + has flying on battlefield: `mtg-engine/tests/cards_morbid_and_ltb.rs` `vampire_interloper_cant_block` (flying assertion added this audit)
- Used as a neutral no-trigger attacker fixture in `cards_combat_damage_triggers.rs`.
- No rulings on Scryfall for this card.

Mutation checks:
- Emptying `continuous_effects` (PreventBlock): `vampire_interloper_cant_block` FAILS. Bites.
- Emptying `keywords` (Flying): same test FAILS on the new assertion. Bites.
