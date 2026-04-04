## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Destroy target land. Into the Maw of Hell deals 13 damage to target creature.
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Two-target requirement enforcement: pass (requires legal land AND creature to cast)
- Partial resolution when one target becomes illegal: pass (card checks each target individually before applying effects)
- Spell fizzling when both targets become illegal: pass (engine checks for any legal target before calling on_resolve)
- Damage type classification: pass (uses NonCombatDamageDealt event type correctly)
- Land destruction mechanism: pass (uses try_destroy function)
- Spell cleanup after resolution: pass (uses move_spell_after_resolve)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic card data (mana cost, type): `innistrad_simple_cards.rs:450` 
- Two-target casting requirement: NOT TESTED
- Partial resolution with one illegal target: NOT TESTED  
- Spell fizzling with both targets illegal: NOT TESTED
- Land destruction functionality: NOT TESTED
- 13 damage to creature functionality: NOT TESTED
- Oracle text ruling about target legality: NOT TESTED