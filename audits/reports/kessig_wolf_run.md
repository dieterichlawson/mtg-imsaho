# Audit: Kessig Wolf Run

## Oracle (Official)
- **Name:** Kessig Wolf Run
- **Cost:** (none — Land)
- **Type:** Land
- **Oracle:** {T}: Add {C}. {X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.
- **P/T:** N/A

## Implementation
- Name: "Kessig Wolf Run" -- CORRECT
- Cost: None -- CORRECT
- Type: Land -- CORRECT
- Mana ability: {T} for {C} -- CORRECT
- Activated ability: simplified as {1}{R}{G},{T} for +1/+0 and trample (can be activated multiple times) -- SIMPLIFICATION noted in oracle_text and comments
- Grants trample via UntilEndOfTurnKeyword -- CORRECT
- Grants +1/+0 via UntilEndOfTurnEffect -- CORRECT

## Issues
1. **ISSUE (simplification):** X in the mana cost is simplified to 1. The engine doesn't support variable X costs for activated abilities. Noted in code.

## Verdict: PASS (with noted simplification)

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: {T}: Add {C}. {X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.
**Scryfall type line**: Land
**Status**: PASS

Findings:
- Cost: None (land): correct.
- Type Land: correct.
- P/T N/A: correct.
- Mana ability: {T} for {C}, requires_tap, checks battlefield and untapped: correct.
- Activated ability: cost includes ManaSymbol::X, ManaSymbol::Colored(Red), ManaSymbol::Colored(Green), requires_tap: correct.
- on_activate_ability reads `last_activated_x_value` for X: correct.
- Grants +X/+0 via UntilEndOfTurnEffect and trample via UntilEndOfTurnKeyword: correct.
- Target requirement: Creature: correct.
- Anti-pattern check: No spell resolution involved (land/activated ability). No `move_object(id, Zone::Graveyard)` misuse.
- No CombatDamageDealt misuse.
- No triggered_abilities declared, none needed: correct.
- sorcery_speed_only: false: correct (the ability can be activated at instant speed).
- Tests found in kessig_wolf_run.rs and tier14_cards.rs.
- Previous simplification note about X=1 appears to be FIXED -- the implementation now correctly uses ManaSymbol::X and reads last_activated_x_value.

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch (https://scryfall.com/card/isd/243/kessig-wolf-run)
**Oracle text**: {T}: Add {C}. {X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.
**Type line**: Land
**Status**: PASS

Findings:
- Cost: None (land): correct.
- Type Land, no subtypes: correct.
- P/T N/A: correct.
- Mana ability: {T} for {C}, requires_tap, checks battlefield and untapped: correct.
- Activated ability: cost includes ManaSymbol::X, ManaSymbol::Colored(Red), ManaSymbol::Colored(Green), requires_tap: correct.
- on_activate_ability reads last_activated_x_value for X: correct.
- Grants +X/+0 via UntilEndOfTurnEffect and trample via UntilEndOfTurnKeyword: correct.
- Target requirement: TargetRequirement::Creature: correct.
- sorcery_speed_only: false: correct (activated ability can be used at instant speed).
- once_per_turn: false: correct (though requires tap, so effectively once per turn).
- Anti-pattern check: No spell resolution involved (land/activated ability). No misuse.
- No CombatDamageDealt misuse.
- No triggered_abilities declared, none needed: correct.
- Tests: 4 tests in kessig_wolf_run.rs (can_activate_with_rg_only, cannot_activate_without_rg, x_equals_3_gives_plus_3, x_equals_0_gives_trample_only) plus tests in tier14_cards.rs. Good coverage.

## Audit — 2026-04-02

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: {T}: Add {C}. {X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.
**Type line**: Land
**Status**: PASS

Findings:
- Card data: name "Kessig Wolf Run", cost None (land), card_types Land, no supertypes/subtypes, no P/T: all correct.
- oracle_text field matches Scryfall verbatim: correct.
- Mana ability (ability_index 0): {T} for {C}, requires_tap true, checks battlefield and untapped: correct.
- Activated ability (ability_index 1): cost ManaSymbol::X + Colored(Red) + Colored(Green), requires_tap true, target TargetRequirement::Creature: correct.
- X cost handling: on_activate_ability reads state.last_activated_x_value, applies power_mod = x, toughness_mod = 0 via UntilEndOfTurnEffect: correct (+X/+0).
- Trample grant: pushes Keyword::Trample via until_end_of_turn_keywords: correct.
- Zone check on target (must be on battlefield) before applying effects: correct.
- sorcery_speed_only: false, once_per_turn: false: correct.
- sacrifice_cost: None: correct.
- Tests: 4 tests all pass (can_activate_with_rg_only, cannot_activate_without_rg, x_equals_3_gives_plus_3, x_equals_0_gives_trample_only).
- No mismatches found.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: {T}: Add {C}.\n{X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.
**Type line**: Land
**Status**: PASS

### Code issues
No issues found. Card data matches oracle: name, no mana cost (Land), type Land. Mana ability 0: tap to add {C} (Colorless). Activated ability 1: {X}{R}{G} + tap, targets Creature. On activation, reads X from last_activated_x_value, grants +X/+0 via UntilEndOfTurnEffect and trample via UntilEndOfTurnKeyword. Both abilities check zone == Battlefield and !tapped. No anti-patterns.

## Audit — 2026-04-03 07:08

**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/243/kessig-wolf-run)
**Oracle text**: `{T}: Add {C}.\n{X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.`
**Type line**: `Land`
**Status**: PASS

### Code issues

None found. Implementation matches oracle text exactly.

- Card data: name, type line (Land), no mana cost, no subtypes/supertypes — all correct.
- Oracle text string in `card_data()` matches Scryfall verbatim.
- Mana ability (ability_index 0): taps for {C}, gated on untapped + battlefield — correct.
- Activated ability (ability_index 1): costs {X}{R}{G} + tap, requires `TargetRequirement::Creature`, grants +X/+0 via `UntilEndOfTurnEffect` and trample via `UntilEndOfTurnKeyword` — correct.
- X value read from `state.last_activated_x_value`, which the engine computes as (total mana in pool) minus (non-X cost of 2) — correct.
- `sorcery_speed_only: false` — activated ability usable at instant speed, correct.
- `once_per_turn: false` — no such restriction on the card, correct.
- `sacrifice_cost: SacrificeCost::None` — correct, no sacrifice required.

### Tricky interactions checked (min 3)

1. **X=0 activation**: With only {R}{G} in pool, X=0 is legal. The creature gets +0/+0 but still gains trample. Engine computes X = 2 - 2 = 0. Verified by `x_equals_0_gives_trample_only` test.
2. **Targets any creature (not just own)**: `TargetRequirement::Creature` allows targeting any creature on the battlefield, including opponent's. This is correct per oracle text ("Target creature" with no restriction).
3. **Until-end-of-turn cleanup**: Both `until_end_of_turn_effects` and `until_end_of_turn_keywords` are cleared during the cleanup step in `engine.rs:3021-3022`. Effects properly expire.
4. **Cannot activate when tapped**: Both `mana_abilities()` and `activated_abilities()` check `!obj.tapped` before returning abilities — prevents double-activation and correctly reflects the tap cost.

### Test coverage

6 tests total, all passing:

**Dedicated tests** (`tests/kessig_wolf_run.rs`):
- `can_activate_with_rg_only` — X=0 activation is legal
- `cannot_activate_without_rg` — must have both R and G
- `x_equals_3_gives_plus_3` — X=3 gives +3/+0 and trample, all mana spent
- `x_equals_0_gives_trample_only` — X=0 gives trample but no power boost

**Tier 14 integration tests** (`tests/tier14_cards.rs`):
- `kessig_wolf_run_grants_power_and_trample` — X=1 with {1}{R}{G}
- `kessig_wolf_run_taps_for_mana` — mana ability produces {C}
