## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Whenever another creature you control with power 2 or less enters, you may pay {1}. If you do, draw a card.
**Type line**: Creature — Human Soldier
**Status**: ISSUE

### Code issues

- **"You may pay {1}" is auto-paid instead of presenting a player choice** (`mtg-engine/src/cards/isd/mentor_of_the_meek.rs`, lines 55–80)
  - Oracle text says: `you may pay {1}. If you do, draw a card.`
  - Code does: `// "You may pay {1}" — auto-pay if the controller has any mana in pool (pays {1}). ... if pool.total() >= 1 { ... if paid { crate::engine::draw_cards(state, controller, 1); } }` — automatically deducts mana and draws without presenting a yes/no choice to the player. The engine has `ResolutionChoiceKind::YesNo` specifically for this purpose (used correctly by e.g. `murder_of_crows.rs`), but Mentor of the Meek ignores it. A player with mana who wants to keep it cannot decline.

- **Power checked at resolution time, not at ETB trigger time** (`mtg-engine/src/cards/isd/mentor_of_the_meek.rs`, line 51; `mtg-engine/src/triggers.rs`, lines 366–392)
  - Oracle text says (ruling 2025-01-24): `Mentor of the Meek's ability checks the power of the other creature only as it enters. If that creature's power is 2 or less, the ability will trigger. Once the ability triggers, raising that creature's power above 2 won't affect that ability. Similarly, reducing the creature's power to 2 or less after it enters won't cause the ability to trigger.`
  - Code does: In `collect_triggers`, the `EnterWatch` trigger is dispatched for **all entering creatures** regardless of power (the only filter is `o.power.is_some()` — is it a creature). The power ≤ 2 check is deferred to resolution time: `let power = state.effective_power(entered_id, registry).unwrap_or(99); if power > 2 { return; }`. This creates two wrong-behavior windows: (a) a creature enters at power ≤ 2, the trigger is collected, but if an opponent then raises its power above 2 before resolution, the code incorrectly skips the draw; (b) a creature enters at power > 3, the trigger is collected (should not have been), and if power drops to ≤ 2 before resolution, the code incorrectly draws. Both contradict the ruling.

- **Test enshrines wrong auto-pay behavior** (`mtg-engine/tests/tier15_cards.rs`, lines 71–99)
  - Oracle text says: `you may pay {1}. If you do, draw a card.`
  - Code does: The test pre-loads colorless mana and calls `on_any_creature_enters` directly, then asserts `hand_count == 1` without any `awaiting_action` / YesNo interaction. This confirms and validates the automatic-draw (no-choice) behavior rather than the optional "you may" behavior.

### Tricky interactions checked

- **"another" creature (self-exclusion)**: PASS — `on_any_creature_enters` checks `entered_id == self_id` and returns if true. The trigger collection also excludes the entering object from the watcher list (`o.id != *object`).
- **"under your control" restriction**: PASS — `on_any_creature_enters` checks `entered_controller != controller` and returns if wrong controller.
- **"you may" optionality (player can decline)**: FAIL — see Issue 1 above. The code auto-pays without offering a choice.
- **Power check timing (only at ETB, not at resolution)**: FAIL — see Issue 2 above. Power is re-evaluated at resolution time, not snapshotted at ETB time.
- **Power check includes continuous effects and counters (per ruling)**: PASS — `state.effective_power(entered_id, registry)` incorporates aura modifiers, counters, and until-end-of-turn effects, matching the ruling: "If a creature enters with +1/+1 counters or a continuous effect … those effects apply when checking."
- **"you may" cannot be paid multiple times to draw multiple cards (per ruling)**: PASS — the code only draws once per trigger resolution.
- **Mentor's own ETB does not trigger itself**: PASS — the watcher scan in `collect_triggers` filters out `o.id != *object`, so Mentor does not watch its own entry. The "another" check in `on_any_creature_enters` also guards against this.
- **Mentor on battlefield check before triggering**: PASS — `on_any_creature_enters` verifies `o.zone == Zone::Battlefield` before proceeding.
- **Mana deduction logic when auto-paying**: PASS (conditional) — when the code auto-pays, it correctly prefers colorless, then tries colors in order. Logic is sound; the bug is that it runs at all without player consent.

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:
- Small creature (power ≤ 2) entering under controller's control triggers draw: `tier15_cards.rs:72` — BUT the test enshrines wrong auto-draw behavior (no player choice presented).
- Power exactly 2 threshold: NOT TESTED explicitly (test uses 1/1).
- Power > 2 entering does not trigger draw: NOT TESTED.
- Power raised from ≤ 2 to > 2 between ETB and resolution: NOT TESTED.
- Power reduced from > 2 to ≤ 2 between ETB and resolution: NOT TESTED.
- Player declines "you may" (chooses not to pay {1}): NOT TESTED.
- Player has no mana — ability triggers but cannot pay: NOT TESTED.
- "another" exclusion (self does not trigger): NOT TESTED.
- Creature entering under opponent's control does not trigger: NOT TESTED.
- +1/+1 counters or aura effects apply to power check: NOT TESTED.
- Cannot pay {1} multiple times to draw multiple cards: NOT TESTED.
