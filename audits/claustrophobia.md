## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/48/claustrophobia?utm_source=api
**Type line**: `Enchantment — Aura` — {1}{U}{U}
**Oracle text**:
```
Enchant creature
When this Aura enters, tap enchanted creature.
Enchanted creature doesn't untap during its controller's untap step.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "When this Aura enters, **tap** enchanted creature" is a separate ETB trigger,
  and "doesn't untap during its controller's untap step" is the static half —
  two abilities, not one: PASS
- The ETB trigger taps whatever it is attached to at resolution, so removing the
  Aura in response leaves the creature untapped: PASS
- `PreventUntap` is scoped `Attached`, so it stops when the Aura leaves: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The ETB tap and the untap prevention: `cards_auras.rs`, `enchantments.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/48/claustrophobia?utm_source=api
**Type line**: `Enchantment — Aura` — {1}{U}{U}
**Oracle text**:
```
Enchant creature
When this Aura enters, tap enchanted creature.
Enchanted creature doesn't untap during its controller's untap step.
```

**Rulings fetched**:
- [2015-06-22] Claustrophobia can target and enchant a tapped or untapped creature.
- [2015-06-22] The enchanted creature can still be untapped in other ways. Claustrophobia will remain attached, and the creature will continue to not untap during its controller's untap step.

**Status**: ISSUE (1, fixed)

### Code issues found and fixed

**One: the enters trigger read the attachment off the Aura, which the Aura's
own destruction clears.**

- Oracle text says: `When this Aura enters, tap enchanted creature.`
- Code did:
  ```rust
  if let Some(target_id) = state.get_object(object_id).and_then(|o| o.attached_to) {
  ```

CR 113.7a: once the enters trigger is on the stack it exists independently of
the Aura, so destroying Claustrophobia in response does not counter it.
CR 608.2g: "enchanted creature" is then the creature it was last attached to.
But leaving the battlefield clears `attached_to` (CR 400.7), so the trigger
found nothing and the creature was never tapped — removal in response was a
clean answer to a resolved Aura.

Confirmed before changing anything rather than argued. With the Aura destroyed
between resolving and its trigger resolving:

```
aura zone=Graveyard attached_to=None last=Some(ObjectId(1)) creature tapped=false
```

The engine already stashes the answer: `move_object` writes
`card_state["last_attached_to"]` on the way out, for exactly this. What was
missing was the accessor. `state.attached_player` has existed for the Curse
side of this — its doc comment says "a Curse's triggered ability on the stack
resolves even if the Curse is destroyed in response, and it still knows whom it
cursed" — with no counterpart for creatures. `state.attached_creature` is now
that counterpart, and Claustrophobia uses it.

This is a one-off, not a cluster: a scan of `src/cards` found Claustrophobia to
be the only card reading `attached_to` inside a resolution hook. Curiosity's
read is in `should_trigger_on_damage_to_player`, a trigger *condition* evaluated
at event time while the Aura is on the battlefield (CR 603.2), which is right;
Blazing Torch's looks up the attachment from the other direction.

### Card data checked against the fetched text

| field | oracle | code |
|---|---|---|
| cost | `{1}{U}{U}` | `Generic(1), Blue, Blue` OK |
| type | `Enchantment - Aura` | `Enchantment`, `["Aura"]` OK |
| enchant | "Enchant creature" | `TargetRequirement::Creature` OK |
| static | "Enchanted creature doesn't untap during its controller's untap step" | `ContinuousEffect::PreventUntap { scope: Attached }` OK |
| trigger | "When this Aura enters, tap enchanted creature" | `TriggerKind::EntersBattlefield` with a matching hook OK |
| oracle text | verbatim match | OK |

Scryfall lists `Enchant` under keywords; it is expressed here as
`target_requirement`, which is what CR 702.5 makes it — a static ability
defining what the Aura can be attached to. Consistent with the other Auras in
the set.

### Tricky interactions checked

- **Ruling 2015-06-22: "Claustrophobia can target and enchant a tapped or
  untapped creature."** **Pass** — nothing in the targeting asks. Was untested;
  now is, both that a tapped creature is offered as a target and that the
  enchantment lands on it.
- **Ruling 2015-06-22: "The enchanted creature can still be untapped in other
  ways. Claustrophobia will remain attached, and the creature will continue to
  not untap during its controller's untap step."** **Pass**, and worth being
  precise about why: `PreventUntap` is consulted only by
  `state.untaps_normally`, whose one caller is the untap step in `engine.rs`.
  So it stops that step and nothing else. Was untested; now is — the creature is
  untapped by other means, the Aura stays attached, and it still will not untap
  normally. Making `untaps_normally` a blanket prohibition fails the new test.
- **The Aura destroyed in response to its own enters trigger.** **Was broken,
  now fixed** — and the new test also asserts the other half: the *static*
  ability is gone with the Aura, so the creature untaps normally from then on.
- **The target became illegal before the Aura resolved.** **Pass** — the spell
  fizzles under CR 608.2b, and `helpers::resolve_aura` refuses to attach to
  anything not on the battlefield, so the Aura goes to the graveyard rather
  than entering unattached.
- **The enchanted creature leaves.** **Pass**, handled generally — the Aura
  falls off and dies to state-based action (CR 704.5m).
- **The Aura taps by writing `tapped = true`.** Consistent with the ten other
  cards in the set that tap something; the engine has no tap event to emit.

### Test coverage

- taps the creature on entry and stays attached:
  `cards_vanilla_and_keywords.rs::claustrophobia_taps_creature`
- the enchanted creature does not untap in a real untap step, against a control
  creature on the same battlefield:
  `cards_morbid_and_ltb.rs::claustrophobia_prevents_untap`
- **destroyed in response, the tap still happens — and the static ability does
  not**: `cards_vanilla_and_keywords.rs::claustrophobia_still_taps_if_the_aura_is_destroyed_in_response` (new)
- **the first ruling — a tapped creature is a legal target**:
  `::claustrophobia_can_enchant_an_already_tapped_creature` (new)
- **the second ruling — untapped some other way, still attached, still will not
  untap normally**:
  `::claustrophobia_does_not_stop_a_creature_being_untapped_some_other_way` (new)

Mutation-checked: reading `attached_to` again fails the destroyed-in-response
test, and dropping the last-known fallback from the accessor fails it too.
