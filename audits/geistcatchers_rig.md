## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: When this creature enters, you may have it deal 4 damage to target creature with flying.
**Type line**: Artifact Creature — Construct
**Status**: PASS
### Code issues
No issues found.

---

## Audit — 2026-04-02 21:09
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/223/geistcatchers-rig), cached 2026-04-01
**Oracle text**: When this creature enters, you may have it deal 4 damage to target creature with flying.
**Type line**: Artifact Creature — Construct
**Status**: PASS

### Code issues

1. **Style inconsistency (minor, non-functional)**: Implementation uses inline `AwaitingAction`/`ResolutionChoiceKind` construction instead of the helper functions `controller_of()` and `present_optional_target_choice()` used by comparable cards (e.g., Slayer of the Wicked, Morkrut Banshee). Functionally equivalent but deviates from codebase conventions.

2. **Oracle text wording (cosmetic)**: Code uses `"When Geistcatcher's Rig enters the battlefield"` while modern oracle text reads `"When this creature enters"`. Functionally identical.

### Tricky interactions checked (min 3)

1. **Targets any creature with flying, not just opponent's**: The filter at line 40-43 has no controller restriction, correctly allowing the player to target their own creatures with flying. Verified correct per oracle text ("target creature with flying" has no ownership restriction).

2. **"You may" timing vs. targeting**: Per the 2011-09-22 ruling, the target is chosen when the ability triggers, but the decision to deal damage is made on resolution. The implementation combines both into a single `optional: true` `ChooseTarget` step, allowing the player to skip targeting entirely rather than being forced to target but allowed to decline damage. This is a known engine simplification consistent with how other "you may" ETB cards work in this codebase (e.g., Slayer of the Wicked). In practice the outcome is the same -- the player can always avoid dealing damage.

3. **Flying detection via `has_keyword`**: The implementation correctly uses `state.has_keyword(o.id, Keyword::Flying, registry)` which checks runtime keyword state (including temporary keyword removal via `until_end_of_turn_removed_keywords`). A creature that has had flying removed until end of turn would correctly not be a valid target.

4. **Self-targeting exclusion**: The `o.id != object_id` filter prevents targeting itself. While Geistcatcher's Rig has no flying and would already be filtered out by the flying check, the explicit exclusion is harmless.

### Test coverage

**No tests exist.** There are zero unit tests or integration tests for Geistcatcher's Rig in the test suite.
