## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: You may have this creature enter as a copy of any creature on the battlefield, except it has "{U}{B}, {T}: Destroy target creature with the same name as this creature."
**Type line**: Creature — Shapeshifter
**Status**: ISSUE

### Code issues

- **Activated ability inaccessible after copying (engine.rs + evil_twin.rs)**
  - Oracle text says: `"except it has '{U}{B}, {T}: Destroy target creature with the same name as this creature.'"`
  - Code does: The `CopyCreature` handler at `engine.rs:2458` sets `obj.card_id = card_id` (the target creature's card_id). After this, action generation at `engine.rs:326` calls `registry.get(obj_card_id)` which now returns the *copied creature's* behavior (e.g., Grizzly Bears), not `EvilTwin`'s. The copied creature's `activated_abilities()` returns nothing. `EvilTwin::activated_abilities()` (`evil_twin.rs:68–90`), the only place the destroy ability is defined, is never invoked for the object after copying. The ability is therefore never presented to the player and can never be activated. The `is_evil_twin` card_state marker (`evil_twin.rs:54`) is intended to signal this, but it is checked only inside `EvilTwin::activated_abilities()`, which is never reached after the card_id changes. The dispatch path (`engine.rs:326`: `registry.get(obj_card_id)`) has no fallback to the original card's behavior.

- **ETB abilities of the copied creature never trigger (triggers.rs)**
  - Oracle text says (from rulings): `"Any enters-the-battlefield abilities of the copied creature will trigger when Evil Twin enters the battlefield."`
  - Code does: `collect_triggers` (`triggers.rs:344–363`) reads the current `card_id` off the object at trigger-collection time, which is Evil Twin's own card_id (the copy hasn't happened yet). The resulting `PendingTrigger::EnteredBattlefield` carries Evil Twin's card_id. When that trigger resolves (`triggers.rs:893–899`) it calls `EvilTwin::on_enter_battlefield()`, presents the copy choice, and the player eventually resolves it. No new `EnteredBattlefield` event is ever emitted after the copy is applied. Consequently, if Evil Twin copies a creature that has ETB triggered abilities (e.g., a creature with an ETB draw or ETB destroy), those triggers never fire for Evil Twin.

- **`is_evil_twin` marker set before the optional copy choice is made (evil_twin.rs:53–55)**
  - Oracle text says: `"You may have this creature enter as a copy of any creature on the battlefield, except it has…"` — the destroy ability is part of the *copy* clause; it should only exist if a copy is made.
  - Code does: `evil_twin.rs:49–65` sets `is_evil_twin` in `card_state` before calling `present_optional_target_choice`. If the player declines the optional choice, `obj.card_id` remains as Evil Twin's card_id and `is_evil_twin` is still set, so `EvilTwin::activated_abilities()` returns the destroy ability for a 0/0 that never copied anything. Practically harmless (SBAs kill it immediately) but technically wrong.

### Tricky interactions checked

- **Destroy ability accessible after copying**: FAIL — `obj.card_id` changes to target's card_id after `CopyCreature`; engine dispatches activated abilities via `registry.get(obj.card_id)` (engine.rs:326), which returns the target's behavior, not `EvilTwin`'s; the destroy ability is never returned or presented.
- **"You may" optionality correctly implemented**: PASS — `present_optional_target_choice` is called (evil_twin.rs:57–64), which invokes `present_target_choice` with `optional: true` (helpers.rs:156), adding `ResolvedChoice::ChosenTarget(None)` as a legal action (engine.rs:199–200). Player can decline.
- **Player declining when no creatures on battlefield**: PASS — `if !targets.is_empty()` guard (evil_twin.rs:49) prevents the choice from being presented when no targets exist; Evil Twin enters as 0/0 and dies.
- **`SameNameAsSource` filter logic**: PASS — `matches_ability_target_filter` (engine.rs:1269–1274) correctly compares `source.name == obj.name`. After copying, Evil Twin's name matches the copy target, so the ability would correctly target a creature of the same name — *if it could be activated, which it can't due to Issue 1*.
- **Self-targeting with the destroy ability**: PASS — oracle text says "target creature with the same name," no "another" restriction. `is_valid_target` (evil_twin.rs:93–105) only checks zone and power, not self-exclusion. The filter allows self-targeting, which is rules-correct.
- **"Destroy" vs "sacrifice" distinction**: PASS — `on_activate_ability` (evil_twin.rs:110) calls `crate::destruction::try_destroy`, which correctly checks indestructible and regeneration before destroying.
- **Mana cost and type line**: PASS — {2}{U}{B}, Creature — Shapeshifter, 0/0 are all correct in `card_data()`.
- **Copying a creature that is itself a copy (e.g., another Evil Twin)**: The `CopyCreature` handler propagates `is_evil_twin` from the target if it has the flag (engine.rs:2466–2467), and reads `card_id` from the target object's runtime state (engine.rs:2444–2449), which after a prior copy would be the original card's id. This is mechanically consistent, though the activated ability is still inaccessible for the same reason as Issue 1.
- **ETB abilities of the copied creature triggering**: FAIL — see Code Issues above; no mechanism fires the copied creature's ETBs for Evil Twin.
- **Copying a token**: The `CopyCreature` handler reads `name`, `subtypes`, `power`, `toughness` from the object (runtime values stored on the token), and `keywords` from `registry.card_data(o.card_id)`. If the token's `card_id` has no registry entry (some tokens may not), `keywords` defaults to `vec![]` (engine.rs:2444–2446), potentially missing keywords the token had on its object. This is a marginal concern depending on how tokens are created; not flagged as a primary issue.

### Test coverage

- **Destroy ability accessible after copying**: NOT TESTED — `evil_twin_copies_creature_on_etb` (tier15_cards.rs:1756) checks `is_evil_twin` marker and copied stats but never calls `legal_actions` to verify the ability appears in generated actions.
- **Player declining to copy**: NOT TESTED
- **Player declining when no creatures on battlefield**: NOT TESTED
- **ETB abilities of copied creature triggering**: NOT TESTED
- **Copying a creature that has ETB abilities**: NOT TESTED
- **Copying another Evil Twin**: NOT TESTED
- **Copying a token**: NOT TESTED
- **"You may" optionality**: NOT TESTED (no test checks the `None` path through `ChosenTarget`)
- **Destroy ability targeting correctness (SameNameAsSource filter)**: NOT TESTED
