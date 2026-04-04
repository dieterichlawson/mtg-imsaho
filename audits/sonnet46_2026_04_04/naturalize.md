## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Destroy target artifact or enchantment.
**Type line**: Instant
**Status**: ISSUE

### Code issues

- `is_valid_target` checks only registry data for card types, missing artifact/enchantment tokens
  - File: `mtg-engine/src/cards/isd/naturalize.rs`, lines 40–42
  - Oracle text says: `Destroy target artifact or enchantment.` (no restriction on tokens)
  - Code does: `registry.card_data(obj.card_id).map(|d| d.card_types.contains(&CardType::Artifact) || d.card_types.contains(&CardType::Enchantment)).unwrap_or(false)` — tokens always have `card_id: CardId(0)` (sentinel), which returns `None` from `registry.card_data()`, so `.unwrap_or(false)` returns `false`. An artifact or enchantment token would never be a valid target. The correct pattern (used by Stony Silence in `engine.rs` lines 280–283) checks both: `registry.card_data(obj.card_id).map(|d| d.card_types.contains(&CardType::Artifact)).unwrap_or(false) || obj.card_types.contains(&CardType::Artifact)`.

### Tricky interactions checked

- Indestructible artifact/enchantment survives Naturalize: PASS — `resolve_destroy` calls `try_destroy`, which checks `state.has_keyword(id, Keyword::Indestructible, registry)` before destroying.
- Regeneration replaces Naturalize's destruction: PASS — `try_destroy` checks `regeneration_shields > 0` and calls `regenerate()` instead of destroying.
- `move_spell_after_resolve` used (not raw `move_object`): PASS — `helpers::resolve_destroy` calls `state.move_spell_after_resolve(spell_id)` at line 101, so Naturalize correctly goes to graveyard normally and exile if cast via flashback.
- Target leaves battlefield before resolution: PASS — `resolve_destroy` checks `obj.zone == Zone::Battlefield` before calling `try_destroy`; if the target is gone, nothing happens and the spell still moves to graveyard.
- Targeting an artifact token: FAIL — as described above, `is_valid_target` returns `false` for any token (sentinel `CardId(0)`) even if `obj.card_types` contains `CardType::Artifact` or `CardType::Enchantment`. No artifact/enchantment tokens currently exist in the card set, so this does not affect any implemented scenario, but the implementation is incorrect in principle.
- Targeting a creature (should be illegal): PASS — `is_valid_target` only returns `true` if the target's card types include `Artifact` or `Enchantment`; creatures have neither in their registry data and are therefore excluded.
- Mana cost {1}{G}: PASS — code uses `Generic(1)` + `Colored(Color::Green)`.
- Card type Instant: PASS — `card_types: vec![CardType::Instant]`.
- Oracle text field matches oracle: PASS — `oracle_text: "Destroy target artifact or enchantment.".into()`.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:
- Destroys enchantment: `mtg-engine/tests/tier2_spells.rs:238` (`naturalize_destroys_enchantment`)
- Cannot target creature: `mtg-engine/tests/tier2_spells.rs:262` (`naturalize_cant_target_creature`)
- Destroys artifact: NOT TESTED
- Indestructible artifact/enchantment survives: NOT TESTED
- Regeneration replaces destruction: NOT TESTED
- Target leaves battlefield before resolution: NOT TESTED
- Artifact/enchantment token targeting (latent bug): NOT TESTED
