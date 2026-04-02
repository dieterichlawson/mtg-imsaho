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

## Audit — 2026-04-02
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Trample\nSplinterfright's power and toughness are each equal to the number of creature cards in your graveyard.\nAt the beginning of your upkeep, mill two cards.
**Mana cost**: {2}{G}
**Type line**: Creature — Elemental
**P/T**: */*
**Status**: PASS
### Checks
- **Name**: "Splinterfright" -- CORRECT
- **Mana cost**: Generic(2) + Green -- CORRECT ({2}{G})
- **Type**: Creature with Elemental subtype -- CORRECT
- **P/T**: Base 0/0 with dynamic_pt -- CORRECT (*/*)
- **Keywords**: Trample -- CORRECT
- **dynamic_pt**: Counts creature cards (power.is_some()) in controller's graveyard -- CORRECT
- **Triggered ability**: Upkeep trigger, mills 2 cards via mill_cards helper -- CORRECT
- **on_upkeep**: Checks zone is Battlefield, only triggers on controller's upkeep -- CORRECT
### Code issues
None. Card data and behavior match oracle text.
