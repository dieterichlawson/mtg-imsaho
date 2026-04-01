## Audit — 2026-04-01

**Scryfall Oracle text**: Gain control of target creature until end of turn. Untap that creature. It gains trample and haste until end of turn.
**Scryfall type line**: Sorcery
**Status**: ISSUE

- Name: correct ("Traitorous Blood")
- Cost: {1}{R}{R} -- correct
- Type: Sorcery -- correct
- Target: TargetRequirement::Creature -- correct
- Implementation: gains control, untaps, grants haste and trample -- correct
- Control reverts at end of turn via `until_end_of_turn_control_changes` -- correct

**Issue: Struct name typo.** The struct is named `TraiterousBlood` (note the 'e' in 'Traiterous') instead of `TraitorousBlood`. This is a minor code quality issue but does not affect functionality since the card name string is correct.

**Note on Oracle text:** Scryfall says "trample and haste" (trample listed first), while the implementation oracle_text says "haste and trample" (haste listed first). Both keywords are granted, so this is cosmetic only.

- Tests exist in `tier12_cards.rs`
