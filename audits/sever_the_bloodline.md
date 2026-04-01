## Audit — 2026-04-01

**Scryfall Oracle text**: Exile target creature and all other creatures with the same name as that creature.\nFlashback {5}{B}{B}
**Scryfall type line**: Sorcery
**Mana cost**: {3}{B}
**Status**: PASS

Implementation correctly models:
- Name, mana cost {3}{B}, type Sorcery
- Flashback {5}{B}{B}
- Targets a creature, then exiles it and all other creatures with the same name
- Uses name-matching filter across all battlefield creatures
- Tests: `sever_the_bloodline_exiles_all_with_same_name` and `sever_the_bloodline_has_flashback` in tier12_cards.rs

No issues found.
