# Audit: Past in Flames

## Reference (Scryfall/API)
- **Name:** Past in Flames
- **Mana Cost:** {3}{R}
- **Type:** Sorcery
- **Oracle:** Each instant and sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost. Flashback {4}{R}

## Implementation: `past_in_flames.rs`
- **Name:** Past in Flames -- CORRECT
- **Mana Cost:** {3}{R} -- CORRECT
- **Type:** Sorcery -- CORRECT
- **P/T:** None -- CORRECT
- **Keywords:** None (flashback via flashback_cost field) -- CORRECT
- **Flashback cost:** {4}{R} -- CORRECT
- **oracle_text field:** Matches oracle -- CORRECT
- **Behavior:** On resolve, moves self to graveyard first, then iterates instants/sorceries in controller's graveyard, grants flashback via until_end_of_turn_flashback with cost equal to each card's mana cost -- CORRECT
- **Ruling compliance:** Only affects cards in graveyard at resolution time, correctly implemented since it snapshots state.objects.values() at resolution -- CORRECT

## Verdict: PASS

## Audit -- 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Each instant and sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.\nFlashback {4}{R}
**Type line**: Sorcery
**Status**: PASS
### Code issues
None. Card data matches oracle: name, cost {3}{R}, type Sorcery, flashback {4}{R}, grants flashback to instants/sorceries in graveyard until end of turn with cost equal to mana cost. Behavior is correct.
