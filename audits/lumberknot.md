## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/191/lumberknot?utm_source=api
**Type line**: `Creature — Treefolk` — {2}{G}{G}, 1/1
**Oracle text**:
```
Hexproof (This creature can't be the target of spells or abilities your opponents control.)
Whenever a creature dies, put a +1/+1 counter on this creature.
```

**Status**: PASS

### Code issues
No issues found.

- "Whenever **a** creature dies" — no controller, token or subtype filter, and
  none is applied.
- Worth recording: the wording is "a creature", not "another", so strictly the
  ability also triggers on Lumberknot's own death, and `AnyCreatureDies` excludes
  the source. Immaterial — the effect is "put a +1/+1 counter on this creature",
  and a counter on a permanent that has left the battlefield does nothing. Noted
  rather than changed, since no observable behaviour differs.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_dispatch.rs` (which watchers a death event reaches, and how often), `trigger_source_independence.rs` (a death trigger outliving its source).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/191/lumberknot?utm_source=api
**Type line**: `Creature — Treefolk` — {2}{G}{G}, 1/1
**Oracle text**:
```
Hexproof (This creature can't be the target of spells or abilities your opponents control.)
Whenever a creature dies, put a +1/+1 counter on this creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Whenever **a** creature dies" — any creature, either player's, including
  tokens: PASS
- Hexproof means opponents cannot target it, but it can still be swept by a
  Blasphemous Act: PASS
- The counters accumulate on a 1/1 base, so it grows without bound: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The death trigger and hexproof: `cards_morbid_and_ltb.rs`, `hexproof_filter.rs`

## Audit — 2026-08-28 18:13

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Lumberknot"`, https://scryfall.com/card/isd/191/lumberknot. Cross-checked the other ten `AnyCreatureDies` watchers' oracle text in the same pass.
**Oracle text**:
```
Hexproof (This creature can't be the target of spells or abilities your opponents control.)
Whenever a creature dies, put a +1/+1 counter on this creature.
```
**Type line**: Creature — Treefolk
**Mana cost**: {2}{G}{G}   **P/T**: 1/1   **Keywords**: Hexproof
**Rulings**: none on Scryfall for this card.
**Status**: ISSUE (fixed; no board outcome changes — see below)

### Code issues

**It printed "a creature dies" and declared only the watcher trigger.**

- Oracle text says: `Whenever a creature dies, put a +1/+1 counter on this creature.` Not
  "another".
- Code did: `triggered_abilities: vec![ TriggeredAbilityDef { kind: TriggerKind::AnyCreatureDies, .. } ]`
  and no `on_dies`.

`AnyCreatureDies` is the watcher kind and means *another* creature: the death-watch scan filters
`o.id != dead_id`, so a permanent never sees its own death through it. A card whose text also
covers its own death declares `SelfDies` beside it — which is exactly what Falkenrath Noble and
Selhoff Occultist do for "this creature or another creature dies", and what Lumberknot was
missing.

**I checked the other ten watchers rather than assume**, because a blanket "another" in the
engine would have been the wrong shape if several cards disagreed with it. They do not — of the
eleven, six print "another" (Unruly Mob, Village Cannibals, Thraben Sentry, Murder of Crows,
Rage Thrower, Galvanic Juggernaut), two print "this creature or another creature" and correctly
declare both kinds (Falkenrath Noble, Selhoff Occultist), and three print neither word but
cannot reach their own death anyway (Gutter Grime is an enchantment; Abattoir Ghoul's condition
is damage it dealt, which it cannot deal to itself; and Lumberknot, which is this one). So the
two-kind split is right and only this card declared the wrong set.

**No board outcome changes.** The counter goes on a Lumberknot that is already in the graveyard,
and CR 121.1 puts it nowhere. What changes is that the ability triggers at all — a stack object
with a priority window around it that the rules say is there, and which a player could respond
to.

### Tricky interactions checked
- **"A creature", not "a creature you control"**: PASS — an opponent's creature dying counts.
- **"A creature", not "another"**: fixed above.
- **One counter per death, several deaths at once**: PASS — each death is its own event.
- **Hexproof**: from `keywords`, so it goes through the characteristics layer that
  `hexproof_filter.rs` covers. Nothing card-specific.
- **A token dying**: counts — the death event fires for tokens.
- **Lumberknot dying alongside another creature**: both deaths trigger; both resolve doing
  nothing.

### Test coverage
- an opponent's creature dying gives a counter:
  `cards_death_triggers_and_tokens.rs:485 a_death_watcher_counts_the_deaths_its_text_names` (row)
  and `:585` for the stack
- its own death triggers:
  `cards_death_triggers_and_tokens.rs:~665 lumberknot_triggers_on_its_own_death_too` (NEW)
- two deaths give two counters:
  `cards_death_triggers_and_tokens.rs:~685 lumberknot_counts_each_death_once` (NEW)
- hexproof: `hexproof_filter.rs` (keyword-level, set-wide)

Mutation-checked: removing the `SelfDies` declaration — putting the card back exactly as it was
— fails the new self-death test and only it.

**Honestly reported**: emptying `on_dies` fails nothing. Its body cannot be falsified in this
pool, because the counter it adds can never land — nothing in Innistrad returns a creature from
a graveyard to the battlefield at instant speed, so there is never a Lumberknot there to receive
it. The body stays because it is the card's printed effect; writing it empty would encode "this
never does anything", which is a different and less true claim.

### Changes made
- `lumberknot.rs`: `SelfDies` declared beside `AnyCreatureDies`, and `on_dies` implemented.
- `cards_death_triggers_and_tokens.rs`: two new tests.
