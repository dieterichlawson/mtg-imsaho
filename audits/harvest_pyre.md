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

## Audit — 2026-04-01 14:38

**Oracle text source**: Scryfall card page via WebSearch (https://scryfall.com/card/isd/146/harvest-pyre)
**Oracle text**: As an additional cost to cast this spell, exile X cards from your graveyard. Harvest Pyre deals X damage to target creature.
**Type line**: Instant
**Mana cost**: {1}{R}
**Status**: ISSUE

Findings:
1. **Name**: "Harvest Pyre" -- correct.
2. **Mana cost {1}{R}**: Correct (`Generic(1), Red`).
3. **Type (Instant)**: Correct.
4. **Target requirement (Creature)**: Correct per oracle (`target creature`).
5. **Spell cleanup**: Uses `state.move_spell_after_resolve(object_id)` (line 77). Correct.
6. **NonCombatDamageDealt**: Emitted at line 64. Correct.
7. **`damaged_by` tracking**: Present at line 62 (`obj.damaged_by.push(object_id)`). Correct.
8. **Only exiles own graveyard**: Filters `o.owner == controller` (line 45). Correct.
9. **Excludes self from exile**: Filters `o.id != object_id` (line 45). Correct (Harvest Pyre itself may be in graveyard zone during resolution tracking, but this is defensive).
10. **Tests**: No dedicated test file found. Previously noted in tier8_cards.rs.

Issues:
1. **Oracle text mismatch** (file: `mtg-engine/src/cards/harvest_pyre.rs`, line 25):
   - Oracle text says: `As an additional cost to cast this spell, exile X cards from your graveyard. Harvest Pyre deals X damage to target creature.`
   - Code oracle_text says: `As an additional cost to cast Harvest Pyre, exile any number of cards from your graveyard. Harvest Pyre deals damage to target creature equal to the number of cards exiled this way.`
   - The current Scryfall oracle uses "X cards" and "X damage" templating. The code uses older wording ("any number of cards" / "equal to the number of cards exiled this way").

2. **Additional cost executed at resolve time instead of cast time** (file: `mtg-engine/src/cards/harvest_pyre.rs`, lines 44-52):
   - Oracle text says: `As an additional cost to cast this spell, exile X cards from your graveyard.`
   - Code does: Exiles cards in `on_resolve` (lines 44-52), not at cast time. The `additional_cost` field is `None` (line 29).
   - Per rules, additional costs are paid during casting. If the spell is countered, the cards should still be exiled (cost already paid), but currently they would not be.

3. **Always exiles all cards instead of player choosing X** (file: `mtg-engine/src/cards/harvest_pyre.rs`, lines 44-47):
   - Oracle text says: `exile X cards from your graveyard`
   - Code does: Exiles all graveyard cards (`state.objects.values().filter(|o| o.zone == Zone::Graveyard && o.owner == controller && o.id != object_id)`).
   - The player should choose how many cards (X) to exile. Code comment on line 43 acknowledges this limitation.

4. **Zero damage still targets but deals no damage** (file: `mtg-engine/src/cards/harvest_pyre.rs`, lines 54-75):
   - When count is 0 (empty graveyard), the code skips damage entirely (goes to the `else` branch at line 73). With the X templating, X=0 is valid and should still resolve (dealing 0 damage to the target). This is a minor edge case difference.

## Audit — 2026-04-01 16:00

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: As an additional cost to cast this spell, exile X cards from your graveyard.
Harvest Pyre deals X damage to target creature.
**Type line**: Instant
**Status**: ISSUE

### Code issues
1. **Oracle text field uses outdated wording** (`mtg-engine/src/cards/isd/harvest_pyre.rs` line 25):
   - Oracle text says: `As an additional cost to cast this spell, exile X cards from your graveyard. Harvest Pyre deals X damage to target creature.`
   - Code oracle_text says: `As an additional cost to cast Harvest Pyre, exile any number of cards from your graveyard. Harvest Pyre deals damage to target creature equal to the number of cards exiled this way.`
   - The current oracle uses "exile X cards" and "deals X damage" templating. The code uses older wording ("any number of cards" / "equal to the number of cards exiled this way").

2. **Additional cost executed at resolve time instead of cast time** (`mtg-engine/src/cards/isd/harvest_pyre.rs` lines 44-52):
   - Oracle text says: `As an additional cost to cast this spell, exile X cards from your graveyard.`
   - Code does: Exiles cards in `on_resolve` (lines 44-52). The `additional_cost` field is `None` (line 29).
   - Per rules, additional costs are paid during casting. If the spell is countered, the cards should already be exiled (cost paid), but currently they would not be.

3. **Always exiles all cards instead of player choosing X** (`mtg-engine/src/cards/isd/harvest_pyre.rs` lines 44-47):
   - Oracle text says: `exile X cards from your graveyard`
   - Code does: `state.objects.values().filter(|o| o.zone == Zone::Graveyard && o.owner == controller && o.id != object_id)` -- exiles ALL graveyard cards.
   - The player should choose how many cards (X) to exile. Code comment on line 41-43 acknowledges this limitation ("the engine lacks a 'choose a number' UI").

Card data verified: mana cost {1}{R} correct, card_types (Instant) correct, target_requirement (Creature) correct per oracle ("target creature"). Uses `move_spell_after_resolve`: correct. Emits `NonCombatDamageDealt`: correct for spell damage. Tracks `damaged_by` (line 62): correct. Only exiles own graveyard (line 45 `o.owner == controller`): correct. Excludes self from exile (line 45 `o.id != object_id`): correct.

### Tricky interactions checked
- NonCombatDamageDealt event (not CombatDamageDealt): pass
- damaged_by tracking for death trigger interactions: pass
- Only exiles own graveyard: pass
- Spell uses move_spell_after_resolve: pass
- Countering the spell should still exile cards (additional cost): ISSUE (cost paid at resolution)
- Player choice of X: ISSUE (auto-exiles all)

### Test coverage
- Deals damage equal to exiled count: `mtg-engine/tests/tier8_cards.rs:527` (harvest_pyre_deals_damage_equal_to_exiled_count)
- Empty graveyard deals no damage: `mtg-engine/tests/tier8_cards.rs:554` (harvest_pyre_empty_graveyard_deals_no_damage)
- Only exiles own graveyard: `mtg-engine/tests/tier8_cards.rs:568` (harvest_pyre_only_exiles_own_graveyard)
- Fizzle (target leaves battlefield): NOT TESTED
- Spell countered (cards should already be exiled): NOT TESTED
- Player chooses X (subset of graveyard): NOT TESTED (known limitation)

## Audit — 2026-04-01 17:00

**Oracle text source**: Scryfall API (cached)
**Oracle text**: As an additional cost to cast this spell, exile X cards from your graveyard.
Harvest Pyre deals X damage to target creature.
**Type line**: Instant
**Status**: ISSUE

### Code issues
1. **Always exiles all graveyard cards instead of player choosing X** (`mtg-engine/src/cards/isd/harvest_pyre.rs` line 29, `mtg-engine/src/engine.rs` lines 1344-1359):
   - Oracle text says: `exile X cards from your graveyard`
   - Code does: `additional_cost: Some(AdditionalCost::ExileAllFromGraveyard)` which in engine.rs exiles ALL cards from the caster's graveyard (`state.objects.values().filter(|o| o.zone == Zone::Graveyard && o.owner == player && o.id != *object_id)`).
   - The player should choose how many cards (X) to exile. This matters strategically (e.g., keeping flashback cards, Gnaw to the Bone fuel, Boneyard Wurm power).

Previously flagged issues that have been fixed:
- Oracle text field: now uses current "exile X cards" / "deals X damage" wording (was "exile any number of cards" / "deals damage equal to the number of cards exiled this way").
- Additional cost timing: now uses `additional_cost: Some(AdditionalCost::ExileAllFromGraveyard)` which is handled at cast time in engine.rs (lines 1338-1360), not at resolve time. If the spell is countered, the exile has already happened. Fixed.
- damaged_by tracking: line 50 has `obj.damaged_by.push(object_id)`. Present and correct.

Card data verified correct:
- Mana cost: {1}{R}
- Card types: Instant
- oracle_text: matches Scryfall
- Target requirement: Creature -- correct per oracle "target creature"
- additional_cost: ExileAllFromGraveyard (set, not None)
- on_resolve: reads exile_count from card_state, deals that much damage, emits NonCombatDamageDealt, tracks damaged_by, calls move_spell_after_resolve
- Only exiles own graveyard: engine.rs line 1346 filters `o.owner == player`
- Excludes self from exile: engine.rs line 1346 filters `o.id != *object_id`

### Tricky interactions checked
- Additional cost paid at cast time (survives counterspell): pass (fixed)
- NonCombatDamageDealt event emitted: pass
- damaged_by tracking: pass
- Only exiles own graveyard: pass
- Spell cleanup via move_spell_after_resolve: pass
- Player choice of X (exile subset): ISSUE (auto-exiles all)

### Test coverage
- Deals damage equal to exiled count: `mtg-engine/tests/tier8_cards.rs:527` (harvest_pyre_deals_damage_equal_to_exiled_count)
- Empty graveyard deals no damage: `mtg-engine/tests/tier8_cards.rs:554` (harvest_pyre_empty_graveyard_deals_no_damage)
- Only exiles own graveyard: `mtg-engine/tests/tier8_cards.rs:568` (harvest_pyre_only_exiles_own_graveyard)
- Fizzle (target leaves battlefield): NOT TESTED
- Spell countered (cards should already be exiled): NOT TESTED
- Player chooses X (subset of graveyard): NOT TESTED (known limitation)
