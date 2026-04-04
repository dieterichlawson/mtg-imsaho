## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant creature
Enchanted creature gets -2/-2.
**Type line**: Enchantment — Aura
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Aura attachment mechanics: PASS - Uses standard resolve_aura helper which correctly sets attached_to field and moves to battlefield
- EffectScope::Attached evaluation: PASS - Verified in state.rs:700-704 that attached scope checks source.attached_to == creature_id continuously
- Continuous effect application: PASS - ModifyPT with -2/-2 correctly applied via continuous effects system, verified in state.rs
- Target creature dies to state-based actions: PASS - Test confirms 2/2 creature becomes 0/0 and dies correctly 
- Effect removal when aura leaves battlefield: PASS - EffectScope::Attached stops applying when source object no longer exists or attached_to is cleared

### Test coverage
- Dead Weight kills small creature: `mtg-engine/tests/innistrad_cards.rs:238-254`
- Aura attachment mechanics: TESTED via resolve_aura helper
- Continuous effect application: TESTED via test showing 2/2 becomes 0/0
- State-based action creature death: TESTED via creature dying from 0 toughness
- Effect scope when attached: NOT DIRECTLY TESTED (tested implicitly via other aura tests)