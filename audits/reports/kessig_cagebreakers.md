# Audit: Kessig Cagebreakers

## Oracle (Official)
- **Name:** Kessig Cagebreakers
- **Cost:** {4}{G}
- **Type:** Creature — Human Rogue
- **Oracle:** Whenever Kessig Cagebreakers attacks, create a 2/2 green Wolf creature token that's tapped and attacking for each creature card in your graveyard.
- **P/T:** 3/4

## Implementation
- Name: "Kessig Cagebreakers" -- CORRECT
- Cost: {4}{G} -- CORRECT
- Type: Creature -- CORRECT
- Subtypes: ["Human", "Rogue"] -- CORRECT
- P/T: 3/4 -- CORRECT
- Oracle text matches -- CORRECT
- Triggered ability: Attacks trigger -- CORRECT
- Counts creature cards in graveyard -- CORRECT
- Creates 2/2 green Wolf tokens -- CORRECT
- Tokens are tapped and attacking -- CORRECT
- Token subtypes: ["Wolf"] -- CORRECT
- Token colors: [Green] -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit: Kessig Cagebreakers
**Date:** 2026-04-02

### Oracle Text (Scryfall)
- **Type:** Creature -- Human Rogue
- **Cost:** {4}{G}
- **P/T:** 3/4
- **Oracle:** Whenever this creature attacks, create a 2/2 green Wolf creature token that's tapped and attacking for each creature card in your graveyard.

### Card Data
- **Name:** Kessig Cagebreakers -- PASS
- **Cost:** {4}{G} -- PASS
- **Types:** Creature -- PASS
- **Subtypes:** Human, Rogue -- PASS
- **P/T:** 3/4 -- PASS

### Oracle Text Match
- Code uses "Whenever Kessig Cagebreakers attacks" vs oracle "Whenever this creature attacks". Cosmetic only.
- PASS (minor wording variance)

### Behavior Audit
- **Attack trigger:** Uses TriggerKind::Attacks and on_attacks handler. -- PASS
- **Graveyard count:** Counts creature cards in controller's graveyard at resolution. -- PASS
- **Token creation:** Creates 2/2 green Wolf creature tokens with correct subtypes. -- PASS
- **Tapped and attacking:** Sets tapped = true, summoning_sick = false, inserts into combat.attackers. -- PASS
- **Defending player:** Tokens can attack the same player/planeswalker as Cagebreakers, determined from combat state. -- PASS

### Result: PASS

## Audit — 2026-04-03 07:08
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/189/kessig-cagebreakers), cached 2026-04-01
**Oracle text**: Whenever this creature attacks, create a 2/2 green Wolf creature token that's tapped and attacking for each creature card in your graveyard.
**Type line**: Creature — Human Rogue
**Status**: PASS

### Code issues

1. **Oracle text field mismatch (cosmetic)**: The stored `oracle_text` says "Whenever Kessig Cagebreakers attacks" while Scryfall now uses "Whenever this creature attacks" (modern template). Functionally equivalent per MTG rules — the card's name and "this creature" mean the same thing.

2. **Parallel Lives + tapped-and-attacking tokens (engine-level)**: `create_token_with_subtypes` returns only the primary token's ID. Extra tokens created by Parallel Lives doubling (lines 326-345 of state.rs) are not returned, so only the primary token gets `tapped = true` and is inserted into `combat.attackers`. The Parallel Lives copies would enter untapped and not attacking. This is a systemic engine issue shared with Geist of Saint Traft (same pattern), not specific to this card.

No card-specific behavioral bugs found.

### Tricky interactions checked (min 3)

1. **Graveyard count timing**: Per ruling (2011-09-22), "You count the number of creature cards in your graveyard when the triggered ability resolves." The implementation counts at the start of `on_attacks`, which is called when the trigger resolves. Correct.

2. **Tokens not declared as attackers**: Per ruling (2011-09-22), "Although the tokens are attacking, they were never declared as attacking creatures (for purposes of abilities that trigger whenever a creature attacks, for example)." The implementation directly inserts token IDs into `combat.attackers` without going through the declare-attackers flow, so they should not trigger "whenever a creature attacks" abilities. Correct.

3. **Token attack target**: Per ruling (2011-09-22), "You declare which player or planeswalker each token is attacking as you put it onto the battlefield. It doesn't have to be the same player or planeswalker Kessig Cagebreakers is attacking." The implementation sends all tokens at the same defending player as Cagebreakers (line 56-58). In the engine's 2-player model, there is only one possible opponent, so this is correct for the current scope.

4. **Zero creatures in graveyard**: If no creature cards are in the graveyard, `creature_count` is 0 and the function returns early (line 52-54). No tokens created. Correct.

5. **Non-creature cards in graveyard**: The filter at lines 46-50 checks `card_types` for `CardType::Creature` via the registry, falling back to `power.is_some()` for unregistered cards. Only creature cards are counted. Correct.

### Test coverage

- `kessig_cagebreakers_creates_wolf_tokens_on_attack` (tier15_cards.rs:104): Places 3 creatures in graveyard, triggers on_attacks, verifies 3 Wolf tokens are created on the battlefield, verifies they are tapped, verifies 4 total attackers (Cagebreakers + 3 wolves).
- No test for zero creatures in graveyard (no tokens created).
- No test for Parallel Lives interaction.
- No AI scenario tests found.
