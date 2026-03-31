# Audit: Boneyard Wurm

## Oracle (Scryfall/API)
- **Name:** Boneyard Wurm
- **Cost:** {1}{G}
- **Type:** Creature — Wurm
- **Oracle:** Boneyard Wurm's power and toughness are each equal to the number of creature cards in your graveyard.
- **P/T:** */*

## Implementation: `boneyard_wurm.rs`
- **Name:** Boneyard Wurm -- CORRECT
- **Cost:** {1}{G} -- CORRECT
- **Type:** Creature — Wurm -- CORRECT
- **Base P/T:** 0/0 -- CORRECT (star/star cards use 0/0 base)
- **dynamic_pt:** Returns (creature_cards_in_gy, creature_cards_in_gy) -- CORRECT
- **Graveyard counting:** Counts objects in controller's graveyard with power.is_some() (i.e., creatures) -- CORRECT

## Verdict: PASS -- No issues found
