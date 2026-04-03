# Audit: Lumberknot

## Oracle (Official)
- **Name:** Lumberknot
- **Cost:** {2}{G}{G}
- **Type:** Creature — Treefolk
- **Oracle:** Hexproof. Whenever a creature dies, put a +1/+1 counter on Lumberknot.
- **P/T:** 1/1

## Implementation
- Name: "Lumberknot" -- CORRECT
- Cost: {2}{G}{G} -- CORRECT
- Type: Creature -- CORRECT
- Subtypes: ["Treefolk"] -- CORRECT
- P/T: 1/1 -- CORRECT
- Keywords: [Hexproof] -- CORRECT
- Oracle text matches -- CORRECT
- Triggered ability: AnyCreatureDies -- CORRECT
- on_any_creature_dies: adds +1/+1 counter if on battlefield -- CORRECT

## Issues
None.

## Verdict: PASS

## Audit - 2026-04-02

### Oracle Reference
- **Name:** Lumberknot
- **Cost:** {2}{G}{G}
- **Type:** Creature — Treefolk
- **P/T:** 1/1
- **Oracle Text:** Hexproof (This creature can't be the target of spells or abilities your opponents control.) / Whenever a creature dies, put a +1/+1 counter on this creature.

### Card Data Checks
- [x] Name: "Lumberknot" — correct
- [x] Cost: {2}{G}{G} — correct
- [x] Types: Creature — correct
- [x] Subtypes: Treefolk — correct
- [x] P/T: 1/1 — correct
- [x] Keywords: Hexproof — correct
- [x] Triggered ability: AnyCreatureDies — correct

### Behavior Checks
- [x] Hexproof keyword present — correct
- [x] `on_any_creature_dies` triggers for any creature dying — correct
- [x] Only adds counter if self is on battlefield — correct
- [x] Adds PlusOnePlusOne counter — correct
- [x] Triggers on any creature (not just own or opponents') — correct per oracle

### Result: PASS

## Audit — 2026-04-03 07:14
**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/191/lumberknot), cached 2026-04-01
**Oracle text**: Hexproof (This creature can't be the target of spells or abilities your opponents control.)
Whenever a creature dies, put a +1/+1 counter on this creature.
**Type line**: Creature — Treefolk
**Mana cost**: {2}{G}{G}
**P/T**: 1/1
**Status**: PASS

### Code issues
None found. Implementation matches oracle text exactly.

- Name: "Lumberknot" -- matches
- Cost: Generic(2), Green, Green -- matches {2}{G}{G}
- Types: Creature -- matches
- Subtypes: ["Treefolk"] -- matches
- P/T: 1/1 -- matches
- Keywords: [Hexproof] -- matches
- Oracle text in code: "Hexproof\nWhenever a creature dies, put a +1/+1 counter on Lumberknot." -- matches
- Trigger kind: AnyCreatureDies -- correct
- Handler: adds 1 PlusOnePlusOne counter, guarded by battlefield zone check -- correct
- No controller filter on deaths (triggers on any creature, any controller) -- correct per oracle

### Tricky interactions checked (min 3)
1. **Board wipe / simultaneous death**: If Lumberknot dies simultaneously with other creatures, the engine processes each CreatureDied event sequentially. For its own death, it is excluded from watcher list (triggers.rs:419 `o.id != dead_id`). For other creatures' deaths, the watcher zone check at resolution (triggers.rs:908) prevents counter placement since Lumberknot is already in graveyard. Correct behavior.
2. **Hexproof targeting protection**: Engine's `can_be_targeted` (engine.rs:758) checks `has_keyword(target_id, Keyword::Hexproof)` and blocks opponent targeting while allowing controller targeting. Verified this works for Lumberknot since Hexproof is in its keywords vec.
3. **Token creature deaths**: Token creatures generate CreatureDied events in the engine's trigger system, so Lumberknot correctly gains counters when tokens die. No special-casing needed.
4. **Counter accumulation across multiple deaths**: Each death is a separate trigger, each adding 1 counter independently. No cap or deduplication issues.

### Test coverage
- `lumberknot_gains_counter_on_any_death` (tier3_cards.rs): Verifies opponent's creature dying gives Lumberknot +1/+1 counter, checks effective P/T becomes 2/2. PASSES.
- AI card knowledge in llm.rs includes Lumberknot with hexproof and death-trigger description.
- Card registered in mod.rs registry.
