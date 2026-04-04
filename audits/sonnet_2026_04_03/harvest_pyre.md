## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: As an additional cost to cast this spell, exile X cards from your graveyard. Harvest Pyre deals X damage to target creature.
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Additional cost timing**: pass — Cards are exiled at cast time (lines 1612-1628 in engine.rs) before spell goes on stack, not at resolution. If spell is countered, exiled cards stay exiled (correct per MTG rules).
- **Player choice of X**: pass — Engine generates separate CastSpell actions for each X value from 0 to graveyard size (lines 593-610 in engine.rs), allowing player to choose.
- **X=0 handling**: pass — When X=0, no cards are exiled and no damage is dealt (count=0, line 45 in harvest_pyre.rs). Test coverage confirms this works.
- **Own graveyard only**: pass — Code correctly filters by `o.owner == player` (line 1614 in engine.rs), cannot exile opponent's graveyard cards.
- **Self-exclusion**: pass — Code correctly excludes the spell itself with `o.id != *object_id` (line 1614 in engine.rs) to prevent trying to exile Harvest Pyre from itself.
- **Target becomes illegal**: pass — Resolution checks `obj.zone == Zone::Battlefield` (line 48 in harvest_pyre.rs), so spell properly does nothing if target is removed before resolution.
- **Damage tracking**: pass — Correctly sets `damage_marked`, `damaged_by`, and emits `NonCombatDamageDealt` event (lines 49-56 in harvest_pyre.rs).
- **Spell cleanup**: pass — Uses `move_spell_after_resolve` (line 63 in harvest_pyre.rs) for proper cleanup.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- **X=4 (exile all available)**: `mtg-engine/tests/tier8_cards.rs:573-593`
- **Player choice of partial X (X=2 of 4 available)**: `mtg-engine/tests/tier8_cards.rs:596-631`
- **X=0 deals no damage**: `mtg-engine/tests/tier8_cards.rs:633-655`
- **Legal actions include all X values**: `mtg-engine/tests/tier8_cards.rs:658-684`
- **Only exiles own graveyard**: `mtg-engine/tests/tier8_cards.rs:687-728`
- **Additional cost timing (countered spell keeps exiled cards)**: NOT TESTED
- **Target becoming illegal before resolution**: NOT TESTED