## Audit — 2026-08-27 (Tier D — card data and static abilities only)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/60/invisible-stalker?utm_source=api
**Type line**: `Creature — Human Rogue` — {1}{U}, 1/1
**Oracle text**:
```
Hexproof (This creature can't be the target of spells or abilities your opponents control.)
This creature can't be blocked.
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


## Audit — 2026-08-28 20:06

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Hexproof (This creature can't be the target of spells or abilities your opponents control.)
This creature can't be blocked.
**Type line**: Creature — Human Rogue
**P/T**: 1/1
**Status**: PASS

### Code issues
No issues found. `mtg-engine/src/cards/isd/invisible_stalker.rs` matches: {1}{U}, Human Rogue, 1/1, `Keyword::Hexproof`, `CantBeBlocked { OnSelf }`.

### Tricky interactions checked
- Hexproof is one-sided: opponents' spells/abilities can't target it, the controller's can (auras, Travel Preparations). Both directions tested with the Stalker itself. PASS
- Hexproof does not stop untargeted effects: Blasphemous Act / "each creature" sweeps still hit it (no targeting). Engine-generic — `is_target_legal` only gates Target-carrying actions. PASS
- "Can't be blocked" enforced in the shared `can_block_attacker` (offer and submit alike); a 5/5 vanilla cannot block it. PASS
- It is a Human: counts for Champion of the Parish, Elite Inquisitor's condition checks, etc. (subtype data). PASS

### Test coverage
- Hexproof vs opponent + own-targeting allowed: `mtg-engine/tests/keywords.rs` `hexproof_prevents_opponent_targeting`
- Unblockable: `mtg-engine/tests/cards_morbid_and_ltb.rs` `invisible_stalker_unblockable`
- No rulings on Scryfall for this card.

Mutation checks:
- Emptying `keywords` (Hexproof): `hexproof_prevents_opponent_targeting` FAILS. Bites.
- Emptying `continuous_effects` (CantBeBlocked): `invisible_stalker_unblockable` FAILS. Bites.
