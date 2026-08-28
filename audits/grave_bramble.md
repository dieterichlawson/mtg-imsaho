## Audit — 2026-08-27 (Tier D — card data and static abilities only)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/184/grave-bramble?utm_source=api
**Type line**: `Creature — Plant` — {1}{G}{G}, 3/4
**Oracle text**:
```
Defender, protection from Zombies
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


## Audit — 2026-08-28 20:04

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Defender, protection from Zombies
**Type line**: Creature — Plant
**P/T**: 3/4
**Status**: PASS

### Code issues
No issues found. `mtg-engine/src/cards/isd/grave_bramble.rs` matches: {1}{G}{G}, Plant, 3/4, `Keyword::Defender`, `ProtectionFromSubtype { "Zombie", OnSelf }`.

### Tricky interactions checked
- Defender means it never attacks, so "Zombies can't block it" is unreachable; the protection's live arms are damage-while-blocking, targeting, and the one-direction rule. All three now covered.
- One-directional: the Bramble CAN block Zombies (protection stops Zombies blocking IT, not the reverse). Tested. PASS
- Targeting arm: Grimgrin (Zombie source) cannot target it — trigger-target filtering consults `has_protection_from` with the source id. Tested. PASS
- Damage arm: blocking a Walking Corpse, the Bramble takes 0 while dealing its full 3 (protection is not mutual). NEW test this audit.
- Zombie tokens count as Zombies for the protection (accessor union). PASS

### Test coverage
- Defender can't attack / can block (uses Grave Bramble itself): `mtg-engine/tests/keywords.rs` `defender_cannot_attack`, `defender_can_block`
- Can block Zombies: `ability_target_protection.rs` `bug_protection_incorrectly_prevents_blocking_zombies`
- Untargetable by a Zombie source: `ability_target_protection.rs` `bug_protection_doesnt_prevent_zombie_source_targeting`
- Damage prevented while blocking: `ability_target_protection.rs` `grave_bramble_blocks_a_zombie_and_takes_no_damage` (NEW)
- No rulings on Scryfall for this card.

Mutation checks:
- Emptying `continuous_effects` (with a sink so it compiles): the new damage test FAILS. Bites. (First attempt dropped the import and didn't compile — discarded.)
- Emptying `keywords` (Defender): `defender_cannot_attack` FAILS. Bites.
