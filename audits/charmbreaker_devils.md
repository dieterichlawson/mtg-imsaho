# Audit: Charmbreaker Devils

## Scryfall Reference
- **Name:** Charmbreaker Devils
- **Cost:** {5}{R}
- **Type:** Creature -- Devil
- **Oracle:** At the beginning of your upkeep, return an instant or sorcery card at random from your graveyard to your hand. Whenever you cast an instant or sorcery spell, this creature gets +4/+0 until end of turn.
- **P/T:** 4/4
- **Keywords:** none

## Implementation: `charmbreaker_devils.rs`
- **Name:** Charmbreaker Devils -- CORRECT
- **Cost:** {5}{R} -- CORRECT
- **Type:** Creature -- CORRECT
- **Subtypes:** ["Devil"] -- CORRECT
- **P/T:** 4/4 -- CORRECT
- **Keywords:** none -- CORRECT
- **Triggers:** Upkeep (return instant/sorcery), SpellCast (+4/+0) -- CORRECT
- **Behavior:** on_upkeep returns random instant/sorcery from graveyard -- CORRECT
- **Behavior:** on_spell_cast gives +4/+0 until end of turn -- ISSUE (see below)

## Issues
1. **ISSUE: on_spell_cast does not check if the spell cast is an instant or sorcery.** The oracle text says "Whenever you cast an instant or sorcery spell", but the implementation triggers on ANY spell cast by the controller. It should filter to only instant/sorcery spells.
