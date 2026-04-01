## Audit — 2026-04-01

**Scryfall Oracle text**: Whenever you cast a spell from your graveyard, Burning Vengeance deals 2 damage to any target.
**Scryfall type line**: Enchantment
**Status**: ISSUE

- Mana cost {2}{R}: correct
- Card type Enchantment: correct
- Triggered ability TriggerKind::SpellCast: correct
- on_spell_cast checks controller match: correct
- Checks cast_with_flashback for "from graveyard": correct
- Presents target choice for 2 damage: correct
- Uses PendingEffect::DealDamage with source_id = self_id: correct

Issues found:
1. **Trigger kind is too narrow**: TriggerKind::SpellCast description says "instant or sorcery spell" but the Oracle text says "a spell from your graveyard" — which could include any spell type cast from the graveyard, not just instants/sorceries. However, in practice with the Innistrad flashback mechanic, only instants and sorceries have flashback, so this works correctly in context.
2. **Log message is premature**: Line 67-68 logs "deals 2 damage to opponent" before the target is actually chosen via present_target_choice. The damage might go to a creature, not the opponent.

Tests exist in tier12_cards.rs covering flashback trigger and non-flashback ignore.
