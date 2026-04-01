## Audit — 2026-04-01

**Scryfall Oracle text**: {2}{G}: This creature gets +2/+2 until end of turn. Activate only once each turn.
**Scryfall type line**: Creature — Wolf
**Status**: PASS

No issues found. Mana cost {1}{G}, 2/2, Wolf subtype all correct. Activated ability has correct cost {2}{G}, once_per_turn=true, instant speed. Uses UntilEndOfTurnEffect for the buff. Good test coverage: correct stats, +2/+2 buff, once-per-turn restriction.
