# Master Work Items List (2026-03-29)

## Engine Bugs

| # | Item | Severity |
|---|------|----------|
| 1 | Implement legend rule in SBAs | Critical |
| 2 | Implement +1/+1 and -1/-1 counter annihilation in SBAs | Critical |
| 3 | Implement spell fizzle (counter by game rules when all targets illegal) | High |
| 4 | Fix combat step skipping when no attackers declared | High |
| 5 | Implement APNAP trigger ordering | Medium |
| 6 | Fix cleanup step to loop on SBAs/triggers | Medium |
| 7 | Fix mana pool emptying at step boundaries (not just phases) | Medium |
| 8 | Add empty-stack check for sorcery-speed casting | Medium |
| 9 | Ensure zone changes create new object identity (blink correctness) | Medium |
| 10 | Remove redundant resolve_top_of_stack call | Low |

## Card Bugs

| # | Item | Severity |
|---|------|----------|
| 11 | Fix Falkenrath Noble -- trigger only on creatures you control | Critical |
| 12 | Fix Spectral Flight -- apply +2/+2 (currently only grants Flying) | High |
| 13 | Fix Furor of the Bitten -- apply +2/+2 and "attacks each combat" | High |
| 14 | Fix Bonds of Faith -- apply +2/+2 to Humans | High |
| 15 | Fix Claustrophobia oracle text -- add ETB tap clause | Low |

## Code Simplicity

| # | Item | Impact |
|---|------|--------|
| 16 | Extract shared aura resolve helper to eliminate 27-file boilerplate (~540 LOC) | High |
| 17 | Add resolve_damage_spell() helper for 8+ damage cards | High |
| 18 | Add resolve_destroy_spell() helper for 8+ destruction cards | High |
| 19 | Split engine.rs (~1,400 lines) into focused submodules | High |
| 20 | Replace oracle text string checks with CardBehavior trait methods | High |
| 21 | Replace string-based P/T bonus parsing with structured CardData fields | Medium |
| 22 | Consolidate anthem_power_bonus() / anthem_toughness_bonus() | Medium |
| 23 | Replace unwrap() calls in submit_action with descriptive errors | Medium |
| 24 | Add card-specific persistent state to GameObject (Fiend Hunter hack) | Medium |
| 25 | Expand test helpers in common/mod.rs | Low |

## Testing Infrastructure

| # | Item | Impact |
|---|------|--------|
| 26 | Create ScriptedPlayer for deterministic AI tests | High |
