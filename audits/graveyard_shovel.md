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

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch
**Oracle text**: {2}, {T}: Target player exiles a card from their graveyard. If it's a creature card, you gain 2 life.
**Type line**: Artifact
**Status**: PASS

Mana cost {2}: correct (Generic(2)). Type Artifact: correct. No subtypes: correct. Activated ability: cost {2} + tap, `TargetRequirement::PlayerOnly`: correct -- the oracle targets a player, and per Scryfall ruling ("The targeted player chooses which card to exile when the ability resolves"), the targeted player makes the choice.

`is_valid_target` checks that the targeted player has cards in their graveyard: correct. `activated_abilities` checks object is on battlefield and not tapped: correct. When one card in graveyard, auto-exiles: correct (no choice needed). When multiple cards, creates `AwaitingAction::ResolutionChoice` for the targeted player to choose: correct. Creature check uses card registry data with fallback to `power.is_some()`: reasonable. Life gain of 2 emits `LifeChanged` event: correct. `once_per_turn: false` and `sorcery_speed_only: false`: correct (can activate at instant speed, multiple times per turn if untapped).

Tests in `tests/innistrad_simple_cards.rs` cover: card data verification, exile + life gain for creature. No anti-patterns found.

## Audit — 2026-04-02

**Oracle text (Scryfall, cached 2026-04-01)**: {2}, {T}: Target player exiles a card from their graveyard. If it's a creature card, you gain 2 life.
**Ruling (2011-09-22)**: The targeted player chooses which card to exile when the ability resolves.
**Type line**: Artifact
**Status**: PASS

Card data: name "Graveyard Shovel", mana cost {2} (Generic(2)), type Artifact, oracle text matches Scryfall verbatim. All correct.

Activated ability: cost {2} generic + tap (`requires_tap: true`), `TargetRequirement::PlayerOnly`. Correct -- the oracle targets a player, not a card. `once_per_turn: false`, `sorcery_speed_only: false`: correct.

Target validation (`is_valid_target`): checks targeted player has at least one card in their graveyard. Correct.

Availability (`activated_abilities`): checks object is on battlefield and not tapped, plus at least one card exists in any graveyard. Correct.

Resolution (single card): auto-exiles the only card, checks creature type via CardRegistry, gains 2 life for controller with `LifeChanged` event if creature. Correct -- no choice needed with one card.

Resolution (multiple cards): sets up `AwaitingAction::ResolutionChoice` for the targeted player to choose which card to exile. Matches the ruling. The `ExileFromGraveyardGainLife` handler in engine.rs exiles the chosen card, checks creature type, and grants 2 life to controller with `LifeChanged` event. Correct.

Life gain: "you" = controller of Graveyard Shovel, correctly captured. `LifeChanged` event emitted in both single-card and multi-card resolution paths.

Tests (6, all passing): `targets_player_not_card`, `auto_exiles_single_card`, `no_life_gain_for_non_creature`, `multiple_cards_creates_resolution_choice`, `resolution_choice_exiles_and_gains_life`, `cannot_target_player_with_empty_graveyard`. Good coverage of all paths.

No issues found.
