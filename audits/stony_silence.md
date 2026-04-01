# Audit: Stony Silence

## Oracle (Scryfall)
- **Name:** Stony Silence
- **Cost:** {1}{W}
- **Type:** Enchantment
- **Oracle:** Activated abilities of artifacts can't be activated.
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/stony_silence.rs`
- **Name:** Stony Silence ✅
- **Cost:** {1}{W} ✅
- **Type:** Enchantment ✅
- **Oracle text:** matches ✅

### Issue
- **NOT IMPLEMENTED:** The card's static ability (preventing activation of artifact abilities) is not enforced by the engine. The code comments explicitly document this as a known limitation. The card exists for deck building and oracle text purposes only.

## Verdict: ISSUE -- static ability not enforced (documented known limitation)

## Audit -- 2026-04-01 09:00

**Scryfall Oracle text**: Activated abilities of artifacts can't be activated.
**Scryfall type line**: Enchantment
**Status**: ISSUE

Findings:
1. **Mana cost {1}{W}**: Correct.
2. **Type (Enchantment)**: Correct. No subtypes, no supertypes. Correct.
3. **Oracle text**: Matches Scryfall exactly.
4. **Static ability not enforced**: The code comments (lines 7-11) explicitly document that the engine lacks an ability restriction system. The card is registered for deck building/oracle purposes only. The static ability has no effect in game.
5. **No anti-patterns detected**: Card has no on_resolve (enchantments enter battlefield via default), no triggered abilities, no damage. Clean implementation of a stub.
6. **Tests**: Found in `mtg-engine/tests/innistrad_simple_cards.rs`.

Issues:
- Static ability ("Activated abilities of artifacts can't be activated") is not enforced (documented known limitation).

## Audit — 2026-04-01 14:13

**Oracle text source**: Scryfall card page via WebSearch (https://scryfall.com/card/isd/36/stony-silence)
**Oracle text**: Activated abilities of artifacts can't be activated.
**Type line**: Enchantment
**Mana cost**: {1}{W}
**Status**: ISSUE

Findings:
1. **Name**: "Stony Silence" -- correct.
2. **Mana cost {1}{W}**: Correct (Generic(1), White).
3. **Type (Enchantment)**: Correct. No subtypes, no supertypes -- correct.
4. **Oracle text**: Matches Scryfall exactly.
5. **Static ability not enforced**: The code comments (lines 7-11) explicitly document that the engine lacks an ability restriction system. The card is registered for deck building/oracle purposes only. The static ability has zero in-game effect.
6. **No anti-patterns**: No on_resolve (enchantments enter battlefield), no triggered abilities, no damage. Clean stub.
7. **Tests**: Found in `mtg-engine/tests/innistrad_simple_cards.rs`. Test only verifies card data (type, cost). No test for the static ability effect (as expected, since it's not implemented).

Issues:
- Static ability ("Activated abilities of artifacts can't be activated") is not enforced. Documented as known limitation in code comments. Rulings confirm this affects mana abilities and only artifacts on the battlefield.
