# Audit: Bramblecrush

## Oracle Reference
- **Name:** Bramblecrush
- **Mana Cost:** {2}{G}{G}
- **Type:** Sorcery
- **Oracle Text:** Destroy target noncreature permanent.

## Card Data Audit
- **Name:** Correct ("Bramblecrush")
- **Mana Cost:** Correct (Generic(2), Green, Green)
- **Type:** Correct (Sorcery)
- **Subtypes:** Correct (none)
- **P/T:** Correct (None)

## Behavior Audit
- **Targeting:** `TargetRequirement::PermanentWithFilter(TargetFilter::Noncreature)`. Correct.
- **is_valid_target:** Checks target is on battlefield and not a Creature type. Correct.
- **on_resolve:** Uses `resolve_destroy` helper which checks indestructible/regeneration. Correct.

## Result: PASS
