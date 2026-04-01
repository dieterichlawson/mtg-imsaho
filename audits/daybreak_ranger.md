# Audit: Daybreak Ranger // Nightfall Predator

## Scryfall Reference
- **Front Face: Daybreak Ranger**
  - **Cost:** {2}{G}
  - **Type:** Creature -- Human Archer Ranger Werewolf
  - **Oracle:** {T}: This creature deals 2 damage to target creature with flying. At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
  - **P/T:** 2/2

- **Back Face: Nightfall Predator**
  - **Cost:** (none)
  - **Type:** Creature -- Werewolf
  - **Oracle:** {R}, {T}: This creature fights target creature. At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
  - **P/T:** 4/4

## Implementation: `daybreak_ranger.rs`
- **Front face name:** Daybreak Ranger -- CORRECT
- **Cost:** {2}{G} -- CORRECT
- **Front subtypes:** ["Human", "Archer", "Werewolf"] -- ISSUE (see below)
- **Front P/T:** 2/2 -- CORRECT
- **Back face name:** Nightfall Predator -- CORRECT
- **Back subtypes:** ["Werewolf"] -- CORRECT
- **Back P/T:** 4/4 -- CORRECT
- **Front ability:** {T}: 2 damage to flying creature -- CORRECT
- **Back ability:** {R}, {T}: fight target creature -- CORRECT
- **Transform logic:** Werewolf standard (no spells / 2+ spells) -- CORRECT
- **Uses NonCombatDamageDealt for front ability:** Yes -- CORRECT

## Issues
1. **ISSUE: Missing "Ranger" subtype on front face.** Scryfall type line is "Creature -- Human Archer Ranger Werewolf" but implementation has subtypes ["Human", "Archer", "Werewolf"] -- missing "Ranger".

## Audit -- 2026-04-01 09:00

**Scryfall Oracle text**: (Front) {T}: Daybreak Ranger deals 2 damage to target creature with flying. At the beginning of each upkeep, if no spells were cast last turn, transform Daybreak Ranger. (Back) {R}, {T}: Nightfall Predator fights target creature. At the beginning of each upkeep, if a player cast two or more spells last turn, transform Nightfall Predator.
**Scryfall type line**: (Front) Creature -- Human Archer Ranger Werewolf. (Back) Creature -- Werewolf.
**Status**: ISSUE

Findings:
1. **Mana cost {2}{G}**: Correct.
2. **Front face P/T 2/2**: Correct.
3. **Front face subtypes**: Code has `["Human", "Archer", "Ranger", "Werewolf"]` (line 34). Previous audit said "Ranger" was missing, but the current code includes it. This is now correct.
4. **Back face name (Nightfall Predator)**: Correct.
5. **Back face subtypes ["Werewolf"]**: Correct per Scryfall.
6. **Back face P/T 4/4**: Correct (via `dynamic_pt`).
7. **Front ability ({T}: 2 damage to flying creature)**: Correctly implemented. Target validation checks `has_keyword(Keyword::Flying)`.
8. **Back ability ({R}, {T}: fight target creature)**: Cost and tap requirement correct. However, `is_valid_target` (line 128) restricts Nightfall Predator's fight to `obj.controller != caster` (only opponent's creatures). **Scryfall oracle says "fights target creature" with NO restriction -- it can target any creature, including your own.** This is a bug.
9. **Transform triggers**: Front transforms if no spells cast last turn (correct). Back transforms if any player cast 2+ spells last turn (correct). Both use `on_upkeep` hook with `triggered_abilities: [TriggerKind::Upkeep]` on front face.
10. **Back face `triggered_abilities` is empty**: The back face data (line 65) has `triggered_abilities: vec![]`. While the `on_upkeep` hook handles both faces, this could be a declaration mismatch if the engine uses `triggered_abilities` to decide whether to call `on_upkeep`.
11. **Anti-patterns**: Uses `NonCombatDamageDealt` for front face damage (correct). Uses `crate::combat::fight` for back face (correct).
12. **Tests**: Found in `mtg-engine/tests/werewolf_cards.rs`.

Issues:
1. **Nightfall Predator fight targeting too restrictive**: Code only allows targeting opponent's creatures (`obj.controller != caster`), but oracle text says "target creature" with no restriction.
2. **Back face triggered_abilities empty**: May cause engine to skip `on_upkeep` for the back face transform check.

## Audit — 2026-04-01 14:13

**Oracle text source**: Scryfall card page via WebSearch (https://scryfall.com/card/isd/176/daybreak-ranger-nightfall-predator)
**Oracle text (front)**: {T}: Daybreak Ranger deals 2 damage to target creature with flying. At the beginning of each upkeep, if no spells were cast last turn, transform Daybreak Ranger.
**Oracle text (back)**: {R}, {T}: Nightfall Predator fights target creature. At the beginning of each upkeep, if a player cast two or more spells last turn, transform Nightfall Predator.
**Type line (front)**: Creature — Human Archer Ranger Werewolf
**Type line (back)**: Creature — Werewolf
**Mana cost**: {2}{G}
**P/T (front)**: 2/2
**P/T (back)**: 4/4
**Status**: ISSUE

Findings:
1. **Name**: "Daybreak Ranger" / "Nightfall Predator" -- correct.
2. **Mana cost {2}{G}**: Correct (Generic(2), Green).
3. **Front face subtypes**: Code has `["Human", "Archer", "Ranger", "Werewolf"]` (line 34). Matches Scryfall "Human Archer Ranger Werewolf". Correct.
4. **Front face P/T 2/2**: Correct.
5. **Back face subtypes ["Werewolf"]**: Correct per Scryfall.
6. **Back face P/T 4/4**: Correct (via `dynamic_pt`).
7. **Front ability ({T}: 2 damage to creature with flying)**: Correctly implemented. `is_valid_target` checks `has_keyword(Keyword::Flying)` for the front face. NonCombatDamageDealt emitted. `damaged_by` tracked. All correct.
8. **Back ability ({R}, {T}: fight target creature)**: Cost is ManaCost(Red), requires_tap: true. Uses `crate::combat::fight`. However, `is_valid_target` at line 128 restricts Nightfall Predator's fight to `obj.controller != caster` (only opponent's creatures). **The oracle text says "fights target creature" with NO controller restriction -- it can target ANY creature, including your own.** This is a bug.
9. **Transform triggers**: Front transforms if no spells cast last turn and not first turn (correct). Back transforms if any player cast 2+ spells last turn (correct). Both handled in `on_upkeep`.
10. **triggered_abilities**: Front face declares `[TriggerKind::Upkeep]` (line 42-47). Back face has `triggered_abilities: vec![]` (line 65). If the engine relies on `triggered_abilities` declarations to dispatch `on_upkeep`, the back face transform-back check would never fire.
11. **Tests**: Found in `mtg-engine/tests/werewolf_cards.rs`. Tests cover transform, front face ability description, and back face fight ability description. No test for targeting restrictions on fight.

Issues:
1. **Nightfall Predator fight targeting too restrictive**: `is_valid_target` (line 128) requires `obj.controller != caster`, but oracle says "target creature" (any creature). Should allow targeting own creatures too.
2. **Back face triggered_abilities empty**: Back face data has `triggered_abilities: vec![]` but has an upkeep-triggered transform ability. This may prevent the engine from calling `on_upkeep` for the back face.

## Audit — 2026-04-01 16:00

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/176/daybreak-ranger-nightfall-predator?utm_source=api
**Oracle text (front)**: {T}: This creature deals 2 damage to target creature with flying. At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
**Oracle text (back)**: {R}, {T}: This creature fights target creature. (Each deals damage equal to its power to the other.) At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line (front)**: Creature — Human Archer Ranger Werewolf
**Type line (back)**: Creature — Werewolf
**Front P/T**: 2/2
**Back P/T**: 4/4
**Status**: ISSUE

### Code issues

1. **Nightfall Predator fight targeting too restrictive** (`mtg-engine/src/cards/isd/daybreak_ranger.rs`, lines 126-128):
   - Oracle text says: `{R}, {T}: This creature fights target creature.`
   - Code does: `obj.controller != caster` -- only allows targeting opponent's creatures. The oracle says "target creature" with no restriction. Nightfall Predator should be able to fight any creature, including your own.

### Tricky interactions checked
- Front face targets only creatures with flying: PASS (line 131 checks `has_keyword(Keyword::Flying)`)
- Front face deals 2 damage (not fight): PASS (manually marks damage, does not call `fight`)
- Back face fight uses `crate::combat::fight`: PASS (line 143)
- Back face costs {R} + tap: PASS (ManaCost(Red), requires_tap: true at lines 91-92)
- Werewolf transform conditions: PASS (front: no spells last turn, back: any player 2+ spells)
- First turn no-transform: PASS (line 17 checks `!state.is_first_turn`)
- NonCombatDamageDealt for front face damage: PASS (line 153)
- damaged_by tracking for front face: PASS (line 150)
- Subtypes include all four (Human, Archer, Ranger, Werewolf): PASS (line 34)
- dynamic_pt returns (4,4) when transformed: PASS (line 74)

### Test coverage
- Transforms to Nightfall Predator: `werewolf_cards.rs:310` (daybreak_ranger_transforms_to_nightfall_predator)
- Front face has activated ability with "flying" in description: `werewolf_cards.rs:326` (daybreak_ranger_has_activated_ability_on_front_face)
- Back face has fight ability: `werewolf_cards.rs:338` (nightfall_predator_has_fight_ability)
- Front face deals damage to creature with flying: NOT TESTED
- Back face fight resolves correctly: NOT TESTED
- Nightfall Predator can target own creatures: NOT TESTED (bug: code incorrectly restricts this)
- Transform back when 2+ spells cast: NOT TESTED

## Audit — 2026-04-01 13:35

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/176/daybreak-ranger-nightfall-predator?utm_source=api
**Oracle text (front)**: {T}: This creature deals 2 damage to target creature with flying. At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
**Oracle text (back)**: {R}, {T}: This creature fights target creature. (Each deals damage equal to its power to the other.) At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line (front)**: Creature — Human Archer Ranger Werewolf
**Type line (back)**: Creature — Werewolf
**Front P/T**: 2/2
**Back P/T**: 4/4
**Ruling**: [2016-07-13] See Shadows over Innistrad mechanics article for DFC rules.
**Status**: PASS

### Code issues
No issues found.

The previous audit flagged that Nightfall Predator's fight targeting was too restrictive (only opponent's creatures). This has been fixed. The current `is_valid_target` code (lines 121-136) returns `true` for any creature when the card is transformed, meaning Nightfall Predator can fight any creature including your own. A test for this was added at `werewolf_cards.rs:353`.

### Tricky interactions checked
- Front face targets only creatures with flying: PASS (line 132 checks `has_keyword(Keyword::Flying)`)
- Front face deals 2 damage (not fight): PASS (manually marks damage at line 150, does not call `fight`)
- Back face fight allows any creature target: PASS (line 129 returns `true` with no controller restriction)
- Back face fight uses `crate::combat::fight`: PASS (line 144)
- Back face costs {R} + tap: PASS (ManaCost(Red), requires_tap: true at lines 91-92)
- Werewolf transform conditions: PASS (front: no spells last turn, back: any player 2+ spells)
- First turn no-transform: PASS (line 17 checks `!state.is_first_turn`)
- NonCombatDamageDealt for front face damage: PASS (lines 154-157)
- damaged_by tracking for front face: PASS (line 151)
- Subtypes include all four (Human, Archer, Ranger, Werewolf): PASS (line 34)
- dynamic_pt returns (4,4) when transformed: PASS (line 74)

### Test coverage
- Transforms to Nightfall Predator: `werewolf_cards.rs:312` (daybreak_ranger_transforms_to_nightfall_predator)
- Front face has activated ability with "flying" in description: `werewolf_cards.rs:328` (daybreak_ranger_has_activated_ability_on_front_face)
- Back face has fight ability: `werewolf_cards.rs:340` (nightfall_predator_has_fight_ability)
- Nightfall Predator can fight own creature: `werewolf_cards.rs:353` (nightfall_predator_can_fight_own_creature)
- Front face deals damage to creature with flying: NOT TESTED
- Transform back when 2+ spells cast: NOT TESTED

## Audit — 2026-04-01 18:30

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/176/daybreak-ranger-nightfall-predator?utm_source=api
**Oracle text (front)**: {T}: This creature deals 2 damage to target creature with flying. At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
**Oracle text (back)**: {R}, {T}: This creature fights target creature. (Each deals damage equal to its power to the other.) At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line (front)**: Creature — Human Archer Ranger Werewolf
**Type line (back)**: Creature — Werewolf
**Front P/T**: 2/2
**Back P/T**: 4/4
**Keywords**: Transform, Fight
**Ruling**: [2016-07-13] See Shadows over Innistrad mechanics article for DFC rules.
**Status**: PASS

### Code issues
No issues found.

All previously flagged issues have been resolved:
1. Nightfall Predator's fight targeting now allows any creature (lines 121-136 of `is_valid_target` return `true` for any creature when transformed). No controller restriction. A test was added for this at `werewolf_cards.rs:353`.
2. All four front-face subtypes (Human, Archer, Ranger, Werewolf) are present at line 34.

### Tricky interactions checked
- Front face targets only creatures with flying: PASS (line 132 checks `has_keyword(Keyword::Flying)`)
- Front face deals 2 damage (not fight): PASS (manually marks damage at line 148-151, does not call `fight`)
- Front face damage source is the creature itself: PASS (line 153 `obj.damaged_by.push(object_id)`)
- Back face fight allows any creature target: PASS (line 129 returns `true` with no controller restriction)
- Back face fight uses `crate::combat::fight`: PASS (line 144)
- Back face costs {R} + tap: PASS (ManaCost(Red), requires_tap: true at lines 91-92)
- Werewolf transform conditions: PASS (front: no spells last turn, back: any player 2+ spells)
- First turn no-transform: PASS (line 17 checks `!state.is_first_turn`)
- NonCombatDamageDealt for front face damage: PASS (lines 154-158)
- damaged_by tracking for front face: PASS (line 151)
- Subtypes include all four (Human, Archer, Ranger, Werewolf): PASS (line 34)
- dynamic_pt returns (4,4) when transformed: PASS (line 74)
- triggered_abilities declaration matches on_upkeep hook: PASS (TriggerKind::Upkeep at line 43)
- Oracle text uses "This creature" but code uses "Daybreak Ranger" in oracle_text field: cosmetic difference, not a functional issue

### Test coverage
- Transforms to Nightfall Predator: `werewolf_cards.rs:312` (daybreak_ranger_transforms_to_nightfall_predator)
- Front face has activated ability with "flying" in description: `werewolf_cards.rs:328` (daybreak_ranger_has_activated_ability_on_front_face)
- Back face has fight ability: `werewolf_cards.rs:340` (nightfall_predator_has_fight_ability)
- Nightfall Predator can fight own creature: `werewolf_cards.rs:353` (nightfall_predator_can_fight_own_creature)
- Front face deals damage to creature with flying: NOT TESTED
- Transform back when 2+ spells cast: NOT TESTED
