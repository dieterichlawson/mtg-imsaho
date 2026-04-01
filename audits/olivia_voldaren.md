# Audit: Olivia Voldaren

## Official Oracle
- **Name:** Olivia Voldaren
- **Cost:** {2}{B}{R}
- **Type:** Legendary Creature — Vampire
- **Oracle:** Flying. {1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren. {3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.
- **P/T:** 3/3

## Implementation: `mtg-engine/src/cards/olivia_voldaren.rs`
- **Name:** Olivia Voldaren -- CORRECT
- **Cost:** {2}{B}{R} -- CORRECT
- **Type:** Creature, Legendary -- CORRECT
- **Subtypes:** Vampire -- CORRECT
- **P/T:** 3/3 -- CORRECT
- **Keywords:** Flying -- CORRECT

### Ability 0: {1}{R} ping
- **Cost:** {1}{R} -- CORRECT
- **Targets:** Another creature -- CORRECT (enforced in on_activate_ability with self-check)
- **Effect:** 1 damage, makes Vampire, +1/+1 counter on Olivia -- CORRECT
- **NonCombatDamageDealt event:** Emitted -- CORRECT
- **damaged_by tracking:** Added -- CORRECT

### Ability 1: {3}{B}{B} steal
- **Cost:** {3}{B}{B} -- CORRECT
- **Target:** Vampire creature -- CORRECT (checked in on_activate_ability)

## Issues (pre-fix)
1. **Control duration missing:** Oracle says "Gain control of target Vampire **for as long as you control Olivia Voldaren**." The implementation changes controller permanently without the "for as long as" condition. If Olivia leaves the battlefield, the stolen creature should revert to its original controller.
2. **Ability 1 target filter too broad:** The activated ability definition uses `TargetFilter::Any` for ability 1 but should filter to Vampires only. The Vampire check is only in on_activate_ability, which means the AI may try to target non-Vampires and waste the activation.

## Verdict (pre-fix)
**FAIL** -- 2 issues: (1) Steal effect should end when Olivia leaves; (2) Ability 1 target filter should be Vampire-only.

---

## Audit — 2026-04-01

**Scryfall Oracle text**: Flying. {1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren. {3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.
**Scryfall type line**: Legendary Creature — Vampire
**Status**: PASS

Both issues from the previous audit have been fixed:
1. Ability 1 now uses `TargetFilter::HasSubtype("Vampire")` to restrict targeting to Vampires only.
2. Stolen creatures are tracked in Olivia's card_state and returned to their original controllers via `on_leave_battlefield` when Olivia leaves the battlefield.
3. Tests cover: ability 0 damage+vampire+counter, ability 0 can't target self, ability 1 steals vampire, ability 1 rejects non-vampire, stolen creatures return when Olivia leaves, ability 1 target filter requires vampire.

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: Flying / {1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren. / {3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.
**Scryfall type line**: Legendary Creature — Vampire
**Status**: ISSUE

Mana cost {2}{B}{R}: correct. Supertype Legendary: correct. Subtype Vampire: correct. P/T 3/3: correct. Flying keyword: correct.

Ability 0 ({1}{R} ping): Cost correct. Deals 1 damage, adds Vampire subtype, adds +1/+1 counter on Olivia: all correct. Correctly emits `NonCombatDamageDealt` event (not CombatDamageDealt). "Another" restriction enforced in `on_activate_ability` with `*target_id == object_id` check: correct. `LeavesBattlefield` triggered ability declared for returning stolen creatures: correct.

Ability 1 ({3}{B}{B} steal): Cost correct. Target filter `HasSubtype("Vampire")`: correct. Steals creature, tracks original controller, returns on Olivia leaving: correct.

Issues found:
1. **Ability 0 target filter is TargetFilter::Any instead of excluding self**: The `ActivatedAbilityDef` for ability 0 uses `TargetFilter::Any` which allows targeting any creature including Olivia herself at the UI/targeting level. The "another" restriction is only enforced in `on_activate_ability` (line 100). This means the engine may present self-targeting as a valid option before the ability resolves. The filter should ideally exclude self at the definition level for correct target presentation.

Tests present in `tests/olivia_voldaren.rs` and `tests/tier14_cards.rs`. No graveyard move anti-pattern (not a spell).

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch
**Oracle text**: Flying. {1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren. {3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.
**Type line**: Legendary Creature — Vampire
**Status**: ISSUE

Mana cost {2}{B}{R}: correct. Supertype Legendary: correct. Subtype Vampire: correct. P/T 3/3: correct. Flying keyword: correct.

Ability 0 ({1}{R}): Cost correct. `requires_tap: false`: correct (not a tap ability). Deals 1 damage, adds Vampire subtype, adds +1/+1 counter on Olivia: all correct. Emits `NonCombatDamageDealt` event: correct. Tracks `damaged_by`: correct. "Another" self-targeting check (`*target_id == object_id` returns early): correct.

Ability 1 ({3}{B}{B}): Cost correct. `requires_tap: false`: correct. Target filter `HasSubtype("Vampire")`: correct. Steals Vampire, tracks original controller in card_state, returns on Olivia leaving battlefield via `on_leave_battlefield`: correct ("for as long as you control Olivia Voldaren"). `LeavesBattlefield` triggered ability declared: correct.

Issues found:
1. **Ability 0 target filter is `TargetFilter::Any` instead of excluding self** (persists from prior audit): The `ActivatedAbilityDef` for ability 0 uses `TargetFilter::Any`, which means the UI/targeting system would present Olivia herself as a valid target. The "another" restriction is only enforced in `on_activate_ability` (line 100). The filter should ideally use a filter that excludes self at the definition level for correct target presentation. Functionally the ability works correctly since the self-target is rejected at resolution, but it's a UI/targeting correctness issue.

Tests in `tests/olivia_voldaren.rs` cover: ability 0 damage+vampire+counter, ability 0 can't target self, ability 1 steals vampire, ability 1 rejects non-vampire, stolen creatures return when Olivia leaves, ability 1 target filter requires vampire. No graveyard or damage anti-patterns.

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch
**Oracle text**: Flying / {1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren. / {3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.
**Type line**: Legendary Creature — Vampire
**Status**: ISSUE

Mana cost {2}{B}{R}: correct. Supertype Legendary: correct. Subtype Vampire: correct. P/T 3/3: correct. Flying keyword: correct.

Ability 0 ({1}{R}): Cost correct. Deals 1 damage: correct. Adds Vampire subtype (in addition to existing types): correct. Adds +1/+1 counter on Olivia: correct. Emits `NonCombatDamageDealt` event: correct (not CombatDamageDealt). Tracks `damaged_by`: correct. "Another" restriction enforced in `on_activate_ability` with `*target_id == object_id` check: correct.

Ability 1 ({3}{B}{B}): Cost correct. Target filter `HasSubtype("Vampire")`: correct. Steals creature, tracks original controller via card_state, returns on Olivia leaving via `on_leave_battlefield`: correct. `LeavesBattlefield` triggered ability declared: correct.

Per Scryfall ruling: "If Olivia Voldaren deals lethal damage to a creature with its first activated ability, that creature will become a Vampire before dying." The code sets damage and adds Vampire subtype in the same block before SBAs would be checked: correct.

Issues found (persisting from prior audit):
1. **Ability 0 target filter is TargetFilter::Any instead of excluding self**: The `ActivatedAbilityDef` for ability 0 uses `TargetFilter::Any` which allows the UI/targeting system to present Olivia as a valid target. The "another" restriction is only enforced in `on_activate_ability` (line 100). Ideally the filter should exclude self at the definition level for correct target presentation. Low severity since the ability resolves correctly.

Tests in `tests/olivia_voldaren.rs`: 6 tests covering damage+vampire+counter, self-targeting rejection, vampire stealing, non-vampire rejection, stolen creature return on leave, and target filter validation. Good coverage. No graveyard anti-patterns.

## Audit — 2026-04-01 12:00

**Oracle text source**: Scryfall via WebSearch
**Oracle text**: Flying\n{1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren.\n{3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.
**Type line**: Legendary Creature — Vampire
**Status**: ISSUE

Mana cost {2}{B}{R}: correct. Supertype Legendary: correct. Subtype Vampire: correct (only subtype). P/T 3/3: correct. Flying keyword: correct.

Ability 0 ({1}{R}): Cost correct. `requires_tap: false`: correct. Deals 1 damage: correct. Emits `NonCombatDamageDealt` event: correct. Tracks `damaged_by`: correct. Adds Vampire subtype in addition to existing types: correct. Adds +1/+1 counter on Olivia: correct. "Another" restriction enforced via `*target_id == object_id` check in `on_activate_ability` (line 100): functionally correct.

Ability 1 ({3}{B}{B}): Cost correct. `requires_tap: false`: correct. Target filter `HasSubtype("Vampire")`: correct. Changes controller: correct. Tracks original controller in card_state for reversion: correct. `on_leave_battlefield` returns stolen creatures: correct ("for as long as you control Olivia Voldaren"). `LeavesBattlefield` triggered ability declared: correct.

Per Scryfall ruling: "If Olivia Voldaren deals lethal damage to a creature with its first activated ability, that creature will become a Vampire before dying." Code sets damage and adds Vampire subtype in the same block before SBAs: correct.

Tests in `tests/olivia_voldaren.rs`: 6 tests covering ability 0 (damage+vampire+counter), self-targeting rejection, ability 1 (steal vampire, reject non-vampire), stolen creatures return on leave, target filter validation. Good coverage.

Issues found:
1. **Ability 0 target filter is `TargetFilter::Any` instead of excluding self** (`/home/user/mtg-imsaho/mtg-engine/src/cards/olivia_voldaren.rs`, line 59):
   - Oracle text says: `{1}{R}: Olivia Voldaren deals 1 damage to another target creature.`
   - Code does: `target_requirement: Some(TargetRequirement::CreatureWithFilter(TargetFilter::Any))` at the ability definition level (line 59), which allows the UI/targeting system to present Olivia herself as a valid target. The "another" restriction is only enforced at resolution in `on_activate_ability` (line 100: `if *target_id == object_id { return; }`). Functionally correct but the targeting system would show an invalid option. Low severity.

## Audit — 2026-04-01 18:00

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying
{1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren.
{3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.
**Type line**: Legendary Creature — Vampire
**Status**: ISSUE

### Code issues
1. **Ability 0 target filter does not exclude self at definition level** (`/Users/dlaw/mtg/mtg-engine/src/cards/isd/olivia_voldaren.rs`, line 59):
   - Oracle text says: `{1}{R}: Olivia Voldaren deals 1 damage to another target creature.`
   - Code does: `target_requirement: Some(TargetRequirement::CreatureWithFilter(TargetFilter::Any))` -- uses `TargetFilter::Any` which does not exclude self. The "another" restriction is only enforced at resolution in `on_activate_ability` (line 100: `if *target_id == object_id { return; }`). The engine's `generate_ability_targets` function (engine.rs line 980-986) will present Olivia herself as a valid target for ability 0. Low severity since the ability correctly no-ops when self-targeting.

2. **Engine does not apply CreatureWithFilter's filter in generate_ability_targets** (`/Users/dlaw/mtg/mtg-engine/src/engine.rs`, line 980):
   - The `generate_ability_targets` function handles `CreatureWithFilter(_)` identically to `Creature` (line 980: `TargetRequirement::Creature | TargetRequirement::CreatureWithFilter(_)`), never calling `matches_target_filter`. For ability 1 with `TargetFilter::HasSubtype("Vampire")`, this means non-Vampire creatures are presented as valid targets. The Vampire check is only enforced at resolution in `on_activate_ability` (line 129-132). This is an engine-level issue that affects Olivia's ability 1 targeting correctness.

All other card data verified correct: name, mana cost {2}{B}{R}, Legendary supertype, Vampire subtype, P/T 3/3, Flying keyword, oracle text matches. Ability costs correct. `NonCombatDamageDealt` event emitted (not `CombatDamageDealt`). `damaged_by` tracked. Vampire subtype addition preserves existing subtypes. +1/+1 counter on Olivia correct. `LeavesBattlefield` triggered ability declared. `on_leave_battlefield` returns stolen creatures to original controllers. Per ruling: "If Olivia Voldaren deals lethal damage to a creature with its first activated ability, that creature will become a Vampire before dying" -- code sets damage and adds subtype in same block before SBAs: correct.

### Tricky interactions checked
- "Another" self-exclusion at resolution level: pass (functionally correct)
- "Another" self-exclusion at targeting level: ISSUE (see #1)
- Vampire subtype added "in addition to" existing types: pass
- Stolen creatures returned when Olivia leaves: pass
- NonCombatDamageDealt (not Combat): pass
- damaged_by tracking: pass
- Lethal damage + Vampire subtype ordering: pass
- CreatureWithFilter filter not applied by engine: ISSUE (see #2)

### Test coverage
- Ability 0: damage + Vampire + counter: `tests/olivia_voldaren.rs` (line 23)
- Ability 0: cannot target self: `tests/olivia_voldaren.rs` (line 51)
- Ability 1: steals Vampire: `tests/olivia_voldaren.rs` (line 68)
- Ability 1: rejects non-Vampire: `tests/olivia_voldaren.rs` (line 86)
- Stolen creatures return when Olivia leaves: `tests/olivia_voldaren.rs` (line 104)
- Ability 1 target filter requires Vampire: `tests/olivia_voldaren.rs` (line 134)
- Ability 0 via engine submit_action: `tests/tier14_cards.rs` (line 463)
- Ability 1 via engine submit_action: `tests/tier14_cards.rs` (line 500)
- Olivia leaves without controlling stolen creatures: NOT TESTED
- Ruling: losing control of Olivia before ability 1 resolves: NOT TESTED
- Engine presents self as valid target for ability 0: NOT TESTED
- Engine presents non-Vampires as valid targets for ability 1: NOT TESTED

## Re-Audit — 2026-04-01 20:00

**Oracle text source**: Scryfall API (via oracle_lookup.py)
**Oracle text**: Flying
{1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren.
{3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.
**Type line**: Legendary Creature — Vampire
**Status**: PASS

### Code issues
No issues found. Both issues from the prior audit (2026-04-01 18:00) have been fixed:

1. **Ability 0 target filter now uses `TargetFilter::Another`** (`olivia_voldaren.rs` line 59): Previously `TargetFilter::Any`, now correctly excludes self at the targeting level. The engine's `matches_ability_target_filter` function (engine.rs line 1010) implements `Another` as `obj.id != source_id`.

2. **Engine now applies `CreatureWithFilter` filter for activated abilities** (engine.rs lines 1068-1075): Previously the engine handled `CreatureWithFilter(_)` identically to `Creature` in `generate_ability_targets`. Now there is a dedicated match arm that calls `matches_ability_target_filter`, so ability 1's `HasSubtype("Vampire")` filter is properly applied at the targeting level.

All card data verified correct: name "Olivia Voldaren", mana cost {2}{B}{R}, Legendary supertype, Vampire subtype, P/T 3/3, Flying keyword, oracle text matches. Ability 0 cost {1}{R}: correct, `requires_tap: false`: correct. Ability 1 cost {3}{B}{B}: correct, `requires_tap: false`: correct. `NonCombatDamageDealt` event emitted: correct. `damaged_by` tracked: correct. Vampire subtype addition preserves existing subtypes: correct. +1/+1 counter on Olivia via `state.add_counters`: correct. `LeavesBattlefield` triggered ability declared: correct. `on_leave_battlefield` returns stolen creatures to original controllers: correct.

### Tricky interactions checked
- "Another" self-exclusion at targeting level: pass (TargetFilter::Another, line 59)
- "Another" self-exclusion at resolution level: pass (line 100 check)
- Vampire subtype added "in addition to" existing types: pass
- Stolen creatures returned when Olivia leaves: pass
- NonCombatDamageDealt (not CombatDamageDealt): pass
- damaged_by tracking: pass
- Lethal damage + Vampire subtype before SBAs: pass (same block, per ruling)
- CreatureWithFilter(HasSubtype("Vampire")) applied for ability 1: pass

### Test coverage
- Ability 0: damage + Vampire + counter: `tests/olivia_voldaren.rs:23`
- Ability 0: cannot target self: `tests/olivia_voldaren.rs:51`
- Ability 1: steals Vampire: `tests/olivia_voldaren.rs:68`
- Ability 1: rejects non-Vampire: `tests/olivia_voldaren.rs:86`
- Stolen creatures return when Olivia leaves: `tests/olivia_voldaren.rs:104`
- Ability 1 target filter requires Vampire: `tests/olivia_voldaren.rs:134`
- Ruling: losing control of Olivia before ability 1 resolves: NOT TESTED
- Ruling: lethal damage creature becomes Vampire before dying: NOT TESTED (covered by code structure)

## Audit — 2026-04-01 21:00

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying
{1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren.
{3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.
**Type line**: Legendary Creature — Vampire
**Status**: PASS

### Code issues
No issues found. All card data and behavior verified correct:

- Name "Olivia Voldaren": correct
- Mana cost {2}{B}{R}: correct (Generic(2), Black, Red)
- Supertype Legendary: correct
- Subtype Vampire: correct (single subtype per oracle type line "Legendary Creature -- Vampire")
- P/T 3/3: correct
- Flying keyword: correct
- Oracle text: correct

Ability 0 ({1}{R}):
- Cost {1}{R}: correct. `requires_tap: false`: correct (no tap symbol in oracle)
- Target filter `TargetFilter::Another`: correctly excludes self at targeting level (line 59)
- Deals 1 damage (`damage_marked += 1`): correct. Tracks `damaged_by`: correct
- Emits `NonCombatDamageDealt` event (not `CombatDamageDealt`): correct
- Adds Vampire subtype with "in addition to" check (`!obj.subtypes.contains`): correct
- Adds +1/+1 counter via `state.add_counters`: correct
- Per ruling: creature becomes Vampire before dying from lethal damage -- code sets damage and subtype in same block before SBAs: correct

Ability 1 ({3}{B}{B}):
- Cost {3}{B}{B}: correct. `requires_tap: false`: correct
- Target filter `HasSubtype("Vampire")`: correctly restricts to Vampires at targeting level
- Changes controller to Olivia's controller: correct
- Tracks stolen creatures and original controllers in `card_state`: correct
- `on_leave_battlefield` returns all stolen creatures to original controllers: correct ("for as long as you control Olivia Voldaren")
- `LeavesBattlefield` triggered ability declared in `triggered_abilities`: correct

Note: The ruling "If you activate Olivia Voldaren's last ability, and before that ability resolves you lose control of Olivia Voldaren, the ability will resolve with no effect" is a stack-based interaction. In this engine, activated abilities resolve immediately (not placed on stack as separate items), so this interaction cannot occur. This is an engine-level limitation, not a card implementation bug.

### Tricky interactions checked
- "Another" self-exclusion at targeting level: pass (TargetFilter::Another, line 59)
- "Another" self-exclusion at resolution level: pass (line 100 check)
- Vampire subtype added "in addition to" existing types: pass
- Stolen creatures returned when Olivia leaves: pass
- NonCombatDamageDealt (not CombatDamageDealt): pass
- damaged_by tracking: pass
- Lethal damage + Vampire subtype before SBAs: pass (per ruling)
- CreatureWithFilter(HasSubtype("Vampire")) applied for ability 1: pass

### Test coverage
- Ability 0: damage + Vampire + counter: `tests/olivia_voldaren.rs:23`
- Ability 0: cannot target self: `tests/olivia_voldaren.rs:51`
- Ability 1: steals Vampire: `tests/olivia_voldaren.rs:68`
- Ability 1: rejects non-Vampire: `tests/olivia_voldaren.rs:86`
- Stolen creatures return when Olivia leaves: `tests/olivia_voldaren.rs:104`
- Ability 1 target filter requires Vampire: `tests/olivia_voldaren.rs:134`
- Ruling: losing control of Olivia before ability 1 resolves: NOT TESTED (engine limitation)
- Ruling: lethal damage creature becomes Vampire before dying: NOT TESTED (covered by code structure)

## Audit — 2026-04-01 22:00

**Oracle text source**: Scryfall API (via oracle_lookup.py cache)
**Oracle text**: Flying
{1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren.
{3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.
**Type line**: Legendary Creature — Vampire
**Status**: PASS

### Code issues
No issues found. All card data and behavior verified correct:

- Name "Olivia Voldaren": correct
- Mana cost {2}{B}{R}: correct (Generic(2), Black, Red)
- Supertype Legendary: correct
- Subtype Vampire: correct (single subtype)
- P/T 3/3: correct
- Flying keyword: correct
- Oracle text: correct

Ability 0 ({1}{R}):
- Cost {1}{R}: correct. `requires_tap: false`: correct. `once_per_turn: false`: correct. `sorcery_speed_only: false`: correct.
- Target filter `TargetFilter::Another` (line 59): correctly excludes self at targeting level
- Resolution check `*target_id == object_id` (line 100): correctly rejects self-targeting at resolution as a safety check
- Deals 1 damage (`damage_marked += 1`): correct
- Tracks `damaged_by`: correct (line 106: `obj.damaged_by.push(object_id)`)
- Emits `NonCombatDamageDealt` event (line 112): correct (not `CombatDamageDealt`)
- Adds Vampire subtype with "in addition to" check (`!obj.subtypes.contains`, line 108): correct
- Adds +1/+1 counter on Olivia via `state.add_counters` (line 118): correct
- Per ruling: "If Olivia Voldaren deals lethal damage to a creature with its first activated ability, that creature will become a Vampire before dying." Code sets damage and adds Vampire subtype in the same block before SBAs are checked: correct

Ability 1 ({3}{B}{B}):
- Cost {3}{B}{B}: correct. `requires_tap: false`: correct.
- Target filter `HasSubtype("Vampire")` (line 75): correctly restricts to Vampires at targeting level
- Changes controller to Olivia's controller (line 137): correct
- Tracks stolen creatures and original controllers in `card_state` (lines 144-153): correct
- `on_leave_battlefield` returns all stolen creatures to original controllers (lines 161-195): correct ("for as long as you control Olivia Voldaren")
- `LeavesBattlefield` triggered ability declared in `triggered_abilities` (lines 33-36): correct

Engine note: Activated abilities resolve immediately in this engine (not placed on the stack). The ruling "If you activate Olivia Voldaren's last ability, and before that ability resolves you lose control of Olivia Voldaren, the ability will resolve with no effect" cannot occur. This is an engine-level simplification, not a card implementation bug.

### Tricky interactions checked
- "Another" self-exclusion at targeting level: pass (TargetFilter::Another, line 59)
- "Another" self-exclusion at resolution level: pass (line 100 early return)
- Vampire subtype added "in addition to" existing types: pass
- Stolen creatures returned when Olivia leaves: pass
- NonCombatDamageDealt (not CombatDamageDealt): pass
- damaged_by tracking: pass
- Lethal damage + Vampire subtype before SBAs: pass (per ruling)
- CreatureWithFilter(HasSubtype("Vampire")) applied for ability 1 targeting: pass

### Test coverage
- Ability 0: damage + Vampire + counter: `tests/olivia_voldaren.rs:23`
- Ability 0: cannot target self: `tests/olivia_voldaren.rs:51`
- Ability 1: steals Vampire: `tests/olivia_voldaren.rs:68`
- Ability 1: rejects non-Vampire: `tests/olivia_voldaren.rs:86`
- Stolen creatures return when Olivia leaves: `tests/olivia_voldaren.rs:104`
- Ability 1 target filter requires Vampire: `tests/olivia_voldaren.rs:134`
- Ruling: losing control of Olivia before ability 1 resolves: NOT TESTED (engine limitation)
- Ruling: lethal damage creature becomes Vampire before dying: NOT TESTED (covered by code structure)
- Not in LLM card knowledge: acceptable (complex card, AI can read oracle text)

## Audit — 2026-04-01 16:00

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying
{1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren.
{3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.
**Type line**: Legendary Creature — Vampire
**P/T**: 3/3
**Status**: PASS

### Code issues
No issues found.

Card data is correct: {2}{B}{R}, Legendary Creature - Vampire, 3/3, Flying keyword. Both activated abilities have correct costs, targeting, and effects. Ability 0 correctly uses TargetFilter::Another to enforce "another target creature". Ability 1 correctly uses TargetFilter::HasSubtype("Vampire") to restrict to Vampires. The on_activate_ability correctly: (1) deals 1 damage with damage_marked and damaged_by, (2) adds Vampire subtype while preserving existing subtypes, (3) adds +1/+1 counter on Olivia, (4) emits NonCombatDamageDealt event. Ability 1 correctly changes controller and tracks original controller for reversion. The on_leave_battlefield correctly returns all stolen creatures to original controllers.

Note: The "for as long as you control Olivia Voldaren" condition is only partially implemented. It handles Olivia leaving the battlefield but does not handle Olivia changing controllers (e.g., opponent steals Olivia). This would require a controller-change hook that doesn't exist in the engine. This is an engine-level limitation, not a card-specific bug.

Note: Activated abilities resolve immediately via on_activate_ability rather than going on the stack. This is an engine-level simplification that affects the ruling about losing control of Olivia before ability 1 resolves. Not a card-specific issue.

### Tricky interactions checked
- "Another" target restriction on ability 0: PASS - TargetFilter::Another and runtime check at line 100
- Creature becomes Vampire "in addition to its other types": PASS - subtypes.push without clearing existing
- NonCombatDamageDealt event (not CombatDamageDealt): PASS - line 112
- damaged_by tracked for death triggers: PASS - line 106
- Ruling: lethal damage target becomes Vampire before dying: PASS - subtype added in same block as damage
- Stolen creatures return when Olivia leaves: PASS - on_leave_battlefield at line 161
- move_spell_after_resolve not needed (creature): PASS

### Test coverage
- Ability 0 deals damage, makes Vampire, +1/+1: `olivia_voldaren.rs:23` and `tier14_cards.rs:463`
- Ability 0 cannot target self: `olivia_voldaren.rs:51`
- Ability 1 steals Vampire: `olivia_voldaren.rs:68` and `tier14_cards.rs:500`
- Ability 1 rejects non-Vampire: `olivia_voldaren.rs:86`
- Stolen creatures return when Olivia leaves: `olivia_voldaren.rs:104`
- Ability 1 target filter requires Vampire: `olivia_voldaren.rs:134`
- Ruling: lethal damage creature becomes Vampire before dying: NOT TESTED (covered by code structure)
- Ruling: losing control of Olivia before ability 1 resolves: NOT TESTED (engine limitation)
