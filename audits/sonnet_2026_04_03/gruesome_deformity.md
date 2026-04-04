## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Enchant creature
Enchanted creature has intimidate. (It can't be blocked except by artifact creatures and/or creatures that share a color with it.)
**Type line**: Enchantment — Aura
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Aura attachment targeting: PASS - `TargetRequirement::Creature` correctly implements "Enchant creature" 
- Intimidate blocking rules: PASS - Engine correctly implements intimidate in combat.rs lines 627-643, checking both artifact type and color sharing
- Continuous effect scope: PASS - `EffectScope::Attached` correctly applies intimidate only to attached creature and removes effect when aura is removed
- Multicolored creature interactions: PASS - Color sharing check uses `any()` predicate, correctly allowing any shared color to enable blocking
- Artifact creature blocking: PASS - Engine checks for CardType::Artifact and allows blocking regardless of color
- Colorless attacker handling: PASS - Color sharing check will fail for colorless creatures, leaving only artifact blockers

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Aura grants intimidate: `innistrad_cards.rs:291` (gruesome_deformity_grants_intimidate)
- Intimidate blocks different colors: `keywords.rs:203` (intimidate_blocks_different_color)
- Artifact creatures block intimidate: `keywords.rs:228` (artifact_creature_blocks_intimidate)
- Same color creatures can block intimidate: `keywords.rs:203` (intimidate_blocks_different_color tests white-white blocking)
- Aura attachment and detachment effects: NOT TESTED
- Colorless creature with intimidate: NOT TESTED

### Sources
Sources consulted during audit:
- [Intimidate - MTG Wiki - Fandom](https://mtg.fandom.com/wiki/Intimidate)
- [Intimidate in MTG - Rules, Best Cards + Decks!](https://mykindofmeeple.com/mtg-intimidate-explained-keyword-rules-best-cards-decks/)
- [Fear & Intimidate - MTG Keywords Explained - Card Kingdom Blog](https://blog.cardkingdom.com/mtg-keywords-explained-fear-and-intimidate/)
- [Intimidate - Magic: The Gathering Wiki](https://mtg.wiki/page/Intimidate)
- [Intimidate in MTG: Rules, History, and Best Cards - Draftsim](https://draftsim.com/intimidate-mtg/)
- [Mechanics of Magic Overview: Intimidate | Article by Christopher Lai](https://www.coolstuffinc.com/a/christopherlai-seo-03312025-mechanics-of-magic-overview-intimidate)