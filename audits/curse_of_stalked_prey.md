## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/136/curse-of-stalked-prey?utm_source=api
**Type line**: `Enchantment — Aura Curse` — {1}{R}
**Oracle text**:
```
Enchant player
Whenever a creature deals combat damage to enchanted player, put a +1/+1 counter on that creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "The ability will trigger when **any** creature deals combat damage to
  the enchanted player, including one controlled by another opponent or even by
  the enchanted player (if combat damage gets redirected somehow)." The handler
  tests only that the damaged player is the cursed one — no controller
  restriction: PASS
- "**combat** damage", so `AnyCombatDamageToPlayer` and not the general damage
  trigger: PASS
- The counter goes on the creature that dealt the damage, and only while it is
  still on the battlefield: PASS
- CR 113.7a: destroying the Curse in response does not counter its trigger: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The counter on the damaging creature: `cards_auras.rs`, `combat_rules.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/136/curse-of-stalked-prey?utm_source=api
**Type line**: `Enchantment — Aura Curse` — {1}{R}
**Oracle text**:
```
Enchant player
Whenever a creature deals combat damage to enchanted player, put a +1/+1 counter on that creature.
```

**Rulings fetched**:
- [2011-09-22] The ability will trigger when any creature deals combat damage to the enchanted player, including one controlled by another opponent or even by the enchanted player (if combat damage gets redirected somehow).

**Status**: ISSUE


One ruling: "The ability will trigger when any creature deals combat damage to
the enchanted player, including one controlled by another opponent or even by
the enchanted player (if combat damage gets redirected somehow)."

### Code issues

**The trigger condition was checked at resolution instead of at trigger time.**

- Oracle text says: `Whenever a creature deals combat damage to enchanted
  player, put a +1/+1 counter on that creature.`
- Code did: fire on every creature's combat damage to *any* player, then
  `if state.attached_player(self_id) != Some(damaged_player) { return; }`

CR 603.2 makes "to enchanted player" part of the trigger event: combat damage
to anyone else does not make this ability trigger at all. Firing and then
doing nothing is observably different — the trigger is a real object on the
stack with a priority window around it, visible in the log, respondable.

The cause was an asymmetry in the collector. `triggers/collect/damage.rs`
gates its `AnyDamageToPlayer` watchers on
`should_trigger_on_damage_to_player`, with a comment reading "CR 603.2: the
watcher's own condition on WHO dealt the damage and to WHOM". The
`AnyCombatDamageToPlayer` branch four lines above it had no such gate. Both
cards using that trigger kind were affected, and between them they are exactly
the two halves that comment describes:

- **Curse of Stalked Prey** — condition on *whom* (enchanted player).
- **Rakish Heir** — "Whenever a **Vampire you control** deals combat damage to
  a player", condition on *who*. Every creature's combat damage, an opponent's
  and a non-Vampire's alike, put a Heir trigger on the stack.

Fixed by giving the combat branch the same gate and moving each card's
condition into `should_trigger_on_damage_to_player`. What stays in the
resolution hook is the genuinely resolution-time part: CR 121.1, a counter goes
only on a permanent still on the battlefield, so a creature that dealt its
combat damage and traded in the same step gets nothing.

That split matters for Rakish Heir in particular. Its old hook bundled
"is it a Vampire you control" together with "is it still on the battlefield"
into one `source_is_yours`, so the two rules could not be told apart.

### Not a bug, checked
- The card has no condition on who controls the dealer, which is correct and is
  what the ruling is about — "a creature", not "a creature you control".
- `target_requirement: PlayerOnly` for "Enchant player"; resolution goes
  through `helpers::resolve_curse`.
- CR 113.7a: nothing in the resolution hook reads the Curse at all now, so
  destroying it in response cannot affect the trigger.

### Tricky interactions checked
- Damage to a player the Curse is not on: does not trigger, no stack entry: pass
- Damage to the enchanted player: counter on the dealer: pass
- A creature the *enchanted player* controls dealing damage to themselves
  (the ruling's case): triggers: pass
- A creature that died in the same damage step gets no counter (CR 121.1): pass
- The Curse destroyed in response does not counter the trigger (CR 113.7a):
  pass (`trigger_source_independence.rs:237`)
- Rakish Heir: a non-Vampire does not trigger it: pass
  (`cards_combat_damage_triggers.rs:205`, which drives the real collector and
  so now tests the trigger-time gate)

### Test coverage
- Counter on combat damage to the enchanted player:
  `cards_complex_creatures.rs:88` (rewritten)
- Resolves after the Curse is destroyed: `trigger_source_independence.rs:237`
- **NEW** does not trigger — no stack entry — on damage to another player:
  `cards_complex_creatures.rs:88`
- **NEW** the ruling: any creature, including the enchanted player's own:
  `cards_complex_creatures.rs:115`
- **NEW** no counter on a creature that already died (CR 121.1):
  `cards_complex_creatures.rs:145`

### A test that had stopped testing its restriction
The existing test called `on_any_combat_damage_to_player` directly. Once the
"enchanted player" condition moved to trigger time that hook adds a counter
unconditionally, so the test would have kept passing while testing nothing
about the restriction. Rewritten to push a `CombatDamageDealt` event and run
`process_triggers`, which is the path the combat damage step actually takes.

While writing it I also had to correct my own first version: it called the
damage helper twice in a row and the earlier event was re-processed, giving two
counters instead of one. Both damage events now go into a single batch, which
is how a combat damage step deals them anyway.

