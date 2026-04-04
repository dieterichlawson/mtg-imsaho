## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: You may have this creature enter as a copy of any creature on the battlefield, except it has "{U}{B}, {T}: Destroy target creature with the same name as this creature."
**Type line**: Creature — Shapeshifter  
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **"may" optionality**: pass - uses `present_optional_target_choice` with `optional=true`, player can decline to copy anything
- **Copy any creature on battlefield**: pass - `creature_targets_except` includes all creatures regardless of controller, excludes self  
- **Activated ability persistence**: pass - `is_evil_twin` marker set before choice, preserved regardless of what is copied
- **Same name targeting**: pass - `TargetFilter::SameNameAsSource` compares `source.name == obj.name`, works correctly after copying
- **Copiable characteristics**: pass - CopyCreature handler propagates `is_evil_twin` marker when another effect copies an Evil Twin
- **Simultaneous ETB restriction**: pass - only creatures already on battlefield are available as targets via battlefield zone filter
- **Token copying**: pass - no special handling needed, general copy mechanics handle token characteristics correctly
- **Copy chains**: pass - copying a creature that's already copying something works through general copy mechanics

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic ETB copy mechanics: `tier15_cards.rs:1756` 
- Optional choice presentation: `tier15_cards.rs:1767`
- Characteristic copying (name, P/T): `tier15_cards.rs:1775-1778`
- Marker preservation after copying: `tier15_cards.rs:1780`
- Activated ability functionality: NOT TESTED
- Same-name targeting behavior: NOT TESTED  
- Declining to copy (staying 0/0): NOT TESTED
- Copying tokens: NOT TESTED
- Copy effect copying Evil Twin: NOT TESTED
- Self-targeting with destroy ability: NOT TESTED