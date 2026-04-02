# Audit: Grimoire of the Dead

## Oracle Reference (Scryfall)
- Cost: {4}
- Type: Legendary Artifact
- Oracle: "{1}, {T}, Discard a card: Put a study counter on Grimoire of the Dead.
  {T}, Remove three study counters from Grimoire of the Dead and sacrifice it: Put all creature cards from all graveyards onto the battlefield under your control. They're black Zombies in addition to their other colors and types."

## Implementation: grimoire_of_the_dead.rs

## Issues Found

1. **ISSUE: Study counters tracked via card_state instead of CounterType** - The implementation uses `card_state.insert("study_counters", ObjectId(...))` (line 116) as a hacky way to store the counter count. This means other systems that interact with counters (e.g., proliferate, counter removal effects) won't see these counters. Should use a proper CounterType.

2. **ISSUE: Discard auto-selects card** - The discard cost should let the player choose which card to discard. The implementation auto-picks the first card in hand (line 100).

3. **ISSUE: Ability 0 shows even when tapped** - The activated_abilities check (line 47) verifies zone == Battlefield but the requires_tap field handles the tap requirement. However, the ability shouldn't appear as available if the artifact is already tapped. Looking more closely, the check is at line 47 which only checks zone, not tapped state. The requires_tap field should handle this at the engine level, so this may be fine depending on how the engine filters.

Otherwise correct: cost ({4}), type (Legendary Artifact), oracle text matches, sacrifice ability correctly removes counters and returns all graveyard creatures as black Zombies.

## Verdict: ISSUES FOUND (2 issues)

## Audit — 2026-04-01 15:09

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: {1}, {T}, Discard a card: Put a study counter on Grimoire of the Dead.
{T}, Remove three study counters from Grimoire of the Dead and sacrifice it: Put all creature cards from all graveyards onto the battlefield under your control. They're black Zombies in addition to their other colors and types.
**Type line**: Legendary Artifact
**Ruling**: [2011-09-22] "Creature cards" includes each card with the type creature, even if it has additional types, such as artifact.
**Status**: ISSUE

### Code issues

1. **ISSUE: Discard auto-selects first card instead of presenting player choice** (line 100)
   - Oracle text says: `Discard a card`
   - Code does: `if let Some(&card_id) = hand.first()` — automatically picks the first card in hand without presenting a choice to the player. Should use `ChooseCardFromHand` (as done by Murder of Crows and Frightful Delusion) to let the player select which card to discard.

2. **ISSUE: Study counters tracked via card_state hack instead of proper counter system** (line 116)
   - Oracle text says: `Put a study counter on Grimoire of the Dead`
   - Code does: `obj.card_state.insert("study_counters".into(), ObjectId((current + 1) as u64))` — stores counter count as an ObjectId in a string-keyed map. This means proliferate, counter removal, and other counter-interacting effects cannot see these counters. Should use the engine's counter system if one exists.

### Tricky interactions checked
- Sacrifice as part of ability cost: PASS — uses `SacrificeCost::SacrificeThis` in ability definition and `crate::destruction::sacrifice` in execution
- All graveyards, not just controller's: PASS — code filters all objects with `zone == Zone::Graveyard` regardless of owner
- Creatures become black Zombies in addition to other types: PASS — adds "Zombie" subtype and Black color without removing existing ones
- Creatures enter under your control: PASS — sets `obj.controller = controller`
- Legendary supertype: PASS — `supertypes: vec![Supertype::Legendary]`
- Oracle text field: PASS — matches oracle exactly
- Mana cost: PASS — `{4}` matches

### Test coverage
- Study counter accumulation: `tier15_cards.rs:grimoire_accumulates_study_counters`
- Reanimate all graveyard creatures: `tier15_cards.rs:grimoire_reanimates_all_graveyard_creatures`
- Creatures become Zombies: `tier15_cards.rs:grimoire_reanimates_all_graveyard_creatures` (checks Zombie subtype)
- Player chooses which card to discard: NOT TESTED (auto-selects first card)
- Tapping requirement: NOT TESTED (requires_tap field handles this at engine level)
- Creature cards include artifact creatures (ruling): NOT TESTED

## Audit — 2026-04-01 18:00

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/226/grimoire-of-the-dead
**Oracle text**: {1}, {T}, Discard a card: Put a study counter on Grimoire of the Dead.
{T}, Remove three study counters from Grimoire of the Dead and sacrifice it: Put all creature cards from all graveyards onto the battlefield under your control. They're black Zombies in addition to their other colors and types.
**Mana cost**: {4}
**Type line**: Legendary Artifact
**Rulings**: [2011-09-22] "Creature cards" includes each card with the type creature, even if it has additional types, such as artifact.
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Study counter removal as part of ability 1 cost: PASS — Grimoire is sacrificed as part of the same cost, so counter removal is moot. The counter check (>= 3) gates ability availability correctly.
- Discard as cost for ability 0: PASS — Engine resolves activated abilities immediately (no stack), so handling the discard inside on_activate_ability is functionally equivalent to paying it as a cost. Consistent with other engine cards.
- Grimoire not reanimating itself: PASS — Grimoire is an artifact (not a creature), so it would not match the creature filter. Code also has an explicit `o.id != object_id` exclusion.
- "In addition to" color/type: PASS — Code adds Zombie subtype and black color without removing existing subtypes or colors.
- "All creature cards from all graveyards": PASS — Code iterates all objects in graveyard zone regardless of owner, filtering for creature type. Includes artifact creatures per ruling.
- Creature card filtering includes artifact creatures (ruling 2011-09-22): PASS — Code checks `o.card_types.contains(&CardType::Creature)` in addition to `o.power.is_some()`.
- Creatures enter under controller's control: PASS — Code sets `obj.controller = controller` for each reanimated creature.
- Ability 1 gating on exactly 3 study counters: PASS — Code requires `study_counters >= 3`, which is correct (at least 3).
- Ability 0 gating on having cards in hand: PASS — Code checks `has_cards_in_hand` before offering ability 0.
- Single card auto-discard vs. multi-card choice: PASS — Code presents choice when multiple cards, auto-discards when one card. Both paths add study counter correctly.
- Legendary flag: PASS — `is_legendary = true` set in on_resolve, consistent with other legendary permanents.
- LLM card knowledge: PASS — Card is listed in `mtg-player/src/llm.rs` line 138.

### Test coverage
- Discard with choice (multiple cards in hand): `tier15_cards.rs:grimoire_discard_presents_choice_and_adds_study_counter` (line 1452)
- Auto-discard (single card in hand): `tier15_cards.rs:grimoire_single_card_in_hand_auto_discards` (line 1493)
- Accumulating 3 study counters: `tier15_cards.rs:grimoire_accumulates_three_study_counters` (line 1523)
- Reanimating all graveyard creatures as black Zombies: `tier15_cards.rs:grimoire_reanimates_all_graveyard_creatures` (line 1566)
- Ability 1 not available without 3 counters: `tier15_cards.rs:grimoire_ability_1_not_available_without_3_counters` (line 1607)
- Ruling (creature cards includes artifact creatures): NOT TESTED (no artifact creature in graveyard test)
- Reanimating opponent's creatures under your control: tested in `grimoire_reanimates_all_graveyard_creatures` (P1's creature returns under P0 control)
- Fizzle / empty graveyards: NOT TESTED
