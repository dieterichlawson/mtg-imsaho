## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {2}, {T}: Target player exiles a card from their graveyard. If it's a creature card, you gain 2 life.
**Type line**: Artifact
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Target player selection: PASS - can target any player including self, correctly restricted to players with cards in graveyard
- Targeted player chooses which card: PASS - when multiple cards exist, creates ResolutionChoice for targeted player; single cards auto-exiled
- "You gain 2 life" refers to controller: PASS - life gain correctly goes to controller of Graveyard Shovel, not targeted player
- Creature type detection: PASS - checks both registry card types and power.is_some() fallback for tokens
- Empty graveyard targeting: PASS - correctly prevents targeting players with empty graveyards
- Ability availability: PASS - ability only available when tapped artifact is on battlefield and any player has graveyard cards

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Targets player not card: `graveyard_shovel.rs:22-47` 
- Single card auto-exile: `graveyard_shovel.rs:49-70`
- No life gain for non-creatures: `graveyard_shovel.rs:72-91`
- Multiple cards create resolution choice: `graveyard_shovel.rs:93-119`
- Resolution choice exiles chosen card and gains life: `graveyard_shovel.rs:121-154`
- Cannot target empty graveyard: `graveyard_shovel.rs:156-177`
- Targeted player chooses which card (key ruling): `graveyard_shovel.rs:93-119` and `graveyard_shovel.rs:121-154`