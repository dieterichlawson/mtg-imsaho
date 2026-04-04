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

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Put a +1/+1 counter on each of up to two target creatures.\nFlashback {1}{W} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Sorcery
**Mana Cost**: {1}{G}
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Travel Preparations", cost {1}{G}, type Sorcery, flashback cost {1}{W}. Target requirement UpToTargets(2, Creature) correctly allows up to two creature targets. on_resolve adds one +1/+1 counter to each target on the battlefield. All correct.
