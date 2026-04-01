## Audit — 2026-04-01

**Scryfall Oracle text**: Devil's Play deals X damage to any target.\nFlashback {X}{R}{R}{R}
**Scryfall type line**: Sorcery
**Status**: PASS

- Mana cost {X}{R}: correct.
- Type Sorcery: correct.
- Flashback cost {X}{R}{R}{R}: correct.
- Targets AnyTarget: correct (can hit creatures or players).
- Uses `resolve_damage` helper which emits `NonCombatDamageDealt`: correct.
- `resolve_damage` calls `move_spell_after_resolve`: correct.
- When X=0, spell resolves without dealing damage and calls `move_spell_after_resolve`: correct behavior (0 damage does nothing).
- X value read from `o.x_value`: correct.
- Tests exist in `tier14_cards.rs` (`devils_play_deals_x_damage`, `devils_play_x_zero`).
