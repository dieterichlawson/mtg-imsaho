## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: `Defender\n{T}: Exchange your life total with this creature's toughness.`
**Type line**: Creature — Plant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked

- **Effective toughness vs. base toughness in the exchange**: The exchange must use the *effective* toughness (including counters, auras, anthems, until-EOT effects) as the player's new life total, while setting only the *base* toughness (`obj.toughness`) to the old life total, so that modifiers re-apply on top. Code reads `state.effective_toughness(object_id, registry)` for the life value and writes only `obj.toughness = Some(current_life)`, leaving all modifiers intact. This correctly implements the ruling: "Any toughness-modifying effects, counters, Auras, or Equipment will apply after its toughness is set to your former life total." **PASS**

- **"Tree not on battlefield when ability resolves" ruling**: The ruling states the exchange has no effect if the Tree is not on the battlefield at resolution time. In this engine, `Action::ActivateAbility` resolves the ability immediately without placing it on the stack (`on_activate_ability` is called directly at engine.rs:1802). Opponents can never respond to the activation, so the Tree is always on the battlefield when the exchange runs. The code's `on_activate_ability` only guards `None` (object doesn't exist), not `zone != Zone::Battlefield`, but this gap is unreachable in the current engine. **PASS** (harmless in current engine)

- **Tap cost correctly limits re-use**: `activated_abilities` only returns the ability definition when `obj.zone == Zone::Battlefield && !obj.tapped`. After activation the engine sets `tapped = true` (engine.rs:1740) before calling `on_activate_ability`. The ability is therefore unavailable until the Tree untaps, without needing `once_per_turn: true`. **PASS**

- **Capture order for the exchange values**: `current_toughness` (effective toughness) and `current_life` are both read from state before any mutations. Only after capturing both values does the code set `player.life = current_toughness` and then `obj.toughness = Some(current_life)`. This prevents either value from influencing the other. **PASS**

- **LifeChanged event emission**: The ruling states "you will gain or lose an amount of life necessary so that your life total equals Tree of Redemption's former toughness. Other effects that interact with life gain or life loss will interact with this effect accordingly." A `GameEvent::LifeChanged` event is emitted with correct `old` and `new_life` fields. **PASS**

- **Sorcery-speed restriction**: The oracle text imposes no timing restriction on `{T}`. The code sets `sorcery_speed_only: false`. **PASS**

- **Defender keyword**: Oracle text says "Defender". Code has `keywords: vec![Keyword::Defender]`. **PASS**

- **Card data correctness**:
  - Mana cost {3}{G}: `vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Green)]`. **PASS**
  - Type Creature — Plant: `card_types: vec![CardType::Creature]`, `subtypes: vec!["Plant".into()]`. **PASS**
  - P/T 0/13: `power: Some(0), toughness: Some(13)`. **PASS**

- **Log message accuracy**: Log says `"Tree of Redemption: exchanged life ({old_life}) with toughness ({current_toughness})"` where `old_life` is the life total before the swap and `current_toughness` is the effective toughness before the swap. Accurate. **PASS**

### Test coverage

- Basic exchange (20 life ↔ 13 toughness): `tier15_cards.rs:762` — TESTED
- Exchange when toughness-modifying effects/counters/auras are present (Lunarch Mantle ruling): NOT TESTED
- "Tree not on battlefield when ability resolves" fizzle: NOT TESTED (also unachievable in current engine)
- LifeChanged event causing secondary life-gain/loss triggers: NOT TESTED (no cards in current set react to LifeChanged)
- Activation through full engine action (tap cost paid, legal_actions check): NOT TESTED (test calls `on_activate_ability` directly, bypassing the engine path)
