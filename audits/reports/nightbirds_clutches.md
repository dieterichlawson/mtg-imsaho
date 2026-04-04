# Audit: Nightbird's Clutches

## Reference (Scryfall/API)
- **Name:** Nightbird's Clutches
- **Mana Cost:** {1}{R}
- **Type:** Sorcery
- **Oracle:** Up to two target creatures can't block this turn. Flashback {3}{R}

## Implementation: `nightbirds_clutches.rs`
- **Name:** Nightbird's Clutches -- CORRECT
- **Mana Cost:** {1}{R} -- CORRECT
- **Type:** Sorcery -- CORRECT
- **P/T:** None -- CORRECT
- **Keywords:** None (flashback handled via flashback_cost field) -- CORRECT
- **Flashback cost:** {3}{R} -- CORRECT
- **Target:** UpToTargets(2, Creature) -- CORRECT
- **Behavior:** Adds targeted creatures to `until_end_of_turn_cant_block` -- CORRECT

## Verdict: PASS

## Audit -- 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Up to two target creatures can't block this turn.\nFlashback {3}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Status**: PASS
### Code issues
None. Card data matches oracle: name, cost {1}{R}, type Sorcery, flashback {3}{R}, targets up to two creatures, prevents blocking until end of turn. Behavior is correct.
