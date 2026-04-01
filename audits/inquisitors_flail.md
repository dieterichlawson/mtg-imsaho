## Audit — 2026-04-01

**Scryfall Oracle text**: If equipped creature would deal combat damage, it deals double that damage instead.
If another source would deal combat damage to equipped creature, it deals double that damage to equipped creature instead.
Equip {2}
**Scryfall type line**: Artifact — Equipment
**Status**: ISSUE

- Mana cost {2}: correct
- Card types Artifact, subtype Equipment: correct
- Equip {2}: correct (sorcery speed, targets creature)
- ISSUE: The double-damage replacement effect is approximated by granting +P/+0 equal to the creature's power via dynamic_pt. This is not equivalent to doubling combat damage: (1) it changes the creature's actual power, which affects fight effects and other power-referencing abilities, not just combat damage; (2) it doesn't account for additional damage modifications that happen after power is calculated; (3) it's applied on the Equipment's dynamic_pt rather than the creature's, which is unusual.
- ISSUE: The defensive half (taking double combat damage) is not implemented at all, as noted in the comments.
- Tests exist in tier9_cards.rs covering card data, power doubling, and equip ability

## Audit — 2026-04-01 (independent)

**Scryfall Oracle text**: If equipped creature would deal combat damage, it deals double that damage instead. If another source would deal combat damage to equipped creature, it deals double that damage to equipped creature instead. Equip {2}
**Scryfall type line**: Artifact -- Equipment
**Status**: ISSUE

- ISSUE (documented simplification): Offensive double damage approximated via dynamic_pt power bonus. This changes actual power (affects fight, power-based abilities) not just combat damage. Defensive doubling not implemented.
- ISSUE: The oracle_text in code says "another source" for the defensive half but Scryfall says "another creature". Code comment is correct though.
