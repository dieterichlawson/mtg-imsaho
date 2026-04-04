# Audit: Markov Patrician

## Official Oracle
- **Name:** Markov Patrician
- **Cost:** {2}{B}
- **Type:** Creature — Vampire
- **Oracle:** Lifelink
- **P/T:** 3/1

## Implementation: `mtg-engine/src/cards/markov_patrician.rs`
- **Name:** Markov Patrician -- CORRECT
- **Cost:** {2}{B} -- CORRECT
- **Type:** Creature -- CORRECT
- **Subtypes:** Vampire -- CORRECT
- **Oracle:** Lifelink -- CORRECT
- **P/T:** 3/1 -- CORRECT
- **Keywords:** Lifelink -- CORRECT

## Verdict
**PASS** -- No issues found.

## Audit - 2026-04-02

### Oracle Reference
- **Name:** Markov Patrician
- **Cost:** {2}{B}
- **Type:** Creature — Vampire
- **P/T:** 3/1
- **Oracle Text:** Lifelink (Damage dealt by this creature also causes you to gain that much life.)

### Card Data Checks
- [x] Name: "Markov Patrician" — correct
- [x] Cost: {2}{B} — correct
- [x] Types: Creature — correct
- [x] Subtypes: Vampire — correct
- [x] P/T: 3/1 — correct
- [x] Keywords: Lifelink — correct
- [x] Oracle text: "Lifelink" — correct

### Behavior Checks
- [x] No abilities beyond the keyword — correct (vanilla + keyword creature)
- [x] Lifelink handled by engine keyword system — correct

### Result: PASS

## Audit — 2026-04-03 07:14
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/107/markov-patrician)
**Oracle text**: Lifelink (Damage dealt by this creature also causes you to gain that much life.)
**Type line**: Creature — Vampire
**Status**: PASS

### Code issues
None. All card data fields match oracle text exactly:
- Name: "Markov Patrician" -- correct
- Mana cost: {2}{B} (Generic(2), Colored(Black)) -- correct
- Type: Creature -- correct
- Subtype: Vampire -- correct
- P/T: 3/1 -- correct
- Keywords: Lifelink -- correct
- No supertypes, no additional abilities -- correct for a vanilla creature with keyword

### Tricky interactions checked (min 3)
1. **Lifelink on unblocked combat damage to player**: Verified in `deal_damage_to_player` (combat.rs:538-549). When source has Keyword::Lifelink, controller gains life equal to damage dealt. Tested in `lifelink_gains_life_on_combat_damage` (keywords.rs:300-314).
2. **Lifelink on blocked combat damage to creature**: Verified in `deal_damage_to_creature` (combat.rs:473-484). Controller gains life even when damage is dealt to a blocking creature, not just to a player. Tested in `lifelink_gains_life_from_creature_damage` (keywords.rs:317-332).
3. **Vampire subtype interactions**: Markov Patrician is used in tests as a Vampire for Vampiric Fury (innistrad_cards.rs:389-396), Victim of Night targeting restrictions (tier2_spells.rs:134-145), and Wooden Stake equipment interaction (tier9_equipment.rs:341-342). All verified passing.
4. **Keyword resolution via has_keyword**: Engine checks static card keywords, continuous effect grants, conditional keywords, and until-end-of-turn grants (state.rs:987-1043). Keyword removal also handled. Lifelink is correctly in the Keyword enum (types.rs:295).

### Test coverage
- `markov_patrician_has_lifelink` (innistrad_cards.rs:96-103): Verifies keyword and P/T stats.
- `lifelink_gains_life_on_combat_damage` (keywords.rs:300-314): Verifies life gain on unblocked attack (P0 life 20->23, P1 life 20->17).
- `lifelink_gains_life_from_creature_damage` (keywords.rs:317-332): Verifies life gain when blocked (P0 life 20->23).
- Also used as a Vampire test subject in `vampiric_fury_buffs_vampires`, `victim_of_night_cant_target_vampire`, and Wooden Stake equipment tests.
