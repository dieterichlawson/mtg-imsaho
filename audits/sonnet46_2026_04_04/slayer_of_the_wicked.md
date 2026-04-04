## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: When this creature enters, you may destroy target Vampire, Werewolf, or Zombie.
**Type line**: Creature — Human Soldier
**Status**: ISSUE

### Code issues

- Subtype check only reads registry data, missing token subtypes (`slayer_of_the_wicked.rs` lines 41–43)
  - Oracle text says: `"you may destroy target Vampire, Werewolf, or Zombie"`
  - Code does: `registry.card_data(o.card_id).map(|d| d.subtypes.iter().any(|s| s == "Vampire" || s == "Werewolf" || s == "Zombie")).unwrap_or(false)` — tokens are created with `card_id: CardId(0)` (sentinel, line 364 of `state.rs`), so `registry.card_data(CardId(0))` returns `None` and `.unwrap_or(false)` causes all tokens to be excluded. Zombie tokens (e.g., from Moan of the Unhallowed), Vampire tokens, and Werewolf tokens are never included in the valid-targets list even though they are legal targets. The correct pattern (shown in `state.rs` lines 665–672) checks registry data first and then falls through to `creature.subtypes.iter().any(...)` for tokens.

### Tricky interactions checked

- **"you may" optionality**: PASS — `present_optional_target_choice` is called with `optional=true`; `available_actions` includes `ResolvedChoice::ChosenTarget(None)` as a choice when `optional=true` (`engine.rs` line 200), letting the player decline to destroy. The `ChooseTarget` resolution arm (engine.rs lines 2003–2007) applies the effect only when `Some(t)` is returned.
- **Empty target list (no valid targets on board)**: PASS — `present_target_choice` (helpers.rs line 126) returns early with no action when `targets.is_empty()`, so the ability silently does nothing, which is correct.
- **Targeting any controller (not just opponent)**: PASS — the target-collection loop in `on_enter_battlefield` includes all battlefield objects with the matching subtype, regardless of controller. Oracle text has no controller restriction.
- **ETB trigger dispatch**: PASS — `collect_triggers` in `triggers.rs` (lines 344–364) creates `PendingTrigger::EnteredBattlefield` for all registered cards; `resolve_next_trigger` (lines 893–898) verifies Slayer is still on the battlefield, then calls `on_enter_battlefield`.
- **ETB trigger fires only while Slayer is on battlefield**: PASS — `resolve_next_trigger` checks `o.zone == Zone::Battlefield` before calling `on_enter_battlefield` (triggers.rs line 895).
- **`PendingEffect::Destroy` uses `try_destroy` (respects Indestructible/Regeneration)**: PASS — `apply_pending_effect` (engine.rs line 2207) calls `crate::destruction::try_destroy`, which checks Indestructible (destruction.rs line 35) and Regeneration shields (line 40) before destroying.
- **`try_destroy` vs `sacrifice`**: PASS — oracle says "destroy"; code uses `try_destroy`, not `sacrifice`.
- **Token subtype check**: FAIL — see Code Issues above. Zombie/Vampire/Werewolf tokens have `card_id: CardId(0)` and subtypes stored in `obj.subtypes`; the filter only reads `registry.card_data(o.card_id)`, which returns `None` for tokens.
- **Self-exclusion (`o.id != object_id`)**: PASS (harmless) — Slayer is a Human Soldier, not a Vampire/Werewolf/Zombie, so it could never be a valid target anyway. The guard is redundant but not harmful.
- **Mana cost, P/T, card types, subtypes, keywords**: PASS — `{3}{W}` (Generic(3) + Colored(White)), 3/2, `[Creature]`, `["Human", "Soldier"]`, `[]` keywords all match oracle.

### Test coverage

- Slayer destroys a Zombie (Walking Corpse, a registered card): `tier3_cards.rs:176` — TESTED
- Player declines to destroy (ChosenTarget(None)): NOT TESTED
- Slayer targets a Zombie token: NOT TESTED (the token-subtype bug would be caught here)
- Slayer targets a Vampire token: NOT TESTED
- Slayer targets a Werewolf token: NOT TESTED
- Target is Indestructible (destruction blocked): NOT TESTED
- Target Regenerates (destruction replaced): NOT TESTED
- No valid targets on board (ability does nothing): NOT TESTED
