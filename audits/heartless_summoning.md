## Audit — 2026-04-01

**Scryfall Oracle text**: Creature spells you cast cost {2} less to cast.
Creatures you control get -1/-1.
**Scryfall type line**: Enchantment
**Status**: PASS

- Mana cost {1}{B}: correct
- Card type Enchantment: correct
- Continuous effect ModifyPT -1/-1 with scope Global(CreatureFilter::You): correct
- Continuous effect ReduceCost with reduction 2, filter CreatureSpells: correct
- Tests exist in tier14_cards.rs covering -1/-1 debuff, creature cost reduction, and non-creature non-reduction

## Audit — 2026-04-01

**Scryfall Oracle text**: Creature spells you cast cost {2} less to cast. / Creatures you control get -1/-1.
**Scryfall type line**: Enchantment
**Status**: PASS

No issues found. Mana cost {1}{B} correct. Continuous effects correctly model both -1/-1 to creatures (Global(CreatureFilter::You)) and cost reduction for creature spells (ReduceCost with SpellFilter::CreatureSpells). Tests exist (tier14_cards.rs).
