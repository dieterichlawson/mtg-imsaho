## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Defender, protection from Zombies
**Type line**: Creature — Plant
**Status**: ISSUE

### Code issues
- **LLM card knowledge missing protection from Zombies** (mtg-player/src/llm.rs line 64): 
  - Oracle text says: `Defender, protection from Zombies`
  - Code does: `Grave Bramble ({1}{G}{G} 3/4 defender): Can't attack, but great blocker.` (omits protection from Zombies)
- **Engine limitation - protection does not prevent targeting** (mtg-engine/src/engine.rs:758):
  - Oracle text says: `protection from Zombies` (which includes DEBT: can't be Targeted)
  - Code does: `can_be_targeted` only checks hexproof, not protection. Zombie spells could target Grave Bramble despite protection.
- **Engine limitation - protection does not prevent enchanting/equipping** (mtg-engine/src/cards/helpers.rs:18):
  - Oracle text says: `protection from Zombies` (which includes DEBT: can't be Enchanted/Equipped)  
  - Code does: `resolve_aura` only checks if target is on battlefield, doesn't validate protection. Zombie auras could enchant Grave Bramble.

### Tricky interactions checked
- **Damage prevention from Zombies**: pass (combat.rs:440 uses has_protection_from_creature to prevent damage)
- **Blocking restrictions with Zombies**: pass (combat.rs:696-699 prevents Zombie blocking via has_protection_from_creature)  
- **Defender preventing attacks**: pass (Keyword::Defender correctly prevents attacking)
- **Protection is one-directional for damage**: pass (Grave Bramble still deals damage to Zombies)
- **Zombie spells targeting Grave Bramble**: fail (can_be_targeted ignores protection)
- **Zombie auras enchanting Grave Bramble**: fail (resolve_aura ignores protection)

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Damage prevention from Zombies: `mtg-engine/tests/card_mechanics.rs:309` (grave_bramble_protection_prevents_zombie_damage)
- Defender cannot attack: `mtg-engine/tests/keywords.rs:111` (defender_cannot_attack) 
- Defender can block: `mtg-engine/tests/keywords.rs:124` (defender_can_block)
- Protection prevents targeting by Zombie spells: NOT TESTED
- Protection prevents enchanting by Zombie auras: NOT TESTED
- Protection allows damage dealing to Zombies: `mtg-engine/tests/card_mechanics.rs:326` (same test as damage prevention)