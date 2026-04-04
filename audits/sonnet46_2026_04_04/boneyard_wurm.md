## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Boneyard Wurm's power and toughness are each equal to the number of creature cards in your graveyard.
**Type line**: Creature — Wurm
**Status**: ISSUE

### Code issues

- Graveyard zone display shows base P/T (0/0) instead of dynamically computed P/T
  - Oracle text says: `Boneyard Wurm's power and toughness are each equal to the number of creature cards in your graveyard.`
  - Ruling says: `The ability that defines Boneyard Wurm's power and toughness works in all zones, not just the battlefield. If Boneyard Wurm is in your graveyard, it will count itself.`
  - Code does: `card_view` in `mtg-engine/src/view.rs:221` uses `power: obj.power` for graveyard (and hand/exile) objects, which returns the raw base `Some(0)` instead of calling `state.effective_power(obj.id, registry)`. The `CardView` struct (`view.rs:42`) has only `power: Option<i32>` (no `effective_power` field), while `PermanentView` (`view.rs:58–61`) has both `power` and `effective_power`. As a result, Boneyard Wurm is reported as 0/0 whenever it is not on the battlefield, even though `effective_power` (which calls `dynamic_pt`) would compute the correct value if invoked.
  - The underlying `dynamic_pt` function (`boneyard_wurm.rs:32–39`) is itself correct: it does not zone-restrict, and `objects_in_zone(Zone::Graveyard, controller)` would include the Wurm itself when it is in the graveyard (its `power: Some(0)` satisfies the `o.power.is_some()` filter). However, `dynamic_pt` is only called by `effective_power`/`effective_toughness`, which are in turn called only for battlefield objects in the view layer and in combat/SBA code. No path in the engine calls `effective_power` for a graveyard or hand object, so the CDA's "works in all zones" property is unreachable.
  - Affected file: `mtg-engine/src/view.rs`, function `card_view` (line ~213), specifically `power: obj.power` at line 221.

### Tricky interactions checked

- **CDA applies on battlefield**: PASS — `effective_power`/`effective_toughness` call `dynamic_pt` for battlefield objects; result is correctly returned for combat, SBA, and `PermanentView` display.
- **CDA applies in graveyard (ruling)**: FAIL — `card_view` bypasses `effective_power` and uses raw `obj.power` for graveyard objects. Boneyard Wurm in the graveyard is shown as 0/0 rather than the creature-card count. The `dynamic_pt` implementation itself would return the correct value if called, but no code path calls it for graveyard objects.
- **Counts itself when in graveyard**: FAIL (same root cause as above) — `dynamic_pt` correctly includes the Wurm in `objects_in_zone(Zone::Graveyard, controller)` because `obj.power.is_some()` is true for the Wurm (`power: Some(0)` set in `card_data()`), but since `dynamic_pt` is never called for graveyard objects, the self-count is never exercised.
- **Token filtering**: PASS — Tokens have `is_token: true` and are removed from `state.objects` by SBA rule 704.5d (`sba.rs:307–315`) before any player priority window. `dynamic_pt` is only invoked at view/combat time (after SBAs run), so tokens are never present in the graveyard when the count is computed.
- **"Your graveyard" = controller's graveyard**: PASS — `objects_in_zone(Zone::Graveyard, controller)` in `state.rs:600–608` filters by `obj.owner == player` for the Graveyard zone. Cards go to their owner's graveyard, so `owner == controller` correctly identifies the controller's graveyard contents.
- **`o.power.is_some()` as creature-card proxy**: PASS — In this engine and card set (Innistrad), all creature cards have `CardData.power = Some(value)` and all non-creature cards (lands, instants, sorceries, non-creature artifacts) have `power: None`. The Splinterfright comment (`splinterfright.rs:23–24`) explicitly documents this convention: `"power.is_some() is used as proxy"`.
- **Non-creature cards not counted**: PASS — Land/instant/sorcery/enchantment/non-creature artifact objects in the graveyard have `power: None` and are excluded by the `o.power.is_some()` filter.
- **Continuous re-evaluation (not snapshot)**: PASS — `dynamic_pt` is called fresh on every invocation of `effective_power`/`effective_toughness`; it reads live state and is not cached. P/T changes instantly as creatures enter/leave the graveyard.
- **Base P/T in card_data set to Some(0)**: PASS — Required so the engine identifies the Wurm as a creature (`power.is_some()` proxy) for combat, SBA 0-toughness checks, and ETB/death event detection. The comment in Splinterfright confirms this is intentional.

### Test coverage

- **Basic P/T = number of creatures in graveyard (battlefield)**: `tier7_cards.rs:19` — `boneyard_wurm_pt_equals_creatures_in_graveyard` — TESTED (verifies 0/0 with empty graveyard, 3/3 with 3 creature cards moved to graveyard)
- **CDA works in graveyard zone (ruling: "works in all zones")**: NOT TESTED
- **Counts itself when in graveyard (ruling)**: NOT TESTED
- **Non-creature cards in graveyard do not affect count**: NOT TESTED
- **Tokens do not affect count**: NOT TESTED
- **P/T updates dynamically as creatures die/are exiled**: NOT TESTED (only a static 3-creature snapshot is tested)
