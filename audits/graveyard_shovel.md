# Audit: Graveyard Shovel

## Oracle Reference (Scryfall)
- Cost: {2}
- Type: Artifact
- Oracle: "{2}, {T}: Exile target card from a graveyard. If it was a creature card, you gain 2 life."

## Implementation: graveyard_shovel.rs

## Issues Found

1. **ISSUE: Auto-targets instead of player choice** - Oracle says "target card from a graveyard" meaning the player should choose which card to exile. The implementation auto-selects, preferring creature cards for life gain (line 63-65). This removes strategic choice (e.g., choosing to exile a key non-creature card from opponent's graveyard even though creature cards exist).

Otherwise correct: cost ({2}), type (Artifact), oracle text, activated ability cost ({2} + tap), exile effect, and life gain for creature cards all match.

## Verdict: ISSUES FOUND (1 issue - auto-targeting)

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: {2}, {T}: Target player exiles a card from their graveyard. If it's a creature card, you gain 2 life.
**Scryfall type line**: Artifact
**Status**: PASS

Mana cost {2}: correct. Type Artifact: correct. Activated ability cost {2} + tap: correct. Targets a player (`TargetRequirement::PlayerOnly`): correct -- the oracle targets a player, and that player chooses which card to exile.

Target validation checks that the targeted player has at least one card in their graveyard: correct. When only one card exists, auto-exiles it: correct (no choice needed). When multiple cards exist, creates `AwaitingAction::ResolutionChoice` for the targeted player to choose: correct (per Scryfall ruling, "The targeted player chooses which card to exile when the ability resolves"). Creature check for life gain uses card registry data: correct. Life gain of 2 for creature cards: correct. Life change event emitted: correct.

The previous audit noted "auto-targeting" as an issue, but the current implementation correctly has the targeted player choose which card to exile when there are multiple options. Tests present in `tests/graveyard_shovel.rs` and `tests/innistrad_simple_cards.rs`. No anti-patterns found.
