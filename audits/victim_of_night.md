# Audit: Victim of Night

## Scryfall Reference
- **Name:** Victim of Night
- **Cost:** {B}{B}
- **Type:** Instant
- **Oracle:** Destroy target non-Vampire, non-Werewolf, non-Zombie creature.
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/victim_of_night.rs`
- Name: "Victim of Night" -- MATCH
- Cost: {B}{B} -- MATCH
- Types: Instant -- MATCH
- Target: CreatureWithFilter(NotSubtypes(["Vampire", "Werewolf", "Zombie"])) -- MATCH
- is_valid_target: Checks battlefield, creature, not Vampire/Werewolf/Zombie -- MATCH
- on_resolve: Uses resolve_destroy -- CORRECT (destroy, not exile)

## Verdict
**PASS** — Correctly implements conditional destruction with subtype exclusions.
