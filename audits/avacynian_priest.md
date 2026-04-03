# Audit: Avacynian Priest

## Oracle (Scryfall)
- **Name:** Avacynian Priest
- **Cost:** {1}{W}
- **Type:** Creature — Human Cleric
- **Oracle:** {1}, {T}: Tap target non-Human creature.
- **P/T:** 1/2

## Implementation: `mtg-engine/src/cards/avacynian_priest.rs`
- **Name:** Avacynian Priest ✅
- **Cost:** {1}{W} ✅
- **Type:** Creature ✅
- **Subtypes:** Human, Cleric ✅
- **P/T:** 1/2 ✅
- **Oracle text:** matches ✅
- **Activated ability:** {1}, {T}: Tap target non-Human creature ✅
- **Target filtering:** Excludes Humans via `is_valid_target` ✅
- **Tap effect:** Sets `tapped = true` on target ✅
- **requires_tap:** true ✅
- **Triggered abilities:** none ✅

## Verdict: PASS — no issues found

## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: {1}, {T}: Tap target non-Human creature.
**Type line**: Creature — Human Cleric
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Avacynian Priest", cost {1}{W}, 1/2, type Creature — Human Cleric. Activated ability costs {1} + tap, targets non-Human creatures on the battlefield. is_valid_target correctly excludes Humans. on_activate_ability sets target tapped. All correct.

## Audit — 2026-04-02 (full-reaudit)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 20:33

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: {1}, {T}: Tap target non-Human creature.
**Type line**: Creature — Human Cleric
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Human subtype exclusion (dynamic subtypes via `o.subtypes` and registry `card_data` both checked): pass
- Can target own creatures (no controller restriction in `is_valid_target`, `_caster` unused for filtering): pass
- Cannot activate when already tapped (`requires_tap: true` enforced by engine): pass
- Instant-speed activation (`sorcery_speed_only: false`): pass
- No once-per-turn restriction (`once_per_turn: false`): pass

### Test coverage
- Correct stats (P/T, subtypes): `activated_abilities.rs:273`
- Taps non-Human creature: `activated_abilities.rs:284`
- Cannot target Human creatures: `activated_abilities.rs:312`
- Cannot activate when tapped: `activated_abilities.rs:333`
- Targeting own creatures: NOT TESTED
- Interaction with creatures gaining Human type dynamically: NOT TESTED

## Audit — 2026-04-02 21:23

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: {1}, {T}: Tap target non-Human creature.
**Type line**: Creature — Human Cleric
**Status**: PASS

### Code issues
None. All card data matches oracle exactly:
- Name "Avacynian Priest", cost {1}{W}, P/T 1/2, type Creature — Human Cleric.
- Activated ability: costs {1} + tap, targets non-Human creatures via `TargetRequirement::Creature` filtered by `is_valid_target`.
- `is_valid_target` correctly excludes Humans by checking both registry subtypes and runtime object subtypes, requires battlefield zone and creature (power.is_some()).
- `on_activate_ability` sets `tapped = true` on the target.
- `once_per_turn: false`, `sorcery_speed_only: false` — both correct per oracle.

### Tricky interactions checked (min 3)
1. **Human subtype exclusion with dynamic subtypes**: `is_valid_target` checks both `registry.card_data(o.card_id).subtypes` and `o.subtypes` for "Human", so creatures that gain the Human type at runtime are correctly excluded. PASS.
2. **Can target own non-Human creatures**: No controller check in `is_valid_target` — the oracle has no controller restriction, so targeting own creatures is valid. PASS.
3. **Cannot activate when already tapped**: `requires_tap: true` is enforced by the engine's legal action generation. Verified by test `avacynian_priest_requires_tap`. PASS.
4. **Can target already-tapped creatures**: No untapped check in `is_valid_target`. Oracle says "Tap target non-Human creature" which does not require the target to be untapped (tapping an already-tapped creature is legal but does nothing meaningful). PASS.
5. **Instant-speed activation**: `sorcery_speed_only: false` allows activation during any phase, including combat (e.g., tapping blockers). PASS.

### Test coverage
- `avacynian_priest_has_correct_stats` (line 273): verifies P/T 1/2 and subtypes Human, Cleric
- `avacynian_priest_taps_non_human_creature` (line 284): taps a Wolf, verifies both target tapped and priest tapped
- `avacynian_priest_cannot_target_humans` (line 312): verifies Human creature (Elder Cathar) cannot be targeted
- `avacynian_priest_requires_tap` (line 333): verifies cannot activate a second time while tapped
- All 4 tests pass.
