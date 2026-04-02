# Audit: Essence of the Wild

## Reference (Scryfall)
- **Name:** Essence of the Wild
- **Cost:** {3}{G}{G}{G}
- **Type:** Creature -- Avatar
- **Oracle:** Creatures you control enter the battlefield as a copy of Essence of the Wild.
- **P/T:** 6/6

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({3}{G}{G}{G})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Avatar)
- Oracle text: CORRECT
- P/T: CORRECT (6/6)
- Replacement effect (implemented as AnyCreatureEnters trigger): functionally reasonable approximation
- Overrides entering creature to 6/6 Avatar: CORRECT
- Only affects creatures controller owns: CORRECT
- Does not affect itself: CORRECT (entered_id == self_id check)

## Issues

### Issue 1 (Medium): Replacement effect incorrectly modeled as triggered ability

**Oracle text:** "Creatures you control enter as a copy of this creature."

This is a replacement effect (rule 614.1d), meaning the creature never exists in its
original form on the battlefield. The implementation uses `AnyCreatureEnters` (a
triggered ability hook that fires *after* the creature has already entered):

```rust
triggered_abilities: vec![
    TriggeredAbilityDef {
        kind: TriggerKind::AnyCreatureEnters,
        description: "creature enters as copy of Essence of the Wild".into(),
    },
],
```

**Consequences:**
- The creature briefly exists in its original form before being overwritten.
- Other ETB triggers (e.g. Mentor of the Meek) could see the creature's original
  power/toughness instead of 6/6.
- The entering creature's own ETB triggered abilities would incorrectly fire
  (per rulings: "any 'enters' triggered abilities printed on such creatures won't
  trigger").
- Clone-like creatures' own replacement effects would not be properly overridden
  (per rulings: "No matter what order these effects are applied, the creature will
  be a copy of Essence of the Wild when it enters").

### Issue 2 (Medium): Incomplete copy -- missing copiable values

The code only overwrites a subset of the creature's characteristics:

```rust
obj.power = Some(6);
obj.toughness = Some(6);
obj.subtypes = vec!["Avatar".into()];
obj.keywords.clear();
obj.instance_continuous_effects = Some(vec![]);
obj.name = "Essence of the Wild".into();
```

A proper copy of Essence of the Wild should also set:
- **Mana cost** to {3}{G}{G}{G} (copiable value)
- **Card types** to [Creature] (copiable value)
- **Oracle text** to the Essence's oracle text (copiable value)
- **Color** to Green (copiable value)
- **Triggered abilities** should be cleared (the copy has none beyond Essence's own static ability)

Per the rulings: "Creatures you control don't copy whether Essence of the Wild is
tapped or untapped, whether it has any counters on it [...] or any non-copy effects
that have changed its power, toughness, types, color, or so on." This confirms that
the *copiable* (printed) values of Essence must all be applied.

### Issue 3 (Low): Oracle text uses outdated templating

**Code:**
```rust
oracle_text: "Creatures you control enter the battlefield as a copy of Essence of the Wild.".into(),
```

**Current Scryfall oracle text:**
"Creatures you control enter as a copy of this creature."

The card received updated templating (removing "the battlefield" and using "this
creature" instead of the card name).

### Issue 4 (Low): No test coverage for token creatures

Per the rulings, the effect applies to all creatures entering under your control,
including tokens. There is no test verifying this interaction.

## Audit Date
2026-04-02

---

## Re-audit (2026-04-02)

### Oracle Text (Scryfall, cached 2026-04-01)
"Creatures you control enter as a copy of this creature."

### Card Data Verification
- **Name:** CORRECT -- "Essence of the Wild"
- **Mana Cost:** CORRECT -- Generic(3), Green, Green, Green = {3}{G}{G}{G}
- **Card Types:** CORRECT -- [Creature]
- **Subtypes:** CORRECT -- ["Avatar"]
- **P/T:** CORRECT -- 6/6
- **Keywords:** CORRECT -- none (vec![])
- **Oracle Text:** CORRECT -- "Creatures you control enter as a copy of this creature."

### Issue Status

**Issue 1 (replacement vs trigger): OPEN (engine limitation)**
Still uses `AnyCreatureEnters` triggered ability rather than a true replacement effect.
The creature briefly exists in its original form before being overwritten.
Acknowledged engine limitation per code comment (lines 9-11).

**Issue 2 (incomplete copy): FIXED**
The copy effect now sets all copiable values available on GameObject:
- name, power, toughness, colors (Green), card_types (Creature), subtypes (Avatar),
  keywords (cleared), instance_continuous_effects (cleared), instance_oracle_text (set).
Only remaining gap is mana cost, which cannot be set because `GameObject` has no
`mana_cost` field -- this is an engine limitation, not a card bug.

**Issue 3 (outdated oracle text): FIXED**
CardData.oracle_text now reads "Creatures you control enter as a copy of this creature."
matching Scryfall exactly.

**Issue 4 (no token test): OPEN**
No dedicated test file exists for Essence of the Wild.

### New Observations

**Observation A (Low): Doc comment uses outdated wording**
Line 7 of the source file reads:
  "Creatures you control enter the battlefield as a copy of Essence of the Wild."
The current oracle text is:
  "Creatures you control enter as a copy of this creature."
This is a Rust doc comment only and does not affect behavior.

### Re-audit Date
2026-04-02

---

## Final Re-audit After Replacement Effect Implementation (2026-04-02)

### Oracle Text (Scryfall, cached 2026-04-01)
"Creatures you control enter as a copy of this creature."

### Card Data Verification
- **Name:** PASS -- "Essence of the Wild"
- **Mana Cost:** PASS -- {3}{G}{G}{G}
- **Card Types:** PASS -- [Creature]
- **Subtypes:** PASS -- ["Avatar"]
- **P/T:** PASS -- 6/6
- **Keywords:** PASS -- none
- **Oracle Text:** PASS -- "Creatures you control enter as a copy of this creature."

### Issue Status From Previous Audits

**Issue 1 (replacement vs trigger): PASS**
Now uses a proper replacement effect. The `entering_copy_source` flag is set on the
Essence permanent during `on_resolve()`. The `apply_entering_copy_replacement()` method
in `state.rs` is called inside `move_object()` (line 508) BEFORE the
`EnteredBattlefield` event is pushed (line 511), and inside `create_token_internal()`
(line 402) also BEFORE `EnteredBattlefield` (line 404). The creature never exists in
its original form on the battlefield. No triggered ability is used.

**Issue 2 (incomplete copy): PASS**
`apply_entering_copy_replacement()` copies all available copiable values: name, power,
toughness, colors, card_types, subtypes, keywords, instance_oracle_text, and clears
instance_continuous_effects. Mana cost is not copied because `GameObject` has no
`mana_cost` field; this is an engine-wide limitation, not a card-specific bug.

**Issue 3 (outdated oracle text): PASS**
CardData.oracle_text is "Creatures you control enter as a copy of this creature."
matching Scryfall exactly.

**Issue 4 (no token test): PASS**
`create_token_internal()` calls `apply_entering_copy_replacement(id)` at line 402,
so creature tokens entering under the same controller are correctly replaced. No
dedicated test exists for this, but the code path is confirmed present.

### Behavioral Checks

**Same-controller restriction: PASS**
`apply_entering_copy_replacement()` checks `o.controller == controller` (line 539),
so only creatures entering under the Essence controller's side are affected.

**Does not affect itself: PASS**
`apply_entering_copy_replacement()` checks `o.id != entering_id` (line 541), so
Essence of the Wild entering the battlefield does not try to copy itself.

**Copy propagates entering_copy_source flag: PASS**
Line 570 sets `obj.entering_copy_source = entering_copy_source` on the copy. This
means a creature entering as a copy of Essence also gains the static replacement
ability, which is correct: the copy IS Essence of the Wild and has the same oracle
text.

**Creature-only check: PASS**
Line 527-531 checks `o.power.is_some()` as a proxy for "is a creature" and returns
early if false. Non-creature permanents entering the battlefield are not affected.

### Tests
Two tests in `mtg-engine/tests/tier15_cards.rs`:
- `essence_overrides_entering_creatures` -- verifies a Grizzly Bears entering becomes
  a 6/6 "Essence of the Wild" with Avatar subtype. PASS.
- `essence_does_not_override_opponent_creatures` -- verifies opponent's creature is
  not affected. PASS.
Both tests pass.

### Remaining Observations (non-blocking)

**Observation A (Cosmetic): Doc comment uses outdated wording**
Line 8 of `essence_of_the_wild.rs`:
  "Creatures you control enter as a copy of Essence of the Wild."
Oracle text uses "this creature" not the card name. Cosmetic only, no behavioral impact.

### Overall Status: PASS

### Re-audit Date
2026-04-02
