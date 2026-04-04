## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Geistflame deals 1 damage to any target.
Flashback {3}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: PASS
### Code issues
No issues found.

## Audit — 2026-04-02 21:09
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Geistflame deals 1 damage to any target.
Flashback {3}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: ISSUE

### Code issues

1. **Oracle text field missing flashback line** (`mtg-engine/src/cards/isd/geistflame.rs:22`):
   The `oracle_text` field is `"Geistflame deals 1 damage to any target."` but should be `"Geistflame deals 1 damage to any target.\nFlashback {3}{R}"` to match the Scryfall oracle text and be consistent with every other flashback card in the codebase (e.g., Think Twice uses `"Draw a card.\nFlashback {2}{U}"`).

2. **LLM card knowledge missing flashback** (`mtg-player/src/llm.rs:82`):
   The AI player card knowledge says `- Geistflame ({R} instant): Deal 1 damage to any target.` but does not mention flashback. Every other flashback card in the knowledge base includes `flashback {cost}` in parentheses (e.g., `Think Twice ({1}{U} instant, flashback {2}{U})`). An AI player reading this knowledge would not know Geistflame has flashback.

### Tricky interactions checked (min 3)

1. **Flashback cast from graveyard exiles after resolve**: Tested in `flashback_spell_is_exiled_after_resolve`. The `resolve_damage` helper calls `move_spell_after_resolve`, which correctly checks `cast_with_flashback` and sends to exile. PASS.
2. **Flashback fizzle still exiles**: Tested in `flashback_spell_fizzle_goes_to_exile`. When the target becomes illegal before resolution, the spell fizzles but is still exiled (not returned to graveyard). PASS.
3. **Flashback countered still exiles**: Tested in `flashback_spell_countered_is_exiled`. A Counterspell targeting a flashback Geistflame correctly sends it to exile. PASS.
4. **Normal cast goes to graveyard**: Tested in `normal_cast_goes_to_graveyard`. When cast from hand (not flashback), goes to graveyard as expected. PASS.
5. **Damage to creature**: Tested in `geistflame_deals_1_damage`. Deals exactly 1 damage to a 2/2 creature, which survives. PASS.

### Test coverage

- `tier2_spells::geistflame_deals_1_damage` — 1 damage to creature, creature survives
- `flashback::flashback_offered_from_graveyard` — Geistflame flashback offered when in graveyard with {3}{R}
- `flashback::flashback_not_offered_from_hand` — Normal cast from hand at {R}, not flashback
- `flashback::flashback_not_offered_without_mana` — No flashback without sufficient mana
- `flashback::flashback_spell_is_exiled_after_resolve` — Flashback Geistflame exiled after resolve
- `flashback::normal_cast_goes_to_graveyard` — Normal cast goes to graveyard
- `flashback::flashback_spell_countered_is_exiled` — Countered flashback goes to exile
- `fizzle::flashback_spell_fizzle_goes_to_exile` — Fizzled flashback goes to exile
- `fizzle::flashback_spell_fizzle_no_resolved_event` — Fizzled flashback no SpellResolved event
- Missing: test for dealing 1 damage to a player (not just creature)
