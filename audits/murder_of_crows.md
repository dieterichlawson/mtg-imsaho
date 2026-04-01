## Audit — 2026-04-01

**Scryfall Oracle text**: Flying\nWhenever another creature dies, you may draw a card. If you do, discard a card.
**Scryfall type line**: Creature — Bird
**Status**: PASS

- Name: Correct ("Murder of Crows")
- Cost: {3}{U}{U} - Correct
- Type: Creature — Bird - Correct
- P/T: 4/4 - Correct
- Keywords: Flying - Correct
- Trigger: AnyCreatureDies (excludes self via "another") - Correct trigger kind. The "another" exclusion is handled by the engine's AnyCreatureDies trigger kind which excludes the dying creature being the trigger source.
- "You may draw" is presented as a YesNo choice to the controller. Correct.
- Checks that Murder of Crows is on the battlefield before triggering. Correct.
- Tests: card_fixes.rs has `murder_of_crows_presents_draw_choice`.

No issues found.
