## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/149/instigator-gang-wildblood-pack?utm_source=api
**Type line**: `Creature — Human Werewolf` — {3}{R}, 2/3
**Oracle text**:
```
Attacking creatures you control get +1/+0.
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Wildblood Pack — `Creature — Werewolf`, 5/5
```
Trample
Attacking creatures you control get +3/+0.
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
- "Attacking creatures you control get +1/+0" (front) and "+3/+0" (back) are
  `Global(ControlledByYou + Attacking)`, not `GlobalOther` — the Gang itself gets
  the bonus when it attacks, which is what the wording says.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`werewolf_cards.rs` (29 tests over the family), `intervening_if.rs` (the spell-count condition), `transform_dfc.rs` (CR 712 face characteristics).
## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/149/instigator-gang-wildblood-pack?utm_source=api
**Type line**: `Creature — Human Werewolf` — {3}{R}, 2/3
**Oracle text**:
```
Attacking creatures you control get +1/+0.
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Wildblood Pack — `Creature — Werewolf`, 5/5
```
Trample
Attacking creatures you control get +3/+0.
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**Attacking** creatures you control get +1/+0" — `EffectScope::Global` (not
  `GlobalOther`), so the Gang attacking gets its own bonus, which is what
  "attacking creatures you control" means: PASS
- The bonus is a *static* ability, so it applies to a creature that becomes an
  attacker later in the same combat and stops the moment the Gang leaves — not
  an until-end-of-turn anthem that outlives its source (CR 611.2c): PASS
- Trample is on the **back** face only. Scryfall's `Keywords` list merges both
  faces, so the front face correctly has no trample in its card data: PASS
- +1/+0 on the front, +3/+0 on the back: PASS
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
- The attacking-creatures anthem and its static scope: `snapshot_anthems.rs:a_static_anthem_stops_when_its_source_leaves`, `werewolf_cards.rs`
