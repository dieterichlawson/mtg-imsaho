## Audit — 2026-04-01

**Scryfall Oracle text**: Destroy target artifact or enchantment.
**Scryfall type line**: Instant
**Status**: PASS

- Name: Correct ("Naturalize")
- Cost: {1}{G} - Correct
- Type: Instant - Correct
- Target: Artifact or enchantment on battlefield - Correct (uses TargetFilter::HasCardType with both types)
- is_valid_target checks zone == Battlefield and card type is Artifact or Enchantment. Correct.
- on_resolve uses helpers::resolve_destroy. Correct.
- Tests: tier2_spells.rs has `naturalize_destroys_enchantment` and `naturalize_cant_target_creature`.

No issues found.
