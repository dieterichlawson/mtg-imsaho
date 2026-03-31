# Audit: Demonmail Hauberk

## Scryfall Reference
- **Name:** Demonmail Hauberk
- **Cost:** {4}
- **Type:** Artifact -- Equipment
- **Oracle:** Equipped creature gets +4/+2. Equip--Sacrifice a creature.
- **P/T:** N/A
- **Keywords:** Equip

## Implementation: `demonmail_hauberk.rs`
- **Name:** Demonmail Hauberk -- CORRECT
- **Cost:** {4} -- CORRECT
- **Type:** Artifact -- CORRECT
- **Subtypes:** ["Equipment"] -- CORRECT
- **P/T:** N/A -- CORRECT
- **Continuous effect:** ModifyPT { power: 4, toughness: 2, scope: Attached } -- CORRECT
- **Equip cost:** Free mana, SacrificeCost::SacrificeCreature -- CORRECT
- **Equip sorcery speed:** true -- CORRECT

## Issues
None
