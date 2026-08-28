## Audit — 2026-08-27 (Tier D — card data and static abilities only)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/198/orchard-spirit?utm_source=api
**Type line**: `Creature — Spirit` — {2}{G}, 2/2
**Oracle text**:
```
This creature can't be blocked except by creatures with flying or reach.
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
**Oracle text**: This creature can't be blocked except by creatures with flying or reach.
**Type line**: Creature — Spirit
**P/T**: 2/2
**Status**: PASS

### Code issues
No issues found. `mtg-engine/src/cards/isd/orchard_spirit.rs` matches: {2}{G}, Spirit, 2/2, `CanOnlyBeBlockedBy { Or(HasKeyword(Flying), HasKeyword(Reach)), OnSelf }`. No keywords declared — correctly NOT flying.

### Tricky interactions checked
- It has the evasion of a flier without being one: no `Keyword::Flying`, so "creature with flying" filters (and flying-hate) don't see it, and it cannot block a flier itself. PASS
- The restriction is `OnSelf`: it constrains who may block Orchard Spirit, not whom Orchard Spirit may block. PASS
- Flying-granted-later blockers qualify: the filter reads `has_keyword` at block-declaration time (continuous, not printed). PASS
- Enforced in the shared `can_block_attacker` path, which `legal_actions` block generation and block submission both use. PASS

### Test coverage
- All three rows (ground no / flying yes / reach yes): `mtg-engine/tests/cards_evasion_and_graveyard_pt.rs` `orchard_spirit_is_blocked_only_by_flying_or_reach`
- Incidental casting-cost coverage in `mana_filters.rs`.
- No rulings on Scryfall for this card.

Mutation check: dropping the Reach arm from the Or fails the table on "Somberwald Spider (reach)". Bites.
