# Audit: Garruk Relentless / Garruk, the Veil-Cursed

## Oracle Reference (Scryfall)
**Front Face: Garruk Relentless**
- Cost: {3}{G}
- Type: Legendary Planeswalker -- Garruk
- Loyalty: 3
- Oracle: "When Garruk Relentless has two or fewer loyalty counters on him, transform him.
  0: Garruk Relentless deals 3 damage to target creature. That creature deals damage equal to its power to him.
  0: Create a 2/2 green Wolf creature token."

**Back Face: Garruk, the Veil-Cursed**
- Type: Legendary Planeswalker -- Garruk
- Color: Black, Green
- Oracle: "+1: Create a 1/1 black Wolf creature token with deathtouch.
  -1: Sacrifice a creature. If you do, search your library for a creature card, reveal it, put it into your hand, then shuffle.
  -3: Creatures you control gain trample and get +X/+X until end of turn, where X is the number of creature cards in your graveyard."

## Implementation: garruk_relentless.rs

## Issues Found

1. **ISSUE: Back face not fully implemented** - The comment says "Simplified: Front face only. Back face (Garruk, the Veil-Cursed) is not implemented." The back face has 3 loyalty abilities (+1 wolf with deathtouch, -1 sacrifice+tutor, -3 overrun). After transform, these abilities are unavailable. This is a significant gameplay simplification.

2. **ISSUE: Transform trigger is wrong type** - Oracle says "When Garruk Relentless has two or fewer loyalty counters on him, transform him" - this is a state-triggered ability, not something checked only after loyalty abilities activate. The implementation only checks after on_loyalty_ability, so it won't trigger if Garruk takes damage from combat or other sources.

3. **ISSUE: Missing NonCombatDamageDealt event for creature-to-planeswalker damage** - When the targeted creature deals damage back to Garruk, the implementation just removes loyalty counters directly (line 90-94) without emitting a NonCombatDamageDealt event.

4. **ISSUE: Front face oracle text says "to him" but code oracle says "to him"** - Matches. OK.

5. **MINOR: Wolf tokens from front face** - Front face creates 2/2 green Wolf tokens. This matches oracle. OK.

## Verdict: ISSUES FOUND (3 issues)

## Audit — 2026-04-01 08:20

**Scryfall Oracle text (front)**: When Garruk Relentless has two or fewer loyalty counters on him, transform him. 0: Garruk Relentless deals 3 damage to target creature. That creature deals damage equal to its power to him. 0: Create a 2/2 green Wolf creature token.
**Scryfall Oracle text (back)**: +1: Create a 1/1 black Wolf creature token with deathtouch. −1: Sacrifice a creature. If you do, search your library for a creature card, reveal it, put it into your hand, then shuffle your library. −3: Creatures you control gain trample and get +X/+X until end of turn, where X is the number of creature cards in your graveyard.
**Scryfall type line**: Legendary Planeswalker — Garruk // Legendary Planeswalker — Garruk
**Status**: PASS (with accepted simplifications)

Previous issue #1 (back face not implemented) is now FIXED. All 3 back face loyalty abilities are implemented:
- +1: Creates 1/1 black Wolf with deathtouch (ability_index 10)
- -1: Sacrifices weakest creature, searches library for creature card (ability_index 11)
- -3: Gives all controlled creatures +X/+X and trample until end of turn (ability_index 12)

The loyalty_abilities trait method now takes state and object_id parameters to support returning different abilities based on is_transformed.

Previous issue #2 (transform trigger type) remains an accepted simplification — transform is checked after loyalty ability activation only.
Previous issue #3 (missing damage event for creature-to-planeswalker) remains an accepted simplification.

Test coverage: 6 tests covering front face wolf creation, transform condition, back face deathtouch wolf, sacrifice-to-tutor, overrun effect, and loyalty abilities list verification.

## Audit — 2026-04-01 09:00

**Scryfall Oracle text (front)**: When Garruk Relentless has two or fewer loyalty counters on him, transform him. 0: Garruk Relentless deals 3 damage to target creature. That creature deals damage equal to its power to him. 0: Create a 2/2 green Wolf creature token.
**Scryfall Oracle text (back)**: +1: Create a 1/1 black Wolf creature token with deathtouch. -1: Sacrifice a creature. If you do, search your library for a creature card, reveal it, put it into your hand, then shuffle. -3: Creatures you control gain trample and get +X/+X until end of turn, where X is the number of creature cards in your graveyard.
**Scryfall type line**: Legendary Planeswalker — Garruk // Legendary Planeswalker — Garruk
**Status**: ISSUE

Findings:
- Mana cost {3}{G}: correct.
- Types: Legendary Planeswalker, subtypes Garruk: correct.
- Starting loyalty 3: correct.
- Front face abilities:
  - 0: Deals 3 damage to target creature, creature deals power back: correct. Uses NonCombatDamageDealt event for the 3 damage: correct.
  - 0: Create 2/2 green Wolf token with subtypes ["Wolf"]: correct.
- Back face abilities:
  - +1: Create 1/1 black Wolf with deathtouch, subtypes ["Wolf"]: correct.
  - -1: Sacrifice creature, search library for creature card: correct.
  - -3: Creatures get +X/+X and trample where X = creature cards in graveyard: correct.
- Transform check (loyalty <= 2 triggers transform): implemented after every loyalty ability activation.
- ISSUE 1 (carried forward): Transform is a state-triggered ability per oracle, but implementation only checks after on_loyalty_ability. Won't trigger if Garruk loses loyalty from combat damage or other non-loyalty-ability sources.
- ISSUE 2 (carried forward): When target creature deals damage back to Garruk (ability 0), no NonCombatDamageDealt event is emitted for the creature-to-planeswalker damage. Only loyalty counters are removed directly.
- ISSUE 3: Wolf tokens from both faces have correct subtypes ["Wolf"] passed to create_token_with_subtypes: correct. No missing token subtypes.
- Anti-pattern check: on_resolve uses move_object to battlefield (correct for planeswalker permanent). No spell-to-graveyard anti-pattern.
- No CombatDamageDealt misuse for non-combat damage: correct (uses NonCombatDamageDealt).
- triggered_abilities vec is empty despite having a state-triggered transform ability. This is technically a missing declaration, though the transform check is handled inline in on_loyalty_ability.
- Tests found in tier15_cards.rs.
