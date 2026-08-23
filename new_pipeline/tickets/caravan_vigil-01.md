---
id: caravan_vigil-01
status: fixed
card: Caravan Vigil
audit_run_id: 2026-04-19-caravan_vigil-audit
audit_model: sonnet
audit_tokens: 22204
audit_duration: 427
fixed_sha: 612f503d41eea0d946bd0831975e605882f64669
fixed_at: 2026-08-23T20:18:55Z
fix_note: morbid choice was skipped whenever 2+ basic lands were found; both branches now go through finish_search
---

## Audit Finding

**Oracle text:**
> You may put that card onto the battlefield instead of putting it into your hand if a creature died this turn.

**Code:**
> // Multiple basic lands — player chooses.
state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
    player: controller,
    source: object_id,
    choice: ResolutionChoiceKind::ChooseFromLibrary {
        description: "Caravan Vigil: choose a basic land card".into(),
        options: basic_lands,
        searcher: controller,
        source_id: object_id,
    },
});
// The engine's ChooseFromLibrary handler moves the card to hand
// and shuffles the library. For the non-morbid case this is correct.
// For the morbid case, we'd need a card-specific handler, but
// this is still strictly better than auto-picking the first land.

**Description:**
When the player's library contains 2+ basic land cards, `on_resolve` falls into the multi-land branch (caravan_vigil.rs:106-121) and emits a `ChooseFromLibrary` prompt. The generic `ChooseFromLibrary` handler in engine.rs (lines 3072-3085) always moves the chosen card to `Zone::Hand` and never calls `move_spell_after_resolve`. This produces two interlocked bugs. First, when morbid is active (`creature_died_this_turn = true`), the player never gets the choice to put the land onto the battlefield — the land unconditionally goes to hand regardless of whether a creature died this turn, directly violating the oracle text. Second, Caravan Vigil itself is left in `Zone::Stack` after the `ChooseFromLibrary` choice resolves (no cleanup is performed). On the next round of priority passes, `resolve_top_of_stack` calls `on_resolve` again, which re-searches the library. With N basic lands and morbid active this cascades: the first N-1 lands are each taken by a separate `ChooseFromLibrary` pass (going to hand without a morbid offer), and only on the Nth pass — when exactly 1 basic land remains — does `finish_search` run and offer the morbid battlefield choice. The card's own comment at lines 117-120 explicitly acknowledges the morbid half of this bug: "For the morbid case, we'd need a card-specific handler." The fix is to replace the multi-land `ChooseFromLibrary` branch with a card-specific library-selection mechanism (analogous to the single-land path calling `finish_search`) so that after the player picks a land, the same morbid-offer and spell-cleanup logic runs regardless of how many basic lands were present.

**Engine path:** mtg-engine/src/cards/isd/caravan_vigil.rs:106

**Required check:** 8j

## Tests

### morbid_multiple_basic_lands_no_battlefield_choice
Scenario: Player casts Caravan Vigil with morbid active (a creature died this turn) and their library contains 2+ basic lands; after selecting a land, verify the player is prompted to choose hand vs battlefield (not auto-placed in hand).

### multiple_basic_lands_spell_cleaned_up_after_choice
Scenario: Player casts Caravan Vigil with their library containing 2+ basic lands (morbid irrelevant); after the player selects a basic land, verify Caravan Vigil has moved to the graveyard and is not re-resolved on subsequent priority passes.

