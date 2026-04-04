## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Destroy target noncreature permanent.
**Type line**: Sorcery
**Status**: ISSUE

### Code issues
- Token targeting logic in `is_valid_target` at mtg-engine/src/cards/isd/bramblecrush.rs:41-43
  - Oracle text says: `Destroy target noncreature permanent.`
  - Code does: Only checks registry data via `registry.card_data(obj.card_id).map(|d| !d.card_types.contains(&CardType::Creature))`. For tokens with `card_id: CardId(0)`, this returns `None` and `.unwrap_or(false)` makes ALL tokens invalid targets, including hypothetical non-creature artifact/enchantment tokens that should be targetable.

### Tricky interactions checked
- **Indestructible permanents**: PASS - Uses `resolve_destroy` helper which calls destruction pipeline that correctly checks indestructible
- **Regeneration interaction**: PASS - Destruction pipeline correctly handles regeneration shields before destroying
- **Token targeting**: FAIL - Cannot target any tokens (creature or non-creature) due to registry-only type checking
- **Creature vs non-creature validation**: PASS - Correctly excludes creatures from registry data
- **Spell cleanup**: PASS - Uses `move_spell_after_resolve` after destroy effect

### Test coverage
- **Basic destruction of noncreature permanent**: `tier2_spells.rs:278` (bramblecrush_destroys_land)
- **Cannot target creatures**: `tier2_spells.rs:297` (bramblecrush_cant_target_creature)  
- **Indestructible interaction**: `card_fixes.rs:248` (bramblecrush_respects_indestructible)
- **Token targeting edge case**: NOT TESTED
- **Regeneration vs destroy**: NOT TESTED (covered generically in destruction.rs but not for Bramblecrush specifically)

Sources:
- [Indestructible - MTG Wiki - Fandom](https://mtg.fandom.com/wiki/Indestructible)
- [Bramblecrush MTG - Innistrad #172 (English) | Magic: The Gathering](https://gatherer.wizards.com/ISD/en-us/172/bramblecrush)
- [Rules on Indestructible — MTG Q&A](https://tappedout.net/mtg-questions/rules-on-indestructible/)
- [Regenerate - MTG Wiki - Fandom](https://mtg.fandom.com/wiki/Regenerate)
- [Understanding Regenerate Rules and Legacy in MTG](https://printmtg.com/understanding-regenerations-rules-and-legacy-in-mtg/)
- [Indestructible - MTG Wiki](https://mtg.gamepedia.com/Indestructible)
- [Indestructible in MTG: Rules, History, and Best Cards - Draftsim](https://draftsim.com/indestructible-mtg/)
- [Bramblecrush · Innistrad (ISD) #172 · Scryfall Magic: The Gathering Search](https://scryfall.com/card/isd/172/bramblecrush)