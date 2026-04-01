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
