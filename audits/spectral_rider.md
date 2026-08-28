## Audit — 2026-08-27 (Tier D — card data and static abilities only)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/35/spectral-rider?utm_source=api
**Type line**: `Creature — Spirit Knight` — {W}{W}, 2/2
**Oracle text**:
```
Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)
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


## Audit — 2026-08-28 20:27

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)
**Type line**: Creature — Spirit Knight
**P/T**: 2/2
**Status**: PASS

### Code issues
No issues found. `mtg-engine/src/cards/isd/spectral_rider.rs` matches: {W}{W}, Spirit Knight (both subtypes), 2/2, Intimidate. French vanilla otherwise.

### Tricky interactions checked
- Intimidate reads the attacker's CURRENT colors and the blocker's artifact-ness at block time (shared `can_block_attacker` path). Both arms tested with this card: same-color blocks, off-color can't, artifact creature always can. PASS
- It is white via its mana cost; a color-changing effect would move who can block it (continuous read). PASS
- Spirit: lord-buffed via shared paths. PASS

### Test coverage
- Color arm: `mtg-engine/tests/keywords.rs` `intimidate_blocks_different_color`
- Artifact arm: `keywords.rs` `artifact_creature_blocks_intimidate`
- No rulings on Scryfall for this card.

Mutation check: emptying `keywords` (Intimidate) fails `intimidate_blocks_different_color`. Bites.
