## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Destroy target artifact.
Flashback {G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Targeting**: Card correctly requires targeting an artifact permanent via `TargetRequirement::PermanentWithFilter` and validates targets are artifacts on battlefield in `is_valid_target`
- **Flashback mechanics**: Card has `flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Colored(Color::Green)]))` which enables casting from graveyard for {G}. Engine sets `cast_with_flashback = true` for flashback spells and `move_spell_after_resolve` correctly exiles flashback spells afterward
- **Destroy vs indestructible**: Card calls `crate::cards::helpers::resolve_destroy` which uses `crate::destruction::try_destroy` to properly handle indestructible and regeneration
- **Timing restrictions**: As an instant with flashback, timing restrictions are enforced by the engine - flashback can only be used when you could normally cast the spell
- **Spell cleanup**: Card uses `move_spell_after_resolve` which correctly moves normal spells to graveyard but exiles flashback spells
- **Mana cost verification**: Card data shows cost as {1}{R} with flashback cost {G}, matching oracle text exactly

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- **Basic destroy artifact**: NOT TESTED
- **Flashback casting from graveyard**: NOT TESTED  
- **Flashback exile after resolution**: NOT TESTED
- **Targeting artifact permanents**: NOT TESTED
- **Indestructible artifact interaction**: NOT TESTED
- **Timing restrictions with flashback**: NOT TESTED

Sources:
- [Ancient Grudge rulings - MTG Assist](https://www.mtgassist.com/cards/Innistrad/Ancient-Grudge/rulings/?price_type=paper)
- [Ancient Grudge · Modern Masters 2017 (MM3) #88](https://scryfall.com/card/mm3/88/ancient-grudge)