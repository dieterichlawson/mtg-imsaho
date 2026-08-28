## Audit — 2026-08-27 (Tier D — card data and static abilities only)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/45/battleground-geist?utm_source=api
**Type line**: `Creature — Spirit` — {4}{U}, 3/3
**Oracle text**:
```
Flying
Other Spirit creatures you control get +1/+0.
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
Other Spirit creatures you control get +1/+0.
**Type line**: Creature — Spirit
**P/T**: 3/3
**Status**: PASS

### Code issues
No issues found. `mtg-engine/src/cards/isd/battleground_geist.rs` matches: {4}{U}, Spirit, 3/3, Flying, `ModifyPT { power: 1, toughness: 0, scope: GlobalOther(ControlledByYou AND HasSubtype("Spirit")) }`.

### Tricky interactions checked
- "Other" excludes itself; two Geists each pump the other (falls out of GlobalOther per-source evaluation). PASS
- Only Spirits, only yours: filter is And(ControlledByYou, HasSubtype). An opponent's Spirit and your non-Spirit both untouched. PASS
- Spirit tokens (no registry face) are pumped: `HasSubtype` goes through the accessor union (`has_subtype`), guarded in `subtype.rs`. PASS
- Static anthem, not a snapshot: a Spirit that enters later is pumped, one that leaves stops being pumped — the effect is re-evaluated in `effective_power`. PASS
- It is itself a Spirit: buffed by a second copy, and by Gallows Warden's +0/+1. PASS

### Test coverage
- All four claims (other / Spirit / you control / +1/+0): `mtg-engine/tests/cards_evasion_and_graveyard_pt.rs` `a_spirit_lord_buffs_other_spirits_you_control_and_nothing_else` (table shared with Gallows Warden)
- No rulings on Scryfall for this card.

Mutation check: `GlobalOther` -> `Global` (self included) fails the table test on "'other' excludes itself". Bites.
