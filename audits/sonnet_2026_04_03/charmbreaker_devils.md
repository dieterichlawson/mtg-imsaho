## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: At the beginning of your upkeep, return an instant or sorcery card at random from your graveyard to your hand. Whenever you cast an instant or sorcery spell, this creature gets +4/+0 until end of turn.
**Type line**: Creature — Devil
**Status**: ISSUE

### Code issues
- Source leaving battlefield prevents ability resolution (charmbreaker_devils.rs:47-50)
  - Oracle text says: `At the beginning of your upkeep, return an instant or sorcery card at random from your graveyard to your hand.`
  - Code does: Returns early if `o.zone != Zone::Battlefield`, preventing ability resolution if Charmbreaker Devils leaves battlefield between trigger and resolution. Per MTG rules, triggered abilities should resolve using last known information even if source leaves battlefield.

### Tricky interactions checked
- Random selection from graveyard: pass
- Non-targeting behavior of upkeep ability: pass  
- Spell type filtering (instant/sorcery only): pass
- Multiple spell triggers per turn: pass
- Only controller's spells trigger second ability: pass
- Until end of turn effects cleanup: pass
- Empty graveyard handling: pass
- Source leaving battlefield between trigger and resolution: fail

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Random instant/sorcery return from graveyard: NOT TESTED
- +4/+0 buff on instant/sorcery cast: `tier7_cards.rs:247`
- Non-targeting upkeep ability: NOT TESTED
- Multiple triggers per turn: NOT TESTED
- Empty graveyard handling: NOT TESTED
- Source leaving battlefield during ability resolution: NOT TESTED