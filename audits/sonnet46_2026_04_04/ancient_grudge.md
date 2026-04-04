## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Destroy target artifact.
Flashback {G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Flashback cost stored and paid correctly: `flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Colored(Color::Green)]))` matches `{G}`. Engine path at engine.rs:1498-1505 uses `data.flashback_cost` when `is_flashback = true`. PASS
- `cast_with_flashback` flag set on stack object: engine.rs:1636-1638 sets `obj.cast_with_flashback = true` when `is_flashback`. PASS
- Flashback spell exiled on resolution: `resolve_destroy` calls `state.move_spell_after_resolve(spell_id)` (helpers.rs:101), which checks `cast_with_flashback` and exiles if true (state.rs:1132-1141). PASS
- Flashback spell exiled when countered: `Counterspell::on_resolve` calls `state.move_spell_after_resolve(*target_id)` (counterspell.rs:50), same exile logic applies. Confirmed by test `flashback_spell_countered_is_exiled`. PASS
- Flashback spell exiled when fizzled (all targets illegal): stack.rs:83-85 calls `state.move_spell_after_resolve(object_id)` on fizzle, same exile logic. PASS
- Destroy goes through full destruction pipeline: `resolve_destroy` calls `try_destroy` (helpers.rs:97), which checks Indestructible keyword and regeneration shields (destruction.rs:33-49). Oracle says "Destroy" so this is correct. PASS
- Instant timing respected for flashback: engine.rs:692-706 checks `is_instant = data.card_types.contains(&CardType::Instant)`, allows flashback at any time (not restricted to sorcery speed). PASS
- Target validity check — regular artifact cards: `is_valid_target` checks `registry.card_data(obj.card_id).map(|d| d.card_types.contains(&CardType::Artifact))`. For non-token artifacts (Sol Ring, Witchbane Orb, Galvanic Juggernaut, etc.) the registry lookup succeeds. PASS
- Target validity check — copy tokens of artifact creatures: `create_token_copy` sets `obj.card_id = card_id` (state.rs:444-446), so registry lookup works for copy tokens of artifacts. PASS
- Target validity check — pure artifact tokens: `is_valid_target` only checks registry, not `obj.card_types`. For pure artifact tokens (card_id = CardId(0)), registry lookup returns None and `.unwrap_or(false)` would return false, making them untargetable. However, no cards in the current cardpool create pure artifact tokens (all `create_token_with_subtypes` calls use `vec![CardType::Creature]`). NOT AN ISSUE in the current cardpool.
- Target must be on battlefield: `is_valid_target` checks `o.zone == Zone::Battlefield` (ancient_grudge.rs:37-40). PASS
- Flashback not offered without mana: engine.rs:708 checks `mana::can_pay` against flashback cost before generating cast actions. PASS
- `is_flashback` detection: engine.rs:1491-1492 sets `is_cast_from_graveyard = in_graveyard && behavior.can_cast_from_graveyard()` and `is_flashback = in_graveyard && !is_cast_from_graveyard`. Ancient Grudge doesn't implement `can_cast_from_graveyard()` so it defaults to false; flashback correctly detected when cast from graveyard. PASS

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Flashback exiles spell after resolution: `flashback.rs` — tested via `flashback_spell_is_exiled_after_resolve` (uses Geistflame), NOT TESTED for Ancient Grudge specifically
- Flashback spell exiled when countered: `flashback.rs:129` — `flashback_spell_countered_is_exiled` (uses Geistflame), NOT TESTED for Ancient Grudge specifically
- Flashback offered from graveyard with sufficient mana: `flashback.rs:23` — `flashback_offered_from_graveyard` (uses Geistflame), NOT TESTED for Ancient Grudge specifically
- Flashback not offered without sufficient mana: `flashback.rs:65` — `flashback_not_offered_without_mana`, NOT TESTED for Ancient Grudge specifically
- Normal cast goes to graveyard: `flashback.rs:110` — `normal_cast_goes_to_graveyard`, NOT TESTED for Ancient Grudge specifically
- Destroy target artifact (basic effect): NOT TESTED
- Indestructible artifact cannot be destroyed: NOT TESTED
- Artifact with regeneration survives: NOT TESTED
- Timing — flashback cast at instant speed: NOT TESTED
- Fizzle path (target leaves battlefield in response): NOT TESTED
