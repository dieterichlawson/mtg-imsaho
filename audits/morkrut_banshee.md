## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/110/morkrut-banshee?utm_source=api
**Type line**: `Creature — Spirit` — {3}{B}{B}, 4/4
**Oracle text**:
```
Morbid — When this creature enters, if a creature died this turn, target creature gets -4/-4 until end of turn.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Morbid — ... **if** a creature died this turn" is an intervening-if
  (CR 603.4): checked when the trigger would go on the stack *and* again on
  resolution, via `should_trigger`: PASS
- -4/-4 until end of turn kills a 4/4 by state-based action, so indestructible
  does not save it: PASS
- The trigger is targeted, so it is not put on the stack at all with no legal
  creature to point at: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Morbid as an intervening-if: `intervening_if.rs`, `cards_morbid_and_ltb.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/110/morkrut-banshee?utm_source=api
**Type line**: `Creature — Spirit` — {3}{B}{B}, 4/4
**Oracle text**:
```
Morbid — When this creature enters, if a creature died this turn, target creature gets -4/-4 until end of turn.
```

**Rulings fetched**:
- [2020-08-07] Morkrut Banshee's morbid ability triggers only once, not once for each creature that has died this turn. If no creatures have died by the time it enters the battlefield, its ability won't trigger at all.
- [2020-08-07] If there are no other creatures on the battlefield when the morbid ability triggers, the ability must target Morkrut Banshee itself.

**Status**: ISSUE (1, fixed)

### Code issues found and fixed

**One: morbid was enforced in `is_valid_target`, which is the wrong rule in
the wrong place.**

```rust
/// Enforce morbid at target selection: if no creature died this turn,
/// no creature is a legal target, so the trigger is removed per CR 603.3c.
fn is_valid_target(&self, state: &GameState, _caster: PlayerId, _target: &Target, _registry: &CardRegistry) -> bool {
    state.creature_died_this_turn
}
```

- Oracle text says: `Morbid — When this creature enters, if a creature died
  this turn, target creature gets -4/-4 until end of turn.`
- Ruling 2020-08-07 says: `If no creatures have died by the time it enters the
  battlefield, its ability won't trigger at all.`
- Code does: reports every object as a legal target whenever a creature has
  died, and no object as one otherwise.

"If a creature died this turn" placed between the trigger event and the effect
is an intervening-if (CR 603.4): the ability does not trigger at all, and the
condition is checked a second time on resolution. Reaching the same board state
through CR 603.3c — put the ability on the stack, then take it off for want of
targets — is a different sequence of events, and the engine writes "Trigger
removed: no legal targets" in the game log for an ability that by 603.4 never
triggered. Reaper from the Abyss had exactly this and lost it; this card was
missed.

It was also redundant: `should_trigger` already calls
`helpers::morbid_should_trigger`, which is the CR 603.4 check, and
`on_enter_battlefield` re-checks on resolution.

The second half of the harm is quieter. Because the override ignored its
`_target`, the card asserted that *any* object is a legal target for it. That
is only harmless because `stack.rs` ANDs the card's `is_valid_target` with the
engine's `TargetRequirement::Creature` rather than letting either stand alone —
the card was contributing nothing but a wrong answer. Removed; the trait
default (`true`) plus the declared requirement is the whole truth.

This is a one-off, not a cluster: Morkrut Banshee was the only card in the set
whose `is_valid_target` ignored the target it was passed.

### Card data checked against the fetched text

| field | oracle | code |
|---|---|---|
| cost | `{3}{B}{B}` | `Generic(3), Black, Black` OK |
| type | `Creature - Spirit` | `Creature`, `["Spirit"]` OK |
| P/T | 4/4 | `Some(4)/Some(4)` OK |
| keywords | Morbid | none declared - correct, morbid is an ability word (CR 207.2c) |
| oracle text | verbatim match | OK |
| trigger | enters, targets a creature | `TriggerKind::EntersBattlefield` with `TargetRequirement::Creature` OK |

### Tricky interactions checked

- **Ruling: "triggers only once, not once for each creature that has died this
  turn."** **Pass** — it is an enters trigger and morbid is a yes/no condition,
  never a count. Was untested; now is, with three creatures dead.
- **Ruling: "If there are no other creatures on the battlefield when the morbid
  ability triggers, the ability must target Morkrut Banshee itself."**
  **Pass** — the Banshee is on the battlefield when its own enters trigger goes
  on the stack, so it is among the candidates, and with one legal target the
  engine takes it. Tested.
- **CR 603.4, both directions** — no death, no trigger at all (no stack entry,
  no priority window); a death, one trigger. **Pass**, tested.
- **CR 603.4's second check on resolution.** **Pass** — kept in
  `on_enter_battlefield`. Not reachable in play, since
  `creature_died_this_turn` only clears at the turn boundary, but it is what
  the rule asks for.
- **CR 608.2b, the target became illegal.** **Pass**, tested via the shared
  re-check sweep (a target that gained hexproof in response).
- **CR 113.7a, the Banshee killed in response to its own enters trigger.**
  **Pass** — nothing in "target creature gets -4/-4 until end of turn" is about
  the Banshee, and the handler correctly ignores its own source. Was untested;
  now is.
- **"until end of turn"** — a creature that survives is back to its printed
  size next turn (CR 514.2). Was untested; now is.
- **-4/-4 on a 4/4 kills it** by state-based action rather than destruction, so
  indestructible does not save it (CR 704.5f). Covered by the self-target test,
  which accepts the Banshee either at 0 toughness or already in the graveyard.

### Test coverage

- morbid gates the trigger, both directions:
  `intervening_if.rs::morbid_etb_triggers_only_when_a_creature_died`
- the target became illegal between announcement and resolution:
  `trigger_target_recheck.rs::a_trigger_whose_target_became_illegal_changes_nothing`
- the ruling — with nothing else on the board it must target itself:
  `cards_morbid_and_ltb.rs::morkrut_banshee_can_target_self`
- **the ruling — one trigger however many creatures died**:
  `cards_morbid_and_ltb.rs::morkrut_banshees_morbid_triggers_once_however_many_creatures_died` (new)
- **the debuff is until end of turn**:
  `cards_morbid_and_ltb.rs::morkrut_banshees_minus_four_wears_off` (new)
- **CR 113.7a, killed in response**:
  `cards_morbid_and_ltb.rs::morkrut_banshees_debuff_lands_after_the_banshee_is_killed_in_response` (new)

Both new behavioural tests mutation-checked: changing the debuff to -3/-3 fails
the until-end-of-turn test, and adding a `still_on_battlefield` bail fails the
killed-in-response test.
