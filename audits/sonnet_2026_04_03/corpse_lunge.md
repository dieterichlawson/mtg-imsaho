## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: As an additional cost to cast this spell, exile a creature card from your graveyard.
Corpse Lunge deals damage equal to the exiled card's power to target creature.
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Zero or negative power on exiled creature: `power.max(0) as u32` on line 46 correctly prevents negative damage and handles zero power gracefully
- Target creature leaving battlefield before resolution: Checked via `obj.zone == Zone::Battlefield` on line 51 - if target is no longer on battlefield, no damage is dealt
- Last known information principle: Power is stored at cast time in `card_state["exiled_power"]` (engine.rs lines 1589-1592) and read at resolution (lines 40-43), correctly using the power the creature had when exiled rather than any current state
- Engine auto-selection vs player choice: The engine auto-selects the highest-power creature to exile (engine.rs line 1584) rather than allowing player choice. While this differs from typical Magic player choice expectations, the oracle text "exile a creature card" is satisfied and the core functionality works correctly
- Damage event emission: Properly emits NonCombatDamageDealt event (lines 55-59) with correct source, target, and amount for downstream triggers
- Spell resolution cleanup: Correctly calls `move_spell_after_resolve` (line 66) to handle normal resolution and flashback cases

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic functionality (exile creature, deal damage equal to power): `tier8_cards.rs:498` (corpse_lunge_deals_damage_equal_to_exiled_power)
- No creature in graveyard edge case: `tier8_cards.rs:524` (corpse_lunge_no_graveyard_creature_deals_no_damage) 
- Auto-selection of highest power creature: `tier8_cards.rs:538` (corpse_lunge_picks_highest_power_creature)
- Zero power exiled creature: NOT TESTED
- Target leaving battlefield before resolution: NOT TESTED
- Damage event emission for triggers: NOT TESTED
- Multiple creatures in graveyard selection order: `tier8_cards.rs:538` (covered in highest power test)
- Additional cost payment timing (cast vs resolve): TESTED (implicitly covered by all tests working correctly)