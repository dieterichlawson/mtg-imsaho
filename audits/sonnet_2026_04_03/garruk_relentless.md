## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: When Garruk has two or fewer loyalty counters on him, transform him.
0: Garruk deals 3 damage to target creature. That creature deals damage equal to its power to him.
0: Create a 2/2 green Wolf creature token.

Back face: +1: Create a 1/1 black Wolf creature token with deathtouch.
−1: Sacrifice a creature. If you do, search your library for a creature card, reveal it, put it into your hand, then shuffle.
−3: Creatures you control gain trample and get +X/+X until end of turn, where X is the number of creature cards in your graveyard.

**Type line**: Legendary Planeswalker — Garruk
**Status**: ISSUE

### Code issues
- Oracle text field contains incorrect wording (mtg-engine/src/cards/isd/garruk_relentless.rs:97)
  - Oracle text says: `When Garruk has two or fewer loyalty counters on him, transform him`
  - Code does: `"When Garruk Relentless has two or fewer loyalty counters on him, transform Garruk Relentless"`

- Oracle text field contains incorrect wording for first ability (mtg-engine/src/cards/isd/garruk_relentless.rs:97)  
  - Oracle text says: `Garruk deals 3 damage to target creature`
  - Code does: `"Garruk Relentless deals 3 damage to target creature"`

### Tricky interactions checked
- Transform timing with 0 loyalty: pass - SBA correctly puts planeswalker in graveyard before transform trigger can resolve
- Mandatory sacrifice vs optional: pass - code correctly enforces "must sacrifice if you control a creature" per ruling
- State-triggered ability timing: pass - SBA system checks condition (≤2 loyalty) and adds trigger appropriately  
- Loyalty ability restriction per turn: pass - engine tracks abilities_activated_this_turn with sentinel value
- "That creature deals damage" fight timing: pass - correctly applies mutual damage with proper damage events
- Until end of turn cleanup: pass - +X/+X and trample effects properly added to cleanup lists
- Multiple creatures sacrifice choice: pass - correctly presents choice when multiple creatures available
- Library search and shuffle: pass - correctly searches for creature cards and shuffles library
- X-value calculation snapshot: pass - counts creature cards in graveyard when ability resolves, not continuously

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Transform at 2 or fewer loyalty: `tier15_cards.rs:2256`
- Wolf token creation (front face): `tier15_cards.rs:2234`
- Deathtouch wolf creation (back face): `tier15_cards.rs:2281`
- Sacrifice and tutor with single creature: `tier15_cards.rs:2309`
- Sacrifice choice with multiple creatures: `tier15_cards.rs:2347`
- Library shuffling after tutor: `tier15_cards.rs:2391`
- +X/+X and trample based on graveyard count: `tier15_cards.rs:2439`
- Transform loyalty abilities shown correctly: `tier15_cards.rs:2479`
- Zero loyalty edge case (SBA vs transform): NOT TESTED
- Reveal step in tutor ability: NOT TESTED