# Audit: Somberwald Spider

## Oracle (Scryfall)
- **Name:** Somberwald Spider
- **Cost:** {4}{G}
- **Type:** Creature -- Spider
- **Oracle:** Reach. Morbid -- When Somberwald Spider enters the battlefield, if a creature died this turn, put two +1/+1 counters on Somberwald Spider.
- **P/T:** 2/4

## Implementation: `mtg-engine/src/cards/somberwald_spider.rs`
- **Name:** Somberwald Spider ✅
- **Cost:** {4}{G} ✅
- **Type:** Creature ✅
- **Subtypes:** Spider ✅
- **P/T:** 2/4 ✅
- **Keywords:** Reach ✅
- **Triggered ability:** EntersBattlefield ✅
- **Morbid check:** checks `state.creature_died_this_turn` ✅
- **Counters:** adds 2 PlusOnePlusOne counters ✅

## Verdict: PASS -- no issues found
