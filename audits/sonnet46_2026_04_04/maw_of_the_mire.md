## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Destroy target land. You gain 4 life.
**Type line**: Sorcery
**Status**: ISSUE

### Code issues

- `is_valid_target` only consults the registry for `CardType::Land`, missing land tokens whose types are stored on `obj.card_types` (not in registry)
  - Oracle text says: `Destroy target land.`
  - Code does: `registry.card_data(obj.card_id).map(|d| d.card_types.contains(&CardType::Land)).unwrap_or(false)` — tokens have `card_id: CardId(0)`, so `registry.card_data(CardId(0))` returns `None`, and `unwrap_or(false)` returns `false`. Land tokens would therefore never pass `is_valid_target`, making them untargetable by this spell. The correct check should also consult `obj.card_types` directly (as `matches_target_filter` in `engine.rs` line 1392-1394 does: `types.iter().any(|t| obj.card_types.contains(t))`). No land tokens exist in the current card pool, so this cannot manifest today, but the structural bug is present.

### Tricky interactions checked

- **Fizzle when target leaves battlefield before resolution**: Engine (`stack.rs` lines 79-86) calls `is_target_legal` before `on_resolve`; if land is off-battlefield, spell fizzles and life is not gained — matches ruling `[2011-09-22]`. PASS
- **Redundant fizzle check inside `on_resolve`**: Card code at line 56-59 re-checks zone; this is redundant (engine already handled it) but harmless — `move_spell_after_resolve` is called once either way. PASS
- **Indestructible land**: `try_destroy` checks `Keyword::Indestructible` and returns `DestroyResult::Indestructible` without destroying the permanent. Spell still fully resolves (target was legal), so life is still gained — correct per MTG rules. PASS
- **Regeneration on land**: `try_destroy` checks `regeneration_shields > 0` and replaces destruction with tap/damage-clear. Life still gained. PASS
- **Life gain conditionality**: Oracle says "Destroy target land. You gain 4 life." — both are unconditional effects of a resolving spell, not conditional on the destroy succeeding. Code gains life after calling `try_destroy` regardless of whether destroy succeeded. PASS
- **"You" = controller, not caster**: Code derives `controller` from the spell object on the stack (`state.get_object(object_id).map(|o| o.controller)`), which is the casting player. Correct. PASS
- **`move_spell_after_resolve` double-call safety**: `on_resolve` calls `move_spell_after_resolve` at end; `stack.rs` then checks `if obj.zone == Zone::Stack` before calling again — since spell was already moved, no double-move occurs. PASS
- **Land token targeting (registry-only `is_valid_target`)**: `card_id: CardId(0)` for all tokens → `registry.card_data` returns `None` → `unwrap_or(false)` → land tokens cannot be targeted. FAIL (see Code issues)

### Test coverage

- Basic card data (types, mana value): `innistrad_simple_cards.rs:463` — TESTED
- Destroy land + gain 4 life (normal case): `innistrad_simple_cards.rs:472` — TESTED
- Fizzle when target is illegal at resolution (ruling `[2011-09-22]`): NOT TESTED
- Indestructible land (spell resolves, life still gained, land survives): NOT TESTED
- Land token targeting (tokens untargetable due to registry-only check): NOT TESTED
