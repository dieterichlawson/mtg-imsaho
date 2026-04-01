# Audit: Memory's Journey

## Official Oracle
- **Name:** Memory's Journey
- **Cost:** {1}{U}
- **Type:** Instant
- **Oracle:** Target player shuffles up to three target cards from their graveyard into their library. Flashback {G}

## Implementation: `mtg-engine/src/cards/memorys_journey.rs`
- **Name:** Memory's Journey -- CORRECT
- **Cost:** {1}{U} -- CORRECT
- **Type:** Instant -- CORRECT
- **Flashback:** {G} -- CORRECT
- **Target:** PlayerOnly -- PARTIAL (see below)
- **on_resolve:** Moves up to 3 graveyard cards to library, shuffles -- CORRECT behavior

## Issues
1. **Targeting simplified:** The oracle says "Target player shuffles up to three **target** cards from their graveyard." The cards in the graveyard are also targets, not just the player. The implementation auto-picks the first 3 cards from the graveyard using `.take(3)` rather than targeting specific cards. This is a simplification -- the caster should choose which cards to shuffle back.

## Verdict
**FAIL** -- 1 issue: Graveyard card selection is auto-picked (first 3) instead of targeted.

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: Target player shuffles up to three target cards from their graveyard into their library. / Flashback {G}
**Scryfall type line**: Instant
**Status**: ISSUE

Mana cost {1}{U}: correct. Type Instant: correct. Flashback cost {G}: correct. Uses `move_spell_after_resolve`: correct (no graveyard anti-pattern).

on_resolve: moves targeted cards from graveyard to library, then shuffles the owning player's library: correct behavior.

Issues found:
1. **Targeting model uses ModalChoice instead of proper multi-target**: The oracle says "Target player shuffles up to three target cards from their graveyard into their library." This means the spell has two sets of targets: (1) a target player, and (2) up to three target cards from that player's graveyard. The implementation uses `TargetRequirement::ModalChoice` with two modes (caster's graveyard vs opponent's graveyard), which is a workaround. The actual oracle targeting does not have modes -- it targets a player and then cards from that player's graveyard. The current approach could break if there are more than 2 players, or if the caster wants to target themselves but choose cards from their own graveyard specifically.
2. **Player target not explicit**: The spell should explicitly target a player (which matters for hexproof/shroud on players), but the current implementation infers the player from which mode is chosen, never explicitly targeting a player object.

Tests present in `tests/memorys_journey.rs` and `tests/tier11_cards.rs`.

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch
**Oracle text**: Target player shuffles up to three target cards from their graveyard into their library. Flashback {G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: ISSUE

Mana cost {1}{U}: correct. Type Instant: correct. Flashback cost {G}: correct (`flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Colored(Color::Green)]))`). Uses `move_spell_after_resolve`: correct.

`on_resolve`: Moves targeted cards from graveyard to library (adding to `library_order`), then shuffles the owning player's library: correct behavior. Player determination is inferred from the first targeted card's owner: functional but indirect.

Issues found:
1. **Targeting model uses ModalChoice instead of proper player+cards targeting** (persists from prior audit): Oracle says "Target player shuffles up to three target cards from their graveyard into their library." This means the spell has two categories of targets: a target player and up to three target cards from that player's graveyard. The implementation uses `TargetRequirement::ModalChoice` with two modes (caster's graveyard vs opponent's graveyard), which is a workaround. This does not correctly model the player as a separate target -- per Scryfall ruling, if the player becomes an illegal target, the entire spell fizzles even if the cards are still legal targets. The current implementation would not enforce this.
2. **Player not explicitly targeted**: Per Scryfall rulings, player hexproof/shroud should prevent this spell from targeting that player. The current implementation never explicitly targets a player, so player-targeting protections would be bypassed.
3. **Per ruling, if no cards targeted or all card targets are illegal, the player still shuffles their library**: The current implementation would not shuffle the library if all card targets become illegal (the `target_player` would be None since it's derived from the first card target).

Tests in `tests/tier11_cards.rs` cover: shuffling cards into library, flashback cost verification. No graveyard or damage anti-patterns.
