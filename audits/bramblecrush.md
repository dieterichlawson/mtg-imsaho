## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Destroy target noncreature permanent.
**Type line**: Sorcery
**Status**: PASS
### Code issues
No issues found.

## Audit — 2026-04-02 20:37

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Destroy target noncreature permanent.
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

Note: `is_valid_target` checks `registry.card_data(obj.card_id).card_types` (printed card types) rather than `obj.card_types` (current types on the battlefield). This is a codebase-wide pattern shared by Naturalize, Victim of Night, and others -- not a Bramblecrush-specific bug. If type-changing effects are ever implemented, all these cards would need updating together.

### Tricky interactions checked
- Indestructible permanent survives Bramblecrush (destruction pipeline via `try_destroy`): pass
- Cannot target creatures (checked via `TargetFilter::Noncreature` and `is_valid_target`): pass
- Can target any noncreature permanent type: lands, artifacts, enchantments, planeswalkers: pass (all are valid as long as they lack `CardType::Creature`)
- Spell goes to graveyard after resolution (`move_spell_after_resolve`): pass

### Test coverage
- Destroys a noncreature permanent (land): `tier2_spells.rs:280` (bramblecrush_destroys_land)
- Cannot target a creature: `tier2_spells.rs:299` (bramblecrush_cant_target_creature)
- Respects indestructible on noncreature permanents: `card_fixes.rs:251` (bramblecrush_respects_indestructible)
- Can target artifacts or enchantments specifically: NOT TESTED (covered indirectly by land test + Naturalize tests)
