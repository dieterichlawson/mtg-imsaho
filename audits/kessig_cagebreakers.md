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
