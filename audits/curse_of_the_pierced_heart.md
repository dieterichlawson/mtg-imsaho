## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/138/curse-of-the-pierced-heart?utm_source=api
**Type line**: `Enchantment — Aura Curse` — {1}{R}
**Oracle text**:
```
Enchant player
At the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.
```

**Status**: ISSUE

### Code issues
See below.


- The ordinary case — no planeswalkers — wrote the life total by hand instead of
  dealing damage.
  - Oracle text says: `this Aura deals 1 damage to that player or a planeswalker that player controls`
  - Code did: `let new_life = old - 1; state.get_player_mut(cursed_player).life = new_life;`
    followed by its own `NonCombatDamageDealt` and `LifeChanged` events
  - So the case that happens every game skipped `damage::deal_damage` and
    everything it applies: protection, prevention, damage multipliers, and the
    watchers that key on the pipeline. The planeswalker branch right beside it
    already used `PendingEffect::DealDamage`. Both branches now build one
    effect and differ only in whether there is a choice to present. The
    `only_the_damage_pipeline_marks_damage` guard, which watched `damage_marked`
    and so could not see a hand-written *player* life change, was extended to
    catch this shape.

### Tricky interactions checked
- "At the beginning of **enchanted player's** upkeep" — CR 603.2: the trigger
  event is that player's upkeep beginning, so `TriggerScope::AttachedPlayer`
  keeps it off the stack during anyone else's: PASS
- CR 113.7a: destroying the Curse in response does not counter its trigger, and
  `attached_player` still knows whom it cursed: PASS
- Enchant **player**, so `TargetRequirement::PlayerOnly` and the Curse subtype:
  PASS
- "…to that player **or a planeswalker that player controls**" — the choice is
  the *Curse controller's*, not the cursed player's: PASS
- `obj.card_types` is empty for a non-token permanent, so the planeswalker scan
  goes through `has_card_type`, which reads the active face: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The damage and the planeswalker choice: `cards_auras.rs`, `curse_and_equip_scope.rs`
- The pipeline guard: `test_suite_guards.rs:only_the_damage_pipeline_marks_damage`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/138/curse-of-the-pierced-heart?utm_source=api
**Type line**: `Enchantment — Aura Curse` — {1}{R}
**Oracle text**:
```
Enchant player
At the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (fixed)

### Code issues

Two found, both fixed in this pass.

1. **Choice options were built in HashMap order.** `mtg-engine/src/cards/isd/curse_of_the_pierced_heart.rs:69` (before the fix)
   - Oracle text says: `this Aura deals 1 damage to that player or a planeswalker that player controls.`
   - Code did: `let planeswalkers: Vec<Target> = state.objects.values().filter(|o| o.zone == Zone::Battlefield && o.controller == cursed_player)` — and those objects go straight into `ResolutionChoiceKind::ChooseTarget { options, .. }`, which the controller picks from by position. `GameState::objects` is a `HashMap`, whose iteration order is seeded per process, so with two planeswalkers on the cursed player's side the same game replayed twice offered them under different indices.
   - Now: `state.objects_in_zone(Zone::Battlefield, cursed_player)`, which sorts by id.

2. **The chooser was read as `o.controller`, which the object no longer has once the Curse is gone.** `curse_of_the_pierced_heart.rs:62` (before the fix)
   - Code did: `let Some(controller) = state.get_object(self_id).map(|o| o.controller) else { return };`
   - Leaving the battlefield resets `controller` to `owner` (CR 400.7 cleanup, `state.rs`), so the chooser became the Curse's owner. The comment directly above this line is about surviving the Curse's destruction — "CR 113.7a: destroying the Curse in response does not counter its trigger" — so the one case the author had in mind was the case that read the wrong field. CR 608.2g: the ability is controlled by the last known controller.
   - Now: `let controller = state.last_known_controller(self_id);`

Also: the stack description was `"deal 1 damage to enchanted player"`, which omitted the planeswalker half the card offers. Now `"deals 1 damage to enchanted player or a planeswalker they control"`.

**Engine-wide follow-up.** Issue 1 is the third instance of the same root cause (after the shared target helpers and Divine Reckoning). Rather than fix a fourth one later, the ordering guarantee now lives in the accessors: `objects_in_zone` / `all_objects_in_zone` already sorted by id, `GameState::objects_in_id_order` was added for zone-agnostic scans, all 34 remaining card sites moved onto them, and `card_data_invariants::no_card_iterates_the_object_map_directly` fails the build on a card that goes back to the raw map.

### Checked and correct

- Cost `{1}{R}`, type `Enchantment — Aura Curse`, subtypes `["Aura", "Curse"]`, no P/T — all match the type line.
- `target_requirement: PlayerOnly` implements `Enchant player`.
- `step_trigger_scope` returns `TriggerScope::AttachedPlayer`, so the trigger goes on the stack during the *enchanted* player's upkeep, not its controller's (CR 603.2).
- The damage goes through `PendingEffect::DealDamage` -> `damage::deal_damage`, so protection, prevention and damage watchers all apply. (An earlier version wrote the life total by hand; that is already fixed and the comment records it.)
- `options.len() == 1` applies the damage directly instead of prompting with one option — correct, there is no choice to make.
- `optional: false` — the damage is mandatory, the card has no "may".
- The chooser is the Curse's controller, not the cursed player: "deals ... to that player or a planeswalker that player controls" has no "that player chooses".
- Planeswalkers are found via `state.has_card_type(id, Planeswalker, registry)`, which reads the active face; scanning `o.card_types` (grants only) had made the planeswalker half dead code for every real planeswalker, and that is already fixed and covered.

### Tricky interactions checked

- Curse destroyed in response to its own trigger (CR 113.7a): trigger still resolves — `attached_player` falls back to `last_attached_to_player`. PASS (and the chooser bug above was found here).
- Trigger fires only on the enchanted player's upkeep, not the controller's: PASS.
- Curse leaving the battlefield detaches from the player: PASS.
- Damage to a player under protection / prevention: PASS, goes through the damage pipeline.
- Cursed player controls a non-token planeswalker: PASS.
- Cursed player controls two planeswalkers: order is now stable.

### Test coverage

- deals 1 damage on the enchanted player's upkeep: `cards_upkeep_triggers_and_curses.rs:177`
- fires only on the enchanted player's upkeep: `trigger_targets_declared.rs:85`, `curse_and_equip_scope.rs:57`
- non-token planeswalker is seen: `characteristics_card_sweep.rs:63`
- detaches on leaving the battlefield: `zone_change_resets_object.rs:69`
- damage goes through the pipeline (protection/prevention): `player_protection.rs:79`, `:112`, `:157`
- options offered in a stable order: `cards_upkeep_triggers_and_curses.rs` `curse_of_pierced_heart_offers_its_options_in_a_stable_order` (NEW)
- last known controller chooses after the Curse leaves: `cards_upkeep_triggers_and_curses.rs` `curse_of_pierced_heart_asks_its_last_controller_after_it_leaves` (NEW, mutation-checked)
- no card reads the raw object map: `card_data_invariants.rs` `no_card_iterates_the_object_map_directly` (NEW, mutation-checked)

### Rulings

Not available this session: the egress proxy now blocks `api.scryfall.com`, `scryfall.com` and `gatherer.wizards.com`, and this card has no rulings in `data/oracle_cache.json`. A web search for its rulings surfaced a rulings page reporting none for this card, which is consistent with how plain the ability is. Oracle text itself came from the Scryfall-sourced cache, not from memory.

