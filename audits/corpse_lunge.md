## Audit — 2026-04-01

**Scryfall Oracle text**: As an additional cost to cast this spell, exile a creature card from your graveyard.
Corpse Lunge deals damage equal to the exiled card's power to target creature.
**Scryfall type line**: Instant
**Status**: ISSUE

1. **Missing damaged_by tracking** (`mtg-engine/src/cards/corpse_lunge.rs`, line 57-60): When dealing damage to the target creature, the code marks `damage_marked` and emits `NonCombatDamageDealt` but does not push to `obj.damaged_by`. This means cards that check "damaged_by" (e.g., for deathtouch or other tracking) won't see this damage source.
2. **Additional cost not presented as player choice** (`mtg-engine/src/cards/corpse_lunge.rs`, lines 43-46): The code auto-picks the highest-power creature in graveyard to exile. Oracle says "exile a creature card from your graveyard" — the player chooses which creature to exile. However, auto-picking highest power is a reasonable simplification (strictly optimal in most cases).
