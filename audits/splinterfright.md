# Audit: Splinterfright

## Oracle (Scryfall)
- **Name:** Splinterfright
- **Cost:** {2}{G}
- **Type:** Creature -- Elemental
- **Oracle:** Trample. Splinterfright's power and toughness are each equal to the number of creature cards in your graveyard. At the beginning of your upkeep, mill two cards.
- **P/T:** */*

## Implementation: `mtg-engine/src/cards/splinterfright.rs`
- **Name:** Splinterfright ✅
- **Cost:** {2}{G} ✅
- **Type:** Creature ✅
- **Subtypes:** Elemental ✅
- **Base P/T:** 0/0 (used as fallback) ✅
- **Keywords:** Trample ✅
- **dynamic_pt:** counts creature cards (power.is_some()) in controller's graveyard ✅
- **Triggered ability:** Upkeep ✅
- **on_upkeep:** mills 2 cards, only on controller's upkeep ✅

## Verdict: PASS -- no issues found
