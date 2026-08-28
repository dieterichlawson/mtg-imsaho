## Audit — 2026-08-27 (Tier D — card data and static abilities only)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/19/intangible-virtue?utm_source=api
**Type line**: `Enchantment` — {1}{W}
**Oracle text**:
```
Creature tokens you control get +1/+1 and have vigilance.
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


## Audit — 2026-08-28 20:25

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Creature tokens you control get +1/+1 and have vigilance.
**Type line**: Enchantment
**Status**: PASS

### Code issues
No issues found. `mtg-engine/src/cards/isd/intangible_virtue.rs` matches: {1}{W} Enchantment, `ModifyPT { +1, +1 }` and `GrantKeyword { Vigilance }`, both scoped `Global(ControlledByYouToken)`.

One test gap closed: neither test checked that an OPPONENT'S token gets nothing — added that row.

### Tricky interactions checked
- Tokens only: your non-token creatures untouched. Tested (both files).
- "you control": opponent's tokens untouched. NEW row this audit.
- `ControlledByYouToken` reads `creature.is_token` + controller — copies of tokens are tokens (`is_token` carries), and a token that changes control moves in and out of the anthem live. PASS
- +1/+1 keeps 1/1 Spirits alive through -1/-1 sweeps; interacts with the SBA toughness check via `effective_toughness`. Engine-generic. PASS

### Test coverage
- Buff + vigilance on token, non-token untouched, opponent token untouched: `mtg-engine/tests/cards_death_triggers_and_tokens.rs` `intangible_virtue_buffs_creatures` (extended)
- Same claims (near-duplicate kept from the phase-6 reorg): `cards_morbid_and_ltb.rs` `intangible_virtue_token_only`
- No rulings on Scryfall for this card.

Mutation checks:
- Widening the P/T scope to `ControlledByYou` (non-tokens buffed): FAILS. Bites.
- Replacing the vigilance grant with a no-op effect (import sink kept): FAILS on the vigilance assertion. Bites. (First attempt deleted the line, dropped the only `Keyword` use, and did not compile — discarded.)
