# Audit: Unburial Rites

## Scryfall Reference
- **Name:** Unburial Rites
- **Cost:** {4}{B}
- **Type:** Sorcery
- **Oracle:** Return target creature card from your graveyard to the battlefield. Flashback {3}{W}
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/unburial_rites.rs`
- Name: "Unburial Rites" -- MATCH
- Cost: {4}{B} -- MATCH
- Types: Sorcery -- MATCH
- Flashback: {3}{W} -- MATCH
- Behavior: Returns a creature card from graveyard to battlefield -- MATCH
- Handles single vs. multiple choices -- CORRECT
- Uses PendingEffect::ReturnToBattlefield for multi-choice -- CORRECT

## Verdict
**PASS** — Correctly implements reanimation with flashback.

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Return target creature card from your graveyard to the battlefield. / Flashback {3}{W}
**Type line**: Sorcery
**Status**: PASS

### Card Data
- **Name:** Unburial Rites -- CORRECT
- **Mana Cost:** {4}{B} -- CORRECT
- **Type:** Sorcery -- CORRECT
- **Flashback Cost:** {3}{W} -- CORRECT

### Code issues
None. Oracle_text field omits the flashback reminder text, but flashback_cost is correctly set to {3}{W}. The on_resolve correctly finds creature cards in controller's graveyard (filtered by power.is_some() and owner == controller), moves the chosen one to the battlefield, and handles single/multiple target cases with player choice. Spell cleanup is correct.
