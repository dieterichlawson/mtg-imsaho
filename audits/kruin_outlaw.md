## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/152/kruin-outlaw-terror-of-kruin-pass?utm_source=api
**Type line**: `Creature — Human Rogue Werewolf` — {1}{R}{R}, 2/2
**Oracle text**:
```
First strike
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Terror of Kruin Pass — `Creature — Werewolf`, 3/3
```
Double strike
Werewolves you control have menace. (A creature with menace can't be blocked except by two or more creatures.)
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```

**Status**: PASS

### Code issues
No issues found.

- The transform pair is the set's shared werewolf mechanism, and it turns on a
  distinction the wording makes precisely:
  - front → back, "if **no spells were cast** last turn":
    `num_spells_cast_last_turn.values().sum::<u32>() == 0` — nobody cast anything.
  - back → front, "if **a player** cast two or more spells last turn":
    `.values().any(|&count| count >= 2)` — one player individually reached two,
    not the table's total.
  Summing for the second, or requiring a single caster for the first, would both
  be wrong; neither is done.
- Both are intervening-if clauses (CR 603.4), and the card overrides
  `should_trigger` through `helpers::werewolf_should_trigger`, so with the
  condition unmet nothing reaches the stack.
- That helper also refuses the trigger for a token copy, which cannot transform
  (CR 111.7) — so it does not sit on the stack with nothing to do.
- Back-face P/T comes from `dynamic_pt`, which supplies the *base* in
  `effective_power`, leaving counters and anthems to layer on top.
- "Werewolves you control have menace" on the back face is
  `Global(ControlledByYou + HasSubtype("Werewolf"))`, so it reaches the Terror
  itself as well as your other Werewolves.
- Menace is on the card by errata; the printing this set shipped with read
  "can't be blocked except by two or more creatures". The current oracle grants
  menace, so `Keyword::Menace` is correct here and is not the step-9
  invented-menace anti-pattern.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`werewolf_cards.rs` (29 tests over the family), `intervening_if.rs` (the spell-count condition), `transform_dfc.rs` (CR 712 face characteristics).
## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/152/kruin-outlaw-terror-of-kruin-pass?utm_source=api
**Type line**: `Creature — Human Rogue Werewolf` — {1}{R}{R}, 2/2
**Oracle text**:
```
First strike
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Terror of Kruin Pass — `Creature — Werewolf`, 3/3
```
Double strike
Werewolves you control have menace. (A creature with menace can't be blocked except by two or more creatures.)
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "If Kruin Outlaw somehow transforms **after blockers have been
  declared** but before combat ends, any Werewolves you control that are blocked
  by a single creature **will remain blocked**." CR 509.2 makes blocked-ness
  permanent for the combat, so menace arriving mid-combat unblocks nothing:
  PASS
- "Werewolves you control have menace" — `EffectScope::Global`, so the Terror
  grants it to itself as well: PASS
- Menace here is *printed on the back face by errata*, not invented for an
  effect that needs two blockers — the current oracle text grants it: PASS
- First strike on the front, **double** strike on the back, and they do not
  stack into two extra damage steps: PASS
- The werewolf flip conditions are the shared `werewolf_should_trigger` /
  `werewolf_should_transform` helpers, so "if no spells were cast last turn" and
  "if a player cast two or more spells last turn" are one implementation rather
  than one per card: PASS
- CR 603.4: both are intervening-ifs, checked when the trigger would go on the
  stack *and* again on resolution: PASS
- "At the beginning of **each** upkeep" — `TriggerScope::Each`, so it fires on
  the opponent's turn too: PASS
- The active face's characteristics come from `back_face_data` when transformed
  (CR 712.8) — P/T, keywords, subtypes, continuous effects and triggered
  abilities all switch together: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The menace grant and the strike keywords: `werewolf_cards.rs`, `evasion.rs`, `combat_rules.rs`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/152/kruin-outlaw-terror-of-kruin-pass?utm_source=api
**Type line**: `Creature — Human Rogue Werewolf` — {1}{R}{R}, 2/2
**Oracle text**:
```
First strike
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Terror of Kruin Pass — `Creature — Werewolf`, 3/3
```
Double strike
Werewolves you control have menace. (A creature with menace can't be blocked except by two or more creatures.)
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```

**Rulings fetched**:
- [2011-09-22] If Kruin Outlaw somehow transforms after blockers have been declared but before combat ends, any Werewolves you control that are blocked by a single creature will remain blocked.

**Status**: PASS

### Code issues

No issues in the card. Both faces match the fetched text and the one ruling holds.

**Found while checking the ruling**, in the two modules the audit walked through: `src/combat.rs` and `src/sba.rs` were the last places scanning the object map in HashMap order, and both scans are order-observable. `eligible_attackers` and `eligible_blockers` are the lists a player declares combat from, by position. State-based actions collect the dying creatures in scan order, and that is the order their `CreatureDied` events — and so their death triggers — reach the stack; worse, the state-trigger pass `break`s at the **first** candidate whose condition holds, so map order decided which of several ready state triggers fired. Both swept, and the guard now covers all of `src` except `state.rs` (where the sorted accessors are built, and where the genuinely order-free walks live).

### Checked against the ruling

- `If Kruin Outlaw somehow transforms after blockers have been declared but before combat ends, any Werewolves you control that are blocked by a single creature will remain blocked.` — PASS, and structurally so: the minimum-blocker count is applied in `declare_blockers_with_registry`, at declaration only, and `declare_blockers` records the attacker in `blocked_attackers` (CR 509.2), which nothing later re-examines. Now tested, mutation-checked against a transform that re-validates the count.

### Checked and correct

- Front: `{1}{R}{R}`, `Creature — Human Rogue Werewolf`, 2/2, `keywords: [FirstStrike]`. All three subtypes present.
- Back: `Terror of Kruin Pass`, `Creature — Werewolf`, 3/3, `keywords: [DoubleStrike]`. First strike is **not** carried over — it belongs to the front face only, and `has_keyword` reads the active face.
- `Werewolves you control have menace` is `GrantKeyword { keyword: Menace, scope: Global(And([ControlledByYou, HasSubtype("Werewolf")])) }`. `Global` rather than `GlobalOther` is right: the Terror is itself a Werewolf its controller controls, so it has menace too.
- Scryfall's `keywords` list for the card is `Transform, First strike, Double strike` — menace is absent there because the Terror *grants* it rather than printing it, which is exactly how the code models it (a continuous effect, not a `keywords` entry).
- The grant is `ControlledByYou`, so an opponent's Werewolf gets nothing — the Terror is not helping them.
- The upkeep trigger is the shared werewolf mechanism: `werewolf_should_trigger` (which suppresses it entirely for a token copy), `werewolf_should_transform` (which reads the face the ability triggered from, CR 603.4 + CR 712.8), and `werewolf_on_upkeep`.
- Both faces declare the upkeep trigger, and `TriggerScope` is the default `Each` — "At the beginning of **each** upkeep".

### Tricky interactions checked

- Transform after blockers declared: block stands. PASS.
- Menace applies to the Terror itself: PASS.
- Menace does not reach your non-Werewolves or the opponent's Werewolves: PASS.
- Two blockers are enough — "except by two or more", not "can't be blocked": PASS.
- First strike is lost on transforming, double strike gained: PASS.
- Menace is visible through `has_keyword`, not only inside blocker validation: PASS.

### Test coverage

- transforms and gains double strike: `werewolf_cards.rs:551`
- loses first strike on transforming: `werewolf_subtype_after_transform.rs:47`
- menace reaches your Werewolves only: `werewolf_cards.rs:733`
- the Terror needs two blockers itself, and two suffice: `werewolf_cards.rs:761`
- menace shows up in `has_keyword`: `werewolf_cards.rs:779`
- intervening-if on the upkeep trigger: `intervening_if.rs:130`
- trigger snapshot: `trigger_snapshots.rs:132`
- transform mid-combat leaves a declared block alone: `werewolf_cards.rs` `transforming_after_blockers_leaves_a_single_blocker_in_place` (NEW, mutation-checked)

