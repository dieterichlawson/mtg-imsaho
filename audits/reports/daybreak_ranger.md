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

## Audit — 2026-04-01 20:00

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

All card data matches oracle text. Front face subtypes include all four (Human, Archer, Ranger, Werewolf) at line 34. Front face ability correctly restricts targets to creatures with flying via `is_valid_target` (line 132 checks `has_keyword(Keyword::Flying)`). Back face fight allows any creature target (line 129 returns `true` with no controller restriction). Werewolf transform conditions are correct (front: no spells last turn, back: any player 2+ spells). Fight ability correctly uses `crate::combat::fight` (line 144).

### Tricky interactions checked
- Front face targets only creatures with flying: PASS (line 132 checks `has_keyword(Keyword::Flying)`)
- Front face deals 2 damage (not fight): PASS (manually marks damage at lines 148-153, does not call `fight`)
- Front face damage source is the creature itself: PASS (line 151 `obj.damaged_by.push(object_id)`)
- Back face fight allows any creature target: PASS (line 129 returns `true` with no controller restriction)
- Back face fight uses `crate::combat::fight`: PASS (line 144)
- Back face costs {R} + tap: PASS (ManaCost(Red), requires_tap: true at lines 91-92)
- Werewolf transform conditions: PASS (front: no spells last turn, back: any player 2+ spells)
- First turn no-transform: PASS (line 17 checks `!state.is_first_turn`)
- NonCombatDamageDealt for front face damage: PASS (lines 154-158)
- damaged_by tracking for front face: PASS (line 151)
- dynamic_pt returns (4,4) when transformed: PASS (line 74)
- triggered_abilities declaration matches on_upkeep hook: PASS (TriggerKind::Upkeep at line 43)

### Test coverage
- Transforms to Nightfall Predator: `werewolf_cards.rs:312` (daybreak_ranger_transforms_to_nightfall_predator)
- Front face has activated ability with "flying" in description: `werewolf_cards.rs:328` (daybreak_ranger_has_activated_ability_on_front_face)
- Back face has fight ability: `werewolf_cards.rs:340` (nightfall_predator_has_fight_ability)
- Nightfall Predator can fight own creature: `werewolf_cards.rs:353` (nightfall_predator_can_fight_own_creature)
- Front face deals damage to creature with flying: NOT TESTED
- Transform back when 2+ spells cast: NOT TESTED
- LLM card knowledge: NOT PRESENT

## Audit — 2026-04-01 14:49

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

Card data matches oracle text for both faces. Front face: {2}{G}, Creature - Human Archer Ranger Werewolf, 2/2. Back face via `back_face_data()`: Creature - Werewolf, 4/4, with `dynamic_pt` returning (4,4) when transformed. Front face activated ability: free mana cost, `requires_tap: true`, `TargetRequirement::Creature` with `is_valid_target` filtering for `has_keyword(Keyword::Flying)` on the front face (line 132). Back face activated ability: ManaCost(Red), `requires_tap: true`, `TargetRequirement::Creature` with `is_valid_target` returning `true` for any creature when transformed (line 129 -- no controller restriction). Front face damage: manually applies `damage_marked += 2` and `damaged_by.push(object_id)` with `NonCombatDamageDealt` event (lines 148-158). Back face fight: uses `crate::combat::fight` (line 144). Werewolf transform logic: front transforms when no spells cast last turn and not first turn (line 17), back transforms when any player cast 2+ spells (line 19). `on_upkeep` handles both faces (line 165), fires on every upkeep (no controller check -- correct for "each upkeep"). `triggered_abilities` on front face declares `TriggerKind::Upkeep` (line 43).

### Tricky interactions checked
- Front face targets only creatures with flying: PASS (line 132 checks `has_keyword(Keyword::Flying)`)
- Front face deals 2 damage (not fight): PASS (manually marks damage at lines 148-153)
- Front face damage source is the creature itself: PASS (line 151 `obj.damaged_by.push(object_id)`)
- Back face fight allows any creature target: PASS (line 129 returns `true` with no controller restriction)
- Back face fight uses `crate::combat::fight`: PASS (line 144)
- Back face costs {R} + tap: PASS (ManaCost(Red), requires_tap: true at lines 91-92)
- Werewolf transform conditions: PASS (front: no spells last turn; back: any player 2+ spells)
- Transform fires on each upkeep (not just controller's): PASS (no active_player check in on_upkeep)
- First turn no-transform: PASS (line 17 checks `!state.is_first_turn`)
- NonCombatDamageDealt for front face damage: PASS (lines 154-158)
- damaged_by tracking for front face: PASS (line 151)
- dynamic_pt returns (4,4) when transformed: PASS (line 74)
- triggered_abilities declaration matches on_upkeep hook: PASS (TriggerKind::Upkeep at line 43)

### Test coverage
- Transforms to Nightfall Predator: `werewolf_cards.rs:312` (daybreak_ranger_transforms_to_nightfall_predator)
- Front face has activated ability with "flying" in description: `werewolf_cards.rs:328` (daybreak_ranger_has_activated_ability_on_front_face)
- Back face has fight ability: `werewolf_cards.rs:340` (nightfall_predator_has_fight_ability)
- Nightfall Predator can fight own creature: `werewolf_cards.rs:353` (nightfall_predator_can_fight_own_creature)
- Front face deals damage to creature with flying: NOT TESTED
- Transform back when 2+ spells cast: NOT TESTED

## Audit — 2026-04-02

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/176/daybreak-ranger-nightfall-predator?utm_source=api
**Oracle text (front)**: {T}: This creature deals 2 damage to target creature with flying. At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
**Oracle text (back)**: {R}, {T}: This creature fights target creature. (Each deals damage equal to its power to the other.) At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line (front)**: Creature — Human Archer Ranger Werewolf
**Type line (back)**: Creature — Werewolf
**Front P/T**: 2/2
**Back P/T**: 4/4
**Keywords (Scryfall)**: Transform, Fight
**Status**: PASS

### Code issues
No issues found.

Both faces match oracle text. Detailed verification:

- **Mana cost**: Code `ManaCost::new(vec![ManaSymbol::Generic(2), ManaSymbol::Colored(Color::Green)])` matches `{2}{G}`.
- **Front face subtypes**: Code `["Human", "Archer", "Ranger", "Werewolf"]` (line 34) matches oracle type line "Human Archer Ranger Werewolf".
- **Back face subtypes**: Code `["Werewolf"]` (line 57) matches oracle type line "Werewolf".
- **Front face P/T**: Code `power: Some(2), toughness: Some(2)` (lines 35-36). Correct.
- **Back face P/T**: `dynamic_pt` returns `Some((4, 4))` when transformed (line 74). `back_face_data` also declares `power: Some(4), toughness: Some(4)` (lines 59-60). Correct.
- **Front face ability**: `is_valid_target` checks `state.has_keyword(*id, Keyword::Flying, registry)` (line 132) when not transformed. Damage is applied manually: `obj.damage_marked += 2` (line 150), `obj.damaged_by.push(object_id)` (line 151), and `NonCombatDamageDealt` event emitted (line 154). Correct -- this is non-combat damage, not fight.
- **Back face ability**: Cost is `ManaCost::new(vec![ManaSymbol::Colored(Color::Red)])` with `requires_tap: true` (lines 91-93). Matches oracle `{R}, {T}`. When transformed, `is_valid_target` returns `true` for any creature (line 129) -- no controller restriction. Matches oracle "target creature" with no restriction. Fight is dispatched via `crate::combat::fight(state, object_id, *target_id, registry)` (line 144).
- **Transform conditions**: Front transforms when `total_spells_last_turn == 0 && !state.is_first_turn` (line 17). Back transforms when `state.spells_cast_last_turn.values().any(|&count| count >= 2)` (line 19). Both correct per oracle text.
- **Trigger declaration**: Front face `triggered_abilities` includes `TriggerKind::Upkeep` (line 43). Back face has `triggered_abilities: vec![]` (line 65). This matches the pattern used by all other werewolves (e.g., Reckless Waif) where `on_upkeep` handles both faces and only the front face declares the trigger.

Note: Scryfall lists "Transform" and "Fight" as keywords, but these are action keywords/mechanics rather than static keyword abilities. The code uses `keywords: vec![]` for both faces, which is consistent with all other werewolf implementations (e.g., Reckless Waif). Transform is handled via `should_transform`/`on_upkeep`, and Fight is an activated ability action, not a keyword like Flying or Trample.

Note: `crate::combat::fight` internally calls `deal_damage_to_creature` which emits `CombatDamageDealt` rather than `NonCombatDamageDealt`. Fight damage is not combat damage per MTG rules. This is a systemic issue in the fight function (`mtg-engine/src/combat.rs:158`), not specific to Daybreak Ranger.

### Tricky interactions checked
- Front face targets only creatures with flying: PASS (`is_valid_target` line 132 checks `has_keyword(Keyword::Flying)`)
- Front face deals 2 non-combat damage (not fight): PASS (manually marks damage, emits `NonCombatDamageDealt`)
- Front face damage source tracked: PASS (`damaged_by.push(object_id)` at line 151)
- Back face fight allows any creature target (including own): PASS (line 129 returns `true` unconditionally)
- Back face costs {R} + tap: PASS (ManaCost(Red), requires_tap: true at lines 91-93)
- Werewolf transform conditions correct for both directions: PASS
- Transform fires on each upkeep (not just controller's): PASS (no active_player check in `on_upkeep`)
- First turn no-transform: PASS (line 17 checks `!state.is_first_turn`)
- dynamic_pt returns (4,4) when transformed: PASS (line 74)
- `is_valid_target` multi-copy fragility: The code searches all objects to find the source's transformed state (lines 121-125) rather than receiving the source object_id. If a player controls two Daybreak Rangers in different states, this could misidentify which is transformed. Minor architectural concern, unlikely to cause bugs in practice.

### Test coverage
- Transforms to Nightfall Predator: `werewolf_cards.rs:312` (daybreak_ranger_transforms_to_nightfall_predator)
- Front face has activated ability with "flying" in description: `werewolf_cards.rs:328` (daybreak_ranger_has_activated_ability_on_front_face)
- Back face has fight ability: `werewolf_cards.rs:340` (nightfall_predator_has_fight_ability)
- Nightfall Predator can fight own creature: `werewolf_cards.rs:353` (nightfall_predator_can_fight_own_creature)
- Front face deals damage to creature with flying: NOT TESTED
- Transform back when 2+ spells cast: NOT TESTED (covered generically by reckless_waif tests)
- LLM card knowledge entry: NOT PRESENT in `mtg-player/src/llm.rs`

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: {T}: This creature deals 2 damage to target creature with flying.\nAt the beginning of each upkeep, if no spells were cast last turn, transform this creature.
**Back face oracle text**: {R}, {T}: This creature fights target creature. (Each deals damage equal to its power to the other.)\nAt the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line**: Creature — Human Archer Ranger Werewolf // Creature — Werewolf
**Status**: ISSUE

### Code issues
1. Minor oracle text mismatch: code front face uses `"Daybreak Ranger deals 2 damage"` and `"transform Daybreak Ranger"` instead of current oracle template `"This creature deals 2 damage"` / `"transform this creature"`. Same issue on back face: `"Nightfall Predator fights"` vs `"This creature fights"`. Behavior is correct regardless.
2. The front face TargetRequirement is `Creature` (any creature), but `is_valid_target` further restricts to flying creatures. This works correctly but is an unusual pattern — ideally a `CreatureWithFlying` target requirement would be used if available. No behavioral issue.
3. Transform logic is correct: front transforms if no spells cast last turn (and not first turn), back transforms if any player cast 2+ spells. P/T 2/2 front, 4/4 back via dynamic_pt. Subtypes match oracle for both faces.

## Audit — 2026-04-02 (final-pass)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found. Oracle text field matches current Scryfall template.

## Audit — 2026-04-02 20:50

**Oracle text source**: Scryfall API via `scripts/oracle_lookup.py`, https://scryfall.com/card/isd/176/daybreak-ranger-nightfall-predator?utm_source=api
**Oracle text (front)**: {T}: This creature deals 2 damage to target creature with flying.
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
**Oracle text (back)**: {R}, {T}: This creature fights target creature. (Each deals damage equal to its power to the other.)
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line (front)**: Creature — Human Archer Ranger Werewolf
**Type line (back)**: Creature — Werewolf
**Front P/T**: 2/2
**Back P/T**: 4/4
**Keywords (Scryfall)**: Transform, Fight
**Ruling**: [2016-07-13] See Shadows over Innistrad mechanics article for DFC rules.
**Status**: PASS

### Code issues

No functional issues found.

Minor notes (not bugs):
1. **Unused import**: `TargetFilter` is imported at line 3 but unused (compiler warning).
2. **Misleading comment**: Line 120 says "Nightfall Predator targets any creature you don't control" but the code correctly allows any creature (line 129 returns `true`). The comment is inaccurate but the code is correct.

### Tricky interactions checked (min 3)

1. **Front face targets only creatures with flying**: PASS. `is_valid_target` (line 132) checks `state.has_keyword(*id, Keyword::Flying, registry)` when not transformed. The `TargetRequirement::Creature` combined with `is_valid_target` filtering correctly limits targeting to flying creatures.
2. **Front face deals 2 non-combat damage (not fight)**: PASS. Lines 148-158 manually apply `damage_marked += 2`, track `damaged_by`, and emit `NonCombatDamageDealt`. This is correct -- the front face ability is not fight, it is direct damage.
3. **Back face fight allows any creature target (including own)**: PASS. Line 129 returns `true` for all creatures when transformed, matching oracle text "target creature" with no restriction. Test `nightfall_predator_can_fight_own_creature` verifies this.
4. **Back face costs {R} + tap**: PASS. `ManaCost::new(vec![ManaSymbol::Colored(Color::Red)])` with `requires_tap: true` (lines 91-92).
5. **Back face uses engine fight function**: PASS. Line 144 calls `crate::combat::fight(state, object_id, *target_id, registry)` which correctly deals mutual damage.
6. **Werewolf transform conditions**: PASS. Front: `total_spells_last_turn == 0 && !state.is_first_turn` (line 17). Back: `state.spells_cast_last_turn.values().any(|&count| count >= 2)` (line 19). Both match oracle text.
7. **Transform fires on each upkeep (not just controller's)**: PASS. `on_upkeep` has no active_player check, matching "at the beginning of each upkeep".
8. **First turn no-transform guard**: PASS. Line 17 checks `!state.is_first_turn`.
9. **dynamic_pt returns (4,4) when transformed**: PASS. Line 74.
10. **Upkeep trigger fires for both faces**: PASS. Front face declares `TriggerKind::Upkeep` (line 43). Engine's `trigger_description` checks front face first regardless of transform state, so the trigger fires for both faces. Back face's empty `triggered_abilities` is not a problem.

### Test coverage

- `daybreak_ranger_transforms_to_nightfall_predator` (werewolf_cards.rs:312): Verifies transform, P/T change to 4/4, name change.
- `daybreak_ranger_has_activated_ability_on_front_face` (werewolf_cards.rs:328): Verifies front face ability description contains "flying".
- `nightfall_predator_has_fight_ability` (werewolf_cards.rs:340): Verifies back face ability description contains "Fight".
- `nightfall_predator_can_fight_own_creature` (werewolf_cards.rs:353): Verifies fight targeting allows own creatures, mutual damage dealt correctly.
- **NOT TESTED**: Front face dealing 2 damage to a flying creature (end-to-end).
- **NOT TESTED**: Transform back to Daybreak Ranger when 2+ spells cast (covered generically by other werewolf tests).
