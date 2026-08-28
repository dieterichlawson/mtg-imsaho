## Audit — 2026-08-27 (Tier D — card data and static abilities only)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/169/ambush-viper?utm_source=api
**Type line**: `Creature — Snake` — {1}{G}, 2/1
**Oracle text**:
```
Flash
Deathtouch
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


## Audit — 2026-08-28 20:23

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flash
Deathtouch
**Type line**: Creature — Snake
**P/T**: 2/1
**Status**: PASS

### Code issues
No issues found. `mtg-engine/src/cards/isd/ambush_viper.rs` matches: {1}{G}, Snake, 2/1, Flash + Deathtouch. French vanilla, no behavior hooks.

One coverage gap closed: deathtouch was behavior-tested only through Typhoid Rats, so the Viper's declaration was unguarded — `deathtouch_kills_with_one_damage` now tables every deathtouch creature in the set.

### Tricky interactions checked
- Flash (castable on the opponent's turn) tested with this card. PASS
- Deathtouch: any nonzero damage is lethal via the `dealt_deathtouch_damage` mark + SBA (CR 704.5h); deathtouch+trample minimum-assignment covered generically. PASS
- The classic ambush (flash in as a surprise blocker, deathtouch the attacker): composition of the two tested keywords through the same shared paths. PASS

### Test coverage
- Flash: `mtg-engine/tests/keywords.rs` `flash_creature_castable_at_instant_speed`
- Deathtouch (now including the Viper): `keywords.rs` `deathtouch_kills_with_one_damage` (tabled this audit)
- Fixture duty in `copy_effects.rs` (Snake subtype / green color reads).
- No rulings on Scryfall for this card.

Mutation checks: dropping Deathtouch fails the tabled deathtouch test; dropping Flash fails the flash test. Both bite.
