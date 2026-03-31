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
