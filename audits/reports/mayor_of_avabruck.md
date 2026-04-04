# Audit: Mayor of Avabruck // Howlpack Alpha

## Official Oracle

### Front Face: Mayor of Avabruck
- **Cost:** {1}{G}
- **Type:** Creature — Human Advisor Werewolf
- **Oracle:** Other Human creatures you control get +1/+1. At the beginning of each upkeep, if no spells were cast last turn, transform Mayor of Avabruck.
- **P/T:** 1/1

### Back Face: Howlpack Alpha
- **Type:** Creature — Werewolf
- **Oracle:** Each other creature you control that's a Werewolf or a Wolf gets +1/+1. At the beginning of your end step, create a 2/2 green Wolf creature token. At the beginning of each upkeep, if a player cast two or more spells last turn, transform Howlpack Alpha.
- **P/T:** 3/3

## Implementation: `mtg-engine/src/cards/mayor_of_avabruck.rs`

### Front Face
- **Name:** Mayor of Avabruck -- CORRECT
- **Cost:** {1}{G} -- CORRECT
- **Subtypes:** Human, Advisor, Werewolf -- CORRECT
- **P/T:** 1/1 -- CORRECT
- **Continuous effect:** ModifyPT +1/+1, GlobalOther(You AND Human) -- CORRECT
- **Triggered ability:** Upkeep transform -- CORRECT

### Back Face
- **Name:** Howlpack Alpha -- CORRECT
- **Subtypes:** Werewolf -- CORRECT
- **P/T:** 3/3 (via dynamic_pt) -- CORRECT
- **Continuous effect:** ModifyPT +1/+1, GlobalOther(You AND (Werewolf OR Wolf)) -- CORRECT
- **Triggered ability:** EndStep create 2/2 Wolf token -- CORRECT
- **Wolf token:** 2/2 green creature with "Wolf" subtype -- CORRECT

### Transform Logic
- Front->Back: No spells cast last turn AND not first turn -- CORRECT
- Back->Front: Any player cast 2+ spells last turn -- CORRECT

## Issues
1. **Back face missing Upkeep triggered ability:** The back face `triggered_abilities` only includes `TriggerKind::EndStep` but is missing `TriggerKind::Upkeep` for the transform-back trigger. The `on_upkeep` handler does handle both directions, but the triggered_abilities metadata doesn't list it for the back face.

## Verdict
**FAIL** -- 1 issue: Back face metadata missing Upkeep triggered ability entry.

## Audit — 2026-04-01 09:00

**Scryfall Oracle text (front)**: Other Human creatures you control get +1/+1. / At the beginning of each upkeep, if no spells were cast last turn, transform Mayor of Avabruck.
**Scryfall Oracle text (back)**: Each other creature you control that's a Werewolf or a Wolf gets +1/+1. / At the beginning of your end step, create a 2/2 green Wolf creature token. / At the beginning of each upkeep, if a player cast two or more spells last turn, transform Howlpack Alpha.
**Scryfall type line (front)**: Creature — Human Advisor Werewolf
**Scryfall type line (back)**: Creature — Werewolf
**Status**: ISSUE

Front face: mana cost {1}{G} correct, subtypes Human/Advisor/Werewolf correct, P/T 1/1 correct, ModifyPT +1/+1 for other Humans you control correct, Upkeep triggered ability declared correctly.

Back face: P/T 3/3 correct (via dynamic_pt), subtypes ["Werewolf"] correct. ModifyPT +1/+1 scope GlobalOther(You AND (Werewolf OR Wolf)) correct. EndStep Wolf token creation: 2/2 green Wolf creature token with "Wolf" subtype correct. Only triggers during controller's end step when transformed: correct.

Issues found:
1. **Back face missing Upkeep triggered_abilities declaration** (persists from prior audit): The back face `triggered_abilities` includes `TriggerKind::EndStep` for Wolf token creation but is missing `TriggerKind::Upkeep` for the transform-back trigger. The `on_upkeep` handler correctly handles both directions of transform, but the metadata in `back_face_data` is incomplete. This could cause issues if the engine uses `triggered_abilities` to determine whether to call `on_upkeep` for a card.

Tests present in `tests/werewolf_cards.rs`. No anti-patterns found.

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch
**Oracle text (front)**: Other Human creatures you control get +1/+1. At the beginning of each upkeep, if no spells were cast last turn, transform Mayor of Avabruck.
**Oracle text (back)**: Each other creature you control that's a Werewolf or a Wolf gets +1/+1. At the beginning of your end step, create a 2/2 green Wolf creature token. At the beginning of each upkeep, if a player cast two or more spells last turn, transform Howlpack Alpha.
**Type line (front)**: Creature — Human Advisor Werewolf
**Type line (back)**: Creature — Werewolf
**Status**: ISSUE

Front face: Mana cost {1}{G}: correct. Subtypes Human/Advisor/Werewolf: correct. P/T 1/1: correct. ModifyPT +1/+1 with scope GlobalOther(You AND Human): correct ("Other Human creatures you control get +1/+1"). Upkeep triggered ability declared: correct. Transform logic (no spells last turn, not first turn): correct.

Back face: P/T 3/3 via `dynamic_pt`: correct. Subtypes ["Werewolf"]: correct. ModifyPT +1/+1 with scope GlobalOther(You AND (Werewolf OR Wolf)): correct ("Each other creature you control that's a Werewolf or a Wolf gets +1/+1"). EndStep Wolf token creation: 2/2 green Wolf creature token with "Wolf" subtype, only during controller's end step when transformed: correct. Wolf token colors [Green] and card_types [Creature]: correct.

Issues found:
1. **Back face `triggered_abilities` missing Upkeep entry** (persists from prior audits): The back face `triggered_abilities` includes `TriggerKind::EndStep` for Wolf token creation but omits `TriggerKind::Upkeep` for the transform-back trigger. The `on_upkeep` handler correctly handles both transform directions, but the metadata in `back_face_data()` is incomplete. If the engine relies on `triggered_abilities` to decide whether to invoke `on_upkeep` for a specific card, the back face's transform-back trigger would not fire.

Tests in `tests/werewolf_cards.rs` cover: front face Human buff, transform and Werewolf/Wolf buff, Wolf token creation on end step, no token on front face. No anti-patterns found.

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch, tappedout.net, Scryfall search results
**Oracle text (front)**: Other Human creatures you control get +1/+1. / At the beginning of each upkeep, if no spells were cast last turn, transform Mayor of Avabruck.
**Oracle text (back)**: Each other creature you control that's a Werewolf or a Wolf gets +1/+1. / At the beginning of your end step, create a 2/2 green Wolf creature token. / At the beginning of each upkeep, if a player cast two or more spells last turn, transform Howlpack Alpha.
**Type line (front)**: Creature — Human Advisor Werewolf
**Type line (back)**: Creature — Werewolf
**Status**: ISSUE

Front face: mana cost {1}{G} correct. Subtypes Human/Advisor/Werewolf: correct. P/T 1/1: correct. ModifyPT +1/+1 with scope GlobalOther(You AND Human): correct. Upkeep triggered ability declared: correct. Transform logic: correct.

Back face: P/T 3/3 via dynamic_pt: correct. Subtypes ["Werewolf"]: correct. ModifyPT +1/+1 with scope GlobalOther(You AND (Werewolf OR Wolf)): correct. EndStep creates 2/2 green Wolf creature token with "Wolf" subtype: correct. Only triggers on controller's end step when transformed: correct.

Issues found:
1. **Back face missing Upkeep triggered_abilities declaration** (persists from prior audits): The back face `triggered_abilities` includes `TriggerKind::EndStep` and `TriggerKind::Upkeep` -- wait, on re-inspection of the code (lines 84-93), the back face DOES include both `TriggerKind::Upkeep` (description: "transform if a player cast 2+ spells last turn") and `TriggerKind::EndStep` (description: "create a 2/2 Wolf token"). This issue has been fixed since the prior audit. No issues remain.

Correction: After re-reading the code, the back face `triggered_abilities` vec at lines 84-93 contains BOTH entries. The prior audit's issue #1 has been resolved. Changing status to PASS.

**Revised Status**: PASS

Tests in `tests/werewolf_cards.rs`: human buff, werewolf buff after transform, Wolf token creation on end step, no token on front face. Good coverage. No anti-patterns found.

## Audit — 2026-04-01 12:00

**Oracle text source**: Scryfall card page via WebSearch (https://scryfall.com/card/isd/193/mayor-of-avabruck-howlpack-alpha), confirmed by tappedout.net and Gatherer via WebSearch
**Oracle text (front)**: Other Human creatures you control get +1/+1. At the beginning of each upkeep, if no spells were cast last turn, transform Mayor of Avabruck.
**Oracle text (back)**: Each other creature you control that's a Werewolf or a Wolf gets +1/+1. At the beginning of your end step, create a 2/2 green Wolf creature token. At the beginning of each upkeep, if a player cast two or more spells last turn, transform Howlpack Alpha.
**Type line (front)**: Creature — Human Advisor Werewolf
**Type line (back)**: Creature — Werewolf
**Status**: PASS

No issues found.

Details:
- Front face: Mana cost {1}{G} correct. Subtypes Human/Advisor/Werewolf correct. P/T 1/1 correct. ModifyPT +1/+1 with scope GlobalOther(You AND Human) correct. Upkeep triggered ability declared correctly.
- Back face: P/T 3/3 via dynamic_pt correct. Subtypes ["Werewolf"] correct. ModifyPT +1/+1 with scope GlobalOther(You AND (Werewolf OR Wolf)) correct (semantically matches "Each other creature you control that's a Werewolf or a Wolf gets +1/+1"). EndStep creates 2/2 green Wolf creature token with "Wolf" subtype, only during controller's end step when transformed: correct. Back face triggered_abilities includes BOTH TriggerKind::Upkeep and TriggerKind::EndStep (lines 84-93): correct.
- Transform logic: front-to-back when no spells cast last turn (and not first turn), back-to-front when any player cast 2+ spells last turn: correct.
- Wolf token created via create_token_with_subtypes with colors [Green], types [Creature], subtypes ["Wolf"]: correct.
- Tests in werewolf_cards.rs cover: human buff, werewolf/wolf buff after transform, Wolf token creation on end step, no token on front face. Good coverage.
- No anti-patterns found. No missing triggered_abilities declarations.

## Audit — 2026-04-01 14:00

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/193/mayor-of-avabruck-howlpack-alpha
**Oracle text (front)**:
Other Human creatures you control get +1/+1.
At the beginning of each upkeep, if no spells were cast last turn, transform Mayor of Avabruck.
**Oracle text (back)**:
Each other creature you control that's a Werewolf or a Wolf gets +1/+1.
At the beginning of your end step, create a 2/2 green Wolf creature token.
At the beginning of each upkeep, if a player cast two or more spells last turn, transform Howlpack Alpha.
**Type line (front)**: Creature — Human Advisor Werewolf
**Type line (back)**: Creature — Werewolf
**P/T (front)**: 1/1
**P/T (back)**: 3/3
**Mana cost**: {1}{G}
**Ruling**: [2025-01-24] A creature that is both a Werewolf and a Wolf will only get +1/+1 from Howlpack Alpha's first ability.
**Status**: PASS

### Code issues
No issues found.

### Detailed verification

**Card data (front face)**:
- Name "Mayor of Avabruck": correct
- Cost {1}{G} (Generic(1), Green): correct
- card_types [Creature]: correct
- subtypes ["Human", "Advisor", "Werewolf"]: correct — matches "Creature — Human Advisor Werewolf"
- P/T 1/1: correct
- keywords []: correct (no keywords on oracle)
- continuous_effects ModifyPT +1/+1 with scope GlobalOther(You AND HasSubtype("Human")): correct — matches "Other Human creatures you control get +1/+1"
- triggered_abilities [Upkeep]: correct

**Card data (back face)**:
- Name "Howlpack Alpha": correct
- card_types [Creature]: correct
- subtypes ["Werewolf"]: correct — matches "Creature — Werewolf"
- P/T 3/3 (via dynamic_pt): correct
- keywords []: correct
- continuous_effects ModifyPT +1/+1 with scope GlobalOther(You AND (Werewolf OR Wolf)): correct — matches "Each other creature you control that's a Werewolf or a Wolf gets +1/+1"
- triggered_abilities [Upkeep, EndStep]: correct — both triggers present in back face data (lines 84-93)

**Transform logic** (werewolf_should_transform method):
- Front-to-back: total spells cast last turn == 0 AND not first turn: correct
- Back-to-front: any player cast 2+ spells last turn: correct — matches "if a player cast two or more spells last turn"
- Transform handled in on_upkeep: correct — both faces check at "beginning of each upkeep"

**End step token creation** (on_end_step method):
- Only when transformed (is_transformed check): correct
- Only during controller's end step (active_player == controller): correct — matches "At the beginning of your end step"
- Creates 2/2 green Wolf token with "Wolf" subtype via create_token_with_subtypes: correct

**Ruling check**: "A creature that is both a Werewolf and a Wolf will only get +1/+1 from Howlpack Alpha's first ability." The code uses a single ModifyPT with scope Or(Werewolf, Wolf), which is a single +1/+1 effect applied once per creature. A creature matching both subtypes still only gets +1/+1 from this one effect. Correct.

### Tricky interactions checked
- Front face doesn't buff Werewolves (only Humans): PASS
- Back face doesn't buff Humans (only Werewolves/Wolves): PASS
- GlobalOther excludes self from buff: PASS
- Werewolf+Wolf creature only gets +1/+1 once: PASS (single effect, not two separate effects)
- Wolf token gets +1/+1 from Howlpack Alpha (Wolf subtype matches): PASS (token has "Wolf" subtype)
- No token created on front face: PASS (on_end_step checks is_transformed)
- No token on opponent's end step: PASS (checks active_player == controller)
- Transform doesn't happen on first turn: PASS (werewolf_should_transform checks !state.is_first_turn)

### Test coverage
- Front face buffs other Humans: `werewolf_cards.rs:238` — TESTED
- Mayor doesn't buff itself: `werewolf_cards.rs:248` — TESTED
- Transform and buff Werewolves: `werewolf_cards.rs:252` — TESTED
- Wolf token creation on end step: `werewolf_cards.rs:273` — TESTED
- No token on front face: `werewolf_cards.rs:294` — TESTED
- Ruling: Werewolf+Wolf only gets +1/+1 once: NOT TESTED
- No token on opponent's end step: NOT TESTED
- Transform on first turn blocked: NOT TESTED

## Audit — 2026-04-02

**Oracle text source**: Scryfall API cache (fetched 2026-04-01) — https://scryfall.com/card/isd/193/mayor-of-avabruck-howlpack-alpha
**Oracle text (front)**:
Other Human creatures you control get +1/+1.
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
**Oracle text (back)**:
Each other creature you control that's a Werewolf or a Wolf gets +1/+1.
At the beginning of your end step, create a 2/2 green Wolf creature token.
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line (front)**: Creature — Human Advisor Werewolf
**Type line (back)**: Creature — Werewolf
**P/T (front)**: 1/1
**P/T (back)**: 3/3
**Ruling**: [2025-01-24] A creature that is both a Werewolf and a Wolf will only get +1/+1 from Howlpack Alpha's first ability.
**Status**: PASS (minor cosmetic nits only)

### Detailed verification

**Front face card_data()**:
- Name "Mayor of Avabruck": correct
- Cost Generic(1) + Green = {1}{G}: correct
- card_types [Creature]: correct
- subtypes ["Human", "Advisor", "Werewolf"]: correct
- P/T 1/1: correct
- continuous_effects: `ModifyPT { power: 1, toughness: 1, scope: GlobalOther(And(You, HasSubtype("Human"))) }` -- correctly implements "Other Human creatures you control get +1/+1". Uses `GlobalOther` (not `Global`), so self is excluded: correct.
- triggered_abilities: `[TriggerKind::Upkeep]`: correct

**Back face back_face_data()**:
- Name "Howlpack Alpha": correct
- subtypes ["Werewolf"]: correct
- P/T 3/3 via `dynamic_pt` returning `Some((3, 3))` when `is_transformed`: correct
- continuous_effects: `ModifyPT { power: 1, toughness: 1, scope: GlobalOther(And(You, Or(HasSubtype("Werewolf"), HasSubtype("Wolf")))) }` -- correctly implements "Each other creature you control that's a Werewolf or a Wolf gets +1/+1". Uses `GlobalOther`: correct. Uses `Or` so a creature with both subtypes still gets only +1/+1 from this single effect: correct per ruling.
- triggered_abilities: `[TriggerKind::Upkeep, TriggerKind::EndStep]` (lines 84-93): both present, correct.

**Transform logic** (`werewolf_should_transform`, lines 11-19):
- Front-to-back (`!is_transformed`): `total_spells_last_turn == 0 && !state.is_first_turn` -- correct. Oracle: "if no spells were cast last turn". The `is_first_turn` guard prevents transform on the very first upkeep (no "last turn" exists).
- Back-to-front (`is_transformed`): `spells_cast_last_turn.values().any(|&count| count >= 2)` -- correct. Oracle: "if a player cast two or more spells last turn". Checks per-player counts so any single player casting 2+ triggers it.

**on_upkeep** (lines 109-122): Checks zone == Battlefield, calls `should_transform`, toggles `is_transformed` and updates name. Fires for all upkeeps (both players): correct per "each upkeep".

**on_end_step** (lines 124-144): Checks zone == Battlefield, `is_transformed`, and `active_player == controller`. Creates token via `create_token_with_subtypes("Wolf", controller, 2, 2, [Green], [Creature], [], ["Wolf"])`. Correct: 2/2 green Wolf creature token, only on controller's end step, only on back face.

### Cosmetic nits (non-functional)

1. **oracle_text string (front)**: Code stores `"...transform Mayor of Avabruck."` but current Scryfall oracle reads `"...transform this creature."` (updated template). No behavioral impact -- the string is metadata only.
2. **oracle_text string (back)**: Code stores `"Other Werewolf and Wolf creatures you control get +1/+1."` but current Scryfall oracle reads `"Each other creature you control that's a Werewolf or a Wolf gets +1/+1."` Again metadata only; the actual `ContinuousEffect` filter is correct.

### Anti-pattern checks
- EffectScope::Global vs GlobalOther: Both faces correctly use `GlobalOther` to exclude self. PASS.
- Token subtypes: Wolf token includes `subtypes: vec!["Wolf".into()]`. PASS.
- Missing token color: Token includes `colors: vec![Color::Green]`. PASS.

### Test coverage (`mtg-engine/tests/werewolf_cards.rs`)
- Front face buffs other Humans: line 238 -- TESTED
- Mayor doesn't buff itself: line 248 -- TESTED
- Transform and buff Werewolves after transform: line 252 -- TESTED
- Wolf token creation on end step: line 273 -- TESTED
- No token on front face: line 294 -- TESTED
- Ruling (Werewolf+Wolf only gets +1/+1 once): NOT TESTED
- No token on opponent's end step: NOT TESTED
- Transform blocked on first turn: NOT TESTED

### LLM knowledge
No Mayor of Avabruck / Howlpack Alpha entries found in `mtg-player/src/llm.rs`.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Other Human creatures you control get +1/+1. / At the beginning of each upkeep, if no spells were cast last turn, transform this creature. // Back: Each other creature you control that's a Werewolf or a Wolf gets +1/+1. / At the beginning of your end step, create a 2/2 green Wolf creature token. / At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
**Type line**: Creature — Human Advisor Werewolf // Creature — Werewolf
**Status**: PASS

### Code issues
No issues found. Card data correct: cost {1}{G}, P/T 1/1 front / 3/3 back. Front face correctly grants +1/+1 to other Humans you control via `ModifyPT` with `GlobalOther` scope. Back face grants +1/+1 to other Werewolves or Wolves via `Or` filter. Wolf token creation on end step correctly checks `is_transformed` and `active_player == controller` (your end step only). Token is 2/2 green Wolf creature. Werewolf transform logic correct in both directions.
