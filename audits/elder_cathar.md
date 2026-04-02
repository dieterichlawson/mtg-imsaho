# Audit: Elder Cathar

## Reference (Scryfall)
- **Name:** Elder Cathar
- **Cost:** {2}{W}
- **Type:** Creature -- Human Soldier
- **Oracle:** When Elder Cathar dies, put a +1/+1 counter on target creature you control. If that creature is a Human, put two +1/+1 counters on it instead.
- **P/T:** 2/2

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({2}{W})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Human, Soldier)
- Oracle text: CORRECT
- P/T: CORRECT (2/2)
- Dies trigger: CORRECT (TriggerKind::SelfDies)
- +1/+1 counter on target creature: CORRECT
- Human bonus (2 counters instead of 1): CORRECT
- Targets creature you control: CORRECT (filters by controller)

## Issues

### BUG (minor): Human subtype check ignores obj.subtypes (affects tokens)

Both the single-target path in `on_dies` (elder_cathar.rs lines 51-52) and the multi-target path in `PendingEffect::AddCounters` (engine.rs lines 2022-2025) check Human status via registry only:

```
registry.card_data(o.card_id).map(|d| d.subtypes.iter().any(|s| s == "Human"))
```

They do NOT check `obj.subtypes` on the game object. For normal cards this is fine because subtypes come from the registry. However, for **tokens** whose subtypes are stored on `obj.subtypes` (not in registry), a Human token would incorrectly receive only 1 counter instead of 2.

Compare with `combat.rs` `get_subtypes()` (line 356-369) which correctly merges both `obj.subtypes` and `registry.card_data().subtypes`.

No test currently covers this case (Human token receiving counters from Elder Cathar's death trigger).
