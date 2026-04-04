## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Vigilance\nWhenever another creature you control dies, you may transform this creature.
**Type line**: Creature — Human Soldier (front); Creature — Human Soldier (back: Thraben Militia, 5/4, Trample)
**Status**: ISSUE

### Code issues

- **"you may" is bypassed — card always auto-transforms, player never gets a choice** (`mtg-engine/src/cards/isd/thraben_sentry.rs`, lines 72–76)
  - Oracle text says: `"you may transform this creature"`
  - Code does: `// Auto-transform (simplified "you may" — always yes). if let Some(obj) = state.get_object_mut(self_id) { obj.is_transformed = true; obj.name = "Thraben Militia".into(); }`
  - The engine has a `YesNo` resolution-choice mechanism (`AwaitingAction::ResolutionChoice { choice: ResolutionChoiceKind::YesNo { ... } }`) used by other "you may" DFC cards (e.g., Screeching Bat, Cloistered Youth, Delver of Secrets). Thraben Sentry ignores it entirely and forces the transform. The comment even admits this is wrong: "simplified 'you may' — always yes."

- **Vigilance incorrectly retained on back face after transform** (`mtg-engine/src/cards/isd/thraben_sentry.rs`, lines 73–76)
  - Oracle text says (back face): `"Trample"` (Thraben Militia has Trample; Vigilance is only on the front face)
  - Code does: sets `obj.is_transformed = true` and `obj.name = "Thraben Militia".into()` but does **not** update `obj.keywords`. The object's `keywords` field remains `[Vigilance]` (the front face value).
  - `has_keyword()` in `state.rs` checks `obj.keywords` **first** (step 0, line 1000): `if obj.keywords.contains(&keyword) { return true; }`. Because `obj.keywords` still holds `[Vigilance]`, `has_keyword(Vigilance)` returns `true` for Thraben Militia — incorrectly granting Vigilance to the back face. The helper `apply_transform` in `helpers.rs` (lines 231–265) correctly sets `obj.keywords = back.keywords.clone()` (i.e., `[Trample]`), but the manual transform code in `thraben_sentry.rs` does not call it.

- **Test enshrines wrong auto-transform behavior** (`mtg-engine/tests/tier15_cards.rs`, lines 1392–1409, test `thraben_sentry_transforms_when_creature_dies`)
  - The test calls `on_any_creature_dies` directly and asserts `is_transformed == true` without checking for `state.awaiting_action`. If the bug were fixed to use `YesNo`, the transform would not happen immediately and this assertion would fail. The test actively enshrines the "always yes" violation.

### Tricky interactions checked

- **"you may" optionality**: FAIL — code auto-transforms every time; `AwaitingAction::ResolutionChoice { YesNo }` is never set.
- **"another" (not self) exclusion**: PASS — trigger collection filters `o.id != dead_id` (triggers.rs line 419), so Thraben Sentry never watches its own death.
- **"you control" controller check**: PASS — `on_any_creature_dies` checks `dead_controller != controller` and returns early if the dead creature was not controlled by the same player (line 69).
- **Front-face-only trigger (already transformed check)**: PASS — `is_transformed` guard at line 69 prevents a second transform if already Thraben Militia.
- **Simultaneous deaths — multiple triggers fire**: PASS — each `CreatureDied` event generates a separate `DeathWatch` trigger; the APNAP bucket loop in `collect_triggers` adds one per dying creature. The `is_transformed` guard at resolution ensures only the first trigger to resolve actually transforms.
- **Keywords after transform — Trample present**: PASS — `has_keyword(Trample)` falls through `obj.keywords` (no Trample there) to the registry back-face check, which correctly finds Trample in `back_face_data().keywords`.
- **Keywords after transform — Vigilance incorrectly retained**: FAIL — `obj.keywords` still contains `Vigilance` after the manual transform; `has_keyword(Vigilance)` returns true immediately at step 0, giving Thraben Militia Vigilance it should not have.
- **dynamic_pt correctness (5/4 on back face)**: PASS — `dynamic_pt` returns `Some((5, 4))` when `is_transformed` is true; `effective_power`/`effective_toughness` use this override correctly.
- **Mana cost {3}{W}**: PASS — `ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::White)])`.
- **Card types, supertypes, subtypes**: PASS — `Creature`, no supertypes, `["Human", "Soldier"]` on both faces.
- **Vigilance keyword on front face**: PASS — `keywords: vec![Keyword::Vigilance]`.
- **Trample keyword on back face**: PASS — `back_face_data().keywords: vec![Keyword::Trample]`.
- **Watcher still on battlefield at resolution**: PASS — `resolve_next_trigger` checks `o.zone == Zone::Battlefield` before calling `on_any_creature_dies` (triggers.rs line 908).

### Test coverage

- "you may" optionality (player can decline transform): NOT TESTED — no test verifies that the player is asked, or that declining leaves the sentry un-transformed.
- "another" creature exclusion (not own death): NOT TESTED directly through the full trigger system; the unit test calls `on_any_creature_dies` directly.
- Opponent's creature dying does not trigger: `tier15_cards.rs:1411` (`thraben_sentry_does_not_transform_when_opponent_creature_dies`) — TESTED.
- Transformation occurs when own creature dies: `tier15_cards.rs:1392` (`thraben_sentry_transforms_when_creature_dies`) — TESTED but enshrines wrong behavior (auto-transform instead of YesNo).
- Vigilance not present on back face (Thraben Militia): NOT TESTED.
- Trample present on back face (Thraben Militia): NOT TESTED.
- Multiple simultaneous deaths trigger multiple times: NOT TESTED.
- Only first simultaneous-death trigger transforms (subsequent are no-ops): NOT TESTED.
- dynamic_pt returns 5/4 when transformed: `tier15_cards.rs:1408` — TESTED.
- Moonmist interaction (transforms front-face Human): `moonmist.rs:91` — TESTED.
- Moonmist transforms back-face Human (Thraben Militia) back to front: `moonmist.rs:107` — TESTED.
