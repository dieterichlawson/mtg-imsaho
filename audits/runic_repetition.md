## Audit — 2026-04-01

**Scryfall Oracle text**: Return target exiled card you own with flashback to your hand.
**Scryfall type line**: Sorcery
**Mana cost**: {2}{U}
**Status**: PASS

Implementation correctly models:
- Name, mana cost {2}{U}, type Sorcery
- Target requirement: ExileCard
- `is_valid_target` checks zone is Exile, owner matches caster, and card has flashback
- Resolution moves target from exile to hand
- Tests: `runic_repetition_card_data` and `runic_repetition_returns_flashback_card_from_exile` in innistrad_simple_cards.rs

No issues found.
