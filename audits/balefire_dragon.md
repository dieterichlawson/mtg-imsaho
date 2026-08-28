## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/129/balefire-dragon?utm_source=api
**Type line**: `Creature — Dragon` — {5}{R}{R}, 6/6
**Oracle text**:
```
Flying
Whenever this creature deals combat damage to a player, it deals that much damage to each creature that player controls.
```

**Status**: PASS

### Code issues
No issues found.

- "Whenever **this creature** deals combat damage to a player" — `CombatDamageToPlayer`,
  the self variant. (`AnyCombatDamageToPlayer` is the other one, for a trigger
  watching some *other* creature — Rakish Heir uses that.)
- "it deals **that much** damage to each creature that player controls" — the
  amount is the combat damage dealt, and the affected creatures are the damaged
  player's, not everyone's.
- CR 113.7a: killing the Dragon in response does not save the board; the ability
  is independent of its source once on the stack.
- All four counter-adders check the creature is still on the battlefield before
  adding, so an ability resolving after its source died does nothing rather than
  putting a counter on a permanent that is not there.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_combat_damage_triggers.rs` — including a table-driven coverage check that every card with this trigger shape in the set is exercised.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/129/balefire-dragon?utm_source=api
**Type line**: `Creature — Dragon` — {5}{R}{R}, 6/6
**Oracle text**:
```
Flying
Whenever this creature deals combat damage to a player, it deals that much damage to each creature that player controls.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "The damage dealt by Balefire Dragon's triggered ability **isn't combat
  damage**." It goes through `PendingEffect::DealDamage`, which calls
  `deal_damage` with `DamageKind::NonCombat` — so it emits
  `NonCombatDamageDealt`, does not feed lifelink as combat damage, and does not
  trigger combat-damage watchers: PASS
- "it deals **that much** damage" — the amount is the combat damage that was
  actually dealt, passed into the trigger: PASS
- "each creature **that player** controls" — not your own board: PASS
- CR 113.7a: killing the Dragon with the trigger on the stack does not save the
  defending player's board: PASS
- Protection and prevention apply, because it goes through the pipeline — this
  was one of the cards that used to write damage by hand: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The non-combat damage and protection: `inline_damage.rs`, `cards_burn_and_damage.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/129/balefire-dragon?utm_source=api
**Type line**: `Creature — Dragon` — {5}{R}{R}, 6/6
**Oracle text**:
```
Flying
Whenever this creature deals combat damage to a player, it deals that much damage to each creature that player controls.
```

**Rulings fetched**:
- [2018-12-07] The damage dealt by Balefire Dragon’s triggered ability isn’t combat damage.

**Status**: ISSUE (fixed)

### Code issues

**1. A guard that did the opposite of the comment above it.**

`on_combat_damage_to_player` opened with:

```rust
// CR 113.7a: killing the Dragon with the trigger on the stack does not
// save the defending player's board.
if state.get_object(self_id).is_none() {
    return;
}
```

The sentence and the code disagree. `get_object` returns `None` for exactly
one thing — an object that has ceased to exist — and in this pool that is a
**token copy of the Dragon** (Cackling Counterpart: "Create a token that's a
copy of target creature you control"). A token that connects, has its trigger
put on the stack, and is then destroyed goes to the graveyard and is removed
from the game outright by SBA 704.5d (CR 111.7). The guard fired, the ability
did nothing, and the board was spared — the precise outcome the comment says
must not happen.

Nothing in the handler reads the Dragon: "that much" is the damage the trigger
already carries and "that player" is the player it was dealt to. The engine's
own dispatch says so in `triggers.rs` — *"There is one rule and it is stated
here: the source's zone is not consulted. A handler that genuinely needs its
permanent present checks for itself."* This handler does not need it. Guard
removed.

A *card* Dragon killed in response was already fine and already tested
(`trigger_source_independence.rs`), because a card stays readable in the
graveyard. Every test in that file kills a card, which is why the hole
survived: only a token distinguishes "changed zones" from "ceased to exist".

**2. `PendingEffect::DealDamage` carried a `source_name` nothing read.**

- Code was: `DealDamage { amount: u32, source_id: ObjectId, source_name: String }`
- Both handlers in `engine/effects.rs`: `PendingEffect::DealDamage { amount, source_id, source_name: _ }`

Fifteen construction sites across fourteen card files computed and passed a
name that was discarded; `deal_damage` writes its own log line from the source
object. Here it was worse than dead — Balefire's site read
`.map_or_else(|| "Balefire Dragon".into(), |o| o.name.clone())`, which looks
like last-known-information handling for a source that has gone, and was not.
Field removed from the variant, along with the three now-unused local bindings
it existed for (`helpers::resolve_damage`, `garruk_relentless`, here).

### Card data

`{5}{R}{R}` Creature — Dragon, 6/6, Flying — all pinned pool-wide against the
Scryfall cache by `card_data_invariants.rs`. One triggered ability declared,
`TriggerKind::CombatDamageToPlayer`, matching the one implemented hook.
`triggers/collect/damage.rs` emits that event only for
`DamageTarget::Player`, so combat damage to a planeswalker correctly does not
trigger it.

### Tricky interactions checked

- Source destroyed in response, as a **card**: pass (already tested).
- Source destroyed in response, as a **token**: **was broken, fixed**.
- The ruling — the trigger's damage is not combat damage: pass, and now tested
  through Inquisitor's Flail, which doubles combat damage only.
- "that much" is the damage dealt, not the Dragon's power: pass. Nothing reads
  `effective_power`; the amount rides the trigger event. The old test used 6,
  which is also the Dragon's printed power, so it could not tell the two apart
  — now 4.
- "each creature **that player** controls": pass — `objects_in_zone` keys the
  battlefield on controller, and the Dragon's own side is untouched.
- Damage marked through the pipeline (protection, deathtouch, `damaged_by`,
  lifelink, planeswalker loyalty): pass — `deal_damage` via
  `apply_pending_effect`, never `damage_marked` by hand.

### Recorded, not fixed

**Last known information dies with a token.** `cease_to_exist` drops the object
from `state.objects`, so every characteristics accessor — all of which funnel
through `get_object` — has no answer for it. `last_known_controller` returns
`PlayerId(0)`; `subtypes_of` and `matches_filter` return nothing, so
`has_protection_from` cannot see the source's subtypes or colours; `obj_name`
logs `? (#n)`. CR 608.2g says the ability uses the source's last known
information, and there is none.

Not fixed because I could not build a case that is reachable in this pool, and
the fix is not small. It needs damage from a source that has ceased to exist to
land on something with protection from it, and the damage must be *untargeted*
(protection makes a creature an illegal target, so a targeted ability never
gets that far). The only protection printed in the set is from Vampires,
Werewolves and Zombies (Elite Inquisitor, Grave Bramble) plus Spare from Evil's
temporary grant; the only creatures with a subtype among those that deal damage
through an ability are Olivia Voldaren and Daybreak Ranger, and both of those
abilities target. So: real, and unreachable with these 249 cards.

The shape of the fix, if a later set makes it reachable: a
`last_known: HashMap<ObjectId, GameObject>` written by `cease_to_exist`, and a
`last_known_object(id)` that the CR 608.2g accessors (`face_data`, `name_of`,
`subtypes_of`, `colors_of`, `has_keyword`, `last_known_controller`) consult
after `objects`. `get_object` itself must not fall back — "does this object
exist" and "what were its characteristics" are different questions, and a
great deal of code asks the first through `get_object`.

**Curse of the Pierced Heart carries the same guard** (`curse_of_the_pierced_heart.rs`,
`if state.get_object(self_id).is_none() { return }`), under a comment about the
same rule. There it is inert rather than wrong — the next line,
`let Some(cursed_player) = state.attached_player(self_id) else { return }`,
returns anyway, and a Curse cannot be a token in this pool — so removing it
belongs in that card's own audit, against its own oracle text.

### Test coverage

- the sweep, its amount, and whose creatures:
  `cards_combat_damage_triggers.rs::balefire_dragon_sweeps_opponent_creatures`
  (strengthened — the amount now differs from the Dragon's power)
- the ruling, that the damage is not combat damage:
  `cards_combat_damage_triggers.rs::balefire_dragons_sweep_is_not_combat_damage` (new)
- source killed in response, as a card:
  `trigger_source_independence.rs::balefire_dragon_wipes_the_board_after_being_killed_in_response`
- source ceasing to exist, as a token:
  `trigger_source_independence.rs::balefire_dragon_token_wipes_the_board_after_ceasing_to_exist` (new)

### Mutations run

- Reinstate the `get_object(self_id).is_none()` guard: **fails** the token test,
  passes the card test — which is the whole finding.
- `DamageKind::NonCombat` → `Combat`: **fails** the ruling test (12 vs 6).
- Sweep the Dragon's controller's creatures instead of the damaged player's:
  **fails** both sweep tests.
- Amount taken from `effective_power(self_id)` instead of the trigger:
  **fails** the sweep test (6 vs 4). This is the mutation the old test passed.
- (One earlier mutation, `objects_in_zone` → `all_objects_in_zone`, failed to
  compile — it left `damaged_player` unused — and was redone as the
  controller-swap above.)

Suite: 1508 passing, exit 0, `cargo check --workspace --all-targets` clean.
