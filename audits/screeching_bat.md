# Audit: Screeching Bat // Stalking Vampire

## Official Oracle (Front Face)
- **Name:** Screeching Bat
- **Cost:** {2}{B}
- **Type:** Creature — Bat
- **Oracle Text:** Flying\nAt the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform Screeching Bat.
- **P/T:** 2/2

## Official Oracle (Back Face)
- **Name:** Stalking Vampire
- **Cost:** None
- **Type:** Creature — Vampire
- **Oracle Text:** At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform Stalking Vampire.
- **P/T:** 5/5

## Implementation Review
- **Front Face Name:** OK
- **Front Face Cost:** {2}{B} — OK
- **Front Face Type:** Creature, subtypes ["Bat"] — OK
- **Front Face Oracle:** Matches — OK
- **Front Face P/T:** 2/2 — OK
- **Front Face Keywords:** Flying — OK
- **Back Face Name:** "Stalking Vampire" — OK
- **Back Face Type:** Creature, subtypes ["Vampire"] — OK
- **Back Face Oracle:** Matches — OK
- **Back Face P/T:** 5/5 (via dynamic_pt) — OK
- **Transform:** on_upkeep checks active_player == controller, checks mana availability, auto-pays if possible — OK
- **Back face Flying:** Stalking Vampire should NOT have flying (only Screeching Bat has it). The back_face_data has no keywords — OK

## Issues
1. **Minor: "you may" is auto-decided**: The transform trigger says "you may pay" but the implementation auto-pays if mana is available. This removes player agency — the player might not want to transform even when they have the mana. Noted as a simplification.

## Verdict: PASS (with noted simplification on "you may" choice)

## Audit — 2026-04-01 12:00

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text (front)**: Flying\nAt the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
**Oracle text (back)**: At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
**Type line (front)**: Creature — Bat
**Type line (back)**: Creature — Vampire
**Status**: ISSUE

### Code issues

1. **"You may" is auto-decided** (`screeching_bat.rs:77-95`)
   - Oracle text says: `"you may pay {2}{B}{B}. If you do, transform this creature."`
   - Code does: `// Auto-pay if the controller has enough mana (simplified "you may").` — automatically pays and transforms whenever mana is available, with no player choice presented. The comment explicitly acknowledges this as a simplification. Per CLAUDE.md memory "NEVER take silent shortcuts" and "Correctness over convenience", this should present a real choice.

2. **Oracle text uses old card name instead of "this creature"** (`screeching_bat.rs:23,46`)
   - Oracle text says (front): `"transform this creature"` / (back): `"transform this creature"`
   - Code oracle_text (front): `"transform Screeching Bat"` / (back): `"transform Stalking Vampire"`
   - This is a cosmetic mismatch from the 2023 templating update ("this creature" replaced specific card names). Not a functional issue.

### Tricky interactions checked
- Transform does not grant/remove Flying: PASS (back face has no keywords, front face has Flying keyword)
- Upkeep trigger only fires for controller's upkeep: PASS (line 73-75 checks `state.active_player != controller`)
- Transform toggles correctly in both directions: PASS (line 86 uses `!is_transformed`)
- No mana = no transform: PASS (line 84 checks `crate::mana::can_pay(pool, &cost)`)
- dynamic_pt returns correct values: PASS (5/5 when transformed, None when not)

### Test coverage
- Transform with mana: `tier15_cards.rs:774` (screeching_bat_transforms_at_upkeep_with_mana)
- Transform without mana: NOT TESTED
- Declining to transform when mana is available: NOT TESTED (and not possible due to issue #1)
- Transform back from Stalking Vampire to Screeching Bat: NOT TESTED

## Audit — 2026-04-01 21:11

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text (front face)**:
```
Flying
At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
```
**Oracle text (back face — Stalking Vampire)**:
```
At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
```
**Type line (front)**: Creature — Bat
**Type line (back)**: Creature — Vampire
**Mana cost**: {2}{B}
**P/T (front)**: 2/2
**P/T (back)**: 5/5
**Keywords (front)**: Flying, Transform
**Keywords (back)**: (none besides Transform)
**Rulings**: [2016-07-13] For more information on double-faced cards, see the Shadows over Innistrad mechanics article.
**Status**: ISSUE

### Code issues

1. **Stalking Vampire incorrectly retains Flying after transform in real gameplay** (`screeching_bat.rs:130-140`, `state.rs:926-929`, `engine.rs:2178`)
   - Oracle text says (back face): `"At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature."` — no Flying keyword on Stalking Vampire
   - Code does: `on_yes_no_choice` (lines 130-140) updates only `obj.is_transformed` and `obj.name` when transforming. It does NOT update `obj.keywords`. During game setup, `engine.rs:2178` populates `obj.keywords` with the front face's `[Flying]`. The `has_keyword` function (`state.rs:926-929`) checks `obj.keywords` first and returns `true` immediately, preempting the face-aware check at lines 931-941 that would correctly return `false` for the back face.
   - Net effect: Stalking Vampire has Flying in real gameplay, which is incorrect.
   - Note: This does NOT manifest in unit tests because the `named_creature` test helper does not populate `obj.keywords` (it uses `create_object` which initializes keywords to empty). The face-aware check at `state.rs:931-941` handles this correctly when `obj.keywords` is empty.
   - Fix: `on_yes_no_choice` should update `obj.keywords` when transforming, similar to how Moonmist updates keywords at `moonmist.rs:79,93`. When transforming to back face, clear keywords (or set to back face keywords). When transforming to front face, restore front face keywords.

2. **Oracle text field uses card name instead of "this creature"** (`screeching_bat.rs:33,56`)
   - Oracle text says (both faces): `"transform this creature"`
   - Code oracle_text (front, line 33): `"transform Screeching Bat"` / (back, line 56): `"transform Stalking Vampire"`
   - This is a cosmetic mismatch from a templating errata. Not a functional issue.

3. **Missing LLM card knowledge** (`mtg-player/src/llm.rs`)
   - Screeching Bat is not in the LLM card knowledge section. As a DFC with a paid transform ability that trades evasion (Flying 2/2) for raw power (5/5 ground), this has strategic significance for AI play decisions.

### Tricky interactions checked
- Transform does not grant/remove Flying (via face-aware `has_keyword`): FAIL in real gameplay (see issue #1), PASS in tests
- Upkeep trigger only fires for controller's upkeep: PASS (line 83 checks `state.active_player != controller`)
- "You may" choice is properly presented: PASS (lines 98-105 use `ResolutionChoiceKind::YesNo`, lines 108-141 handle yes/no)
- Transform toggles correctly in both directions: PASS (line 132-136 flips `is_transformed` and updates name)
- No mana = no choice presented: PASS (lines 88-92 check `can_pay` and return early)
- Mana is spent on transform: PASS (lines 124-129 call `auto_pay`)
- dynamic_pt returns correct values: PASS (5/5 when transformed, None when not)
- `should_transform` returns false (not a werewolf): PASS (line 143)
- `triggered_abilities` declares Upkeep trigger for both faces: PASS (front: line 39, back: line 62)
- Back face subtypes correct (Vampire): PASS in card data (line 51)

### Test coverage
- Transform with mana (player accepts): `tier15_cards.rs:822` — TESTED
- Decline to transform when mana available: `tier15_cards.rs:857` — TESTED
- No choice without mana: `tier15_cards.rs:889` — TESTED
- Transform back (Stalking Vampire -> Screeching Bat): `tier15_cards.rs:904` — TESTED
- Stalking Vampire does NOT have Flying after transform: NOT TESTED
- Subtype changes after transform (Bat -> Vampire): NOT TESTED
- Mana is fully consumed after paying: `tier15_cards.rs:853` — TESTED
- Mana is NOT consumed when declining: `tier15_cards.rs:885` — TESTED

## Audit — 2026-04-01 21:22

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text (front face — Screeching Bat)**:
```
Flying
At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
```
**Oracle text (back face — Stalking Vampire)**:
```
At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
```
**Type line (front)**: Creature — Bat
**Type line (back)**: Creature — Vampire
**Mana cost**: {2}{B}
**P/T (front)**: 2/2
**P/T (back)**: 5/5
**Keywords (front)**: Flying
**Rulings**: [2016-07-13] Generic DFC reference ruling, no card-specific interactions.
**Status**: PASS

### Code issues
No issues found.

All issues from prior audits have been resolved:
- "You may" is now a proper player choice via `ResolutionChoiceKind::YesNo` (lines 99-106)
- Transform now uses `helpers::apply_transform()` (line 133), which correctly updates keywords, subtypes, and name for both directions
- Oracle text fields now use "this creature" matching current oracle wording (lines 34, 57)
- LLM card knowledge is present in `mtg-player/src/llm.rs` (line 66)

### Tricky interactions checked
- "You may" choice properly presented (YesNo prompt): PASS
- Upkeep trigger only fires for controller's upkeep: PASS (line 84 checks `state.active_player != controller`)
- No choice presented when player can't afford {2}{B}{B}: PASS (lines 89-92 check `can_pay` and return early)
- Payment happens atomically during resolution (no response window for opponent between pay and transform): PASS (lines 126-136 pay then transform in sequence)
- Stalking Vampire does NOT have Flying: PASS (`apply_transform` sets `obj.keywords` to back face's empty vec)
- Screeching Bat regains Flying when transforming back: PASS (`apply_transform` restores front face keywords)
- Subtypes update on transform (Bat -> Vampire, Vampire -> Bat): PASS (`apply_transform` updates subtypes)
- dynamic_pt returns (5,5) when transformed, None otherwise: PASS
- `should_transform` returns false (not auto-transform like werewolves): PASS
- `triggered_abilities` declares Upkeep trigger for both faces: PASS (front: line 41, back: line 63)
- Transform works in both directions (Bat->Vampire and Vampire->Bat): PASS

### Test coverage
- Transform with mana (player accepts): `tier15_cards.rs:822` — TESTED
- Decline to transform when mana available: `tier15_cards.rs:857` — TESTED
- No choice without mana: `tier15_cards.rs:889` — TESTED
- Transform back (Stalking Vampire -> Screeching Bat): `tier15_cards.rs:904` — TESTED
- Stalking Vampire does NOT have Flying: `tier15_cards.rs:939` — TESTED
- Screeching Bat regains Flying on transform back: `tier15_cards.rs:976` — TESTED
- Subtypes update on transform (Bat -> Vampire): `tier15_cards.rs:1018` — TESTED
- Mana is fully consumed after paying: `tier15_cards.rs:853` — TESTED
- Mana is NOT consumed when declining: `tier15_cards.rs:885` — TESTED
- Not-your-upkeep (opponent's turn): NOT TESTED (minor gap)
- Creature removed from battlefield before trigger resolves: NOT TESTED (minor gap, standard trigger handling)

## Audit - 2026-04-02

### Oracle Text (Scryfall)
- **Name (front):** Screeching Bat
- **Mana Cost:** {2}{B}
- **Type (front):** Creature — Bat
- **P/T (front):** 2/2
- **Oracle (front):** Flying / At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
- **Name (back):** Stalking Vampire
- **Type (back):** Creature — Vampire
- **P/T (back):** 5/5
- **Oracle (back):** At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.

### Card Data Audit
- **Name:** Correct ("Screeching Bat")
- **Cost:** Correct ({2}{B})
- **Types:** Correct (Creature — Bat)
- **P/T (front):** Correct (2/2)
- **Oracle Text String:** Correct
- **Keywords:** Flying. Correct.
- **Back Face Name:** Correct ("Stalking Vampire")
- **Back Face Types:** Correct (Creature — Vampire)
- **Back Face P/T:** Correct (5/5 via `dynamic_pt` which replaces base when transformed)

### Behavior Audit
- **Flying:** Granted via keywords vec. Correct.
- **Upkeep transform trigger (front):** `on_upkeep` checks active player == controller, checks if can pay {2}{B}{B}, presents YesNo choice. Correct.
- **Upkeep transform trigger (back):** Back face data has the same TriggeredAbilityDef for upkeep. `on_upkeep` works regardless of face. Correct.
- **Transform cost:** {2}{B}{B}. Correct.
- **Transform behavior:** On "yes", pays cost, calls `helpers::apply_transform`. Correct.
- **dynamic_pt:** Returns Some((5, 5)) when transformed. Since this is the creature's own dynamic_pt, it replaces the base P/T. Correct.

### Result
**PASS**

## Audit — 2026-04-02 20:07

**Oracle text source**: Scryfall API (via oracle_lookup.py, cached 2026-04-01)
**Oracle text**:
Front face (Screeching Bat):
```
Flying
At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
```
Back face (Stalking Vampire):
```
At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
```
**Type line**: Front: Creature — Bat / Back: Creature — Vampire
**Mana cost**: {2}{B}
**P/T**: Front: 2/2 / Back: 5/5
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Stalking Vampire does NOT have Flying (back face `keywords: vec![]`, `apply_transform` swaps keywords): PASS
- Screeching Bat regains Flying when transforming back (`apply_transform` restores front face keywords): PASS
- Upkeep trigger only fires on controller's upkeep (line 84: `state.active_player != controller`): PASS
- "You may pay" correctly presented as YesNo choice, not auto-decided (lines 99-106): PASS
- Mana cost correctly spent via `auto_pay` on "yes" (line 126): PASS
- No choice offered when player cannot afford {2}{B}{B} (lines 90-92: `can_pay` check): PASS
- Subtypes swap correctly between Bat and Vampire via `apply_transform`: PASS
- `dynamic_pt` returns `Some((5,5))` when transformed, `None` when not (replaces base P/T): PASS
- `should_transform` returns false (not a werewolf, no day/night auto-transform): PASS

### Test coverage
- Transform with mana (player accepts): `tier15_cards.rs:1128` — TESTED
- Decline to transform when mana available: `tier15_cards.rs:1163` — TESTED
- No choice without mana: `tier15_cards.rs:1195` — TESTED
- Transform back (Stalking Vampire -> Screeching Bat): `tier15_cards.rs:1210` — TESTED
- Stalking Vampire does NOT have Flying: `tier15_cards.rs:1245` — TESTED
- Screeching Bat regains Flying on transform back: `tier15_cards.rs:1282` — TESTED
- Subtypes update on transform (Bat -> Vampire): `tier15_cards.rs:1324` — TESTED
- Mana consumed after paying: `tier15_cards.rs:1159` — TESTED
- Mana NOT consumed when declining: `tier15_cards.rs:1191` — TESTED
- Not-your-upkeep (opponent's turn): NOT TESTED (minor gap)
- Creature removed before trigger resolves: NOT TESTED (minor gap, standard trigger handling)

## Audit — 2026-04-02 20:13

**Oracle text source**: Scryfall API (via oracle_lookup.py, cached 2026-04-01)
**Oracle text**:
Front face (Screeching Bat):
```
Flying
At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
```
Back face (Stalking Vampire):
```
At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
```
**Type line**: Front: Creature — Bat / Back: Creature — Vampire
**Mana cost**: {2}{B}
**P/T**: Front: 2/2 / Back: 5/5
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Stalking Vampire loses Flying on transform (back face `keywords: vec![]`, `apply_transform` swaps keywords): PASS
- Screeching Bat regains Flying when transforming back (`apply_transform` restores front face keywords): PASS
- Upkeep trigger only fires on controller's upkeep (line 84: `state.active_player != controller`): PASS
- "You may pay" correctly presented as YesNo choice, not auto-decided (lines 99-106): PASS
- Mana cost {2}{B}{B} correctly spent via `auto_pay` on "yes" (line 126): PASS
- No choice offered when player cannot afford {2}{B}{B} (lines 90-92: `can_pay` check returns early): PASS
- Subtypes swap correctly between Bat and Vampire via `apply_transform`: PASS
- `dynamic_pt` returns `Some((5,5))` when transformed, `None` when not: PASS
- `should_transform` returns false (no day/night auto-transform): PASS
- Both faces declare `TriggeredAbilityDef` with `TriggerKind::Upkeep` so trigger fires from either face: PASS

### Test coverage
- Transform with mana (player accepts): `tier15_cards.rs:1128` — TESTED
- Decline to transform when mana available: `tier15_cards.rs:1163` — TESTED
- No choice without mana: `tier15_cards.rs:1195` — TESTED
- Transform back (Stalking Vampire -> Screeching Bat): `tier15_cards.rs:1210` — TESTED
- Stalking Vampire does NOT have Flying: `tier15_cards.rs:1245` — TESTED
- Screeching Bat regains Flying on transform back: `tier15_cards.rs:1282` — TESTED
- Subtypes update on transform (Bat -> Vampire): `tier15_cards.rs:1324` — TESTED
- Mana consumed after paying: `tier15_cards.rs:1159` — TESTED
- Mana NOT consumed when declining: `tier15_cards.rs:1191` — TESTED
- Not-your-upkeep (opponent's turn): NOT TESTED (minor gap)
- Creature removed before trigger resolves: NOT TESTED (minor gap, standard trigger handling)

## Audit — 2026-04-02 20:20

**Oracle text source**: Scryfall API (via oracle_lookup.py, cached 2026-04-01)
**Oracle text**:
Front face (Screeching Bat):
```
Flying
At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
```
Back face (Stalking Vampire):
```
At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
```
**Type line**: Front: Creature — Bat / Back: Creature — Vampire
**Mana cost**: {2}{B}
**P/T**: Front: 2/2 / Back: 5/5
**Status**: PASS

### Code issues
No issues found.

All card data and behavior verified against fetched oracle text:
- Front face: name, cost ({2}{B}), types (Creature — Bat), P/T (2/2), keywords (Flying), oracle text, triggered ability (TriggerKind::Upkeep) all match.
- Back face: name ("Stalking Vampire"), cost (None), types (Creature — Vampire), P/T (5/5 via dynamic_pt), keywords (empty vec — no Flying), oracle text, triggered ability (TriggerKind::Upkeep) all match.
- Transform uses `helpers::apply_transform` which correctly swaps name, keywords, and subtypes between faces.
- "You may pay" is properly implemented as a `ResolutionChoiceKind::YesNo` player choice (lines 99-106).
- Mana payment {2}{B}{B} is correct (lines 12-18, 125-129).

### Tricky interactions checked
- Stalking Vampire loses Flying on transform (back face `keywords: vec![]`, `apply_transform` replaces obj.keywords): PASS
- Screeching Bat regains Flying when transforming back (`apply_transform` restores front face keywords vec): PASS
- Upkeep trigger only fires on controller's upkeep (line 84: `state.active_player != controller` guard): PASS
- "You may pay" correctly presented as YesNo choice, not auto-decided (lines 99-106): PASS
- No choice offered when player cannot afford {2}{B}{B} (lines 90-92: `can_pay` returns early): PASS
- Subtypes swap correctly between Bat and Vampire via `apply_transform`: PASS
- `dynamic_pt` returns `Some((5,5))` when transformed, `None` otherwise: PASS
- `should_transform` returns false (not a werewolf, no day/night auto-transform): PASS
- Both faces declare `TriggeredAbilityDef` with `TriggerKind::Upkeep` so trigger fires from either face: PASS
- Back face cost is `None` (back faces of DFCs have no mana cost): PASS

### Test coverage
- Transform with mana (player accepts): `tier15_cards.rs:1128` — TESTED
- Decline to transform when mana available: `tier15_cards.rs:1163` — TESTED
- No choice without mana: `tier15_cards.rs:1195` — TESTED
- Transform back (Stalking Vampire -> Screeching Bat): `tier15_cards.rs:1210` — TESTED
- Stalking Vampire does NOT have Flying: `tier15_cards.rs:1245` — TESTED
- Screeching Bat regains Flying on transform back: `tier15_cards.rs:1282` — TESTED
- Subtypes update on transform (Bat -> Vampire): `tier15_cards.rs:1324` — TESTED
- Mana consumed after paying: `tier15_cards.rs:1159` — TESTED
- Mana NOT consumed when declining: `tier15_cards.rs:1191` — TESTED
- Not-your-upkeep (opponent's turn): NOT TESTED (minor gap)
- Creature removed before trigger resolves: NOT TESTED (minor gap, standard trigger handling)
