## Audit — 2026-04-01

**Scryfall Oracle text**: (Front — Cloistered Youth) At the beginning of your upkeep, you may transform Cloistered Youth.
(Back — Unholy Fiend) At the beginning of your end step, you lose 1 life.
**Scryfall type line**: (Front) Creature — Human // (Back) Creature — Horror
**Status**: ISSUE

### Findings

1. **Front face P/T wrong (ISSUE)**: Implementation has `power: Some(1), toughness: Some(1)` (line 26-27). Oracle text for Cloistered Youth is 1/1 on the printed card but the actual Oracle P/T is **1/1**. Wait — checking again: Cloistered Youth is actually a 1/1. However, the doc comment on line 6 says "3/2" which is wrong — 3/2 is not the front face. The back face Unholy Fiend is 3/3. The stored card_data power/toughness of (1,1) is correct for the front face though.

2. **Back face P/T**: Unholy Fiend is listed as 3/3 in the implementation. Oracle text says Unholy Fiend is **3/3**. This is correct.

3. **Transform is not optional (ISSUE)**: Oracle says "you **may** transform Cloistered Youth." The implementation auto-transforms on upkeep (line 82-84) without giving the player a choice. This removes player agency.

4. **Triggered ability kind mismatch (ISSUE)**: The second triggered ability is `TriggerKind::EndStep`, but the back face's Oracle says "At the beginning of your **end step**" — "end step" is correct. However, the back face Oracle actually says "At the beginning of your **end step**, you lose 1 life" — this is Unholy Fiend's upkeep, not end step. Checking the actual Oracle: Unholy Fiend says "At the beginning of your **end step**, you lose 1 life." So EndStep trigger kind is correct.

5. **Life loss implementation**: Uses direct life subtraction rather than damage. This is correct — "lose 1 life" is life loss, not damage.

6. **Tests**: No dedicated tests found.
