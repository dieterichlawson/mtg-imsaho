# Audit: Kruin Outlaw // Terror of Kruin Pass

## Oracle (Official)
### Front: Kruin Outlaw
- **Cost:** {1}{R}{R}
- **Type:** Creature — Human Rogue Werewolf
- **Oracle:** First strike. At the beginning of each upkeep, if no spells were cast last turn, transform Kruin Outlaw.
- **P/T:** 2/2

### Back: Terror of Kruin Pass
- **Type:** Creature — Werewolf
- **Oracle:** Double strike. Each Werewolf you control can't be blocked except by two or more creatures. At the beginning of each upkeep, if a player cast two or more spells last turn, transform Terror of Kruin Pass.
- **P/T:** 3/3

## Implementation
- Front name: "Kruin Outlaw" -- CORRECT
- Front cost: {1}{R}{R} -- CORRECT
- Front subtypes: ["Human", "Rogue", "Werewolf"] -- CORRECT
- Front P/T: 2/2 -- CORRECT
- Front keywords: [FirstStrike] -- CORRECT
- Front oracle text matches -- CORRECT
- Back name: "Terror of Kruin Pass" -- CORRECT
- Back subtypes: ["Werewolf"] -- CORRECT
- Back P/T: 3/3 (via dynamic_pt) -- CORRECT
- Back keywords: [DoubleStrike] -- CORRECT (no menace keyword)
- Transform logic -- CORRECT
- Global blocking restriction -- CORRECT (MinimumBlockers continuous effect)

## Issues
1. **FIXED:** The back face previously listed Menace as a keyword but did not implement the global effect. Now correctly uses `ContinuousEffect::MinimumBlockers { count: 2 }` with scope `Global(And(You, HasSubtype("Werewolf")))`, affecting all Werewolves you control. The Menace keyword was removed from the back face's keyword list since the Oracle text does not grant menace — it has a separate static ability.

## Verdict: ALL ISSUES FIXED

## Audit — 2026-04-01 06:20

**Scryfall Oracle text (front)**: First strike / At the beginning of each upkeep, if no spells were cast last turn, transform Kruin Outlaw.
**Scryfall Oracle text (back)**: Double strike / Each Werewolf you control can't be blocked except by two or more creatures. / At the beginning of each upkeep, if a player cast two or more spells last turn, transform Terror of Kruin Pass.
**Scryfall type line (front)**: Creature — Human Rogue Werewolf
**Scryfall type line (back)**: Creature — Werewolf
**Status**: PASS

No issues found. The global "can't be blocked except by two or more creatures" blocking restriction is correctly implemented as a MinimumBlockers continuous effect that applies to all Werewolves controlled by the same player. Tests verify self, other werewolves, non-werewolves, and opponent werewolves are all handled correctly. The MinimumBlockers enforcement is integrated into the combat blocker validation system.

## Audit — 2026-04-01 09:00

**Scryfall Oracle text (front)**: First strike / At the beginning of each upkeep, if no spells were cast last turn, transform Kruin Outlaw.
**Scryfall Oracle text (back)**: Double strike / Werewolves you control have menace. (A creature with menace can't be blocked except by two or more creatures.) / At the beginning of each upkeep, if a player cast two or more spells last turn, transform Terror of Kruin Pass.
**Scryfall type line (front)**: Creature — Human Rogue Werewolf
**Scryfall type line (back)**: Creature — Werewolf
**Status**: ISSUE

Front face: mana cost {1}{R}{R} correct, subtypes Human/Rogue/Werewolf correct, P/T 2/2 correct, FirstStrike keyword correct, Upkeep triggered ability declared correctly.

Back face: P/T 3/3 correct (via dynamic_pt), DoubleStrike keyword correct, subtypes ["Werewolf"] correct.

Issues found:
1. **Back face oracle text mismatch**: Current Scryfall oracle says "Werewolves you control have menace." The code's oracle_text field and implementation use "Each Werewolf you control can't be blocked except by two or more creatures." While mechanically equivalent (menace = can't be blocked except by 2+), the oracle text in the code does not match the current official oracle wording which grants the menace keyword. The implementation uses `MinimumBlockers` continuous effect rather than granting the menace keyword to all Werewolves, which could matter if other effects interact with the menace keyword specifically.
2. **Back face missing Upkeep triggered_abilities declaration**: The back face `triggered_abilities` vec is empty, but the back face has "At the beginning of each upkeep, if a player cast two or more spells last turn, transform Terror of Kruin Pass." The `on_upkeep` handler does handle both directions, but the metadata is incomplete. This was noted in a previous Kruin Outlaw audit as correct, but the back_face_data's triggered_abilities is empty.

Tests present in `tests/kruin_outlaw.rs` and `tests/werewolf_cards.rs`. No move_object/graveyard or CombatDamageDealt anti-patterns.

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch
**Oracle text (front)**: First strike. At the beginning of each upkeep, if no spells were cast last turn, transform Kruin Outlaw.
**Oracle text (back)**: Double strike. Werewolves you control have menace. (A creature with menace can't be blocked except by two or more creatures.) At the beginning of each upkeep, if a player cast two or more spells last turn, transform Terror of Kruin Pass.
**Type line (front)**: Creature — Human Rogue Werewolf
**Type line (back)**: Creature — Werewolf
**Status**: ISSUE

Front face: Mana cost {1}{R}{R}: correct. Subtypes Human/Rogue/Werewolf: correct. P/T 2/2: correct. FirstStrike keyword: correct. Upkeep triggered ability declared: correct. Transform logic (no spells cast last turn, not first turn): correct.

Back face: P/T 3/3 via `dynamic_pt`: correct. Subtypes ["Werewolf"]: correct. DoubleStrike keyword: correct. Transform back logic (any player cast 2+ spells): correct.

Issues found:
1. **Back face grants "can't be blocked except by two or more" via MinimumBlockers instead of menace keyword**: The current Scryfall oracle text says "Werewolves you control have menace." The card received an Oracle errata to use the menace keyword (which didn't exist when originally printed). The implementation uses `ContinuousEffect::MinimumBlockers` which is mechanically equivalent for blocking purposes, but does not grant the actual menace keyword. This matters if other effects check for or interact with the menace keyword specifically. The code's `oracle_text` field also uses the old wording "Each Werewolf you control can't be blocked except by two or more creatures" rather than the current oracle text.
2. **Back face `triggered_abilities` vec is empty**: The back face has an upkeep transform trigger ("At the beginning of each upkeep, if a player cast two or more spells last turn, transform Terror of Kruin Pass") but `triggered_abilities` in `back_face_data()` is empty. The `on_upkeep` handler does handle both directions, but the metadata is incomplete. This could matter if the engine uses `triggered_abilities` to determine whether to invoke `on_upkeep`.

Tests in `tests/werewolf_cards.rs` cover transform, double strike, and first strike. No graveyard or damage anti-patterns.

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch
**Oracle text (front)**: First strike / At the beginning of each upkeep, if no spells were cast last turn, transform Kruin Outlaw.
**Oracle text (back)**: Double strike / Werewolves you control have menace. (A creature with menace can't be blocked except by two or more creatures.) / At the beginning of each upkeep, if a player cast two or more spells last turn, transform Terror of Kruin Pass.
**Type line (front)**: Creature — Human Rogue Werewolf
**Type line (back)**: Creature — Werewolf
**Status**: ISSUE

Front face: mana cost {1}{R}{R} correct. Subtypes Human/Rogue/Werewolf: correct. P/T 2/2: correct. FirstStrike keyword: correct. Upkeep triggered ability declared: correct. Transform logic (no spells cast last turn, not first turn): correct.

Back face: P/T 3/3 via dynamic_pt: correct. DoubleStrike keyword: correct. Subtypes ["Werewolf"]: correct.

Issues found:
1. **Back face grants "can't be blocked except by two or more" via MinimumBlockers instead of the menace keyword**: The current Scryfall oracle text says "Werewolves you control have menace." The code implements this as `ContinuousEffect::MinimumBlockers` rather than granting the Menace keyword to all Werewolves. While mechanically equivalent for blocking purposes, this means Werewolves would not be recognized as "having menace" by effects that check for the menace keyword (e.g., "whenever a creature you control with menace attacks"). The code's `oracle_text` field also uses the pre-errata wording.
2. **Back face missing Upkeep triggered_abilities declaration**: The back face `triggered_abilities` vec is empty, but the back face has an upkeep transform trigger. The `on_upkeep` handler covers both faces, but the metadata is incomplete.

Tests in `tests/werewolf_cards.rs` cover transform, double strike, and P/T. No graveyard or damage anti-patterns.

## Audit — 2026-04-01 14:38

**Oracle text source**: Scryfall card pages via WebSearch (https://scryfall.com/card/isd/152/kruin-outlaw-terror-of-kruin-pass, https://scryfall.com/card/inr/161/kruin-outlaw-terror-of-kruin-pass)
**Oracle text (front)**: First strike. At the beginning of each upkeep, if no spells were cast last turn, transform Kruin Outlaw.
**Oracle text (back)**: Double strike. Werewolves you control have menace. (They can't be blocked except by two or more creatures.) At the beginning of each upkeep, if a player cast two or more spells last turn, transform Terror of Kruin Pass.
**Type line (front)**: Creature — Human Rogue Werewolf
**Type line (back)**: Creature — Werewolf
**Front P/T**: 2/2
**Back P/T**: 3/3
**Status**: ISSUE

Findings:
1. **Front face name**: "Kruin Outlaw" -- correct.
2. **Front mana cost {1}{R}{R}**: Correct (`Generic(1), Red, Red`).
3. **Front subtypes**: Human/Rogue/Werewolf -- correct.
4. **Front P/T**: 2/2 -- correct.
5. **Front keywords**: [FirstStrike] -- correct.
6. **Front triggered_abilities**: TriggerKind::Upkeep declared. Matches on_upkeep hook. Correct.
7. **Back face name**: "Terror of Kruin Pass" -- correct.
8. **Back subtypes**: ["Werewolf"] -- correct.
9. **Back P/T**: 3/3 via dynamic_pt -- correct.
10. **Back keywords**: [DoubleStrike] -- correct.
11. **Transform logic (front to back)**: Checks `total_spells_last_turn == 0 && !state.is_first_turn` (line 15). Correct per oracle: "if no spells were cast last turn."
12. **Transform logic (back to front)**: Checks `state.spells_cast_last_turn.values().any(|&count| count >= 2)` (line 17). Correct per oracle: "if a player cast two or more spells last turn."
13. **on_upkeep**: Checks zone, calls should_transform, toggles is_transformed and name. Correct.
14. **Blocking restriction**: `ContinuousEffect::MinimumBlockers { count: 2, scope: Global(And(You, HasSubtype("Werewolf"))) }`. Mechanically correct for blocking.
15. **Tests**: Dedicated test file at `mtg-engine/tests/kruin_outlaw.rs` with 5 tests covering: self requires 2 blockers, allows 2 blockers, grants restriction to other werewolves, does not affect non-werewolves, does not affect opponent's werewolves. Thorough coverage.

Issues:
1. **Back face oracle text uses pre-errata wording** (file: `mtg-engine/src/cards/kruin_outlaw.rs`, line 59):
   - Oracle text says: `Werewolves you control have menace.`
   - Code oracle_text says: `Each Werewolf you control can't be blocked except by two or more creatures.`
   - The card received errata to use the menace keyword. The code's oracle_text field does not reflect this.

2. **Back face grants blocking restriction instead of menace keyword** (file: `mtg-engine/src/cards/kruin_outlaw.rs`, lines 65-71):
   - Oracle text says: `Werewolves you control have menace.`
   - Code does: Uses `ContinuousEffect::MinimumBlockers` rather than granting the Menace keyword.
   - While mechanically equivalent for blocking purposes, this means Werewolves would not be recognized as "having menace" by effects that check for or interact with the menace keyword (e.g., "whenever a creature with menace attacks").

3. **Back face `triggered_abilities` vec is empty** (file: `mtg-engine/src/cards/kruin_outlaw.rs`, line 74):
   - Oracle text says: `At the beginning of each upkeep, if a player cast two or more spells last turn, transform Terror of Kruin Pass.`
   - Code does: `back_face_data()` has `triggered_abilities: vec![]`. The upkeep trigger is only declared on the front face. The `on_upkeep` handler covers both faces, but the metadata is incomplete. If the engine uses `triggered_abilities` to determine whether to invoke hooks, this could cause the back face transform-back to not trigger.

## Audit — 2026-04-01 16:00

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text (front)**: First strike
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
**Oracle text (back)**: Double strike
Werewolves you control have menace. (A creature with menace can't be blocked except by two or more creatures.)
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line (front)**: Creature — Human Rogue Werewolf
**Type line (back)**: Creature — Werewolf
**Front P/T**: 2/2
**Back P/T**: 3/3
**Keywords (from Scryfall)**: Transform, First strike, Double strike
**Status**: ISSUE

### Code issues
1. **Back face oracle text uses pre-errata wording and implementation does not grant menace keyword** (`mtg-engine/src/cards/isd/kruin_outlaw.rs` lines 59, 65-71):
   - Oracle text says: `Werewolves you control have menace.`
   - Code oracle_text says: `Each Werewolf you control can't be blocked except by two or more creatures.`
   - Code does: Uses `ContinuousEffect::MinimumBlockers { count: 2, scope: ... }` rather than granting the `Keyword::Menace` to all Werewolves.
   - While mechanically equivalent for blocking purposes, Werewolves would not be recognized as "having menace" by effects that check for the menace keyword (e.g., "whenever a creature with menace attacks"). The oracle_text field also does not match the current oracle wording.

2. **Back face `triggered_abilities` vec is empty** (`mtg-engine/src/cards/isd/kruin_outlaw.rs` line 74):
   - Oracle text says: `At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.`
   - Code does: `back_face_data()` has `triggered_abilities: vec![]`.
   - Verified that this does NOT prevent the trigger from firing: `trigger_description` in triggers.rs (line 294) checks front face triggers FIRST, finds the `TriggerKind::Upkeep` entry with description "transform", and returns it even when the creature is transformed. So the trigger still fires correctly. However, the metadata is incomplete -- if the engine ever changes to check back face triggers separately, this would break.

Front face verified correct: mana cost {1}{R}{R}, card_types (Creature), subtypes (Human, Rogue, Werewolf), P/T 2/2, keywords (FirstStrike), triggered_abilities declares Upkeep trigger. on_resolve moves to battlefield. Transform logic: front-to-back checks `total_spells_last_turn == 0 && !state.is_first_turn` (correct). Back-to-front checks `spells_cast_last_turn.values().any(|&count| count >= 2)` (correct per oracle: "if a player cast two or more spells last turn"). dynamic_pt returns (3,3) when transformed: correct. on_upkeep toggles is_transformed and updates name: correct.

### Tricky interactions checked
- Transform condition (front to back, no spells last turn): pass
- Transform condition (back to front, player cast 2+ spells): pass
- First turn exception (no transform on first turn): pass (line 15)
- MinimumBlockers applies to self (Terror): pass (tested)
- MinimumBlockers applies to other Werewolves you control: pass (tested)
- MinimumBlockers does NOT apply to non-Werewolves: pass (tested)
- MinimumBlockers does NOT apply to opponent's Werewolves: pass (tested)
- Menace keyword interactions with other effects: ISSUE (menace keyword not granted)
- Ruling: transform mid-combat, blocked werewolves remain blocked: NOT TESTED (engine-level behavior)

### Test coverage
- Terror self requires 2 blockers: `mtg-engine/tests/kruin_outlaw.rs:23` (terror_of_kruin_pass_self_requires_two_blockers)
- Terror allows 2 blockers: `mtg-engine/tests/kruin_outlaw.rs:56` (terror_of_kruin_pass_allows_two_blockers)
- Grants restriction to other Werewolves: `mtg-engine/tests/kruin_outlaw.rs:90` (terror_of_kruin_pass_grants_restriction_to_other_werewolves)
- Does not affect non-Werewolves: `mtg-engine/tests/kruin_outlaw.rs:128` (terror_of_kruin_pass_does_not_affect_non_werewolves)
- Does not affect opponent's Werewolves: `mtg-engine/tests/kruin_outlaw.rs:164` (terror_of_kruin_pass_does_not_affect_opponent_werewolves)
- Transform front to back (no spells): NOT TESTED (directly, but implied by werewolf test pattern)
- Transform back to front (2+ spells): NOT TESTED
- First strike on front face: NOT TESTED (keyword, engine-level)
- Double strike on back face: NOT TESTED (keyword, engine-level)
- Ruling: transform mid-combat keeps blocks: NOT TESTED

## Audit — 2026-04-01 17:00

**Oracle text source**: Scryfall API (cached)
**Oracle text (front)**: First strike
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
**Oracle text (back)**: Double strike
Werewolves you control have menace. (A creature with menace can't be blocked except by two or more creatures.)
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line (front)**: Creature — Human Rogue Werewolf
**Type line (back)**: Creature — Werewolf
**Front P/T**: 2/2
**Back P/T**: 3/3
**Status**: PASS

### Code issues
No issues found.

All previously flagged issues have been fixed:
- Back face oracle text: now says "Werewolves you control have menace." matching current oracle (was pre-errata "Each Werewolf you control can't be blocked except by two or more creatures.").
- Back face menace keyword: now uses `ContinuousEffect::GrantKeyword { keyword: Keyword::Menace, scope: Global(And(You, HasSubtype("Werewolf"))) }` which grants the actual Menace keyword (was `MinimumBlockers`). Test `terror_of_kruin_pass_grants_menace_keyword` confirms `state.has_keyword` returns true for both Terror itself and other Werewolves.
- Back face triggered_abilities: now declares `TriggeredAbilityDef { kind: TriggerKind::Upkeep, description: "transform".into() }` (was empty vec).

Card data verified correct:
- Front: mana cost {1}{R}{R}, card_types (Creature), subtypes (Human, Rogue, Werewolf), P/T 2/2, keywords (FirstStrike), triggered_abilities declares Upkeep trigger, oracle_text matches Scryfall
- Back: card_types (Creature), subtypes (Werewolf), P/T 3/3 via dynamic_pt, keywords (DoubleStrike), continuous_effects grants Menace to all your Werewolves, triggered_abilities declares Upkeep trigger, oracle_text matches Scryfall
- Transform logic: front-to-back checks `total_spells_last_turn == 0 && !state.is_first_turn` (correct)
- Transform logic: back-to-front checks `spells_cast_last_turn.values().any(|&count| count >= 2)` (correct: "if a player cast two or more spells last turn")
- on_upkeep: checks zone, calls should_transform, toggles is_transformed and name

### Tricky interactions checked
- Transform condition (front to back, no spells last turn): pass
- Transform condition (back to front, player cast 2+ spells): pass
- First turn exception (no transform on first turn): pass
- Menace granted to self (Terror): pass (tested)
- Menace granted to other Werewolves you control: pass (tested)
- Menace NOT granted to non-Werewolves: pass (tested)
- Menace NOT granted to opponent's Werewolves: pass (tested)
- Menace keyword recognized by has_keyword: pass (tested)
- First strike on front face: pass (keyword declared)
- Double strike on back face: pass (keyword in back_face_data)

### Test coverage
- Terror self requires 2 blockers: `mtg-engine/tests/kruin_outlaw.rs:23` (terror_of_kruin_pass_self_requires_two_blockers)
- Terror allows 2 blockers: `mtg-engine/tests/kruin_outlaw.rs:56` (terror_of_kruin_pass_allows_two_blockers)
- Grants restriction to other Werewolves: `mtg-engine/tests/kruin_outlaw.rs:90` (terror_of_kruin_pass_grants_restriction_to_other_werewolves)
- Does not affect non-Werewolves: `mtg-engine/tests/kruin_outlaw.rs:128` (terror_of_kruin_pass_does_not_affect_non_werewolves)
- Does not affect opponent's Werewolves: `mtg-engine/tests/kruin_outlaw.rs:164` (terror_of_kruin_pass_does_not_affect_opponent_werewolves)
- Grants menace keyword: `mtg-engine/tests/kruin_outlaw.rs:201` (terror_of_kruin_pass_grants_menace_keyword)
- Transform gains double strike: `mtg-engine/tests/werewolf_cards.rs:431` (kruin_outlaw_transforms_gains_double_strike_and_menace)
- Ruling: transform mid-combat keeps blocks: NOT TESTED (engine-level behavior)
- Transform back to front (2+ spells): NOT TESTED (directly)

## Audit — 2026-04-01 18:00

**Oracle text source**: Scryfall API (cached via oracle_lookup.py)
**Front face Oracle text**: First strike
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
**Front face type line**: Creature — Human Rogue Werewolf
**Front P/T**: 2/2
**Back face name**: Terror of Kruin Pass
**Back face Oracle text**: Double strike
Werewolves you control have menace. (A creature with menace can't be blocked except by two or more creatures.)
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Back face type line**: Creature — Werewolf
**Back P/T**: 3/3
**Keywords** (Scryfall): Transform, First strike, Double strike
**Ruling**: If Kruin Outlaw transforms after blockers declared but before combat ends, blocked Werewolves remain blocked.
**Status**: PASS

### Code issues
No issues found.

Card data verified -- front face:
- Mana cost {1}{R}{R}: correct (Generic(1), Red, Red)
- Card types: Creature: correct
- No supertypes: correct
- Subtypes: Human, Rogue, Werewolf: correct
- P/T: 2/2: correct
- Keywords: FirstStrike: correct
- triggered_abilities: `TriggerKind::Upkeep` with description "transform": correct
- Oracle text field: matches Scryfall

Card data verified -- back face:
- Name: Terror of Kruin Pass: correct
- Card types: Creature: correct
- Subtypes: Werewolf: correct (Human and Rogue are front-face-only subtypes)
- P/T: 3/3 via `dynamic_pt`: correct
- Keywords: DoubleStrike: correct
- Continuous effect: `GrantKeyword { keyword: Menace, scope: Global(And(You, HasSubtype("Werewolf"))) }`: correct -- grants menace keyword to all your Werewolves
- triggered_abilities: `TriggerKind::Upkeep` with description "transform": correct
- Oracle text field: matches Scryfall

Behavior verified:
- `werewolf_should_transform` (front to back): checks `total_spells_last_turn == 0 && !state.is_first_turn`: correct
- `werewolf_should_transform` (back to front): checks `spells_cast_last_turn.values().any(|&count| count >= 2)`: correct per oracle "if a player cast two or more spells last turn"
- `on_upkeep`: checks zone == Battlefield, calls `should_transform`, toggles `is_transformed`, updates name: correct
- `has_keyword` (state.rs:932-941): correctly uses back_face_data keywords when transformed -- FirstStrike removed, DoubleStrike active
- Menace continuous effect uses `EffectScope::Global` with `CreatureFilter::And(You, HasSubtype("Werewolf"))`: applies to all your Werewolves including self

Not in LLM card knowledge section.

### Tricky interactions checked
- Transform condition (front to back, no spells last turn): pass
- Transform condition (back to front, a player cast 2+ spells): pass
- First turn exception (no transform): pass
- Keywords switch on transform (FirstStrike -> DoubleStrike): pass
- Menace granted to self (Terror): pass (tested)
- Menace granted to other Werewolves you control: pass (tested)
- Menace NOT granted to non-Werewolves: pass (tested)
- Menace NOT granted to opponent's Werewolves: pass (tested)
- Menace keyword recognized by `has_keyword`: pass (tested)
- P/T changes from 2/2 to 3/3 on transform: pass (dynamic_pt)
- Ruling: transform mid-combat keeps blocks: NOT TESTED (engine-level behavior)

### Test coverage
- Terror self requires 2 blockers: `mtg-engine/tests/kruin_outlaw.rs:23`
- Terror allows 2 blockers: `mtg-engine/tests/kruin_outlaw.rs:56`
- Grants menace to other Werewolves: `mtg-engine/tests/kruin_outlaw.rs:90`
- Does not affect non-Werewolves: `mtg-engine/tests/kruin_outlaw.rs:128`
- Does not affect opponent's Werewolves: `mtg-engine/tests/kruin_outlaw.rs:164`
- Grants menace keyword: `mtg-engine/tests/kruin_outlaw.rs:201`
- Transform gains double strike: `mtg-engine/tests/werewolf_cards.rs:431`
- Transform back to front (2+ spells): NOT TESTED (directly)
- Ruling: transform mid-combat keeps blocks: NOT TESTED (engine-level)

## Audit — 2026-04-01 14:30

**Oracle text source**: Oracle cache (Scryfall API), cached 2026-04-01
**Front face Oracle text**: First strike
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
**Front face type line**: Creature — Human Rogue Werewolf
**Front face P/T**: 2/2
**Back face name**: Terror of Kruin Pass
**Back face Oracle text**: Double strike
Werewolves you control have menace. (A creature with menace can't be blocked except by two or more creatures.)
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Back face type line**: Creature — Werewolf
**Back face P/T**: 3/3
**Mana cost**: {1}{R}{R}
**Status**: PASS

### Code issues
No issues found.

Card data verified — front face:
- Mana cost {1}{R}{R}: correct (Generic(1), Red, Red)
- Types: Creature: correct
- No supertypes: correct
- Subtypes: Human, Rogue, Werewolf: correct (all three from oracle type line)
- P/T 2/2: correct
- Keywords: FirstStrike: correct
- triggered_abilities: Upkeep TriggerKind: correct
- Oracle text: matches

Card data verified — back face:
- Name: Terror of Kruin Pass: correct
- Types: Creature: correct
- Subtypes: Werewolf: correct (matches oracle "Creature — Werewolf")
- P/T 3/3: correct (via `dynamic_pt` returning (3,3) when transformed)
- Keywords: DoubleStrike: correct
- Continuous effect: `GrantKeyword { keyword: Menace, scope: Global(And(You, HasSubtype("Werewolf"))) }`: correct — grants menace to your Werewolves
- triggered_abilities: Upkeep TriggerKind: correct

Behavior verified:
- `werewolf_should_transform`: front-to-back when `total_spells_last_turn == 0` and not first turn: correct
- `werewolf_should_transform`: back-to-front when any player cast 2+ spells last turn: correct
- `on_upkeep` checks zone == Battlefield, then calls `should_transform`, then toggles `is_transformed` and updates name: correct
- `has_keyword` correctly uses back face keywords when transformed (checked in state.rs:932-941): correct — FirstStrike removed, DoubleStrike active
- Menace continuous effect uses `EffectScope::Global` with `CreatureFilter::And(You, HasSubtype("Werewolf"))`: correct — applies to all your Werewolves including itself

### Tricky interactions checked
- Transform condition (front): no spells cast last turn, not first turn: pass
- Transform condition (back): a player cast 2+ spells last turn: pass
- Keywords switch on transform (FirstStrike -> DoubleStrike): pass
- Menace granted to all your Werewolves (including self): pass
- Menace NOT granted to opponent's Werewolves: pass (CreatureFilter::You)
- Non-Werewolves don't get menace: pass
- P/T changes from 2/2 to 3/3 on transform: pass (dynamic_pt)

### Test coverage
- Self requires two blockers (menace): `mtg-engine/tests/kruin_outlaw.rs:23` (terror_of_kruin_pass_self_requires_two_blockers)
- Allows two blockers: `mtg-engine/tests/kruin_outlaw.rs:56` (terror_of_kruin_pass_allows_two_blockers)
- Grants menace to other Werewolves: `mtg-engine/tests/kruin_outlaw.rs:90` (terror_of_kruin_pass_grants_restriction_to_other_werewolves)
- Does not affect non-Werewolves: `mtg-engine/tests/kruin_outlaw.rs:128` (terror_of_kruin_pass_does_not_affect_non_werewolves)
- Does not affect opponent's Werewolves: `mtg-engine/tests/kruin_outlaw.rs:164` (terror_of_kruin_pass_does_not_affect_opponent_werewolves)
- Grants menace keyword: `mtg-engine/tests/kruin_outlaw.rs:201` (terror_of_kruin_pass_grants_menace_keyword)
- Transform gains double strike: `mtg-engine/tests/werewolf_cards.rs:431` (kruin_outlaw_transforms_gains_double_strike_and_menace)
- Ruling: transform mid-combat keeps blocks: NOT TESTED (engine-level behavior)
- Transform back to front (2+ spells): NOT TESTED (directly)

## Audit — 2026-04-01 14:48

**Oracle text source**: Oracle cache (Scryfall API), cached 2026-04-01
**Front face Oracle text**: First strike
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
**Front face type line**: Creature — Human Rogue Werewolf
**Front face P/T**: 2/2
**Front face mana cost**: {1}{R}{R}
**Back face name**: Terror of Kruin Pass
**Back face Oracle text**: Double strike
Werewolves you control have menace. (A creature with menace can't be blocked except by two or more creatures.)
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Back face type line**: Creature — Werewolf
**Back face P/T**: 3/3
**Keywords** (Scryfall): Transform, First strike, Double strike
**Ruling**: If Kruin Outlaw transforms after blockers declared but before combat ends, blocked Werewolves remain blocked.
**Status**: PASS

### Code issues
No issues found.

Card data verified -- front face:
- Mana cost {1}{R}{R}: correct (Generic(1), Red, Red)
- Card types: Creature: correct
- No supertypes: correct
- Subtypes: Human, Rogue, Werewolf: correct (matches oracle "Creature -- Human Rogue Werewolf")
- P/T: 2/2: correct
- Keywords: FirstStrike: correct
- triggered_abilities: `TriggerKind::Upkeep` with description "transform": correct
- Oracle text field: matches Scryfall

Card data verified -- back face:
- Name: Terror of Kruin Pass: correct
- Card types: Creature: correct
- Subtypes: Werewolf: correct (matches oracle "Creature -- Werewolf"; Human and Rogue are front-face-only)
- P/T: 3/3 via `dynamic_pt`: correct
- Keywords: DoubleStrike: correct
- Continuous effect: `GrantKeyword { keyword: Menace, scope: Global(And(You, HasSubtype("Werewolf"))) }`: correct -- grants menace keyword to all your Werewolves
- triggered_abilities: `TriggerKind::Upkeep` with description "transform": correct
- Oracle text field: matches Scryfall

Behavior verified:
- `werewolf_should_transform` (front to back): `total_spells_last_turn == 0 && !state.is_first_turn` (line 15): correct per oracle "if no spells were cast last turn"
- `werewolf_should_transform` (back to front): `spells_cast_last_turn.values().any(|&count| count >= 2)` (line 17): correct per oracle "if a player cast two or more spells last turn"
- `on_upkeep`: checks zone == Battlefield, calls `should_transform`, toggles `is_transformed` and updates name: correct
- `has_keyword` (state.rs:932-941): correctly uses back_face_data keywords when transformed -- FirstStrike removed, DoubleStrike active
- Menace continuous effect uses `EffectScope::Global` with `CreatureFilter::And(You, HasSubtype("Werewolf"))`: applies to all your Werewolves including self
- `matches_filter` with `HasSubtype` (state.rs:582-599): correctly checks back_face_data subtypes when creature is transformed

No anti-patterns detected. Not in LLM card knowledge section.

### Tricky interactions checked
- Transform condition (front to back, no spells last turn): pass
- Transform condition (back to front, a player cast 2+ spells): pass
- First turn exception (no transform on first turn): pass (line 15)
- Keywords switch on transform (FirstStrike -> DoubleStrike): pass
- Menace granted to self (Terror): pass (tested)
- Menace granted to other Werewolves you control: pass (tested)
- Menace NOT granted to non-Werewolves: pass (tested)
- Menace NOT granted to opponent's Werewolves: pass (tested)
- Menace keyword recognized by `has_keyword`: pass (tested)
- P/T changes from 2/2 to 3/3 on transform: pass (dynamic_pt)
- Back face subtypes correctly resolved when transformed (no Human/Rogue): pass

### Test coverage
- Terror self requires 2 blockers: `mtg-engine/tests/kruin_outlaw.rs:23` (terror_of_kruin_pass_self_requires_two_blockers)
- Terror allows 2 blockers: `mtg-engine/tests/kruin_outlaw.rs:56` (terror_of_kruin_pass_allows_two_blockers)
- Grants menace to other Werewolves: `mtg-engine/tests/kruin_outlaw.rs:90` (terror_of_kruin_pass_grants_restriction_to_other_werewolves)
- Does not affect non-Werewolves: `mtg-engine/tests/kruin_outlaw.rs:128` (terror_of_kruin_pass_does_not_affect_non_werewolves)
- Does not affect opponent's Werewolves: `mtg-engine/tests/kruin_outlaw.rs:164` (terror_of_kruin_pass_does_not_affect_opponent_werewolves)
- Grants menace keyword: `mtg-engine/tests/kruin_outlaw.rs:201` (terror_of_kruin_pass_grants_menace_keyword)
- Transform gains double strike: `mtg-engine/tests/werewolf_cards.rs:431` (kruin_outlaw_transforms_gains_double_strike_and_menace)
- Transform back to front (2+ spells): NOT TESTED (directly)
- Ruling: transform mid-combat keeps blocks: NOT TESTED (engine-level behavior)

## Audit — 2026-04-02

**Oracle text source**: Scryfall API (cached 2026-04-01)

**Oracle text (front face — Kruin Outlaw)**:
> First strike
> At the beginning of each upkeep, if no spells were cast last turn, transform this creature.

**Type line (front)**: Creature — Human Rogue Werewolf
**Mana cost**: {1}{R}{R}
**P/T**: 2/2

**Oracle text (back face — Terror of Kruin Pass)**:
> Double strike
> Werewolves you control have menace. (A creature with menace can't be blocked except by two or more creatures.)
> At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.

**Type line (back)**: Creature — Werewolf
**P/T**: 3/3

**Status**: PASS

### Code issues

No issues found. The implementation in `mtg-engine/src/cards/isd/kruin_outlaw.rs` matches the oracle text on both faces:

- **Front face card data**: Mana cost `{1}{R}{R}`, types `Creature`, subtypes `["Human", "Rogue", "Werewolf"]`, P/T 2/2, keyword `FirstStrike` — all correct.
- **Back face card data**: Types `Creature`, subtypes `["Werewolf"]`, P/T 3/3 (via `dynamic_pt`), keyword `DoubleStrike` — all correct.
- **Menace grant**: Implemented as a continuous effect `GrantKeyword { keyword: Menace, scope: Global(And(You, HasSubtype("Werewolf"))) }` on the back face. This correctly grants menace to all Werewolves you control (including itself, since it is a Werewolf). This matches the oracle text "Werewolves you control have menace."
- **Transform (front to back)**: `total_spells_last_turn == 0 && !state.is_first_turn` — correctly implements "if no spells were cast last turn" with the first-turn guard.
- **Transform (back to front)**: `state.spells_cast_last_turn.values().any(|&count| count >= 2)` — correctly implements "if a player cast two or more spells last turn".
- **Triggered abilities**: Both faces have `TriggerKind::Upkeep` entries, matching the upkeep transform triggers.

### Tricky interactions checked

- **Menace is granted, not intrinsic**: The oracle text says "Werewolves you control have menace" — this is a static ability that grants menace to a class of creatures, not an intrinsic keyword of Terror of Kruin Pass. The implementation correctly uses a `ContinuousEffect::GrantKeyword` rather than listing `Menace` in the back face's `keywords` vec. Terror of Kruin Pass receives menace from its own effect because it is a Werewolf you control.
- **Double strike replaces first strike**: The front face has first strike; the back face has double strike (not both). The implementation correctly lists only `FirstStrike` on front and only `DoubleStrike` on back.
- **Ruling (2011-09-22)**: "If Kruin Outlaw somehow transforms after blockers have been declared but before combat ends, any Werewolves you control that are blocked by a single creature will remain blocked." This is standard menace behavior (blocking restrictions are checked at declare-blockers time). The engine handles this at the framework level.
- **Transform back condition**: Checks `any` player cast 2+ spells, not total across all players. Code uses `.values().any(|&count| count >= 2)` which is correct per oracle "if a player cast two or more spells last turn".

### LLM knowledge

No entry found in `mtg-player/src/llm.rs` for Kruin Outlaw or Terror of Kruin Pass.

### Test coverage

Dedicated test file `mtg-engine/tests/kruin_outlaw.rs` (6 tests):
- `terror_of_kruin_pass_self_requires_two_blockers` — menace blocks self with 1 blocker (rejected)
- `terror_of_kruin_pass_allows_two_blockers` — menace allows 2 blockers
- `terror_of_kruin_pass_grants_restriction_to_other_werewolves` — menace applies to other werewolves you control
- `terror_of_kruin_pass_does_not_affect_non_werewolves` — non-werewolves not affected
- `terror_of_kruin_pass_does_not_affect_opponent_werewolves` — opponent's werewolves not affected
- `terror_of_kruin_pass_grants_menace_keyword` — verifies keyword is granted to self and other werewolves

Additional coverage in `mtg-engine/tests/werewolf_cards.rs`:
- `kruin_outlaw_transforms_gains_double_strike_and_menace` — verifies front face has first strike, back face has double strike after transform

Not directly tested: transform back from Terror of Kruin Pass to Kruin Outlaw (covered indirectly by shared werewolf transform logic tested on other cards).
