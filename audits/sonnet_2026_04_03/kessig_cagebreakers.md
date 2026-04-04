## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Whenever this creature attacks, create a 2/2 green Wolf creature token that's tapped and attacking for each creature card in your graveyard.
**Type line**: Creature — Human Rogue
**Status**: ISSUE

### Code issues
- Target selection for attacking tokens (`mtg-engine/src/cards/isd/kessig_cagebreakers.rs:56-58`, lines 74)
  - Oracle text says: `create a 2/2 green Wolf creature token that's tapped and attacking for each creature card in your graveyard`
  - Code does: Automatically assigns all tokens to attack the same player/planeswalker as Kessig Cagebreakers (`let defending_player = state.combat.as_ref().and_then(|c| c.attackers.get(&self_id).copied()).unwrap_or_else(|| state.opponent(controller));` then `combat.attackers.insert(token_id, defending_player);`)
  - Issue: Per MTG rules and Scryfall ruling 2, "You declare which player or planeswalker each token is attacking as you put it onto the battlefield. It doesn't have to be the same player or planeswalker Kessig Cagebreakers is attacking." The player should be given a choice for each token.

### Tricky interactions checked
- Creature count timing: PASS - Code correctly counts creatures in graveyard at resolution time per ruling 1
- Tokens enter tapped and attacking: PASS - Code correctly sets `tapped = true` and adds to `combat.attackers`
- Tokens were never declared as attackers: PASS - Tokens are put directly into combat state without going through declaration
- Trigger fires when Kessig Cagebreakers attacks: PASS - `TriggerKind::Attacks` is correctly declared and trigger system dispatches to `on_attacks`
- Multiple creature deaths create multiple triggers: PASS - Each creature card in graveyard creates one token
- Tokens have correct stats and subtypes: PASS - 2/2 green Wolf creature tokens with correct subtypes
- Source leaving battlefield: PASS - Trigger resolution checks source is still on battlefield, but tokens creation doesn't depend on source remaining

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Creates correct number of tokens based on graveyard count: `mtg-engine/tests/tier15_cards.rs:125-129`
- Tokens are tapped and attacking: `mtg-engine/tests/tier15_cards.rs:131-137`
- Player chooses attack targets for each token: NOT TESTED
- Creature count at resolution time vs trigger time: NOT TESTED
- Tokens never declared as attackers (don't trigger attack watchers): NOT TESTED
- Attack target choice independence from Kessig Cagebreakers target: NOT TESTED
- Multiple tokens can attack different players/planeswalkers: NOT TESTED