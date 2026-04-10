# Audit: Delver of Secrets // Insectile Aberration

## Scryfall Reference
- **Front Face: Delver of Secrets**
  - **Cost:** {U}
  - **Type:** Creature -- Human Wizard
  - **Oracle:** At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform this creature.
  - **P/T:** 1/1

- **Back Face: Insectile Aberration**
  - **Cost:** (none)
  - **Type:** Creature -- Human Insect
  - **Oracle:** Flying
  - **P/T:** 3/2

## Implementation: `delver_of_secrets.rs`
- **Front face name:** Delver of Secrets -- CORRECT
- **Cost:** {U} -- CORRECT
- **Front subtypes:** ["Human", "Wizard"] -- CORRECT
- **Front P/T:** 1/1 -- CORRECT
- **Back face name:** Insectile Aberration -- CORRECT
- **Back subtypes:** ["Human", "Insect"] -- CORRECT
- **Back P/T:** 3/2 -- CORRECT
- **Back keywords:** [Flying] -- CORRECT
- **Trigger:** Upkeep -- CORRECT
- **Behavior:** Checks top card of library for instant/sorcery, transforms if found -- CORRECT

## Issues
None

## Audit — 2026-04-01 15:12

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text (front)**: At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform Delver of Secrets.
**Oracle text (back)**: Flying
**Type line (front)**: Creature — Human Wizard
**Type line (back)**: Creature — Human Insect
**Ruling**: [2011-09-22] You may reveal the card even if it's not an instant or sorcery. Whether or not you reveal it, the card stays on top of your library.
**Status**: ISSUE

### Code issues

1. **"You may" choice is not presented to the player** (`mtg-engine/src/cards/isd/delver_of_secrets.rs:86`)
   - Oracle text says: `You may reveal that card. If an instant or sorcery card is revealed this way, transform Delver of Secrets.`
   - Code does: `if is_instant_or_sorcery { ... obj.is_transformed = true; }` — the code automatically transforms Delver whenever the top card is an instant or sorcery, without giving the player the choice to decline the reveal.
   - The "You may" is strategically relevant: a player might want to avoid revealing information to their opponent, or might not want Delver to transform in certain situations (e.g., if they have equipment or auras that benefit Human creatures specifically).
   - The ruling explicitly confirms: "You may reveal the card even if it's not an instant or sorcery. Whether or not you reveal it, the card stays on top of your library."

### Tricky interactions checked
- Only triggers on controller's upkeep (not each upkeep): PASS — code checks `state.active_player != controller`
- Only triggers on front face: PASS — code checks `is_transformed` and returns if true
- Empty library: PASS — `top_card_id` would be `None`, gracefully handled
- Card stays on top of library after checking: PASS — code only reads the top card, never moves it
- Back face has Flying keyword: PASS
- Dynamic P/T for back face (3/2): PASS

### Test coverage
- Transform when top card is instant: `tier15_cards.rs:delver_transforms_when_top_card_is_instant` — TESTED
- Does not transform when top card is creature: `tier15_cards.rs:delver_does_not_transform_when_top_card_is_creature` — TESTED
- Player choosing NOT to reveal (you may decline): NOT TESTED (bug — choice not implemented)
- Transform when top card is sorcery: NOT TESTED
- Empty library (no crash): NOT TESTED
- Multiple Delvers checking same top card: NOT TESTED
- Back face does not trigger on upkeep: NOT TESTED (implicit from code structure)

## Audit — 2026-04-01 18:30

**Oracle text source**: Oracle cache (Scryfall API), cached 2026-04-01
**Oracle text (front)**: At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform this creature.
**Oracle text (back)**: Flying
**Type line (front)**: Creature — Human Wizard
**Type line (back)**: Creature — Human Insect
**Mana cost**: {U}
**Front P/T**: 1/1
**Back P/T**: 3/2
**Ruling [2011-09-22]**: You may reveal the card even if it's not an instant or sorcery. Whether or not you reveal it, the card stays on top of your library.
**Status**: PASS

### Code issues
No issues found.

**Minor notes (not issues):**

1. **Oracle text string slightly stale** (`delver_of_secrets.rs:44`): The code's `oracle_text` field says "transform Delver of Secrets" while the current Scryfall oracle text says "transform this creature." This is display-only and does not affect behavior. The card was errata'd at some point to use the more generic templating.

2. **Reveal choice only offered for instant/sorcery** (`delver_of_secrets.rs:104`): Per the ruling, "You may reveal the card even if it's not an instant or sorcery." The code only presents the YesNo choice when the top card IS an instant or sorcery. Revealing a non-instant/sorcery would have no mechanical effect (no transform, and no reveal-matters events exist in the engine), so this shortcut produces correct game outcomes. It is a pragmatic optimization, not a bug.

### Card data verification
- Mana cost {U}: PASS — `ManaSymbol::Colored(Color::Blue)` (line 37)
- Card type Creature: PASS (line 39)
- Supertypes none: PASS (line 40)
- Subtypes Human Wizard: PASS — `["Human", "Wizard"]` (line 41)
- Power/toughness 1/1: PASS (lines 42-43)
- Front keywords none: PASS (line 45) — "Transform" is Scryfall metadata, not a game keyword; the engine has no `Keyword::Transform` variant; no other DFC declares it
- Back face name Insectile Aberration: PASS (line 60)
- Back face type Creature: PASS (line 62)
- Back face subtypes Human Insect: PASS — `["Human", "Insect"]` (line 64)
- Back face P/T 3/2: PASS (lines 65-66)
- Back face keywords Flying: PASS — `Keyword::Flying` (line 68)
- Triggered abilities declaration: PASS — `TriggerKind::Upkeep` matches `on_upkeep` hook (line 51)
- Oracle text field: PASS (minor wording difference noted above)

### Behavior verification
- Upkeep trigger only on controller's upkeep: PASS — checks `state.active_player != controller` (line 90)
- Only triggers on front face: PASS — checks `is_transformed` and returns if true (line 90)
- Only triggers on battlefield: PASS — zone check at line 86
- Looks at top card of library: PASS — reads `library_order.first()` (line 14, 97)
- "You may" is optional via YesNo choice: PASS — `ResolutionChoiceKind::YesNo` presented to controller (lines 106-116)
- Player can decline reveal: PASS — `on_yes_no_choice` with `yes=false` returns without transforming (lines 122-125)
- Transform sets `is_transformed = true` and updates name: PASS (lines 138-140)
- Dynamic P/T returns (3,2) when transformed, None when not: PASS (lines 76-81)
- Card stays on top of library: PASS — code never moves the top card
- Empty library: PASS — `library_order.first()` returns None, `top_card_is_instant_or_sorcery` returns false (lines 26-27)
- Engine handles back-face keywords via `effective_keywords` check: PASS — `state.rs:932-938` checks `back_face_data().keywords` when `is_transformed`

### Tricky interactions checked
- Only triggers on controller's upkeep (not each upkeep): PASS
- Only triggers on front face (Insectile Aberration has no upkeep ability): PASS
- Empty library gracefully handled: PASS
- Card stays on top of library after reveal: PASS
- Back face has Flying keyword (engine resolves via back_face_data): PASS
- Dynamic P/T for back face (3/2) vs front face (1/1): PASS
- "You may" choice correctly presented (not auto-applied): PASS
- Declining reveal leaves card untransformed: PASS
- Non-instant/sorcery top card: no choice presented, no transform: PASS

### Test coverage
- Transform when top card is instant and player reveals: `tier15_cards.rs:693` (delver_transforms_when_player_reveals_instant) — TESTED
- Player declining to reveal: `tier15_cards.rs:732` (delver_does_not_transform_when_player_declines_reveal) — TESTED
- Top card is creature (non-instant/sorcery): `tier15_cards.rs:765` (delver_does_not_transform_when_top_card_is_creature) — TESTED
- Card stays on top of library after reveal: `tier15_cards.rs:728` — TESTED (asserted in transform test)
- Card stays on top of library after declining: `tier15_cards.rs:761` — TESTED (asserted in decline test)
- Dynamic P/T 3/2 after transform: `tier15_cards.rs:725` — TESTED
- Transform when top card is sorcery: NOT TESTED
- Empty library (no crash): NOT TESTED
- Multiple Delvers checking same top card: NOT TESTED
- Back face does not trigger on upkeep: NOT TESTED (implicit from code structure)
- Ruling: reveal non-instant/sorcery (no transform): NOT TESTED (shortcut means choice is never offered)

### UI presentation
- YesNo choice description is clear: "Delver of Secrets: reveal {card name} from the top of your library to transform?" — PASS
- Trigger description on stack: "look at top card, may reveal to transform" — PASS
- Card is in LLM card knowledge: PASS — found in `mtg-player/src/llm.rs:61`
- LLM knowledge description is accurate: PASS — "At your upkeep, looks at top card of library. If it's an instant or sorcery, you may reveal it to transform into Insectile Aberration (3/2 flying). ALWAYS choose yes when asked to reveal"

## Audit — 2026-04-02

**Oracle text source**: Oracle cache (Scryfall API), cached 2026-04-01
**Oracle text (front)**: At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform this creature.
**Oracle text (back)**: Flying
**Type line (front)**: Creature — Human Wizard
**Type line (back)**: Creature — Human Insect
**Mana cost**: {U}
**Front P/T**: 1/1
**Back P/T**: 3/2
**Ruling [2011-09-22]**: You may reveal the card even if it's not an instant or sorcery. Whether or not you reveal it, the card stays on top of your library.
**Status**: PASS

### Code issues
No issues found. All prior issues from the 2026-04-01 15:12 audit (missing "you may" choice) have been resolved.

**Minor notes (not issues):**

1. **Oracle text field wording**: Code `oracle_text` at line 44 says `"transform Delver of Secrets"` while current Scryfall oracle text says `"transform this creature."` This is display-only and does not affect behavior.

2. **Reveal choice only offered for instant/sorcery**: Code at line 104: `if top_is_instant_or_sorcery { ... }` — the YesNo prompt is only presented when the top card is an instant or sorcery. Per the ruling, a player may reveal any card (even a non-instant/sorcery), but revealing such a card has no mechanical effect. This shortcut produces correct game outcomes in all cases.

### Card data verification (both faces)
- Mana cost {U}: PASS — `ManaSymbol::Colored(Color::Blue)` (line 37)
- Front card type Creature: PASS (line 39)
- Front supertypes none: PASS (line 40)
- Front subtypes Human Wizard: PASS — `["Human", "Wizard"]` (line 41)
- Front P/T 1/1: PASS (lines 42-43)
- Front keywords none: PASS (line 45)
- Back face name Insectile Aberration: PASS (line 60)
- Back face cost None: PASS (line 61)
- Back face type Creature: PASS (line 62)
- Back face subtypes Human Insect: PASS — `["Human", "Insect"]` (line 64)
- Back face P/T 3/2: PASS (lines 65-66)
- Back face keywords Flying: PASS — `Keyword::Flying` (line 68)
- Back face triggered_abilities empty: PASS (line 72)
- Front triggered_abilities: PASS — `TriggerKind::Upkeep` matches `on_upkeep` hook (line 51)

### Behavior verification
- Upkeep trigger only on controller's upkeep: PASS — code checks `state.active_player != controller` (line 90)
- Only triggers on front face: PASS — code checks `is_transformed` and returns early if true (line 90)
- Only triggers on battlefield: PASS — zone check `o.zone == Zone::Battlefield` (line 86)
- Looks at top card of library: PASS — reads `library_order.first()` (lines 14, 97-100)
- "You may" is optional via YesNo choice: PASS — `ResolutionChoiceKind::YesNo` presented to controller (lines 106-116)
- Player can decline reveal: PASS — `on_yes_no_choice` with `yes=false` returns without transforming (lines 122-125)
- Transform sets `is_transformed = true` and updates name: PASS (lines 138-140)
- Dynamic P/T returns (3,2) when transformed, None otherwise: PASS (lines 76-81)
- Card stays on top of library (never moved): PASS
- Empty library handled gracefully: PASS — `library_order.first()` returns None, function returns false

### Tricky interactions checked
- Only triggers on controller's upkeep (not each upkeep): PASS
- Only triggers on front face (Insectile Aberration has no upkeep ability): PASS
- Empty library gracefully handled: PASS
- Card stays on top of library after reveal or decline: PASS
- Back face Flying keyword resolved via `back_face_data()`: PASS
- Dynamic P/T 3/2 on back face, None on front face (base 1/1 used): PASS
- "You may" choice correctly presented (not auto-applied): PASS
- Declining reveal leaves card untransformed: PASS
- Non-instant/sorcery top card skips choice (no mechanical difference): PASS

### Test coverage
- Transform when instant on top and player reveals: `tier15_cards.rs:693` — TESTED
- Player declining to reveal: `tier15_cards.rs:732` — TESTED
- Top card is creature (no transform, no choice): `tier15_cards.rs:765` — TESTED
- Card stays on top after reveal: `tier15_cards.rs:728` — TESTED
- Card stays on top after decline: `tier15_cards.rs:761` — TESTED
- Dynamic P/T 3/2 after transform: `tier15_cards.rs:725` — TESTED
- Transform when top card is sorcery: NOT TESTED
- Empty library (no crash): NOT TESTED
- Back face does not trigger on upkeep: NOT TESTED (implicit)

### UI presentation
- LLM knowledge entry present in `mtg-player/src/llm.rs:61`: PASS
- LLM description accurate (mentions upkeep, instant/sorcery, reveal, transform, 3/2 flying): PASS
- YesNo choice description clear and informative: PASS

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform this creature.
**Type line**: Creature — Human Wizard
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 20:50

**Oracle text source**: Scryfall API (cached 2026-04-01), https://scryfall.com/card/isd/51/delver-of-secrets-insectile-aberration
**Oracle text (front)**: At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform this creature.
**Oracle text (back)**: Flying
**Type line (front)**: Creature — Human Wizard
**Type line (back)**: Creature — Human Insect
**Mana cost**: {U}
**Front P/T**: 1/1
**Back P/T**: 3/2
**Ruling [2011-09-22]**: You may reveal the card even if it's not an instant or sorcery. Whether or not you reveal it, the card stays on top of your library.
**Status**: ISSUE

### Code issues

1. **Manual transform does not update subtypes or keywords on the object** (`delver_of_secrets.rs:138-141`)
   - `on_yes_no_choice` manually sets `obj.is_transformed = true` and `obj.name = "Insectile Aberration"` but does NOT update `obj.keywords` (to add `Keyword::Flying`) or `obj.subtypes` (to change from `["Human", "Wizard"]` to `["Human", "Insect"]`).
   - The `apply_transform` helper in `helpers.rs:231-265` exists specifically for this purpose and correctly updates `is_transformed`, `name`, `keywords`, and `subtypes` from the `back_face_data()`.
   - **Flying impact**: mitigated. `state.has_keyword()` (`state.rs:1004-1014`) falls back to checking `back_face_data().keywords` when `is_transformed` is true, so Flying is still correctly detected for combat and other engine checks.
   - **Subtypes impact**: real bug. The engine checks subtypes directly via `obj.subtypes.contains(...)` (e.g., `engine.rs:1267`). After transform, the object still has subtypes `["Human", "Wizard"]` instead of `["Human", "Insect"]`. This means:
     - The transformed Insectile Aberration is incorrectly considered a **Wizard** (it should not be).
     - The transformed Insectile Aberration is NOT considered an **Insect** (it should be).
     - Cards like Bonds of Faith, Silver-Inlaid Dagger, Champion of the Parish, and any subtype-matters effects would see incorrect subtypes.
   - **Fix**: Replace the manual transform in `on_yes_no_choice` with `helpers::apply_transform(state, self_id, _registry)`.

2. **No zone check in `on_yes_no_choice`** (`delver_of_secrets.rs:138`)
   - The handler calls `state.get_object_mut(self_id)` and transforms without verifying `obj.zone == Zone::Battlefield`. If Delver is removed from the battlefield between the upkeep trigger and the YesNo choice resolution, the transform would be applied to a non-battlefield object.
   - Minor issue; unlikely in practice but incorrect per rules (a creature that leaves the battlefield before its triggered ability resolves should not be transformed).

**Minor notes (not issues):**
- **Oracle text field wording**: Code `oracle_text` says `"transform Delver of Secrets"` while current Scryfall oracle says `"transform this creature."` Display-only, no behavioral impact.
- **Reveal choice only offered for instant/sorcery**: Per the ruling, a player may reveal any card, but revealing a non-instant/sorcery has no mechanical effect. The shortcut produces correct game outcomes.

### Tricky interactions checked (min 3)
1. **Only triggers on controller's upkeep**: PASS — checks `state.active_player != controller` (line 90)
2. **Only triggers on front face**: PASS — checks `is_transformed` and returns early (line 90)
3. **Empty library**: PASS — `library_order.first()` returns None, gracefully returns false
4. **Card stays on top of library after reveal**: PASS — code never moves the top card
5. **Flying on back face**: PASS — `has_keyword` compensates via `back_face_data()` lookup
6. **Multiple Delvers**: Both would trigger, both look at the same top card, both could independently reveal. The implementation handles this correctly since each trigger is independent and the card is never removed from the library.
7. **Subtypes after transform**: FAIL — `obj.subtypes` not updated (see issue #1 above)

### Test coverage
- Transform when instant on top and player reveals: `tier15_cards.rs:delver_transforms_when_player_reveals_instant` — TESTED
- Player declining to reveal: `tier15_cards.rs:delver_does_not_transform_when_player_declines_reveal` — TESTED
- Top card is creature (no transform, no choice): `tier15_cards.rs:delver_does_not_transform_when_top_card_is_creature` — TESTED
- Card stays on top after reveal: TESTED (asserted in transform test, line 973)
- Card stays on top after decline: TESTED (asserted in decline test, line 1006)
- Dynamic P/T 3/2 after transform: TESTED (line 970)
- **Subtypes after transform**: NOT TESTED (would expose issue #1)
- **Keywords (Flying) after transform**: NOT TESTED (would pass due to `has_keyword` fallback)
- Transform when top card is sorcery: NOT TESTED
- Empty library: NOT TESTED
- Zone check on resolution: NOT TESTED (would expose issue #2)

## Audit — 2026-04-03 22:21

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform this creature.
**Type line**: Creature — Human Wizard (front) / Creature — Human Insect (back)
**Status**: ISSUE

### Code issues

1. **Manual transform does not update subtypes correctly** (`delver_of_secrets.rs:138-141`)
   - Oracle text says: Back face should have subtypes `"Human Insect"` 
   - Code does: Manual transform `obj.is_transformed = true; obj.name = "Insectile Aberration"` but does not update `obj.subtypes` from `["Human", "Wizard"]` to `["Human", "Insect"]`
   - After transformation, cards that check creature subtypes (e.g., tribal effects) will see the wrong subtypes - Insectile Aberration will still be considered a Wizard instead of an Insect

2. **No zone check in transform resolution** (`delver_of_secrets.rs:138`)  
   - Oracle text says: Transform should only apply to creatures on the battlefield
   - Code does: `state.get_object_mut(self_id)` without checking `obj.zone == Zone::Battlefield` 
   - If Delver leaves the battlefield between trigger and resolution, it could still be transformed incorrectly

### Tricky interactions checked
- Controller's upkeep only: PASS — checks `state.active_player != controller`
- Front face only triggers: PASS — checks `is_transformed` guard 
- Empty library handling: PASS — returns false gracefully when library is empty
- Optional reveal mechanic: PASS — presents choice only for instant/sorcery, handles decline correctly
- "Revealed this way" condition: PASS — transform only on player's choice to reveal
- Card stays on top of library: PASS — never moves the top card
- Multiple Delvers scenario: PASS — each trigger resolves independently
- Flying keyword on back face: PASS — `has_keyword` correctly checks `back_face_data()`
- Dynamic P/T updates: PASS — correctly returns (3,2) when transformed

### Test coverage
- Transform when revealing instant: `tier15_cards.rs:938` — TESTED
- No transform when declining reveal: `tier15_cards.rs:977` — TESTED  
- No transform for non-instant/sorcery: `tier15_cards.rs:1010` — TESTED
- Subtypes after transform (Human Insect): NOT TESTED — would expose issue #1
- Zone check on transform resolution: NOT TESTED — would expose issue #2
- Transform when revealing sorcery: NOT TESTED
- Empty library edge case: NOT TESTED
- Multiple Delvers on same card: NOT TESTED

## Audit — 2026-04-03 22:21 (independent)

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform this creature.
**Type line**: Creature — Human Wizard // Creature — Human Insect (back face: Insectile Aberration, 3/2 Flying)
**Status**: ISSUE

### Code issues

1. **Manual transform does not update subtypes on the object** (`mtg-engine/src/cards/isd/delver_of_secrets.rs:138-141`)
   - Oracle text says: Back face type line is `Creature — Human Insect`
   - Code does: `obj.is_transformed = true; obj.name = "Insectile Aberration".into();` — only sets `is_transformed` and `name`, but does NOT update `obj.subtypes` (remains `["Human", "Wizard"]`) or `obj.keywords` (remains `[]`).
   - Every other DFC in the codebase (Cloistered Youth at `cloistered_youth.rs:99`, Screeching Bat at `screeching_bat.rs:133`, Instigator Gang at `instigator_gang.rs:118`, Ludevic's Test Subject at `ludevics_test_subject.rs:102`) uses `helpers::apply_transform()` (`helpers.rs:231-265`), which correctly updates subtypes, keywords, and name, and includes a zone check.
   - **Subtypes impact (real bug)**: `TargetFilter::HasSubtype` in `engine.rs:1267` checks `obj.subtypes.contains(subtype)` directly without any back-face fallback. After transform, checking "Insect" returns false (wrong) and checking "Wizard" returns true (wrong). `CreatureFilter::HasSubtype` in `state.rs:654-672` partially compensates via `back_face_data()` lookup when `is_transformed`, but the fallback on line 672 (`creature.subtypes.iter().any(...)`) still incorrectly matches "Wizard" on the stale `obj.subtypes`.
   - **Keywords impact (mitigated)**: `state.has_keyword()` at `state.rs:1006-1009` compensates by checking `back_face_data().keywords` when `is_transformed`, so Flying is correctly detected despite `obj.keywords` being empty.

2. **No zone check in `on_yes_no_choice`** (`mtg-engine/src/cards/isd/delver_of_secrets.rs:138`)
   - Oracle text says: `transform this creature` (creature must be on the battlefield for a triggered ability to resolve meaningfully)
   - Code does: `if let Some(obj) = state.get_object_mut(self_id) { obj.is_transformed = true; ... }` — no zone check. If Delver is removed from the battlefield between the upkeep trigger and the YesNo choice resolution (e.g., killed in response), the transform is still applied to the object in whatever zone it moved to. The `apply_transform` helper at `helpers.rs:233` includes a zone check (`o.zone == Zone::Battlefield`) that would prevent this.

3. **"You may reveal" choice only offered for instant/sorcery cards** (`mtg-engine/src/cards/isd/delver_of_secrets.rs:104`)
   - Oracle text says: `You may reveal that card. If an instant or sorcery card is revealed this way, transform this creature.`
   - Ruling [2011-09-22] says: `You may reveal the card even if it's not an instant or sorcery.`
   - Code does: `if top_is_instant_or_sorcery { ... present YesNo choice ... }` — the choice is only presented when the top card IS an instant or sorcery. When the top card is another type, no choice is offered and the ability silently completes. Per the oracle text and the ruling, the player should always be offered the option to reveal the top card regardless of type. Revealing a non-instant/sorcery gives opponents information but does not cause a transform.

### Tricky interactions checked
- Only triggers on controller's upkeep: PASS — `state.active_player != controller` guard at line 90
- Only triggers on front face (Insectile Aberration has no upkeep trigger): PASS — `is_transformed` guard at line 90
- Empty library: PASS — `library_order.first()` returns None, gracefully handled
- Card stays on top of library after reveal or decline: PASS — code never moves the top card
- Flying on back face detected by engine: PASS — `has_keyword` falls back to `back_face_data().keywords` when `is_transformed`
- Dynamic P/T (3/2 when transformed, base 1/1 when not): PASS — `dynamic_pt` returns `Some((3, 2))` only when transformed
- Subtypes after transform: FAIL — `obj.subtypes` not updated; Insectile Aberration incorrectly has Wizard and lacks Insect on the object level (see issue #1)
- Zone check on YesNo resolution: FAIL — no zone check before transforming (see issue #2)
- Phantom upkeep trigger on back face: benign — `trigger_description` in `triggers.rs:311-327` always checks front face first regardless of `is_transformed`, so Insectile Aberration gets an UpkeepTrigger dispatched, but `on_upkeep` returns early due to `is_transformed` guard. Not an observable issue.
- Mana cost {U}: PASS — `ManaSymbol::Colored(Color::Blue)` at line 37
- Card data fields: PASS — all types, supertypes, P/T match oracle

### Test coverage
- Transform when instant on top and player reveals: `tier15_cards.rs:938` (delver_transforms_when_player_reveals_instant) — TESTED
- Player declining to reveal: `tier15_cards.rs:977` (delver_does_not_transform_when_player_declines_reveal) — TESTED
- Top card is creature (no transform, no choice): `tier15_cards.rs:1010` (delver_does_not_transform_when_top_card_is_creature) — TESTED
- Card stays on top after reveal: `tier15_cards.rs:973` — TESTED
- Card stays on top after decline: `tier15_cards.rs:1006` — TESTED
- Dynamic P/T 3/2 after transform: `tier15_cards.rs:970` — TESTED
- Subtypes after transform (should be Human Insect, not Human Wizard): NOT TESTED (would expose issue #1)
- Keywords (Flying) after transform on obj level: NOT TESTED (would pass due to has_keyword fallback)
- Zone check on YesNo resolution: NOT TESTED (would expose issue #2)
- Reveal choice for non-instant/sorcery top card per ruling: NOT TESTED (would expose issue #3)
- Transform when top card is sorcery: NOT TESTED
- Empty library: NOT TESTED

## Audit — 2026-04-10 18:30

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform this creature.
**Type line**: Creature — Human Wizard
**Back face — Name**: Insectile Aberration
**Back face — Type line**: Creature — Human Insect
**Back face — Oracle text**: Flying
**Back face — P/T**: 3/2
**Ruling**: [2011-09-22] You may reveal the card even if it's not an instant or sorcery. Whether or not you reveal it, the card stays on top of your library.
**Status**: ISSUE

### Code issues

- `mtg-engine/src/cards/isd/delver_of_secrets.rs:146-149` — Manual transform bypasses `helpers::apply_transform`, leaving `obj.subtypes` and `obj.keywords` out of sync with the back face. After transform, `obj.subtypes` still reads `["Human", "Wizard"]` instead of `["Human", "Insect"]`, and `obj.keywords` stays empty instead of `[Flying]`.
  - Oracle says: back face type line is `Creature — Human Insect`, with keyword `Flying`.
  - Code does:
    ```rust
    if let Some(obj) = state.get_object_mut(self_id) {
        obj.is_transformed = true;
        obj.name = "Insectile Aberration".into();
    }
    ```
    versus the canonical helper (`mtg-engine/src/cards/helpers.rs:256-263`) which also copies `back.keywords` and `back.subtypes` onto the object.
  - Impact: `state.matches_filter` at `mtg-engine/src/state.rs:683` falls through to `creature.subtypes.iter().any(|s| s == subtype)` after the transformed-branch lookup, so a transformed Delver incorrectly matches the `Wizard` subtype (via stale `obj.subtypes`) even though Insectile Aberration is not a Wizard. Similarly `combat.rs:405` collects `obj.subtypes` directly, and the engine has numerous other sites (e.g. `engine.rs:1593`, `engine.rs:1728`, `state.rs:1200`, `state.rs:1247`, ISD cards such as `bitterheart_witch`, `dearly_departed`, `elder_cathar`, `hamlet_captain`, `full_moons_rise`, `bloodline_keeper`, `woodland_cemetery`, etc.) that query `obj.subtypes` / `obj.keywords` directly rather than going through `has_keyword`/`matches_filter`, so the stale values can leak. The fix is to call `helpers::apply_transform(state, self_id, registry)` (like `cloistered_youth.rs:99` does) instead of flipping the flag by hand.

- `mtg-engine/src/cards/isd/delver_of_secrets.rs:106-116` — The YesNo description leaks information about the top card identity and whether it will cause a transform before the player commits to revealing. Oracle text models "look at" (private to controller), not a public announcement. Because `state.log` and the `AwaitingAction` description are part of the shared game state, an opponent observer can see the top card's name and whether it's an instant/sorcery. Minor correctness/information-hygiene concern; the controller already legally knows the info, but it should not be propagated to the description/log visible to opponents.
  - Oracle says: "look at the top card of your library"
  - Code does:
    ```rust
    let description = if top_is_instant_or_sorcery {
        format!("Delver of Secrets: reveal {} from the top of your library to transform?", top_card_name)
    } else {
        format!("Delver of Secrets: reveal {} from the top of your library? (not an instant or sorcery — no transform)", top_card_name)
    };
    ```
  - Also logs the top card name unconditionally at `LogLevel::Debug` on line 101-102.

### Tricky interactions checked

- Triggered ability source leaves battlefield between trigger put-on-stack and resolution: PASS — `triggers.rs:983` re-checks `zone == Battlefield` before calling `on_upkeep`.
- Upkeep trigger only on controller's upkeep (not each upkeep): PASS — `on_upkeep` bails if `state.active_player != controller` (line 90).
- Front-face-only trigger (Insectile Aberration back face has no upkeep trigger): PASS — `on_upkeep` bails on `is_transformed` (line 90); back face `triggered_abilities` is empty.
- "You may reveal" is presented even when top card is non-instant/sorcery (per 2011-09-22 ruling): PASS — `on_upkeep` always presents the choice regardless of the top card's type (lines 117-124).
- Top card stays on library after reveal/non-reveal (per ruling): PASS — code never moves the card; only looks at `library_order.first()`.
- Declining to reveal does not transform: PASS — `on_yes_no_choice` returns early on `!yes` (line 128).
- Revealing a non-instant/sorcery does not transform: PASS — `top_is_instant_or_sorcery` branches in `on_yes_no_choice` (line 143-150).
- Dynamic P/T returns (3,2) only after transform: PASS — `dynamic_pt` keys off `is_transformed` (lines 76-82).
- Flying on back face: effectively PASS via `has_keyword`'s back_face_data lookup (`state.rs:1018-1027`), even though `obj.keywords` is not synced (see issue above). This is fragile and the issue above is the root cause.
- Empty library when trigger resolves: PASS — `top_card_is_instant_or_sorcery` returns false and `on_upkeep` still presents the "reveal?" choice, which is harmless.

### Test coverage

- Upkeep trigger transforms when revealed top card is an instant: `mtg-engine/tests/tier15_cards.rs:950` (`delver_transforms_when_player_reveals_instant`).
- Upkeep trigger does not transform when player declines to reveal: `mtg-engine/tests/tier15_cards.rs:989` (`delver_does_not_transform_when_player_declines_reveal`).
- Upkeep trigger does not transform when revealed top card is a creature (per 2011-09-22 ruling, choice is still presented): `mtg-engine/tests/tier15_cards.rs:1022` (`delver_does_not_transform_when_top_card_is_creature`).
- Reveal leaves the top card on the library: asserted inside `delver_transforms_when_player_reveals_instant` (line 985) and `delver_does_not_transform_when_player_declines_reveal` (line 1018).
- Bug-regression test that choice is presented for non-instant/sorcery top cards: `mtg-engine/tests/audit_bugs.rs:637` (`bug_delver_reveal_suppressed_for_non_instant_sorcery`).
- Transformed subtype correctness (e.g., Delver no longer matches "Wizard" but matches "Insect" + "Human"): NOT TESTED.
- Transformed keyword correctness (`has_keyword` returns Flying; `obj.keywords` sync): NOT TESTED.
- Combat: transformed Delver attacks as 3/2 flier: NOT TESTED (only `dynamic_pt` numeric assertion).
- Transform happens outside of the Werewolf rules (no day/night dependency): NOT TESTED.
- Interaction with Humans-matters cards (Champion of the Parish, Hamlet Captain, etc.) after transform (should no longer count as Human Wizard — still a Human per back face, but no longer a Wizard): NOT TESTED.
- Back face does not retrigger upkeep ability: NOT TESTED.
