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
