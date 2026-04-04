## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform this creature.
**Type line**: Creature — Human Wizard
**Status**: ISSUE

### Code issues

- **Incorrect trigger implementation** (`mtg-engine/src/cards/isd/delver_of_secrets.rs:104-118`)
  - Oracle text says: `You may reveal that card.` (choice should always be offered)
  - Code does: Only offers the choice if the top card is already an instant or sorcery (`if top_is_instant_or_sorcery { /* present choice */ }`)
  - **Supporting ruling**: "You may reveal the card even if it's not an instant or sorcery. Whether or not you reveal it, the card stays on top of your library." (Scryfall ruling from 2011-09-22)

### Tricky interactions checked

- **"may reveal" optionality**: FAIL - choice only offered if card is instant/sorcery, should always be offered
- **Transform condition**: PASS - correctly transforms only if instant/sorcery is revealed  
- **Active player restriction**: PASS - correctly checks `state.active_player != controller`
- **Battlefield zone check**: PASS - correctly verifies creature is on battlefield
- **Library top card stays**: PASS - card remains on library top regardless of choice
- **Source leaves battlefield**: PASS - trigger resolves but no transform occurs if source gone
- **Front face only trigger**: PASS - correctly checks `is_transformed` to avoid back-face triggers

### Test coverage

The issue with reveal choice is actually encoded in the tests:
- **Instant on top, player reveals**: `tier15_cards.rs:940-974` / TESTED
- **Instant on top, player declines**: `tier15_cards.rs:977-1007` / TESTED  
- **Creature on top, no choice offered**: `tier15_cards.rs:1010-1028` / INCORRECTLY TESTED (test expects wrong behavior)
- **Transform only on instant/sorcery reveal**: `tier15_cards.rs:940-974` / TESTED
- **Card stays on library top**: `tier15_cards.rs:972-973, 1005-1006` / TESTED
- **Player choice optionality with non-instant/sorcery**: NOT TESTED (current test expects wrong behavior)