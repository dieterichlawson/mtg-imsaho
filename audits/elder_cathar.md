## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/12/elder-cathar?utm_source=api
**Type line**: `Creature — Human Soldier` — {2}{W}, 2/2
**Oracle text**:
```
When this creature dies, put a +1/+1 counter on target creature you control. If that creature is a Human, put two +1/+1 counters on it instead.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "put a +1/+1 counter on **target creature you control**" — targeted at
  CR 603.3d time, so it is chosen when the death trigger goes on the stack: PASS
- "**If that creature is a Human**, put two +1/+1 counters on it **instead**" —
  two, not one plus one, and the check runs at resolution so a creature that
  became a Human in between gets two: PASS
- `has_subtype` covers granted and token subtypes: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both counter amounts: `cards_morbid_and_ltb.rs`, `subtype.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/12/elder-cathar?utm_source=api
**Type line**: `Creature — Human Soldier` — {2}{W}, 2/2
**Oracle text**:
```
When this creature dies, put a +1/+1 counter on target creature you control. If that creature is a Human, put two +1/+1 counters on it instead.
```

**Rulings fetched**: none published for this card.

**Status**: PASS


Rulings: none are cached for this card and none surfaced in search; a plain
common with one death trigger plausibly has none on Scryfall. The oracle text
above was confirmed against a second independent source (Gatherer/search
result) and matched word for word.

### Code issues
No issues found in behaviour. Two comments described a self-exclusion filter
that does not exist in the code, and both were corrected (see below).

- Card data matches the fetched type line and text exactly: {2}{W}, Creature —
  Human Soldier, 2/2, no keywords.
- The target is declared on the `TriggeredAbilityDef`, so the engine picks it
  when the trigger goes on the stack (CR 603.3d) and re-checks it on the way
  down (CR 608.2b). `on_dies` reads `chosen_targets` and never re-selects.
- "If that creature is a Human" is evaluated inside the resolution handler via
  `state.has_subtype`, which reads the *active* face — so a transformed
  werewolf is correctly not a Human (CR 712.8d).
- `add_counters` is the shared pipeline, with its CR 121.1 battlefield guard.
- `caster` in `is_valid_target` is the trigger's controller, which the
  collector takes from the `CreatureDied` event — the Cathar's last known
  controller, per CR 608.2g. This matters because leaving the battlefield
  resets the object's own `controller` field to its owner (CR 400.7), so a
  card that re-derived "you" from the source here would answer with the owner.

### Comment corrections
- `elder_cathar.rs:49` said `is_valid_target` would "exclude the dying Cathar".
  It contains no such clause and never has. The exclusion is real but comes
  from the zone check: the Cathar is already in the graveyard when its own
  death trigger picks targets. Reworded, and the CR 608.2g/400.7 reasoning
  behind `caster` written down alongside it.
- `subtype.rs:583` pointed at "the `o.id != object_id` filter at line 41" as
  the reason the Cathar cannot target itself. There is no such filter at line
  41 or anywhere else. Same correction.

Neither is a behaviour change, but a comment that names a specific line and a
specific filter is the kind a later reader trusts instead of checking, and the
next person to touch `is_valid_target` could have "restored" a self-exclusion
that was never there — or, worse, removed the zone check believing the id
filter was doing that work.

### Tricky interactions checked
- Human bonus on a Human: pass
- No bonus on a non-Human: pass
- No bonus on a transformed werewolf whose live face is not Human: pass
- The Human check is made at resolution, not when the target was chosen: pass
- "target creature you control" excludes an opponent's creatures: pass
- "you" is the last known controller, not the owner (CR 608.2g / 400.7): pass
- Target changes controller in response — the ability is countered by game
  rules and nothing happens (CR 608.2b): pass
- The dying Cathar is not a legal target for its own trigger: pass, by the
  zone check rather than an id filter

### Test coverage
- Human gets two counters: `cards_morbid_and_ltb.rs:549`
- Non-Human gets one: `cards_morbid_and_ltb.rs:569`
- Transformed werewolf gets one (live face, not the front face):
  `subtype.rs:567`
- **NEW** "you" is the last known controller, not the owner — the Cathar is
  given a different owner so the two answers differ:
  `cards_morbid_and_ltb.rs:588`
- **NEW** the Human check is made on resolution — the target is locked in as a
  Human and transformed in response, which only a resolution-time check gets
  right: `cards_morbid_and_ltb.rs:617`
- **NEW** target changes controller in response, so the ability is countered
  and nothing about either permanent changes:
  `trigger_target_recheck.rs:186` (added to the six-ability sweep)

### Sweep widened
`trigger_target_recheck.rs::a_trigger_whose_target_became_illegal_changes_nothing`
covered six ways for a target to become illegal — leaving the zone, gaining
hexproof, and gaining a forbidden subtype — but every one of its targets
started under the *opponent's* control, so no case could exercise a change of
control. "Target creature you control" is the restriction that a control
change breaks, and Elder Cathar is the card that has it. The case tuple now
carries the controller the target starts under; the five existing cases keep
theirs (P1) unchanged.

### Not changed, and why
Nothing. The card needed no behavioural fix.

