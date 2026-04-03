# Audit: Invisible Stalker

## Oracle (Official)
- **Name:** Invisible Stalker
- **Cost:** {1}{U}
- **Type:** Creature — Human Rogue
- **Oracle:** Hexproof (This creature can't be the target of spells or abilities your opponents control.) Invisible Stalker can't be blocked.
- **P/T:** 1/1

## Implementation
- Name: "Invisible Stalker" -- CORRECT
- Cost: {1}{U} -- CORRECT
- Type: Creature -- CORRECT
- Subtypes: ["Human", "Rogue"] -- CORRECT
- P/T: 1/1 -- CORRECT
- Keywords: [Hexproof] -- CORRECT
- Continuous effects: CantBeBlocked { scope: OnSelf } -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit: Invisible Stalker
**Date:** 2026-04-02

### Oracle Text (Scryfall)
- **Type:** Creature -- Human Rogue
- **Cost:** {1}{U}
- **P/T:** 1/1
- **Oracle:** Hexproof (This creature can't be the target of spells or abilities your opponents control.) / This creature can't be blocked.

### Card Data
- **Name:** Invisible Stalker -- PASS
- **Cost:** {1}{U} -- PASS
- **Types:** Creature -- PASS
- **Subtypes:** Human, Rogue -- PASS
- **P/T:** 1/1 -- PASS

### Oracle Text Match
- Code oracle_text says "Invisible Stalker can't be blocked" vs oracle "This creature can't be blocked." Cosmetic only, no functional difference.
- PASS (minor wording variance)

### Behavior Audit
- **Hexproof:** Listed in keywords vec. -- PASS
- **Can't be blocked:** ContinuousEffect::CantBeBlocked with scope OnSelf. -- PASS

### Result: PASS

## Audit — 2026-04-03 07:04
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/60/invisible-stalker, cached 2026-04-01)
**Oracle text**: Hexproof (This creature can't be the target of spells or abilities your opponents control.)
This creature can't be blocked.
**Type line**: Creature — Human Rogue

**Status**: PASS

### Code issues
None. All card data fields are correct. Minor cosmetic oracle text difference: implementation stores `"Invisible Stalker can't be blocked."` vs oracle `"This creature can't be blocked."` — display-only string, no functional impact. Behavior is driven by `Keyword::Hexproof` and `ContinuousEffect::CantBeBlocked { scope: EffectScope::OnSelf }`, both of which are correctly implemented.

### Tricky interactions checked (min 3)
1. **Hexproof vs opponent targeting**: `engine::can_be_targeted()` checks `has_keyword(Hexproof)` and blocks opponent targeting while still allowing the controller to target their own hexproof creature. Verified in test `hexproof_prevents_opponent_targeting` which uses Invisible Stalker on both sides.
2. **Can't be blocked enforcement**: `combat::can_block_attacker()` checks `has_continuous_effect(CantBeBlocked)` on the attacker and returns `false`, preventing any blocker assignment regardless of blocker abilities (flying, reach, etc.). A 5/5 vanilla creature cannot block it in the test.
3. **Hexproof does not prevent board wipes / untargeted effects**: Hexproof is only checked in `can_be_targeted()` which is only called for targeted spells/abilities. Non-targeted effects (e.g., "destroy all creatures") bypass this check entirely, which is correct MTG rules behavior.

### Test coverage
- `invisible_stalker_has_hexproof` (innistrad_cards.rs:115) — verifies card data includes `Keyword::Hexproof`
- `invisible_stalker_unblockable` (card_mechanics.rs:455) — verifies no creature can block Invisible Stalker
- `hexproof_prevents_opponent_targeting` (keywords.rs:166) — verifies opponent can't target with Lightning Bolt, but controller can target their own hexproof creature
