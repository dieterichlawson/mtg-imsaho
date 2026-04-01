## Audit — 2026-04-01

**Scryfall Oracle text**: Target opponent sacrifices a creature. You gain life equal to that creature's toughness.
**Scryfall type line**: Instant
**Status**: PASS

- Name: correct ("Tribute to Hunger")
- Cost: {2}{B} -- correct
- Type: Instant -- correct
- Target: TargetRequirement::PlayerOnly -- correct (targets an opponent, the opponent then chooses which creature to sacrifice)
- Implementation uses `present_target_choice` to let the opponent choose which creature to sacrifice -- correct (opponent chooses, not caster)
- Life gain uses PendingEffect::SacrificeAndGainLife which should handle gaining life equal to toughness
- Handles the case where opponent has no creatures (does nothing) -- correct
- Tests exist in `tier8_cards.rs`
- No issues found

## Audit — 2026-04-01

**Scryfall Oracle text**: Target opponent sacrifices a creature. You gain life equal to that creature's toughness.
**Scryfall type line**: Instant
**Mana cost**: {2}{B}
**Status**: PASS

No issues found.
