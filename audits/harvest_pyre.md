# Audit: Harvest Pyre

## Oracle Reference (Scryfall)
- Cost: {1}{R}
- Type: Instant
- Oracle: "As an additional cost to cast Harvest Pyre, exile any number of cards from your graveyard.
  Harvest Pyre deals damage to target creature equal to the number of cards exiled this way."

## Implementation: harvest_pyre.rs

## Issues Found

1. **ISSUE: Always exiles ALL graveyard cards instead of player choosing** - Oracle says "exile any number of cards from your graveyard." The implementation exiles all cards (line 44-47, comment on line 43 acknowledges this: "we exile all cards for maximum damage"). The player should choose how many to exile. This matters strategically (e.g., keeping flashback cards in graveyard, keeping cards for Gnaw to the Bone).

2. **ISSUE: Missing damaged_by tracking** - Line 59-62 marks damage on the creature (`obj.damage_marked += count`) and emits NonCombatDamageDealt, but does NOT push to `obj.damaged_by`. This means effects that check what dealt damage to a creature (e.g., for death triggers like Falkenrath Noble tracking) won't know Harvest Pyre was the source.

3. **ISSUE: Additional cost not enforced at cast time** - The additional_cost field is None (line 29). The exile happens during resolution (on_resolve), not as a cost to cast. Per rules, additional costs are paid during casting, which means the cards should be exiled before the spell resolves. If the spell is countered, the cards should still be exiled.

Otherwise correct: cost ({1}{R}), type (Instant), target requirement (Creature), oracle text.

## Verdict: ISSUES FOUND (3 issues)

## Audit -- 2026-04-01 09:00

**Scryfall Oracle text**: As an additional cost to cast this spell, exile X cards from your graveyard. Harvest Pyre deals X damage to target creature.
**Scryfall type line**: Instant
**Status**: ISSUE

Findings:
1. **Mana cost {1}{R}**: Correct.
2. **Type (Instant)**: Correct.
3. **Oracle text in code**: Says "exile any number of cards from your graveyard" and "deals damage equal to the number of cards exiled this way." Scryfall's current oracle uses "exile X cards" and "deals X damage to target creature." The code's oracle text is slightly different wording but functionally equivalent.
4. **Target requirement (Creature)**: Correct per Scryfall ("target creature").
5. **Spell cleanup**: Uses `state.move_spell_after_resolve(object_id)` (line 77) -- correct, not the anti-pattern.
6. **Damage event**: Uses `NonCombatDamageDealt` -- correct for a spell dealing damage.
7. **`damaged_by` tracking**: Code pushes to `obj.damaged_by` at line 62. Correct.
8. **Additional cost at resolve time**: The `additional_cost` field is `None` (line 29), and the exile happens in `on_resolve` (lines 44-52). Per rules, additional costs should be paid at cast time. If the spell is countered, the cards should still be exiled but currently would not be. This is a known issue from previous audit.
9. **Always exiles all cards**: Code exiles all graveyard cards (line 44-47) rather than letting the player choose X. The comment acknowledges this limitation. This means the player cannot strategically keep cards in graveyard.
10. **Tests**: Found in `mtg-engine/tests/tier8_cards.rs`.

Issues carried forward:
1. Additional cost executed at resolve time instead of cast time (anti-pattern).
2. Player cannot choose how many cards to exile (always exiles all).

## Audit — 2026-04-01 14:13

**Oracle text source**: Scryfall card page via WebSearch (https://scryfall.com/card/isd/146/harvest-pyre)
**Oracle text**: As an additional cost to cast this spell, exile X cards from your graveyard. Harvest Pyre deals X damage to target creature.
**Type line**: Instant
**Mana cost**: {1}{R}
**Status**: ISSUE

Findings:
1. **Name**: "Harvest Pyre" -- correct.
2. **Mana cost {1}{R}**: Correct (Generic(1), Red).
3. **Type (Instant)**: Correct.
4. **Oracle text in code**: Says "exile any number of cards" and "deals damage equal to the number of cards exiled this way." Scryfall's current oracle uses "exile X cards" and "deals X damage to target creature." The code's oracle text uses older templating but is functionally equivalent.
5. **Target requirement (Creature)**: Correct per Scryfall ("target creature").
6. **Spell cleanup**: Uses `state.move_spell_after_resolve(object_id)` (line 77) -- correct.
7. **NonCombatDamageDealt**: Emitted (line 64). Correct for spell damage.
8. **`damaged_by` tracking**: Present at line 62 (`obj.damaged_by.push(object_id)`). Correct.
9. **Additional cost at resolve time**: The `additional_cost` field is `None` (line 29), and the exile happens in `on_resolve` (lines 44-52). Per rules, additional costs are paid at cast time. If the spell is countered, the exiled cards should still be exiled but currently would not be.
10. **Always exiles all cards**: Code exiles all graveyard cards (lines 44-47) instead of letting the player choose X. The comment on line 43 acknowledges this limitation ("the engine lacks a 'choose a number' UI"). This means the player cannot strategically keep cards in graveyard (e.g., flashback cards, Gnaw to the Bone fuel).
11. **Only exiles own graveyard**: Code filters `o.owner == controller` (line 45). Correct per oracle.
12. **Tests**: Found in `mtg-engine/tests/tier8_cards.rs`. Tests cover: dealing damage equal to exiled count, empty graveyard deals no damage, only exiles own graveyard. All assertions correct.

Issues:
1. Additional cost (exile cards) is executed at resolve time instead of cast time.
2. Player cannot choose how many cards to exile -- always exiles all (documented limitation).
