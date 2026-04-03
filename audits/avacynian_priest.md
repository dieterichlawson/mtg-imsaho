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
