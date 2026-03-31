# Audit: Past in Flames

## Official Oracle
- **Name:** Past in Flames
- **Cost:** {3}{R}
- **Type:** Sorcery
- **Oracle Text:** Each instant and sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.\nFlashback {4}{R}
- **P/T:** N/A

## Implementation Review
- **Name:** OK
- **Cost:** {3}{R} — OK
- **Type:** Sorcery — OK
- **Oracle Text:** Matches (uses \n separator) — OK
- **Flashback Cost:** {4}{R} — OK
- **on_resolve:** Moves self to graveyard first (so it resolves before granting), then grants flashback to all instants/sorceries in controller's graveyard via until_end_of_turn_flashback — OK
- **Flashback cost = mana cost:** Uses `d.cost.clone().unwrap_or(ManaCost::free())` which correctly sets the flashback cost to the card's own mana cost — OK
- **Deduplication:** Checks `already_has` to avoid duplicating flashback grants — OK

## Issues
None found.

## Verdict: PASS
