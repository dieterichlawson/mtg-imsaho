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
