## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/100/falkenrath-noble?utm_source=api
**Type line**: `Creature — Vampire Noble` — {3}{B}, 2/2
**Oracle text**:
```
Flying
Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life.
```

**Status**: PASS

### Code issues
No issues found.

- "Whenever **this creature or another** creature dies" — declares *both*
  `SelfDies` and `AnyCreatureDies`, which is the correct pair given
  `AnyCreatureDies` excludes the source. One or the other alone would silently
  drop half the wording.
- "target player loses 1 life and **you** gain 1 life" — the drain is not
  symmetric between two players; the gain is the controller's.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_dispatch.rs` (which watchers a death event reaches, and how often), `trigger_source_independence.rs` (a death trigger outliving its source).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/100/falkenrath-noble?utm_source=api
**Type line**: `Creature — Vampire Noble` — {3}{B}, 2/2
**Oracle text**:
```
Flying
Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Whenever **this creature or another creature** dies" — declared as **two**
  trigger kinds, `SelfDies` *and* `AnyCreatureDies`, so it fires on its own death
  as well as on others'. Murder of Crows, whose text says "whenever **another**
  creature dies", declares only the second — the distinction is in the card data,
  not buried in a handler: PASS
- Ruling: "If Falkenrath Noble and another creature die at the **same time**,
  Falkenrath Noble's triggered ability will trigger **for each of them**." Two
  deaths, two triggers, and its own death is one of them: PASS
- "**target player** loses 1 life and you gain 1 life" — the loss is targeted,
  the gain is always the Noble's controller: PASS
- Life **loss**, not damage, and both halves go through `lose_life` / `gain_life`
  so LifeChanged reaches every watcher: PASS
- CR 113.7a: the Noble's own death does not counter its trigger — it resolves
  from the graveyard using last known information: PASS
- It triggers on tokens dying too, since a token is a creature: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both trigger kinds and the drain: `cards_morbid_and_ltb.rs`, `simultaneous_events.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/100/falkenrath-noble?utm_source=api
**Type line**: `Creature — Vampire Noble` — {3}{B}, 2/2
**Oracle text**:
```
Flying
Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life.
```

**Rulings fetched**:
- [2017-03-14] If Falkenrath Noble and another creature die at the same time, Falkenrath Noble's triggered ability will trigger for each of them.

**Status**: PASS


One ruling: "If Falkenrath Noble and another creature die at the same time,
Falkenrath Noble's triggered ability will trigger for each of them."

### Code issues
No issues found.

- Card data matches exactly: {3}{B}, Creature — **Vampire Noble** (both
  subtypes), 2/2, Flying, oracle text verbatim.
- "target player" is declared on both `TriggeredAbilityDef`s as
  `PlayerOnly`, so the engine picks the target when the trigger goes on the
  stack (CR 603.3d) and re-checks it on resolution (CR 608.2b).
- "loses 1 life" uses `lose_life`, not damage — right, since life loss bypasses
  protection and prevention; "you gain 1 life" uses `gain_life`. Both go
  through `change_life` and so emit `LifeChanged`.
- Both hooks read `controller_of`, which is CR 608.2g's last-known controller —
  necessary here because the Noble may itself be the creature that died.

**"This creature or another creature dies" is modelled as two
`TriggeredAbilityDef`s**, `SelfDies` and `AnyCreatureDies`. That looks like it
would double-trigger on the Noble's own death, and it does not, because the
collector's death-watch arm filters `o.id != dead_id` — the dead creature does
not watch its own death, which is exactly what the separate self-dies arm is
for. Worth writing down because the two halves are in different files and
neither is obvious from the other.

### How the ruling works
Both halves of that collector do a job here:

- The Noble dies alongside another creature. For the *other* creature's death
  event, the Noble is a legal watcher even though it died too — the collector
  includes permanents that left in the same event batch, under CR 603.10a,
  because they were still on the battlefield when the simultaneous deaths
  happened.
- For its *own* death event, the death-watch arm skips it and the self-dies
  arm fires instead.

Two triggers, two drains. Drop the `simultaneously_dead` clause and it becomes
one; drop the self-exclusion and it becomes three. Both confirmed by mutation.

### Tricky interactions checked
- An opponent's creature dies: drains: pass
- Your own creature dies: drains: pass
- The Noble itself dies: drains exactly once, not twice: pass
- The ruling — the Noble and another creature die together: two drains: pass
- "You" is the Noble's last known controller when it died (CR 608.2g / 400.7):
  pass
- A player with hexproof is not offered as a target: pass
  (`hexproof_filter.rs:250`)
- The target is chosen rather than auto-picked: pass (`auto_pick.rs:678`)
- APNAP ordering when both players have death triggers: pass (`apnap.rs:101`,
  `:199`)

### Test coverage
- Drains on any creature's death: `cards_death_triggers_and_tokens.rs:238`
- Opponent's creature / your creature / the Noble itself:
  `cards_death_triggers_and_tokens.rs:433`, `:458`, `:482`
- Target is not auto-picked: `auto_pick.rs:678`
- Hexproof player not offered: `hexproof_filter.rs:250`
- APNAP trigger ordering: `apnap.rs:101`, `:199`
- **NEW** the ruling — one trigger per creature when it dies alongside another:
  `cards_death_triggers_and_tokens.rs:508`
- **NEW** the life goes to the last known controller, not the owner:
  `cards_death_triggers_and_tokens.rs:546`

### Note on the existing self-death test
It asserts +1/-1 rather than merely "life changed", which does pin "once, not
twice" — the double-trigger mutation fails it. That was already right; I am
recording it because it is the kind of assertion that is easy to weaken by
accident.

