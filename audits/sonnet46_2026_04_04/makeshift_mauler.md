## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: As an additional cost to cast this spell, exile a creature card from your graveyard.
**Type line**: Creature — Zombie Horror
**Status**: ISSUE

### Code issues

- Engine auto-selects which creature to exile instead of giving the player a choice (`mtg-engine/src/engine.rs` ~line 1574)
  - Oracle text says: `"As an additional cost to cast this spell, exile a creature card from your graveyard."`
  - Code does: `// Pick highest-power creatures first (better default for Corpse Lunge). ... exile_candidates.sort_by(|a, b| b.1.cmp(&a.1)); // Highest power first` — the engine sorts graveyard creatures by power descending and exiles the top N without presenting the player a choice. Under MTG rules (CR 601.2f), when paying an additional cost that exiles a card from a zone, the casting player chooses which card. When there are multiple creature cards in the graveyard, the player cannot choose which to preserve and which to exile. The `CastSpell` action struct has no field to carry that choice (`object_id`, `targets`, `sacrifice`, `exile_count`, `alternative_cost` — no `exile_creature_id`), so the player is structurally prevented from specifying which creature to exile.

### Tricky interactions checked

- Cannot cast without creature card in graveyard: PASS — engine checks `creature_count < *n` and skips generating the cast action (engine.rs ~line 553)
- Exactly one creature exiled (ruling: "you must exile exactly one … you cannot exile additional creature cards"): PASS — `exile_candidates.into_iter().take(n)` with n=1 ensures exactly one is exiled (engine.rs ~line 1585)
- Player choice of which creature to exile when multiple available: FAIL — engine auto-selects highest-power creature; no mechanism for player to choose (engine.rs ~lines 1574–1600)
- Enters the battlefield on resolution (not graveyard): PASS — `on_resolve` calls `state.move_object(object_id, Zone::Battlefield)`, and `resolve_spell` in stack.rs only calls `move_spell_after_resolve` if the object is still on the stack after `on_resolve`, which it isn't
- Card data correctness (mana cost {3}{U}, P/T 4/5, types Creature, subtypes Zombie + Horror, no keywords, oracle text matches): PASS
- Rooftop Storm interaction — Zombies can be cast for {0} but additional exile cost still applies: PASS — `ExileCreaturesFromGraveyard` handling (engine.rs ~line 1568) is unconditional and runs even when `alternative_cost` is set

### Test coverage

For each ruling and tricky interaction, list whether it is tested and where:
- Basic case: mauler enters battlefield and creature is exiled: `tier11_cards.rs:20` (`makeshift_mauler_exiles_creature_from_graveyard`)
- P/T of 4/5 after resolution: `tier11_cards.rs:39` (`makeshift_mauler_is_4_5_zombie`)
- Cannot cast without a creature card in graveyard: NOT TESTED
- Player chooses which creature to exile when multiple are in graveyard: NOT TESTED
- Exactly one creature exiled (not zero, not more): NOT TESTED explicitly for Makeshift Mauler (covered implicitly by the single-creature test)
