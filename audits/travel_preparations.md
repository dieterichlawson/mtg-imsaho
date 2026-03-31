# Audit: Travel Preparations

## Scryfall Reference
- **Name:** Travel Preparations
- **Cost:** {1}{G}
- **Type:** Sorcery
- **Oracle:** Put a +1/+1 counter on each of up to two target creatures. Flashback {1}{W}
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/travel_preparations.rs`
- Name: "Travel Preparations" -- MATCH
- Cost: {1}{G} -- MATCH
- Types: Sorcery -- MATCH
- Flashback: {1}{W} -- MATCH
- Target: UpToTargets(2, Creature) -- MATCH
- Behavior: Puts +1/+1 counter on each target creature on the battlefield -- MATCH

## Verdict
**PASS** — Correctly implemented with flashback.
