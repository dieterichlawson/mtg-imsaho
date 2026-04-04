## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Hexproof (This creature can't be the target of spells or abilities your opponents control.)
This creature can't be blocked.
**Type line**: Creature — Human Rogue
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Hexproof blocks spells from opponents: `can_be_targeted()` in `engine.rs:758` checks `has_keyword(target_id, Keyword::Hexproof)` and returns false when controller != caster — pass
- Hexproof blocks activated abilities from opponents: `can_be_targeted()` is called for ability target generation (lines 1300, 1308, 1346, 1362 of `engine.rs`), not just spells — pass
- Hexproof allows own-player targeting: `can_be_targeted()` returns true when `controller == caster` — pass
- Can't be blocked applies unconditionally to all blockers: `ContinuousEffect::CantBeBlocked { scope: EffectScope::OnSelf }` checked in `combat.rs:686` via `has_continuous_effect` returns false for any blocker — pass
- `EffectScope::OnSelf` resolves to `creature_id == source_id` in `state.rs:699`, correctly limiting the can't-be-blocked effect to Invisible Stalker itself — pass
- Mana cost `{1}{U}` — code has `Generic(1), Colored(Blue)` — pass
- Subtypes Human and Rogue — code has `vec!["Human".into(), "Rogue".into()]` — pass
- P/T 1/1 — code has `power: Some(1), toughness: Some(1)` — pass

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Hexproof keyword present in registry: `innistrad_cards.rs:115` (`invisible_stalker_has_hexproof`)
- Hexproof prevents opponent spell targeting: `keywords.rs:166` (`hexproof_prevents_opponent_targeting`)
- Hexproof allows controller to target own creature: `keywords.rs:186` (same test, second assertion)
- Hexproof prevents activated ability targeting: NOT TESTED (only spell targeting is tested)
- Can't be blocked by any creature: `card_mechanics.rs:455` (`invisible_stalker_unblockable`)
