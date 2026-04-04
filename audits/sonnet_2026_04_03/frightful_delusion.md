## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Counter target spell unless its controller pays {1}. That player discards a card.
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Must target a spell to cast: pass (TargetRequirement::Spell enforces this, can't cast without valid target)
- Counter unless pays {1}: pass (checks mana_pool.total() >= 1, presents PayOrNot choice if payable, auto-counters if not)
- Mandatory discard regardless of payment: pass (PayOrNot handler in engine.rs forces discard in both pay/don't-pay branches, auto-counter path also forces discard)
- Spell cleanup handling: pass (uses move_spell_after_resolve correctly)
- Target validation: pass (is_valid_target checks Zone::Stack)
- Mana payment mechanics: pass (auto_pay handles {1} cost correctly via ManaCost::new(vec![ManaSymbol::Generic(1)]))

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Must target spell to cast: `card_mechanics.rs:576` (frightful_delusion_choice_when_opponent_has_mana) — TESTED
- Basic counter and discard: `tier2_spells.rs:89` (frightful_delusion_counters_and_discards) — TESTED
- Choice presentation when opponent has mana: `card_mechanics.rs:576` (frightful_delusion_choice_when_opponent_has_mana) — TESTED
- Auto-counter when no mana available: `card_mechanics.rs:616` (frightful_delusion_auto_counters_without_mana) — TESTED
- Discard happens even when paid (key ruling): `card_fixes.rs:153` (frightful_delusion_discard_on_pay) — TESTED