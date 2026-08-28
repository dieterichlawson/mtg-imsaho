## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/157/rage-thrower?utm_source=api
**Type line**: `Creature — Human Shaman` — {5}{R}, 4/2
**Oracle text**:
```
Whenever another creature dies, this creature deals 2 damage to target player or planeswalker.
```

**Status**: PASS

### Code issues
No issues found.

- "Whenever **another** creature dies" — `AnyCreatureDies`, and the engine's
  collection filters `o.id != dead_id`, so the source is excluded exactly as
  "another" requires.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_dispatch.rs` (which watchers a death event reaches, and how often), `trigger_source_independence.rs` (a death trigger outliving its source).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/157/rage-thrower?utm_source=api
**Type line**: `Creature — Human Shaman` — {5}{R}, 4/2
**Oracle text**:
```
Whenever another creature dies, this creature deals 2 damage to target player or planeswalker.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "If Rage Thrower **dies at the same time as another creature**, its
  ability will trigger." The other creature's death is the event, and CR 603.6d
  lets the trigger resolve from the graveyard: PASS
- "Whenever **another** creature dies" — declared as `AnyCreatureDies` only, with
  no `SelfDies`, so its own death alone does not trigger it: PASS
- "deals 2 damage to **target player or planeswalker**" — not any target, so it
  cannot be pointed at a creature: PASS
- Damage through the pipeline: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The death trigger and the target restriction: `cards_morbid_and_ltb.rs`, `damage_helper.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/157/rage-thrower?utm_source=api
**Type line**: `Creature — Human Shaman` — {5}{R}, 4/2
**Oracle text**:
```
Whenever another creature dies, this creature deals 2 damage to target player or planeswalker.
```

**Rulings fetched**:
- [2011-09-22] If Rage Thrower dies at the same time as another creature, its ability will trigger.

**Status**: PASS

### Code issues

No issues found. The card needed no change; this audit is coverage.

### Card data

`{5}{R}` Creature — Human Shaman, 4/2, one `AnyCreatureDies` trigger
declaring `TargetRequirement::PlayerOrPlaneswalker` for "target player or
planeswalker" — the target is chosen as the trigger goes on the stack
(CR 603.3d), which is what declaring it on the `TriggeredAbilityDef` means.
All pinned pool-wide by `card_data_invariants.rs`. Damage through
`apply_pending_effect` / `deal_damage`, so it is noncombat damage from the
Thrower and goes through the one pipeline.

**"another creature" is the collector's job, and it does it.** The
death-watch pass filters `o.id != dead_id`, so `TriggerKind::AnyCreatureDies`
means "another creature dies" for every card that declares it. I checked that
this is right for the whole set rather than just this card: eleven cards use
the kind, and the three whose printed text is *not* "another" — Falkenrath
Noble and Selhoff Occultist ("this creature **or** another creature dies") —
declare `SelfDies` alongside it and implement `on_dies` as well, so the pair
covers their own deaths. Lumberknot prints "whenever **a** creature dies" and
declares only `AnyCreatureDies`, so its own death does not trigger it — but
its effect is a +1/+1 counter on itself, which a permanent in the graveyard
cannot receive (CR 121.1), so the trigger would do nothing either way. Noted
for Lumberknot's own audit rather than acted on here.

The handler has no battlefield check, and the comment explains why at length:
a death trigger fires even when its watcher died in the same event, and the
damage is dealt from last known information (CR 608.2h). That is exactly the
card's one ruling.

### Tricky interactions checked

- The ruling — "If Rage Thrower dies at the same time as another creature, its
  ability will trigger": pass, and the *collection* half was untested.
  `trigger_source_independence.rs` covers resolution by pushing the trigger
  onto the stack by hand, which cannot see whether the collector would have
  emitted it.
- Its own death alone: does not trigger, per "another". Untested until now.
- "or planeswalker": pass — two loyalty counters come off (CR 120.3c) and its
  controller's life is untouched. Untested until now.
- Multiple deaths in one pass: one trigger each, each choosing its own target
  — covered by `apnap.rs`.
- The Thrower leaving the battlefield with the trigger on the stack:
  `trigger_source_independence.rs`.

### Test coverage

- deals 2 to the chosen player:
  `cards_death_triggers_and_tokens.rs::rage_thrower_deals_2_on_death`
- dies alongside another creature and still triggers:
  `…::rage_thrower_triggers_when_it_dies_alongside_another_creature` (new)
- its own death alone does not trigger it:
  `…::rage_thrower_does_not_trigger_on_its_own_death_alone` (new)
- a planeswalker is a legal target and loses loyalty:
  `…::rage_thrower_can_throw_its_damage_at_a_planeswalker` (new)
- the damage is dealt after the Thrower itself has died:
  `trigger_source_independence.rs::rage_thrower_deals_its_damage_after_dying_alongside_the_creature`
- trigger ordering against another death trigger: `apnap.rs`

### Mutations run

- The death-watch pass drops `simultaneously_dead`: **fails** the ruling test,
  and only that one.
- The death-watch pass drops `o.id != dead_id`: **fails** the own-death test
  (and the ruling test, since the Thrower would then trigger twice).
- The card declares `PlayerOnly` instead of `PlayerOrPlaneswalker`: **passed**
  the first version of the planeswalker test, which submitted the walker as a
  chosen target and checked the loyalty. The choice handler takes the target it
  is given, so that proved only that the damage lands where it is pointed —
  the same mistake I made on Purify the Grave. Rewritten to assert the
  planeswalker is among the *offered* options, after which the mutation fails
  it. Recorded because the weaker version would otherwise have shipped looking
  like coverage.
- (Two earlier attempts at the collector mutations did not compile — an unused
  `simultaneously_dead` binding, and redundant parentheses — and proved
  nothing; both redone.)

Suite: 1537 passing, exit 0, `cargo check --workspace --all-targets` clean.
