## Audit — 2026-08-27 (Tier D — card data and static abilities only)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/13/elite-inquisitor?utm_source=api
**Type line**: `Creature — Human Soldier` — {W}{W}, 2/2
**Oracle text**:
```
First strike, vigilance
Protection from Vampires, from Werewolves, and from Zombies
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


## Audit — 2026-08-28 20:01

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: First strike, vigilance
Protection from Vampires, from Werewolves, and from Zombies
**Type line**: Creature — Human Soldier
**P/T**: 2/2
**Status**: PASS

### Code issues
No issues found in the card. `mtg-engine/src/cards/isd/elite_inquisitor.rs` matches: {W}{W}, Human Soldier, 2/2, FirstStrike + Vigilance, three `ProtectionFromSubtype` effects (Vampire, Werewolf, Zombie).

One test vacuity found and fixed: `elite_inquisitor_protection_prevents_damage` attacked with Markov Patrician (a 3/1) — the Inquisitor's FIRST STRIKE killed it before it swung, so the test passed with the Vampire protection deleted. Rewritten as a four-row table where every attacker has toughness 3+, with a Spider control row whose damage must land.

Audit-process note: I initially "knew" werewolf DFC front faces to be plain Humans and wrote a test expecting front-face damage to land — the fetched type line says the front face of Villagers of Estwald is "Human Werewolf" (Shadows over Innistrad update), so protection applies to BOTH faces and the code (front subtypes ["Human", "Werewolf"]) is right. The wrong-premise test was discarded; this is exactly why the procedure bans training data.

### Tricky interactions checked
- Protection is DEBT here: Damage (combat, tested per-subtype), Blocking (tested), Targeting (engine-wide via `has_protection_from` in `is_target_legal`, incl. abilities with a typed source — `ability_target_protection.rs`). No Vampire/Werewolf/Zombie auras or equipment exist in the pool for the attachment arm. PASS
- Protection is one-directional: the Inquisitor may still BLOCK Zombies (`ability_target_protection.rs::bug_protection_incorrectly_prevents_blocking_zombies`). PASS
- `has_protection_from` reads `subtypes_of` (active face) — a transformed werewolf is still a Werewolf; both faces tested. PASS
- First strike + protection: the attacker's swing is prevented while the Inquisitor's own first-strike damage still lands (visible in the table rows: the 3-toughness attackers survive with 2 marked). PASS

### Test coverage
- Damage prevention per subtype + control row: `mtg-engine/tests/cards_spells_and_enchantments.rs` `elite_inquisitor_protection_prevents_damage` (rewritten this audit)
- Can't be blocked by Zombies: `cards_spells_and_enchantments.rs` `elite_inquisitor_cant_be_blocked_by_zombies`
- Targeting arm (engine-generic): `ability_target_protection.rs` (Grimgrin vs Grave Bramble, Avacynian Priest vs granted protection)
- No rulings on Scryfall for this card.

Mutation checks:
- Deleting the Vampire protection: old test PASSED (vacuous, recorded above); rewritten table FAILS on the Bloodline Keeper row. Bites now.
- Deleting the Werewolf protection: table FAILS on the untransformed Villagers row. Bites.
