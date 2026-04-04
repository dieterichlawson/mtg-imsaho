## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Destroy all enchantments. You gain 1 life for each enchantment destroyed this way.
**Type line**: Sorcery
**Status**: ISSUE

### Code issues

- Enchantment detection only checks the registry, missing enchantment tokens (`mtg-engine/src/cards/isd/paraselene.rs` lines 36–40)
  - Oracle text says: `"Destroy all enchantments."`
  - Code does: `registry.card_data(o.card_id).map(|d| d.card_types.contains(&CardType::Enchantment)).unwrap_or(false)` — for token objects, `card_id` is the sentinel `CardId(0)`, so `registry.card_data(CardId(0))` returns `None` and the filter returns `false`. Any enchantment token on the battlefield would not be destroyed. The code should additionally check `o.card_types.contains(&CardType::Enchantment)` to cover tokens (as `create_token_internal` sets `card_types` directly on the object, not in the registry).

### Tricky interactions checked

- **Indestructible enchantments not counted in life gain**: `try_destroy` returns `DestroyResult::Indestructible`; the code only increments `destroyed_count` when the result is `DestroyResult::Died`. An indestructible enchantment is neither destroyed nor counted — PASS
- **Regeneration shield on an enchantment not counted**: `try_destroy` returns `DestroyResult::Regenerated` for a shielded permanent; the code does not increment `destroyed_count` in that case. A regenerated enchantment is not "destroyed this way" — PASS
- **Zero enchantments on battlefield**: `destroyed_count` stays 0; the `if destroyed_count > 0` guard prevents any life change or log — PASS
- **Controller gains life, not the caster of a previous turn**: `controller` is read from the Paraselene object at resolution time (`state.get_object(object_id).map(|o| o.controller)`), which is correct — PASS
- **move_spell_after_resolve handles flashback exile**: `state.move_spell_after_resolve(object_id)` is called at the end; this correctly sends flashback-cast copies to Exile rather than Graveyard — PASS
- **Mana cost {2}{W}**: Implemented as `vec![ManaSymbol::Generic(2), ManaSymbol::Colored(Color::White)]`, mana value 3 — PASS
- **Card type Sorcery**: `vec![CardType::Sorcery]` — PASS
- **Oracle text field**: Matches provided oracle text exactly — PASS
- **Enchantment token detection (registry vs. object card_types)**: As described in Code Issues — FAIL

### Test coverage

- Mana cost and card type: `innistrad_simple_cards.rs:415` (`paraselene_card_data`) — TESTED
- Two enchantments destroyed, life gained (+2): `innistrad_simple_cards.rs:424` (`paraselene_destroys_enchantments_and_gains_life`) — TESTED
- Indestructible enchantment not counted in life gain: NOT TESTED
- Regeneration shield on enchantment not counted: NOT TESTED
- Zero enchantments present (no life gain): NOT TESTED
- Enchantment token destroyed by Paraselene: NOT TESTED (the bug case)
- Flashback exile path via move_spell_after_resolve: NOT TESTED
