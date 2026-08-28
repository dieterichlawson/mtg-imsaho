## Audit — 2026-08-27 (Tier D — card data and static abilities only)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/16/gallows-warden?utm_source=api
**Type line**: `Creature — Spirit` — {4}{W}, 3/3
**Oracle text**:
```
Flying
Other Spirit creatures you control get +0/+1.
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


## Audit — 2026-08-28 19:52

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying
Other Spirit creatures you control get +0/+1.
**Type line**: Creature — Spirit
**P/T**: 3/3
**Status**: PASS

### Code issues
No issues found. `mtg-engine/src/cards/isd/gallows_warden.rs` matches: {4}{W}, Spirit, 3/3, Flying, `ModifyPT { power: 0, toughness: 1, scope: GlobalOther(ControlledByYou AND HasSubtype("Spirit")) }` — the same shape as Battleground Geist with the buff on the toughness side.

### Tricky interactions checked
- "Other" excludes itself; only your Spirits; Spirit tokens included via the `has_subtype` accessor union; static re-evaluation (newcomers picked up, leavers dropped). Same analysis as Battleground Geist (220/249). PASS
- Toughness-specific: the +0/+1 feeds `effective_toughness`, which the SBA lethal-damage check reads — a Spirit with 3 damage marked and boosted toughness 4 lives, and dies to the re-check if the Warden leaves. Engine-generic (same mechanism the Angelic Overseer ruling test pins). PASS

### Test coverage
- All four claims (other / Spirit / you control / +0/+1): `mtg-engine/tests/cards_evasion_and_graveyard_pt.rs` `a_spirit_lord_buffs_other_spirits_you_control_and_nothing_else`
- No rulings on Scryfall for this card.

Mutation check: swapping the grant to +1/+0 fails the table test ("Gallows Warden: your own Spirit is buffed"). Bites.
