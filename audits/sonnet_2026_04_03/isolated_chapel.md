## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: This land enters tapped unless you control a Plains or a Swamp.
{T}: Add {W} or {B}.
**Type line**: Land
**Status**: ISSUE

### Code issues
- Oracle text wording mismatch (line 37): Code stores `"Isolated Chapel enters the battlefield tapped unless you control a Plains or a Swamp."` but current oracle text is `"This land enters tapped unless you control a Plains or a Swamp."`. This is outdated templating - modern oracle uses "This land enters tapped" not "[Card Name] enters the battlefield tapped".
- Doc comment uses outdated wording (line 7): Says `Isolated Chapel enters the battlefield tapped` instead of `This land enters tapped`.
- Incomplete subtype checking in `controller_has_matching_land` function (lines 21-22): Only checks runtime object subtypes but not registry data
  - Oracle text says: `This land enters tapped unless you control a Plains or a Swamp.`
  - Code does: Only checks `o.subtypes.iter().any(|s| s == "Plains")` and `o.subtypes.iter().any(|s| s == "Swamp")` but doesn't check `registry.card_data()` subtypes. This could miss lands that have Plains/Swamp subtypes in registry but not copied to runtime object (compare with `check_condition` in `state.rs` which correctly checks both sources).

### Tricky interactions checked
- Self-exclusion on ETB: Correctly excludes `object_id` from the matching land check (`o.id != object_id`), preventing self-reference
- Dual/shock lands as enablers: The check uses OR logic to detect any permanent with Plains OR Swamp subtype, correctly handling lands like Godless Shrine
- Mana ability gating: `mana_abilities()` correctly returns empty when not on battlefield or already tapped, with `requires_tap: true` flag
- Simultaneous ETB timing: Lands entering simultaneously should see each other due to event processing order (all moved to battlefield before triggers resolve)
- Player choice for mana production: Two separate ManaAbilityDef entries allow player to choose between {W} and {B}

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic card data: `innistrad_simple_cards.rs:99` (isolated_chapel_card_data)
- Enters tapped without matching land: NOT TESTED
- Enters untapped with Plains: NOT TESTED  
- Enters untapped with Swamp: NOT TESTED
- Mana production abilities: NOT TESTED
- Dual land type recognition: NOT TESTED