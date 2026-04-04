## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Whenever this creature attacks or blocks, other Humans you control get +1/+1 until end of turn.
**Type line**: Creature — Human Warrior
**Status**: ISSUE

### Code issues

- Trigger does not resolve if Hamlet Captain leaves the battlefield before the trigger resolves.
  - Oracle text says: `Whenever this creature attacks or blocks, other Humans you control get +1/+1 until end of turn.`
  - Code does: In `mtg-engine/src/triggers.rs` lines 980–986, the `AttacksTrigger` resolution is gated by `if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false)`. Equivalently for `BlocksTrigger` at lines 987–993. If the Captain is destroyed in response to the trigger (e.g., opponent plays instant removal), `get_object` still finds it but in `Zone::Graveyard`, causing the condition to return `false` and the trigger to do nothing. Additionally, `mtg-engine/src/cards/isd/hamlet_captain.rs` lines 53–56 repeat the same check inside `buff_humans`: `let controller = match state.get_object(self_id) { Some(o) if o.zone == Zone::Battlefield => o.controller, _ => return, };`. Per MTG rules (CR 603.6b), a triggered ability exists on the stack independently of its source; if the Captain is removed after the trigger is on the stack, the trigger should still resolve and buff the other Humans. The `controller` needed by `buff_humans` is already captured in `AttacksTrigger { controller, .. }` and `BlocksTrigger { controller, .. }`, making the battlefield lookup unnecessary.

### Tricky interactions checked

- **"Other" exclusion**: `buff_humans` filters by `o.id != self_id`, correctly excluding Hamlet Captain itself from the buff — pass.
- **Human subtype check covering tokens**: Code checks both `o.subtypes.iter().any(|s| s == "Human")` (runtime object subtypes, catches tokens) AND `registry.card_data(o.card_id).map(|d| d.subtypes.iter().any(|s| s == "Human"))` (registry data, catches real cards). Both sources consulted — pass.
- **"You control" filter**: `buff_humans` filters by `o.controller == controller` where `controller` is the Captain's controller — correctly buffs only Humans under the same controller — pass.
- **Until end of turn expiry**: `UntilEndOfTurnEffect` entries are cleared in the Cleanup step (`engine.rs` line 3021: `state.until_end_of_turn_effects.clear()`). The +1/+1 correctly expires — pass.
- **Attack trigger dispatch**: `collect_triggers` in `triggers.rs` lines 677–750 handles `GameEvent::AttackersDeclared`, checks `trigger_description` for `TriggerKind::Attacks` (non-empty for Hamlet Captain), and enqueues `AttacksTrigger`. Resolves to `on_attacks` → `buff_humans` — pass for the common case.
- **Block trigger dispatch**: `collect_triggers` in `triggers.rs` lines 752–847 handles `GameEvent::BlockersDeclared`, checks `trigger_description` for `TriggerKind::Blocks` (non-empty for Hamlet Captain), and enqueues `BlocksTrigger`. Resolves to `on_blocks` → `buff_humans` — pass for the common case.
- **Trigger independence (source leaves battlefield)**: If the Captain is destroyed in response to its attack or block trigger, the trigger is skipped due to battlefield checks in `triggers.rs` lines 981 and 988, and again in `hamlet_captain.rs` line 54. Per MTG rules the trigger should still resolve — FAIL (described as ISSUE above).
- **Card registered in registry**: `with_all_cards()` in `cards/mod.rs` line 687 registers `hamlet_captain::HamletCaptain` — pass.

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- **Attacks trigger buffs other Humans**: NOT TESTED
- **Blocks trigger buffs other Humans**: NOT TESTED
- **Self-exclusion ("other")**: NOT TESTED
- **Human subtype check for tokens**: NOT TESTED
- **Non-Human creatures not buffed**: NOT TESTED
- **Until end of turn expiry**: NOT TESTED
- **Trigger resolves even if Captain leaves battlefield before resolution**: NOT TESTED
