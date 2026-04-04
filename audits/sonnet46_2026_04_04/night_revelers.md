## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: This creature has haste as long as an opponent controls a Human.
**Type line**: Creature — Vampire
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **"as long as" continuous evaluation vs. snapshot**: The condition is re-evaluated every time `has_keyword` is called (via `has_conditional_keyword` → `check_condition`). There is no snapshot at ETB. The test at `tier12_cards.rs:106` explicitly verifies that haste is lost when the Human leaves the battlefield. PASS
- **`EffectScope::OnSelf` restricts haste to Night Revelers only**: `effect_applies_to` with `OnSelf` returns `creature_id == source_id`, so the conditional haste is never applied to other creatures even if they share the same effect source. PASS
- **Haste bypasses summoning sickness**: `combat.rs:577` checks `(!o.summoning_sick || state.has_keyword(o.id, Keyword::Haste, registry))`. Since `has_keyword` calls `has_conditional_keyword`, Night Revelers with an active haste condition can attack the turn it enters. PASS
- **Human token detection**: `check_condition` for `OpponentControlsSubtype` checks both `o.subtypes` (runtime object field, set for tokens via `create_token_with_subtypes`) and `registry.card_data(o.card_id).subtypes` (for real cards). A Human token created by another card would be detected by the `o.subtypes` path. PASS
- **"an opponent" in 2-player game**: `check_condition` calls `self.opponent(controller)` which in a 2-player game returns the single opponent. For a 2-player game (the engine's primary use case), this is correct. PASS
- **Scope excludes other players' creatures from gaining haste**: The `OnSelf` scope and `creature_id == source_id` guard in `effect_applies_to` ensure the haste is only granted to Night Revelers itself, not to any other creature. PASS

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- No opponent Human → no haste: `mtg-engine/tests/tier12_cards.rs:95` TESTED
- Opponent controls a Human (real card) → has haste: `mtg-engine/tests/tier12_cards.rs:102` TESTED
- Human leaves battlefield → haste lost (continuous re-evaluation): `mtg-engine/tests/tier12_cards.rs:106` TESTED
- Haste with Human token (subtype on object, not registry): NOT TESTED
- Summoning sickness bypassed by conditional haste (Night Revelers attacks turn it enters): NOT TESTED
