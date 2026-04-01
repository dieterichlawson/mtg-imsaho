## Audit — 2026-04-01

**Scryfall Oracle text**: Enchant creature
When Claustrophobia enters the battlefield, tap enchanted creature. Enchanted creature doesn't untap during its controller's untap step.
**Scryfall type line**: Enchantment — Aura
**Status**: ISSUE

### Findings

1. **Tap on ETB is in on_resolve, not on_enter_battlefield (minor ISSUE)**: The implementation taps the creature in `on_resolve` (line 42) rather than via an ETB trigger. Functionally this is similar since the aura enters as part of resolving, but strictly speaking, Oracle says "When Claustrophobia enters the battlefield, tap enchanted creature" — this is a triggered ability that goes on the stack. The current implementation does it immediately as part of resolution, meaning it cannot be responded to. In practice this rarely matters but is technically incorrect.

2. **Enchant creature oracle text missing from oracle_text field**: The oracle_text field says "When Claustrophobia enters the battlefield, tap enchanted creature. Enchanted creature doesn't untap during its controller's untap step." but omits "Enchant creature" which is part of the Oracle text.

3. **Card data correct**: Name, cost ({1}{U}{U}), type (Enchantment), subtype (Aura) all match.

4. **PreventUntap continuous effect correct**: Uses `ContinuousEffect::PreventUntap` with `EffectScope::Attached`.

5. **Tests**: Found in `card_mechanics.rs` and `innistrad_cards.rs`.
