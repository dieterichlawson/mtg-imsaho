# Audit: Spare from Evil

## Oracle (Scryfall)
- **Name:** Spare from Evil
- **Cost:** {1}{W}
- **Type:** Instant
- **Oracle:** Creatures you control gain protection from non-Human creatures until end of turn.
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/spare_from_evil.rs`
- **Name:** Spare from Evil ✅
- **Cost:** {1}{W} ✅
- **Type:** Instant ✅
- **on_resolve:** collects all creatures controlled by caster ✅
- **Protection filter:** Not(HasSubtype("Human")) -- protection from non-Human creatures ✅
- **Until end of turn:** uses `until_end_of_turn_protection` ✅
- **Spell cleanup:** move_spell_after_resolve ✅

### Note
- Only grants protection to creatures on the battlefield at time of resolution, not to creatures that enter later. This matches standard MTG rules for one-shot effects.

## Verdict: PASS -- no issues found

## Audit — 2026-04-02

**Oracle Text:**
> Creatures you control gain protection from non-Human creatures until end of turn.

**Card Data:**
- Name: Spare from Evil — correct
- Cost: {1}{W} — correct
- Type: Instant — correct

**Behavior:**
- Collects all creatures controlled by caster on the battlefield — correct
- Grants protection from non-Human creatures using CreatureFilter::Not(HasSubtype("Human")) — correct
- Protection stored in until_end_of_turn_protection — correct
- Moves spell to graveyard after resolve — correct

**Result: PASS**
