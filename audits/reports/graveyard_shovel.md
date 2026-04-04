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

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: {2}, {T}: Target player exiles a card from their graveyard. If it's a creature card, you gain 2 life.
**Type line**: Artifact
**Status**: PASS

### Code issues
No issues found. Card data matches oracle: name, mana cost {2}, type Artifact. Activated ability costs {2} + tap correctly. Targets a player (PlayerOnly), validates that the targeted player has cards in graveyard. The targeted player chooses which card to exile (per ruling 2011-09-22), implemented via AwaitingAction for multiple cards or auto-exile for single card. Creature check uses registry card_data with power fallback. Life gain of 2 applied to controller with LifeChanged event. ExileFromGraveyardGainLife pending effect confirmed in engine.rs. No anti-patterns.

## Audit — 2026-04-02 21:12

**Oracle text source**: Scryfall API (cached 2026-04-01) via `scripts/oracle_lookup.py`
**Oracle text**: {2}, {T}: Target player exiles a card from their graveyard. If it's a creature card, you gain 2 life.
**Type line**: Artifact
**Status**: PASS

### Code issues
None found.

- **Card data**: Name "Graveyard Shovel", mana cost `Generic(2)`, type `Artifact`, no subtypes/supertypes. Oracle text stored verbatim matches Scryfall. All correct.
- **Activated ability**: Cost {2} + tap (`requires_tap: true`), `TargetRequirement::PlayerOnly`. `once_per_turn: false`, `sorcery_speed_only: false`. All correct.
- **Target validation** (`is_valid_target`): Only accepts `Target::Player` where that player has at least one card in their graveyard. Correct.
- **Availability** (`activated_abilities`): Checks zone is Battlefield, not tapped, and at least one graveyard card exists globally. Correct.
- **Resolution (single card)**: Auto-exiles the only card (no choice needed), checks creature type via `CardRegistry`, gains 2 life for controller with `LifeChanged` event. Correct.
- **Resolution (multiple cards)**: Sets up `AwaitingAction::ResolutionChoice` with `player: *target_player` so the targeted player chooses. Matches ruling (2011-09-22): "The targeted player chooses which card to exile when the ability resolves." The `ExileFromGraveyardGainLife` handler in `engine.rs` exiles the chosen card, checks creature type, and grants 2 life to controller. Correct.
- **Life gain**: "you" = Graveyard Shovel's controller, correctly captured in both single-card and multi-card paths.

### Tricky interactions checked (min 3)
1. **Targeted player chooses, not controller**: The ruling says the targeted player picks which card to exile. Implementation correctly sets `player: *target_player` in the `ResolutionChoice`. Verified.
2. **Non-creature exile yields no life gain**: Both the single-card path (in `on_activate_ability`) and the multi-card path (in `ExileFromGraveyardGainLife` handler in `engine.rs`) check `CardType::Creature` before granting life. Test `no_life_gain_for_non_creature` confirms. Verified.
3. **Cannot target player with empty graveyard**: `is_valid_target` checks the targeted player has graveyard cards. Test `cannot_target_player_with_empty_graveyard` confirms. Verified.
4. **Controller gains life, not target player**: Life is applied to `controller` (derived from the Shovel's controller), not `target_player`. Tests confirm P0 (controller) gains life when targeting P1. Verified.

### Test coverage
6 tests in `tests/graveyard_shovel.rs`, all passing:
- `targets_player_not_card` — verifies targets are players
- `auto_exiles_single_card` — single graveyard card auto-exiled, life gained
- `no_life_gain_for_non_creature` — non-creature exiled, no life gain
- `multiple_cards_creates_resolution_choice` — sets up choice for targeted player
- `resolution_choice_exiles_and_gains_life` — full resolution flow with creature
- `cannot_target_player_with_empty_graveyard` — validates targeting restriction

Additional test in `tests/innistrad_simple_cards.rs`: `graveyard_shovel_card_data` and `graveyard_shovel_exiles_and_gains_life`.
