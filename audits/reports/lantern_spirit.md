# Audit: Lantern Spirit

## Oracle (Official)
- **Name:** Lantern Spirit
- **Cost:** {2}{U}
- **Type:** Creature — Spirit
- **Oracle:** Flying. {U}: Return Lantern Spirit to its owner's hand.
- **P/T:** 2/1

## Implementation
- Name: "Lantern Spirit" -- CORRECT
- Cost: {2}{U} -- CORRECT
- Type: Creature -- CORRECT
- Subtypes: ["Spirit"] -- CORRECT
- P/T: 2/1 -- CORRECT
- Keywords: [Flying] -- CORRECT
- Activated ability: {U}, returns self to hand -- CORRECT
- No tap required for ability -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit - 2026-04-02

### Oracle Reference
- **Name:** Lantern Spirit
- **Cost:** {2}{U}
- **Type:** Creature — Spirit
- **P/T:** 2/1
- **Oracle Text:** Flying / {U}: Return this creature to its owner's hand.

### Card Data Checks
- [x] Name: "Lantern Spirit" — correct
- [x] Cost: {2}{U} — correct
- [x] Types: Creature — correct
- [x] Subtypes: Spirit — correct
- [x] P/T: 2/1 — correct
- [x] Keywords: Flying — correct
- [ ] Oracle text: minor mismatch (cosmetic)
  - **Oracle:** `"{U}: Return this creature to its owner's hand."`
  - **Implementation:** `"{U}: Return Lantern Spirit to its owner's hand."`
  - Note: Scryfall uses modern "this creature" templating; implementation uses card name. Functionally equivalent.

### Behavior Checks
- [x] Flying keyword granted — correct
- [x] Activated ability costs {U} — correct
- [x] Ability only available on the battlefield — correct
- [x] Ability returns self to owner's hand via `state.move_object(object_id, Zone::Hand)` — correct
- [x] Does not require tap — correct

### Result: PASS

## Audit — 2026-04-03 07:08

**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/62/lantern-spirit), cached 2026-04-01
**Oracle text**: Flying\n{U}: Return this creature to its owner's hand.
**Type line**: Creature — Spirit

### Card data checks
- Name: "Lantern Spirit" — correct
- Cost: {2}{U} (Generic(2), Colored(Blue)) — correct
- Types: Creature — correct
- Subtypes: ["Spirit"] — correct
- P/T: 2/1 — correct
- Keywords: [Flying] — correct
- Oracle text: cosmetic mismatch (not a functional issue)
  - Scryfall: `"{U}: Return this creature to its owner's hand."`
  - Implementation: `"{U}: Return Lantern Spirit to its owner's hand."`
  - Per MTG rules, a card referring to itself by name means "this object." Functionally equivalent.

### Code issues
None.

### Behavior checks
- Activated ability costs {U}, no tap required — correct
- Ability only available on battlefield (zone check at line 33) — correct
- Returns self to hand via `state.move_object(object_id, Zone::Hand)` — correct
- `once_per_turn: false` — correct (no restriction in oracle text)
- `sorcery_speed_only: false` — correct (activatable at instant speed)
- Only controller can activate (engine filters `objects_in_zone(Battlefield, player)`) — correct per ruling

### Tricky interactions checked (min 3)
1. **Owner vs. controller on bounce**: `move_object` sets `zone = Hand` but preserves `owner`. The engine's `objects_in_zone(Hand, player)` filters by `owner` (state.rs line 603). So if an opponent steals Lantern Spirit and activates its ability, it returns to the original owner's hand — correct per oracle "owner's hand."
2. **Ability only on battlefield**: `activated_abilities` checks `obj.zone == Zone::Battlefield` and returns an empty vec otherwise. If Lantern Spirit is in hand/graveyard, no ability is offered — correct.
3. **Zone change state reset**: `move_object` clears tapped, summoning_sick, damage_marked, counters, attached_to, etc. when leaving battlefield. If Lantern Spirit had damage or was tapped, returning to hand resets all of that — correct per MTG zone-change rules.

### Test coverage
- `lantern_spirit_has_correct_stats` — verifies P/T, Flying keyword, Spirit subtype
- `lantern_spirit_returns_to_hand` — verifies ability activation moves creature from battlefield to hand
- Both tests PASS

**Status**: PASS
