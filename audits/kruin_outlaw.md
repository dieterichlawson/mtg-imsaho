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
