# Audit: Geist of Saint Traft

## Oracle Reference (Scryfall)
- Cost: {1}{W}{U}
- Type: Legendary Creature -- Spirit Cleric
- P/T: 2/2
- Oracle: "Hexproof
  Whenever Geist of Saint Traft attacks, create a 4/4 white Angel creature token with flying that's tapped and attacking. Exile that token at end of combat."

## Implementation: geist_of_saint_traft.rs

## Issues Found

1. **ISSUE: Token exiled at end step instead of end of combat** - Oracle says "Exile that token at end of combat." The implementation uses on_end_step (line 89), which is the end step, not end of combat. The comment on line 90 even acknowledges this: "Exile the angel token at end of combat (simplified: at end step)." This means the Angel token survives longer than it should -- it can block during the opponent's next combat if there's a way to untap it, and it persists through the second main phase.

2. **ISSUE: Triggered ability incorrectly uses EndStep TriggerKind** - The triggered_abilities list includes a TriggerKind::EndStep entry for the exile effect (line 35-38). The correct trigger timing is end of combat, not end step.

Otherwise correct: cost, types (Legendary Spirit Cleric), P/T (2/2), hexproof keyword, 4/4 white Angel token with flying, token enters tapped and attacking.

## Verdict: ISSUES FOUND (2 issues - timing)
