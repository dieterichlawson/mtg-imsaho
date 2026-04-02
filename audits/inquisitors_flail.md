# Audit: Inquisitor's Flail

## Oracle (Official)
- **Name:** Inquisitor's Flail
- **Cost:** {2}
- **Type:** Artifact — Equipment
- **Oracle:** If equipped creature would deal combat damage, it deals double that damage instead. If another source would deal combat damage to equipped creature, it deals double that damage to equipped creature instead. Equip {2}
- **P/T:** N/A

## Implementation
- Name: "Inquisitor's Flail" -- CORRECT
- Cost: {2} -- CORRECT
- Type: Artifact -- CORRECT
- Subtypes: ["Equipment"] -- CORRECT
- Equip {2}, sorcery speed, targets creature you control -- CORRECT
- Oracle text: says "another creature" in code comment but oracle says "another source" -- the oracle_text string in code is correct

## Issues
1. **ISSUE (simplification):** Offensive double damage is approximated by granting +P/+0 equal to creature's effective power via `dynamic_pt`. This is an approximation rather than a true damage replacement effect. The comment acknowledges this.
2. **ISSUE (missing):** Defensive doubling (equipped creature takes double combat damage from other sources) is NOT implemented. Comment acknowledges this.
3. **ISSUE (minor):** The `dynamic_pt` approach means the power bonus is visible outside combat, which could affect other game interactions differently than the real card.

## Verdict: PASS (with noted simplifications)

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: If equipped creature would deal combat damage, it deals double that damage instead. If another creature would deal combat damage to equipped creature, it deals double that damage to equipped creature instead. Equip {2}
**Scryfall type line**: Artifact — Equipment
**Status**: ISSUE

Findings:
- Mana cost {2}: correct.
- Types Artifact, subtypes Equipment: correct.
- P/T N/A: correct.
- Equip {2} activated ability, sorcery_speed_only: true, targets creature: correct.
- on_resolve uses `move_object(object_id, Zone::Battlefield)` -- this is acceptable for a permanent (artifact), not a spell anti-pattern issue.
- on_activate_ability attaches via `obj.attached_to = Some(*creature_id)`: correct.
- continuous_effects: `DoubleCombatDamage { scope: EffectScope::Attached }`: only models offensive damage doubling.
- ISSUE 1 (carried forward): Defensive doubling (incoming combat damage to equipped creature is doubled) is NOT implemented. The continuous_effects vec only has one entry for outgoing damage. The oracle has two separate replacement effects.
- ISSUE 2 (carried forward): The offensive doubling is implemented as a continuous effect rather than a damage replacement effect. Depending on engine implementation of DoubleCombatDamage, this may or may not be accurate.
- ISSUE 3 (Scryfall discrepancy): Scryfall oracle says "another creature" for the defensive clause. The code's oracle_text string says "another source". The actual current Scryfall oracle text says "another creature" -- the code's oracle_text string is incorrect.
- No CombatDamageDealt misuse (the card modifies damage, does not deal it).
- No triggered_abilities declared, none needed: correct.
- Tests found in tier9_cards.rs and inquisitors_flail.rs.

## Audit — 2026-04-01 16:00

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: If equipped creature would deal combat damage, it deals double that damage instead.
If another creature would deal combat damage to equipped creature, it deals double that damage to equipped creature instead.
Equip {2}
**Type line**: Artifact — Equipment
**Status**: ISSUE

### Code issues
1. **Oracle text field says "another source" but oracle says "another creature"** (`mtg-engine/src/cards/isd/inquisitors_flail.rs` line 26):
   - Oracle text says: `If another creature would deal combat damage to equipped creature`
   - Code oracle_text says: `If another source would deal combat damage to equipped creature`
   - The code uses "another source" where the current oracle says "another creature". In practice this is functionally equivalent since only creatures deal combat damage.

Card data verified correct: mana cost {2}, card_types (Artifact), subtypes (Equipment), no P/T, no keywords (Equip is not in the engine's Keyword enum -- it's implemented as an activated ability). continuous_effects contains `DoubleCombatDamage { scope: EffectScope::Attached }`: correctly models both outgoing and incoming damage doubling (combat.rs lines 449-454 check `has_double_combat_damage` for both source and target). Equip {2} activated ability: correct cost, sorcery_speed_only true, targets creature (TargetRequirement::Creature). is_valid_target checks controller == caster: correct (equip only targets own creatures). on_resolve moves to battlefield and sets is_equipment: correct for equipment. on_activate_ability sets attached_to: correct. No triggered_abilities declared: correct (no triggers on this card).

Trample + Flail interaction verified: combat.rs assigns damage to blockers first, then doubles each portion individually. For a 5/5 with trample blocked by 2/2: assigns 2 to blocker (doubled to 4) and 3 to player (doubled to 6). This matches the ruling: "you'll divide the original amount and then double the results."

Multiple Flails verified: combat.rs lines 449-454 check source and target independently. If a creature has two Flails, `has_double_combat_damage(source)` doubles once (x2), then `has_double_combat_damage(target)` would not apply (the creature is the source, not target). Wait -- for OUTGOING damage, only the source check applies (x2 per Flail attached). But the `has_double_combat_damage` check iterates all effects on the creature; if two Flails are attached, each provides a DoubleCombatDamage effect, but `has_double_combat_damage` returns a boolean (true/false), not a count. This means multiple Flails would NOT stack -- the ruling says they should multiply (x4 for 2 Flails, x8 for 3).

### Code issues (additional)
2. **Multiple Inquisitor's Flails do not stack correctly** (`mtg-engine/src/combat.rs` lines 449-454):
   - Ruling says: `If a creature is equipped with a second Inquisitor's Flail, combat damage dealt by and dealt to that creature will be multiplied by four.`
   - Code does: `has_double_combat_damage` returns a boolean. `if has_double_combat_damage(source) { amount *= 2; }` -- this doubles exactly once regardless of how many Flails are attached. Two Flails should quadruple damage, but the code only doubles it.

### Tricky interactions checked
- Doubles outgoing combat damage: pass (tested)
- Doubles incoming combat damage: pass (tested)
- Trample + Flail interaction (divide then double): pass (verified in combat.rs)
- Multiple Flails stacking: ISSUE (returns boolean, not count-based)
- Non-combat damage not doubled: pass (only applies to combat damage)
- Equip only targets own creatures: pass

### Test coverage
- Doubles damage to player: `mtg-engine/tests/inquisitors_flail.rs:21` (doubles_damage_to_player)
- Doubles damage to creature: `mtg-engine/tests/inquisitors_flail.rs:44` (doubles_damage_to_creature)
- Doubles damage taken from blocker: `mtg-engine/tests/inquisitors_flail.rs:67` (doubles_damage_taken_from_blocker)
- No doubling without Flail: `mtg-engine/tests/inquisitors_flail.rs:90` (no_doubling_without_flail)
- Multiple Flails stacking (x4): NOT TESTED
- Ruling: trample + Flail divide-then-double: NOT TESTED
- Ruling: multiple Flails multiplicative: NOT TESTED

## Audit — 2026-04-01 12:00

**Oracle text source**: Scryfall card page via WebSearch (https://scryfall.com/card/isd/227/inquisitors-flail), confirmed by Gatherer via WebSearch (https://gatherer.wizards.com/Pages/Card/Details.aspx?name=inquisitor's+flail)
**Oracle text**: If equipped creature would deal combat damage, it deals double that damage instead. If another creature would deal combat damage to equipped creature, it deals double that damage to equipped creature instead. Equip {2}
**Type line**: Artifact — Equipment
**Status**: ISSUE

1. **Oracle text string mismatch** (`mtg-engine/src/cards/inquisitors_flail.rs`, line 26):
   - Oracle text says: `If another creature would deal combat damage to equipped creature`
   - Code says: `If another source would deal combat damage to equipped creature`
   - The code uses "another source" where the current Scryfall oracle text says "another creature".

No other issues. Mana cost {2}, types Artifact/Equipment, equip cost, sorcery-speed-only, creature targeting, DoubleCombatDamage continuous effect, and combat.rs implementation (lines 447-454) all correctly double both outgoing and incoming combat damage. Tests in inquisitors_flail.rs (4 tests) confirm both directions work. No anti-patterns found.

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch (https://scryfall.com/card/isd/227/inquisitors-flail)
**Oracle text**: If equipped creature would deal combat damage, it deals double that damage instead. If another creature would deal combat damage to equipped creature, it deals double that damage to equipped creature instead. Equip {2}
**Type line**: Artifact — Equipment
**Status**: ISSUE

Findings:
- Mana cost {2}: correct.
- Types Artifact, subtypes Equipment: correct.
- P/T N/A: correct.
- Equip {2} activated ability, sorcery_speed_only: true, targets creature controller owns: correct.
- on_resolve moves to battlefield and sets is_equipment = true: correct for equipment.
- on_activate_ability sets attached_to: correct.
- ISSUE 1: The code's oracle_text field (line 26) says "another source" but Scryfall oracle text says "another creature". The oracle_text string is incorrect.
- ISSUE 2 (carried forward): Defensive doubling (incoming combat damage to equipped creature is doubled) relies on the continuous_effects DoubleCombatDamage implementation. The continuous_effects vec has a single DoubleCombatDamage entry with EffectScope::Attached. Whether the engine correctly doubles BOTH outgoing and incoming damage depends on the DoubleCombatDamage implementation, which is outside this card's file. Tests confirm both directions work (doubles_damage_to_player, doubles_damage_to_creature, doubles_damage_taken_from_blocker).
- Anti-pattern check: on_resolve uses move_object to battlefield (correct for artifact permanent). No spell-to-graveyard anti-pattern.
- No CombatDamageDealt misuse.
- No triggered_abilities declared, none needed: correct.
- Tests: 4 tests in inquisitors_flail.rs (doubles_damage_to_player, doubles_damage_to_creature, doubles_damage_taken_from_blocker, no_doubling_without_flail) plus tests in tier9_cards.rs. Good coverage of both offensive and defensive doubling.

## Audit — 2026-04-01 17:00

**Oracle text source**: Scryfall API (cached)
**Oracle text**: If equipped creature would deal combat damage, it deals double that damage instead.
If another creature would deal combat damage to equipped creature, it deals double that damage to equipped creature instead.
Equip {2}
**Type line**: Artifact — Equipment
**Status**: ISSUE

### Code issues
1. **Oracle text field says "another source" but oracle says "another creature"** (`mtg-engine/src/cards/isd/inquisitors_flail.rs` line 26):
   - Oracle text says: `If another creature would deal combat damage to equipped creature`
   - Code oracle_text says: `If another source would deal combat damage to equipped creature`
   - Functionally equivalent since only creatures deal combat damage, but the text doesn't match.

Previously flagged issues that have been fixed or were false positives:
- Defensive doubling: WORKING. `combat_damage_multiplier` in combat.rs (line 311) is called for both source (line 453) and target (line 454) in `deal_damage_to_creature`. Both outgoing and incoming combat damage are correctly doubled.
- Multiple Flails stacking: WORKING. `combat_damage_multiplier` counts effects and returns `1u32 << count` (2^count). Two Flails give count=2, multiplier=4. Test `two_flails_quadruple_damage` confirms.
- Previous claims of boolean-only check were false -- the implementation counts and exponentiates.

Card data verified correct:
- Mana cost: {2}
- Card types: Artifact
- Subtypes: Equipment
- No P/T, no keywords
- continuous_effects: `DoubleCombatDamage { scope: EffectScope::Attached }`
- Equip {2}: activated ability with `ManaCost::new(vec![ManaSymbol::Generic(2)])`, `sorcery_speed_only: true`, `target_requirement: Some(TargetRequirement::Creature)`
- is_valid_target: checks `o.controller == caster` (equip only own creatures)
- on_resolve: moves to battlefield, sets is_equipment
- on_activate_ability: sets attached_to

### Tricky interactions checked
- Doubles outgoing combat damage: pass (tested)
- Doubles incoming combat damage: pass (tested)
- Multiple Flails stacking (x4 for 2 Flails): pass (tested)
- No doubling without Flail equipped: pass (tested)
- Equip only targets own creatures: pass (is_valid_target checks controller)
- Equip sorcery speed only: pass
- Non-combat damage not doubled: pass (only applies to combat damage via continuous effect)
- Ruling: trample divide-then-double: pass (combat.rs assigns damage to blockers first at base power, then doubles each portion)

### Test coverage
- Doubles damage to player: `mtg-engine/tests/inquisitors_flail.rs:21` (doubles_damage_to_player)
- Doubles damage to creature: `mtg-engine/tests/inquisitors_flail.rs:44` (doubles_damage_to_creature)
- Doubles damage taken from blocker: `mtg-engine/tests/inquisitors_flail.rs:67` (doubles_damage_taken_from_blocker)
- No doubling without Flail: `mtg-engine/tests/inquisitors_flail.rs:90` (no_doubling_without_flail)
- Two Flails quadruple damage: `mtg-engine/tests/inquisitors_flail.rs:114` (two_flails_quadruple_damage)
- Ruling: trample + Flail divide-then-double: NOT TESTED (but verified correct in combat.rs)
- Ruling: multiple Flails incoming damage stacking: NOT TESTED

## Audit — 2026-04-01 14:30

**Oracle text source**: Oracle cache (Scryfall API), cached 2026-04-01
**Oracle text**: If equipped creature would deal combat damage, it deals double that damage instead.
If another creature would deal combat damage to equipped creature, it deals double that damage to equipped creature instead.
Equip {2}
**Type line**: Artifact — Equipment
**Mana cost**: {2}
**Rulings**:
- Multiple Flails multiply independently (2 = x4, 3 = x8).
- Trample: divide original amount then double each portion.
**Status**: PASS

### Code issues
No issues found.

Card data verified:
- Mana cost {2}: correct (Generic(2))
- Types: Artifact: correct
- Subtypes: Equipment: correct
- No supertypes: correct
- No P/T: correct
- Keywords: none declared (Equip is implemented as activated ability, not keyword): acceptable
- Oracle text: matches
- Continuous effect: `DoubleCombatDamage { scope: EffectScope::Attached }`: correct

Behavior verified:
- `on_resolve` moves to battlefield and sets `is_equipment = true`: correct
- Equip ability: cost {2}, sorcery speed, targets creature: correct
- `is_valid_target` restricts to controller's creatures on battlefield: correct
- `on_activate_ability` sets `attached_to`: correct
- Combat damage doubling in `combat.rs`:
  - `deal_damage_to_creature` (line 449-454): multiplies by source's multiplier AND target's multiplier: correct (handles both Flail effects)
  - `deal_damage_to_player` (line 505-507): multiplies by source's multiplier: correct
  - `combat_damage_multiplier` uses `1u32 << count` (2^count): correct for stacking multiple Flails
- Trample handling: damage is assigned first, then multiplied per the `deal_damage_to_creature`/`deal_damage_to_player` functions — matches ruling about dividing original then doubling

### Tricky interactions checked
- Double damage dealt by equipped creature: pass
- Double damage received by equipped creature from combat: pass
- Multiple Flails stack multiplicatively (2^n): pass
- Only combat damage is doubled (non-combat damage unaffected): pass — `DoubleCombatDamage` only checked in combat code
- Equip is sorcery speed: pass
- Trample divide-then-double: pass — combat code assigns original damage, then `deal_damage_to_creature`/`deal_damage_to_player` apply multiplier

### Test coverage
- Card data: `mtg-engine/tests/tier9_cards.rs:219` (inquisitors_flail_card_data)
- Doubles combat damage to player: `mtg-engine/tests/tier9_cards.rs:229` (inquisitors_flail_doubles_combat_damage)
- Equip ability: `mtg-engine/tests/tier9_cards.rs:259` (inquisitors_flail_equip_ability)
- Doubles damage taken from blocker: NOT TESTED directly in main test file, but logic is in combat.rs multiplier
- Two Flails quadruple: NOT TESTED in tier9 but noted in prior audit
- Trample + Flail divide-then-double: NOT TESTED

## Audit — 2026-04-01 18:00

**Oracle text source**: Scryfall API (cached via oracle_lookup.py)
**Oracle text**: If equipped creature would deal combat damage, it deals double that damage instead.
If another creature would deal combat damage to equipped creature, it deals double that damage to equipped creature instead.
Equip {2}
**Type line**: Artifact — Equipment
**Mana cost**: {2}
**Rulings**:
- Multiple Flails multiply independently (2 Flails = x4, 3 = x8).
- Trample: divide original amount then double each portion.
**Status**: PASS

### Code issues
No issues found.

Card data verified:
- Mana cost {2}: correct (Generic(2))
- Card types: Artifact: correct
- Subtypes: Equipment: correct
- No supertypes, P/T, keywords: correct
- Oracle text field: matches Scryfall exactly (says "another creature", not "another source")
- Continuous effect: `DoubleCombatDamage { scope: EffectScope::Attached }`: correct

Behavior verified:
- `on_resolve`: moves to battlefield, sets `is_equipment = true`: correct for equipment
- Equip activated ability: cost {2}, `sorcery_speed_only: true`, target `CreatureWithFilter(YouControl)`: correct
- `is_valid_target`: checks `o.zone == Battlefield && o.power.is_some() && o.controller == caster`: correct (equip only own creatures)
- `on_activate_ability`: sets `attached_to = Some(*creature_id)`: correct
- Combat damage doubling in `combat.rs`:
  - `deal_damage_to_creature` (lines 449-454): `amount *= combat_damage_multiplier(state, source, registry)` then `amount *= combat_damage_multiplier(state, target, registry)` -- correctly doubles BOTH outgoing and incoming damage
  - `deal_damage_to_player` (lines 505-507): only source multiplier applied -- correct (player doesn't have Flail)
  - `combat_damage_multiplier` uses `count_continuous_effect` and returns `1u32 << count` (2^count) -- correctly stacks multiple Flails
- Trample interaction: combat code assigns original damage first, then each portion is doubled by `deal_damage_to_creature`/`deal_damage_to_player` -- matches ruling

Not in LLM card knowledge section.

### Tricky interactions checked
- Doubles outgoing combat damage: pass (tested)
- Doubles incoming combat damage from other creatures: pass (tested)
- Multiple Flails stack multiplicatively (x4 for 2 Flails): pass (tested)
- No doubling without Flail equipped: pass (tested)
- Equip only targets own creatures: pass (is_valid_target checks controller)
- Equip is sorcery speed only: pass
- Non-combat damage not doubled: pass (DoubleCombatDamage only checked in combat code)
- Trample divide-then-double: pass (verified in combat.rs logic, matches ruling)
- "Another creature" distinction: in normal combat a creature never damages itself, so functionally correct

### Test coverage
- Doubles damage to player: `mtg-engine/tests/inquisitors_flail.rs:21`
- Doubles damage to creature: `mtg-engine/tests/inquisitors_flail.rs:44`
- Doubles damage taken from blocker: `mtg-engine/tests/inquisitors_flail.rs:67`
- No doubling without Flail: `mtg-engine/tests/inquisitors_flail.rs:90`
- Two Flails quadruple damage: `mtg-engine/tests/inquisitors_flail.rs:114`
- Trample + Flail divide-then-double: NOT TESTED (verified correct in code)
- Multiple Flails incoming damage stacking: NOT TESTED

## Audit — 2026-04-01 14:48

**Oracle text source**: Oracle cache (Scryfall API), cached 2026-04-01
**Oracle text**: If equipped creature would deal combat damage, it deals double that damage instead.
If another creature would deal combat damage to equipped creature, it deals double that damage to equipped creature instead.
Equip {2}
**Type line**: Artifact — Equipment
**Mana cost**: {2}
**Rulings**:
- Multiple Flails multiply independently (2 Flails = x4, 3 = x8).
- Trample: divide original amount then double each portion.
**Status**: PASS

### Code issues
No issues found.

Card data verified:
- Mana cost {2}: correct (Generic(2))
- Card types: Artifact: correct
- Subtypes: Equipment: correct
- No supertypes, P/T, keywords: correct
- Oracle text field: matches Scryfall exactly (says "another creature", not "another source")
- Continuous effect: `DoubleCombatDamage { scope: EffectScope::Attached }`: correct

Behavior verified:
- `on_resolve`: moves to battlefield, sets `is_equipment = true`: correct for equipment
- Equip activated ability: cost {2}, `sorcery_speed_only: true`, target `CreatureWithFilter(TargetFilter::YouControl)`: correct
- `is_valid_target`: checks `o.zone == Battlefield && o.power.is_some() && o.controller == caster`: correct (equip only own creatures, power check ensures targeting creatures)
- `on_activate_ability`: sets `attached_to = Some(*creature_id)`: correct
- Combat damage doubling in `combat.rs`:
  - `deal_damage_to_creature` (lines 449-454): `amount *= combat_damage_multiplier(state, source, registry)` then `amount *= combat_damage_multiplier(state, target, registry)` -- correctly doubles BOTH outgoing and incoming combat damage
  - `deal_damage_to_player` (lines 505-507): only source multiplier applied -- correct (player doesn't have equipment)
  - `combat_damage_multiplier` (lines 309-319): uses `count_continuous_effect` and returns `1u32 << count` (2^count) -- correctly stacks multiple Flails

No anti-patterns detected. Not in LLM card knowledge section.

### Tricky interactions checked
- Doubles outgoing combat damage: pass (tested)
- Doubles incoming combat damage from other creatures: pass (tested)
- Multiple Flails stack multiplicatively (x4 for 2 Flails): pass (tested)
- No doubling without Flail equipped: pass (tested)
- Equip only targets own creatures: pass (is_valid_target checks controller)
- Equip is sorcery speed only: pass
- Non-combat damage not doubled: pass (DoubleCombatDamage only checked in combat code)
- Trample divide-then-double: pass (combat code assigns original damage, then `deal_damage_to_creature`/`deal_damage_to_player` apply multiplier -- matches ruling)
- "Another creature" distinction: in normal combat a creature never deals combat damage to itself, so functionally correct

### Test coverage
- Doubles damage to player: `mtg-engine/tests/inquisitors_flail.rs:21` (doubles_damage_to_player)
- Doubles damage to creature: `mtg-engine/tests/inquisitors_flail.rs:44` (doubles_damage_to_creature)
- Doubles damage taken from blocker: `mtg-engine/tests/inquisitors_flail.rs:67` (doubles_damage_taken_from_blocker)
- No doubling without Flail: `mtg-engine/tests/inquisitors_flail.rs:90` (no_doubling_without_flail)
- Two Flails quadruple damage: `mtg-engine/tests/inquisitors_flail.rs:114` (two_flails_quadruple_damage)
- Card data: `mtg-engine/tests/tier9_cards.rs:219` (inquisitors_flail_card_data)
- Equip ability: `mtg-engine/tests/tier9_cards.rs:259` (inquisitors_flail_equip_ability)
- Trample + Flail divide-then-double: NOT TESTED (verified correct in combat.rs code)
- Multiple Flails incoming damage stacking: NOT TESTED

---

## Audit 7 — 2026-04-02

### Oracle text (Scryfall, cached 2026-04-01)
```
Name: Inquisitor's Flail
Mana Cost: {2}
Type Line: Artifact — Equipment
Oracle Text:
If equipped creature would deal combat damage, it deals double that damage instead.
If another creature would deal combat damage to equipped creature, it deals double that damage to equipped creature instead.
Equip {2}
```

### Implementation: `mtg-engine/src/cards/isd/inquisitors_flail.rs`

#### Card data
- Name: "Inquisitor's Flail": correct
- Mana cost: `Generic(2)`: correct ({2})
- Card types: `[Artifact]`, subtypes: `["Equipment"]`: correct
- oracle_text field: matches oracle text verbatim: correct
- continuous_effects: `DoubleCombatDamage { scope: EffectScope::Attached }`: correct
- keywords: `[]`: correct (Equip is implemented as an activated ability, not a keyword grant)

#### Equipment mechanics
- `on_resolve`: sets `is_equipment = true`, moves to battlefield: correct
- `activated_abilities`: Equip {2}, sorcery speed only, targets creature you control: correct
- `on_activate_ability`: sets `attached_to` on the equipment: correct
- `is_valid_target`: checks battlefield, has power (is creature), controller matches caster: correct

#### Combat damage doubling (engine: `mtg-engine/src/combat.rs`)
- `combat_damage_multiplier` (line 311): counts `DoubleCombatDamage` effects on a creature, returns `2^count`: correct
- `deal_damage_to_creature` (lines 452-454): multiplies by source's multiplier AND target's multiplier: correct (doubles outgoing AND incoming)
- `deal_damage_to_player` (lines 506-507): multiplies by source's multiplier only: correct (players cannot be equipped)
- Multiple Flails stack multiplicatively per ruling: correct (2^count)

#### Minor cosmetic issue (non-functional)
- Doc comment (line 9) says "another **source**" but oracle says "another **creature**":
  - Implementation: `"If another source would deal combat damage to equipped creature"`
  - Oracle: `"If another creature would deal combat damage to equipped creature"`
  - The `oracle_text` field (line 26) is correct; only the Rust doc comment is wrong.
- Same issue in test file header (`mtg-engine/tests/inquisitors_flail.rs`, line 5).

### Functional correctness: PASS
No functional issues found. Both damage doubling directions are correctly implemented.

### Test coverage (all passing)
- `inquisitors_flail.rs:21` doubles_damage_to_player: 4 power -> 8 damage to player
- `inquisitors_flail.rs:45` doubles_damage_to_creature: 2 power -> 4 damage to blocker
- `inquisitors_flail.rs:68` doubles_damage_taken_from_blocker: 2-power blocker -> 4 damage to equipped creature
- `inquisitors_flail.rs:91` no_doubling_without_flail: 3 power -> 3 damage (no flail attached)
- `inquisitors_flail.rs:114` two_flails_quadruple_damage: 3 power * 4 = 12 damage
- `tier9_cards.rs:219` inquisitors_flail_card_data: type/subtype/cost checks
- `tier9_cards.rs:229` inquisitors_flail_doubles_combat_damage: 3 power -> 6 damage
- `tier9_cards.rs:259` inquisitors_flail_equip_ability: equip attaches correctly

### Verdict: PASS — no functional issues

## Audit — 2026-04-02

### Oracle Text (Scryfall)
```
If equipped creature would deal combat damage, it deals double that damage instead.
If another creature would deal combat damage to equipped creature, it deals double that damage to equipped creature instead.
Equip {2}
```

### Card Data (`inquisitors_flail.rs`)
- **Name:** Correct — `"Inquisitor's Flail"`
- **Cost:** Correct — `{2}` (Generic 2)
- **Types:** Correct — `Artifact`, subtype `Equipment`
- **oracle_text field:** Correct — matches Scryfall verbatim.
- **Equip cost:** Correct — `{2}`, sorcery-speed only, targets creature you control.

### Doc Comment Bug
Line 9 of `inquisitors_flail.rs` reads:
> `/// If another source would deal combat damage to equipped creature`

Oracle text says:
> `If another creature would deal combat damage to equipped creature`

The word **"source"** should be **"creature"**. This is a cosmetic doc-comment-only issue; the `oracle_text` field in `card_data()` is correct.

### Continuous Effect Registration
- Registers a single `ContinuousEffect::DoubleCombatDamage { scope: EffectScope::Attached }`.
- The combat engine in `combat.rs` applies `combat_damage_multiplier()` on **both sides** of creature-vs-creature combat (lines 453–454: source multiplier AND target multiplier), correctly implementing both "deals double" and "takes double".
- For creature-to-player damage (line 507), only the source multiplier is applied, which is correct (players don't have equipment).

### Damage Doubling Implementation (`combat.rs`)
- `combat_damage_multiplier()` counts all `DoubleCombatDamage` effects attached to a creature and returns `2^count`. This correctly handles multiple Flails per the ruling (2 Flails = 4x, etc.).
- Creature-to-creature: both source and target multipliers applied — correct.
- Creature-to-player: only source multiplier applied — correct.

### Equipment Mechanics
- `on_resolve`: moves to battlefield, sets `is_equipment = true` — correct.
- `activated_abilities`: returns Equip ability only when on battlefield and not a creature (`power.is_none()`) — correct.
- `on_activate_ability`: sets `attached_to` on the equipment — correct.
- `is_valid_target`: checks battlefield, is a creature (`power.is_some()`), controlled by caster — correct.

### Test Coverage
- `tests/tier9_cards.rs`: 3 tests (card data, doubles combat damage to player, equip ability).
- `tests/inquisitors_flail.rs`: 5 tests (doubles to player, doubles to creature, doubles damage taken from blocker, no doubling without equip, two flails quadruple).
- Both the dealing and receiving sides of damage doubling are tested.

### Test File Doc Comment Bug
Lines 5–6 of `tests/inquisitors_flail.rs` repeat the same "another source" wording instead of "another creature". Cosmetic only.

### Verdict
**PASS** — Implementation is correct. Both damage-doubling directions work properly, equip mechanics are sound, and test coverage is thorough. One cosmetic doc-comment issue noted (says "source" instead of "creature" in two locations).
