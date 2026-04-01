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
