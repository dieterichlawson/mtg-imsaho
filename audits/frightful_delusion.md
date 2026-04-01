## Audit — 2026-04-01

**Scryfall Oracle text**: Counter target spell unless its controller pays {1}. That player discards a card.
**Scryfall type line**: Instant
**Status**: ISSUE

- Mana cost {2}{U}: correct.
- Type Instant: correct.
- Targets spell on stack: correct.
- Counter-unless-pays-{1} logic: correct.
- Presents PayOrNot choice when opponent has mana: correct.

**Issue — Discard happens regardless of whether the spell is countered, but only implemented on the "can't pay" path.** The Oracle says "Counter target spell unless its controller pays {1}. That player discards a card." The discard is a separate sentence and happens regardless of whether the opponent pays {1} or not. In the implementation, the discard only occurs in the "auto-counter" branch (when the opponent can't pay). When the opponent CAN pay, it goes to `PayOrNot` resolution choice and returns early — the discard after paying would need to be handled in the PayOrNot resolution handler. There is a test in `card_fixes.rs` (`frightful_delusion_discard_on_pay`) that specifically tests this, suggesting this may have been fixed in the resolution handler.

- Tests exist in `tier2_spells.rs`, `card_fixes.rs`, and `card_mechanics.rs`.
