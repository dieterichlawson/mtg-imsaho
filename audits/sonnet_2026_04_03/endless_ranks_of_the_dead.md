## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: At the beginning of your upkeep, create X 2/2 black Zombie creature tokens, where X is half the number of Zombies you control, rounded down.
**Type line**: Enchantment
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Upkeep timing (only controller's upkeep): pass
- Zombie counting at resolution time: pass  
- Integer division rounding down: pass
- Minimum threshold (fewer than 2 zombies = 0 tokens): pass
- Multiple copies stacking correctly: pass
- Token subtype detection (registry + object subtypes): pass

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic functionality (5 zombies -> 2 tokens): `tier7_cards.rs:104`
- Minimum threshold (0-1 zombies = 0 tokens): NOT TESTED
- Multiple copies interaction: NOT TESTED
- Resolution timing vs declaration timing: NOT TESTED