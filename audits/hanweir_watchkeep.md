## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/145/hanweir-watchkeep-bane-of-hanweir?utm_source=api
**Type line**: `Creature — Human Warrior Werewolf` — {2}{R}, 1/5
**Oracle text**:
```
Defender
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Bane of Hanweir — `Creature — Werewolf`, 5/5
```
This creature attacks each combat if able.
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
- Bane of Hanweir's "This creature attacks each combat if able" is
  `ContinuousEffect::ForceAttack { OnSelf }`. The "if able" half is the engine's:
  `combat_rules.rs` covers a forced attacker that a "can't attack" effect makes
  unable.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`werewolf_cards.rs` (29 tests over the family), `intervening_if.rs` (the spell-count condition), `transform_dfc.rs` (CR 712 face characteristics).
## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/145/hanweir-watchkeep-bane-of-hanweir?utm_source=api
**Type line**: `Creature — Human Warrior Werewolf` — {2}{R}, 1/5
**Oracle text**:
```
Defender
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Bane of Hanweir — `Creature — Werewolf`, 5/5
```
This creature attacks each combat if able.
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Defender is on the **front** face only, and the back face's "This creature
  attacks each combat if able" is `ForceAttack` scoped `OnSelf` — so the two
  never contradict each other on one face: PASS
- CR 508.1d: a requirement cannot force an illegal attack, so a tapped or
  summoning-sick Bane of Hanweir does not attack: PASS
- The force is on itself, not on other creatures: PASS
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
- Defender, the flip, and the attack requirement: `werewolf_cards.rs`, `combat_requirements.rs`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/145/hanweir-watchkeep-bane-of-hanweir?utm_source=api
**Type line**: `Creature — Human Warrior Werewolf` — {2}{R}, 1/5
**Oracle text**:
```
Defender
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Bane of Hanweir — `Creature — Werewolf`, 5/5
```
This creature attacks each combat if able.
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```

**Rulings fetched**:
- [2016-07-13] For more information on double-faced cards, see the Shadows over Innistrad mechanics article (http://magic.wizards.com/en/articles/archive/feature/shadows-over-innistrad-mechanics).

**Status**: PASS

### Code issues

No issues found. Both faces match the fetched text, and the one ruling is the generic double-faced-card article pointer with nothing card-specific to check.

### Checked and correct

- Front: `{2}{R}`, `Creature — Human Warrior Werewolf`, 1/5, `keywords: [Defender]`. All three subtypes present.
- Back: `Bane of Hanweir`, `Creature — Werewolf`, 5/5, **no** keywords. Correct: "This creature attacks each combat if able" is not a keyword ability, and Scryfall's keyword list for the card is `Transform, Defender` — Defender belonging to the front face only.
- `This creature attacks each combat if able` is `ContinuousEffect::ForceAttack { scope: EffectScope::OnSelf }`. `OnSelf` is right: the Bane forces itself and nothing else.
- Defender is lost on transforming and the force is gained, because `has_keyword` and `continuous_effects_of` both read the active face.
- Both faces declare their upkeep trigger, at the default `TriggerScope::Each` — "At the beginning of **each** upkeep".
- The transform goes through the shared werewolf mechanism: `werewolf_should_trigger` (which suppresses the trigger entirely for a token copy), `werewolf_should_transform` (which reads the face the ability triggered from, CR 603.4 + CR 712.8), and `werewolf_on_upkeep`.
- Oracle text on both faces matches verbatim, including the "this creature" templating on both.

### The "if able" clause, checked in the engine

CR 508.1d requires a creature to attack only if it is able, and both paths that act on `must_attack` respect that:

- The prompt's `must_attack` list is filtered out of `eligible_attackers`, which already excludes tapped, summoning-sick, Defender and "can't attack" creatures.
- The auto-forcing path in `engine/actions/combat.rs` re-checks the same four conditions before dragging a creature into combat.

So a summoning-sick or tapped Bane is not forced, and a Defender creature could not be forced even if something else tried.

### Tricky interactions checked

- The Bane attacks when its controller declares no attackers: PASS.
- The force does not reach other creatures the controller has: PASS (`OnSelf`).
- Summoning-sick Bane: not forced. PASS.
- Tapped Bane: not forced. PASS.
- Front face behind Defender: not an eligible attacker, and declaring it does not make it one. PASS.
- Token copy: cannot transform, and the trigger never reaches the stack. PASS (set-wide werewolf coverage).
- Transform while the front face is blocked/blocking: not applicable — the front face has Defender and cannot attack, and gaining "attacks each combat" mid-combat does not retroactively declare an attack.

### Test coverage

- loses Defender and gains the force on transforming: `werewolf_cards.rs:173`
- intervening-if on the upkeep trigger: `intervening_if.rs:130`
- trigger snapshot: `trigger_snapshots.rs:130`
- the Bane is an attacker even when none are declared, and a bystander is not: `werewolf_cards.rs` `bane_of_hanweir_attacks_whether_you_declare_it_or_not` (NEW, mutation-checked against a mis-scoped force)
- summoning-sick and tapped Banes are not forced: `werewolf_cards.rs` `bane_of_hanweir_is_not_forced_when_it_cannot_attack` (NEW, mutation-checked against dropping the ability checks)
- the front face cannot attack behind Defender: `werewolf_cards.rs` `hanweir_watchkeep_cannot_attack_behind_its_defender` (NEW)

