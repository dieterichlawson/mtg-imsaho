## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Create a token that's a copy of target creature you control.
Flashback {5}{U}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS
### Code issues
No issues found.

## Audit — 2026-04-02 20:37

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Create a token that's a copy of target creature you control.
Flashback {5}{U}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

Card data verified against oracle text:
- Name: "Cackling Counterpart" -- matches
- Mana cost: {1}{U}{U} -- matches
- Type: Instant -- matches
- Oracle text field: matches (reminder text for Flashback omitted, consistent with codebase convention)
- Flashback cost: {5}{U}{U} -- matches
- Target: `CreatureWithFilter(TargetFilter::YouControl)` -- correctly implements "target creature you control"

### Tricky interactions checked
- Target removed before resolution: PASS -- `on_resolve` checks `o.zone == Zone::Battlefield` before creating the copy (line 45); if the target left, no token is created and the spell still goes to graveyard/exile
- Flashback exile: PASS -- `move_spell_after_resolve` checks `cast_with_flashback` flag and exiles if true, sends to graveyard otherwise (state.rs:1132-1141)
- Token is a copy (not just a vanilla creature): PASS -- `create_token_copy` copies name, power, toughness, card_types, subtypes, keywords from the source, and sets `card_id` so the token gets the same `CardBehavior` (state.rs:413-448)
- Parallel Lives doubling: PASS -- `create_token_with_subtypes` (called by `create_token_copy`) checks for Parallel Lives and creates extra copies (state.rs:325-335)

### Test coverage
- Creates token copy with correct name/P/T: `tier12_cards.rs:487` (cackling_counterpart_creates_token_copy)
- Flashback cost present and correct (mana value 7): `tier12_cards.rs:510` (cackling_counterpart_has_flashback)
- Target fizzle (creature removed before resolution): NOT TESTED
- Copying a legendary creature (legend rule): NOT TESTED
- Token copy of a token: NOT TESTED

## Audit — 2026-04-10 18:27

**Oracle text source**: Oracle cache (Scryfall API), cached 2026-04-01
**Oracle text**: Create a token that's a copy of target creature you control.
Flashback {5}{U}{U} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: ISSUE

### Code issues

- `mtg-engine/src/cards/isd/cackling_counterpart.rs:39-41` does not enforce the "you control" target restriction at cast time.
  - Oracle text says: `Create a token that's a copy of target creature you control.`
  - Code does: `fn target_requirement(&self) -> TargetRequirement { TargetRequirement::CreatureWithFilter(TargetFilter::YouControl) }` and provides no override of `is_valid_target`. For spells, `generate_cast_actions_with_targets` in `mtg-engine/src/engine.rs:1192` treats `TargetRequirement::Creature | TargetRequirement::CreatureWithFilter(_)` identically — the inner `TargetFilter` is discarded and the only filter applied to spell targeting is `behavior.is_valid_target`, which defaults to `true` (see `mtg-engine/src/cards/mod.rs:294-296`). A comparable card, `mtg-engine/src/cards/isd/rangers_guile.rs:32-41`, overrides `is_valid_target` to enforce `o.controller == caster`; Cackling Counterpart does not. Result: Cackling Counterpart can currently legally target an opponent's creature when being cast.

- `mtg-engine/src/cards/isd/cackling_counterpart.rs:43-54` (`on_resolve`) does not re-check the target's legality beyond "is on the battlefield."
  - Oracle text says: `target creature you control`
  - Code does: `if state.get_object(*target_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) { ... state.create_token_copy(*target_id, controller, registry); ... }` — it does not verify the target is still a creature or still controlled by the Cackling Counterpart controller on resolution. Per CR 608.2b all targeting conditions must still be satisfied; if the target has changed controller (or lost its creature type) in response, the spell should have no legal targets and be countered on resolution. Because target generation also fails to apply the filter (prior bullet), the only place this check could live is here, and it is missing.

- `mtg-engine/src/cards/isd/cackling_counterpart.rs:47` relies on `state.create_token_copy`, which in `mtg-engine/src/state.rs:382-426` derives the token's characteristics from `registry.card_data(card_id)` of the source permanent. When the source is itself a token, `card_id` is the sentinel `CardId(0)` (see `create_token_internal` at `mtg-engine/src/state.rs:334`), so `registry.card_data(card_id)` yields no data and `colors`, `keywords`, `card_types`, and `subtypes` all default to empty. This directly contradicts the ruling that the token copies the original token's stated characteristics:
  - Oracle ruling says: `If the copied creature is a token, the token that's created copies the original characteristics of that token as stated by the effect that created the token.`
  - Code does: returns a stat-line with `source.power`/`source.toughness` but with empty colors/keywords/card_types/subtypes whenever the source is a token, losing the copied token's identity (e.g., a copy of a green 2/2 Wolf token becomes a colorless 2/2 with no types and no subtypes).

### Tricky interactions checked
- Target becomes illegal on resolution (controller change, loss of creature type): NOT HANDLED — only zone is rechecked.
- Copying a token source (e.g. via Moan of the Unhallowed Zombie tokens, Mayor of Avabruck wolves): BROKEN — characteristics lost (see third issue above).
- Copying a creature with enter-the-battlefield triggers: relies on `create_token_copy` copying `card_id` so that the token's CardBehavior fires ETB hooks — verified present at `mtg-engine/src/state.rs:422-424`. PASS for non-token sources.
- Copy effects must use printed (base) characteristics, not anthem-buffed P/T: `obj.power`/`obj.toughness` are base fields (continuous effects are layered in via `effective_power`), so `create_token_copy` reading them is correct. PASS.
- Flashback: card declares `flashback_cost: Some({5}{U}{U})`. Exile-after-resolve behavior relies on `move_spell_after_resolve` — used correctly at line 53. PASS (assuming engine flashback pipeline correctness).

### Test coverage
- Main effect (creates token copy of target creature you control): `mtg-engine/tests/tier12_cards.rs:491` (`cackling_counterpart_creates_token_copy`).
- Flashback cost present: `mtg-engine/tests/tier12_cards.rs:514` (`cackling_counterpart_has_flashback`) — only verifies `flashback_cost.is_some()` and mana value, not actual cast-from-graveyard-then-exile behavior.
- "You control" target restriction (cannot target opponent's creature): NOT TESTED.
- Fizzle when target becomes illegal in response (zone change): NOT TESTED.
- Fizzle when target's controller changes before resolution: NOT TESTED.
- Copying a token source preserves characteristics: NOT TESTED (a related bug test `bug_cackling_counterpart_colors_not_copied` at `mtg-engine/tests/audit_bugs2.rs:637` tests the narrower printed-card color-copy bug, which is also broken when source is a non-token creature whose mana cost produces no colored symbols — note, for Grizzly Bears cost `{1}{G}`, the current `create_token_copy` does derive green, so that test may currently pass).
- Token inherits copy-copy chain (source itself was copying something): NOT TESTED.
- "If copied creature has {X} in mana cost, X is 0" ruling: NOT TESTED.
- Cast-from-graveyard via flashback then exiled after resolution: NOT TESTED in card's own test file.
