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
## Full audit — 2026-08-27

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

**Rulings fetched**:
- [2016-07-13] For more information on double-faced cards, see the Shadows over Innistrad mechanics article (http://magic.wizards.com/en/articles/archive/feature/shadows-over-innistrad-mechanics).

**Status**: ISSUE (fixed)

### Code issues

Three found, all fixed in this pass. The card's own rules behaviour was correct; every issue was in how it participates in shared machinery.

1. **Back-face oracle text used the pre-errata wording.** `instigator_gang.rs:59`
   - Oracle (back face) says: `At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.`
   - Code had: `"...transform Wildblood Pack."` — the front face had already been updated to "transform this creature" and the back face had not.
   - Now matches the fetched text exactly.

2. **A hand-rolled `on_upkeep` that hardcoded both face names.** `instigator_gang.rs:91` (before the fix)
   - Code did: `let (old_name, new_name) = if was_transformed { ("Wildblood Pack", "Instigator Gang") } else { ("Instigator Gang", "Wildblood Pack") };`
   - Eleven other werewolves have a byte-identical `on_upkeep` that reads the names off the object; Instigator Gang's is the one that drifted. Twelve copies of one behaviour is the shape the engine is supposed to avoid.
   - Now: `helpers::werewolf_on_upkeep(self, state, self_id, registry)`, shared by all twelve.

3. **The transform log claimed transforms that did not happen.** All 19 sites around `helpers::apply_transform`
   - `apply_transform` refuses to flip a token copy of a double-faced card (CR 111.7). Every card that logged the flip around that call announced it regardless, so a token copy of Reckless Waif logged `"Reckless Waif transforms into Merciless Predator"` and stayed a Waif. Screeching Bat's line was `format!("Transforms into {}", ...)`, naming neither the permanent nor its controller.
   - Now `apply_transform` logs it, on the path that actually flips. `card_data_invariants::no_card_announces_its_own_transform` keeps it there.

**Found while doing this: Moonmist bypassed the transform operation entirely** (`moonmist.rs:45-74`, before the fix). It flipped `is_transformed` by hand and then `clone_from`ed the new face's `name`/`power`/`toughness`/`keywords`/`subtypes` onto the object. Those object vectors hold runtime *grants*, not printed characteristics, so the copy threw away everything granted to that permanent — an Olivia-bitten Human stopped being a Vampire at the moment Moonmist flipped it (CR 712.8a: transforming does not make a new object, so a type-changing effect that applied still applies) — and pinned its P/T against later effects. It also skipped the CR 111.7 token refusal. Fixed by calling `apply_transform`; the existing Olivia/Moonmist test now asserts the bite survives, and that assertion is mutation-checked.

### Checked and correct

- Cost `{3}{R}`, type `Creature — Human Werewolf`, 2/3; back face `Creature — Werewolf`, 5/5 with Trample. All match. The front face correctly declares **no** keywords — Scryfall's `keywords` field aggregates both faces, and Trample is printed only on Wildblood Pack.
- `Attacking creatures you control get +1/+0` is a `ContinuousEffect::ModifyPT` scoped to `Global(And([ControlledByYou, Attacking]))`, and the back face's is the same with `power: 3`. `Global`, not `GlobalOther`: "attacking creatures you control" includes the Gang itself when it attacks.
- The two faces' anthems do not stack: `continuous_effects_of` reads `face_data`, i.e. the active face only, so a transformed Gang gives +3/+0 and not +4/+0.
- The upkeep trigger is `TriggerScope::Each` (the default) — "At the beginning of **each** upkeep", not "your upkeep".
- `werewolf_should_trigger` suppresses the trigger entirely for a token copy, so no phantom stack entry.
- `werewolf_should_transform` reads the face the ability triggered *from*, not the face that is up now (CR 603.4 + CR 712.8), which is what makes the Moonmist-in-response case resolve correctly.

### Tricky interactions checked

- Anthem persists after the Gang transforms, at the back face's value: PASS.
- Anthem ends when combat ends, not at end of turn: PASS.
- Creature put onto the battlefield attacking gets the bonus: PASS.
- Gang arriving after attackers are declared buffs those already attacking: PASS.
- Anthem stops when the Gang leaves: PASS.
- Opponent's attackers get nothing: PASS.
- Token copy cannot transform, and no longer says it did: PASS.
- Moonmist in response to the upkeep trigger: PASS.

### Test coverage

- anthem on both faces: `attacking_creatures_anthem.rs:25`
- only attacking creatures you control: `attacking_creatures_anthem.rs:49`
- bonus ends with combat, not with the turn: `attacking_creatures_anthem.rs:70`
- creature put onto the battlefield attacking: `attacking_creatures_anthem.rs:94`
- Gang arriving mid-combat: `attacking_creatures_anthem.rs:114`
- bonus stops when the Gang leaves: `attacking_creatures_anthem.rs:133`
- transforms and gains Trample: `werewolf_cards.rs:385`
- buffs itself when attacking: `werewolf_cards.rs:401`
- buffs other attackers you control: `werewolf_cards.rs:417`
- does not buff the opponent's attackers: `werewolf_cards.rs:435`
- Wildblood Pack's +3/+0: `werewolf_cards.rs:450`
- Moonmist in response to the upkeep trigger: `werewolf_cards.rs:834`
- token copy neither transforms nor announces one: `zone_change_resets_object.rs` `a_token_copy_of_a_werewolf_cannot_transform` (log assertion NEW)
- granted subtype survives Moonmist: `subtype.rs` `bug_au_moonmist_transforms_olivia_bitten_human_dfc` (assertion NEW, mutation-checked)
- no card announces its own transform: `card_data_invariants.rs` `no_card_announces_its_own_transform` (NEW)

### Rulings

One ruling in the Scryfall-sourced cache (2016-07-13), a pointer to the Shadows over Innistrad double-faced-card mechanics article; nothing card-specific to check against.


## Follow-up — 2026-08-28 — back-face colour indicator established

**Colour source**: external, fetched this session — a web search over the Scryfall and mtg.wtf results returned that "Wildblood Pack has a color indicator of Red and is a Creature — Werewolf with Trample". Not from memory.
**Status**: ISSUE (fixed)

### Code issue
- CR 204.2: a back face has no mana cost, so its colour comes from the printed
  colour indicator. `back_face_data` declared none, so a transformed permanent
  was **colourless** — it dodged protection, intimidate, and every
  "non-colour" filter in the set. This was the class opened under Gatstaf
  Shepherd; this card's full audit predated the colour-indicator sweep, and
  `audits/BACK_FACE_COLORS.md` carried it as "not yet established" until now.
- Fixed: `color_indicator: vec![Color::Red]` on the back face (Wildblood Pack is red).

### Test coverage
- The colour is pinned, with the other nineteen back faces, by
  `card_data_invariants.rs::every_back_face_declares_the_colour_its_indicator_prints`,
  whose table also fails the build on any declared back face it does not name.
  Mutation-checked by emptying Ironfang's indicator, which fails the sweep by
  name.
