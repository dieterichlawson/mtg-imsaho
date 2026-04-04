## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying, vigilance
**Type line**: Creature — Griffin
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Flying mechanic: creatures with flying can only be blocked by creatures with flying or reach: pass
- Vigilance mechanic: creatures with vigilance don't tap when attacking: pass  
- Keyword interaction: both flying and vigilance work together without conflicts: pass

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Card has flying keyword: `innistrad_cards.rs:73` / TESTED
- Card has vigilance keyword: `innistrad_cards.rs:74` / TESTED
- Flying blocks correctly (can't be blocked by ground creatures): `keywords.rs:26-36` / TESTED
- Flying blocks correctly (can be blocked by flying creatures): `keywords.rs:42-46` / TESTED
- Vigilance works correctly (doesn't tap when attacking): `keywords.rs:82-84` / TESTED