## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/49/curiosity?utm_source=api
**Type line**: `Enchantment — Aura` — {U}
**Oracle text**:
```
Enchant creature
Whenever enchanted creature deals damage to an opponent, you may draw a card.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "'You' refers to the controller of Curiosity, which may be different
  from the controller of the enchanted creature. 'An opponent' refers to an
  opponent of Curiosity's controller." `should_trigger_on_damage_to_player`
  tests `damaged_player != aura.controller`, and the draw goes to the Aura's
  controller: PASS
- Ruling: "If you control Curiosity and it's enchanting an opponent's creature,
  you won't draw a card when that creature deals damage to you": PASS
- Ruling: "Any damage dealt by the enchanted creature to an opponent will cause
  Curiosity to trigger, **not just combat damage**" — `AnyDamageToPlayer`: PASS
- Ruling: "Curiosity doesn't trigger if the enchanted creature deals damage to a
  planeswalker" — the trigger kind is damage to a *player*: PASS
- Ruling: "You draw one card **each time** the enchanted creature deals damage to
  an opponent, no matter how much damage it deals" — one draw per damage event,
  not per point: PASS
- CR 603.2: both halves of "whenever enchanted creature deals damage to an
  opponent" are part of the triggering event and are read at dispatch, so the
  ability does not go on the stack every time any permanent damages any player:
  PASS
- CR 113.7a: destroying Curiosity in response to its own trigger does not
  counter it — the draw still happens: PASS
- "you **may** draw" — a YesNo choice: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The opponent test and the may-draw: `cards_auras.rs`, `trigger_dispatch.rs`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/49/curiosity?utm_source=api
**Type line**: `Enchantment — Aura` — {U}
**Oracle text**:
```
Enchant creature
Whenever enchanted creature deals damage to an opponent, you may draw a card.
```

**Rulings fetched**:
- [2023-09-01] Curiosity doesn't trigger if the enchanted creature deals damage to a planeswalker or to a battle.
- [2023-09-01] You draw one card each time the enchanted creature deals damage to an opponent, no matter how much damage it deals.
- [2023-09-01] If you control Curiosity and it's enchanting an opponent's creature, you won't draw a card when that creature deals damage to you. The creature has to deal damage to one of your opponents for the ability to trigger.
- [2011-09-22] "You" refers to the controller of Curiosity, which may be different from the controller of the enchanted creature. "An opponent" refers to an opponent of Curiosity's controller.
- [2011-09-22] Any damage dealt by the enchanted creature to an opponent will cause Curiosity to trigger, not just combat damage.

**Status**: ISSUE (fixed)

### Code issues

One found in this card, and it turned out to be set-wide.

1. **The draw was offered to the wrong player when Curiosity had been destroyed in response.** `curiosity.rs:58` and `:75` (before the fix)
   - Ruling (2011-09-22) says: `"You" refers to the controller of Curiosity, which may be different from the controller of the enchanted creature.`
   - Code did: `let Some(controller) = state.get_object(self_id).map(|o| o.controller) else { return };` in `on_any_damage_to_player`, and `state.get_object(self_id).map_or(PlayerId(0), |o| o.controller)` in `on_yes_no_choice`.
   - The comment on the first line reads "CR 113.7a: the draw happens even if Curiosity is destroyed in response to its own trigger" — and that is the one case where the field is wrong, because leaving the battlefield resets `controller` to `owner` (CR 400.7 cleanup). CR 608.2g: last known information includes who controlled it.
   - Now both go through `helpers::controller_of`.

**Set-wide follow-up.** `helpers::controller_of` already existed as the shared accessor and had the same bug in it, so fixing Curiosity alone would have left ten cards using the broken helper and forty more hand-rolling it. `controller_of` now answers `state.last_known_controller`, which is identical for a source still on the battlefield, and fifty hand-rolled sites were moved onto it — including seven `.map(|o| o.controller).unwrap()` sites that would have panicked on a missing object, and thirty-odd `map_or(PlayerId(0), ...)` fallbacks that silently named player 0. `card_data_invariants::no_card_reads_its_sources_controller_by_hand` fails the build on a new one.

**Found while checking the "damage to a planeswalker" ruling.** Trigger collection scans its watchers with `state.objects.values()` at all eleven sites, and the order they come back in is the order simultaneous triggers go on the stack within an APNAP group — so two triggers that fired together resolved in a different order on a replay of the same game. The scans now go through `objects_in_id_order`. Separately noted, not fixed: CR 603.3b lets a player with several simultaneous triggers **choose** the order, and the collector does not ask. Object id is a deterministic stand-in for that choice, not the choice itself; implementing the real thing is a feature, not a bug fix, and is out of scope for this card.

### Checked against each ruling

- `Curiosity doesn't trigger if the enchanted creature deals damage to a planeswalker or to a battle.` — PASS. Both collection sites destructure `DamageTarget::Player(damaged_player)`, so damage to an object never reaches an `AnyDamageToPlayer` watcher.
- `You draw one card each time the enchanted creature deals damage to an opponent, no matter how much damage it deals.` — PASS. `on_any_damage_to_player` ignores `_amount` and offers exactly one draw.
- `If you control Curiosity and it's enchanting an opponent's creature, you won't draw a card when that creature deals damage to you. The creature has to deal damage to one of your opponents.` — PASS. `should_trigger_on_damage_to_player` tests `damaged_player != aura.controller`, i.e. the Aura's controller's opponent, not the creature's controller's opponent.
- `"You" refers to the controller of Curiosity, which may be different from the controller of the enchanted creature. "An opponent" refers to an opponent of Curiosity's controller.` — this is the ruling the fix above is about; both halves now read the Aura's controller.
- `Any damage dealt by the enchanted creature to an opponent will cause Curiosity to trigger, not just combat damage.` — PASS. `TriggerKind::AnyDamageToPlayer` is emitted from both the combat-damage and the non-combat-damage collectors.

### Checked and correct

- Cost `{U}`, `Enchantment — Aura`, subtypes `["Aura"]`, `target_requirement: Creature` for `Enchant creature`.
- `you may draw a card` is a real choice, not an auto-draw: `ResolutionChoiceKind::YesNo`, and declining draws nothing.
- Both halves of "whenever **enchanted creature** deals damage to **an opponent**" are asked at dispatch (CR 603.2) rather than at resolution, so the ability does not go on the stack and quietly do nothing every time any permanent damages any player.
- The card does not clean up its own spell.

### Tricky interactions checked

- Aura destroyed in response to its own trigger: the draw is still offered, and now to the right player.
- A creature other than the enchanted one damaging an opponent: no trigger.
- The enchanted creature damaging its own controller: no trigger.
- Damage to a planeswalker: no trigger.
- Non-combat damage: triggers.

### Test coverage

- draws on combat damage when the player accepts: `cards_combat_damage_triggers.rs:353`
- declining draws nothing: `cards_combat_damage_triggers.rs:401`
- only the enchanted creature, only an opponent, not its own controller: `trigger_independence.rs:22`
- destroyed in response still offers the draw, **to its controller and not its owner**: `trigger_source_independence.rs:88` (player assertion NEW, mutation-checked)
- no card reads its source's controller by hand: `card_data_invariants.rs` `no_card_reads_its_sources_controller_by_hand` (NEW)
- damage to a planeswalker does not trigger: NOT TESTED as such — it holds structurally, because the collectors only match `DamageTarget::Player`, and there is no `DamageTarget::Object` path to an `AnyDamageToPlayer` watcher to test against.
- one draw regardless of amount: NOT TESTED — `_amount` is unused, so there is no branch to exercise.

