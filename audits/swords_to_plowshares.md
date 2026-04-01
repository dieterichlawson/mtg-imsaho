## Audit — 2026-04-01

**Scryfall Oracle text**: Exile target creature. Its controller gains life equal to its power.
**Scryfall type line**: Instant
**Status**: PASS

- Name: correct ("Swords to Plowshares")
- Cost: {W} -- correct
- Type: Instant -- correct
- Target: TargetRequirement::Creature -- correct
- Implementation exiles the creature and grants life equal to its effective power to the creature's controller (not the caster) -- correct
- Uses `effective_power` which accounts for buffs/counters -- correct
- Life gain skipped if power <= 0 -- correct (you can't gain negative life)
- Tests exist in `spells.rs`, `fizzle.rs`, `spell_fizzle.rs`
- No issues found
