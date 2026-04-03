# Audit: Frightful Delusion

## Reference (Scryfall)
- **Name:** Frightful Delusion
- **Cost:** {2}{U}
- **Type:** Instant
- **Oracle:** Counter target spell unless its controller pays {1}. That player discards a card.
- **P/T:** N/A

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({2}{U})
- Type: CORRECT (Instant)
- Oracle text: CORRECT
- Target requirement: CORRECT (Spell)
- Counter unless pays {1}: CORRECT (checks mana_pool.total() >= 1, offers PayOrNot choice)
- That player discards a card: CORRECT (forces discard after counter or if can't pay)
- P/T: CORRECT (N/A)

## Issues
None found. (The PayOrNot resolution handler in engine.rs correctly forces a discard regardless of whether the player pays or not.)

## Audit (2026-04-02)

### Oracle Text (Scryfall, cached 2026-04-01)
Counter target spell unless its controller pays {1}. That player discards a card.

### Rulings
- You must target a spell in order to cast Frightful Delusion.
- The player discards a card even if they pay {1}.

### Implementation Review (`mtg-engine/src/cards/isd/frightful_delusion.rs`)
1. **Mana cost {2}{U}**: CORRECT -- `ManaCost::new(vec![ManaSymbol::Generic(2), ManaSymbol::Colored(Color::Blue)])`.
2. **Type (Instant)**: CORRECT -- `card_types: vec![CardType::Instant]`.
3. **Targeting (target spell)**: CORRECT -- `TargetRequirement::Spell`; `is_valid_target` checks `Target::Object` with `zone == Zone::Stack`, rejects `Target::Player`.
4. **Counter unless pays {1}**: CORRECT -- `on_resolve` checks `mana_pool.total() >= 1`; if payable, presents `PayOrNot` choice; if not, auto-counters. The `PayOrNot` handler in `engine.rs` (line ~1760) deducts {1} via `auto_pay` when paid, or counters the spell when declined.
5. **Mandatory discard ("That player discards a card")**: CORRECT -- Discard happens in both the pay and don't-pay branches of the `PayOrNot` handler (`engine.rs` lines 1774-1796). Also handled in the auto-counter path in `on_resolve` (lines 73-93). Matches ruling: "The player discards a card even if they pay {1}."
6. **"That player" = spell's controller**: CORRECT -- `controller` is derived from the targeted spell object's `.controller` field.
7. **Empty hand**: No discard triggered if hand is empty. Correct behavior.
8. **Single card in hand**: Auto-discards without choice prompt. Acceptable optimization.

### Test Coverage
- `frightful_delusion_counters_and_discards` (tier2_spells.rs): Covers basic counter + discard when opponent has no mana. PASS.
- `frightful_delusion_discard_on_pay` (card_fixes.rs): Verifies discard still happens when opponent pays {1}. PASS.
- `frightful_delusion_choice_when_opponent_has_mana` (card_mechanics.rs): Verifies PayOrNot choice is presented; opponent declines and spell is countered. PASS.
- `frightful_delusion_auto_counters_without_mana` (card_mechanics.rs): Verifies auto-counter with no choice when opponent has 0 mana. PASS.

### Verdict
**PASS** -- No issues found. Implementation correctly matches oracle text and rulings.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Counter target spell unless its controller pays {1}. That player discards a card.
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found. Verified that the PayOrNot resolution handler in engine.rs forces the discard regardless of whether the opponent pays {1}, matching the ruling that "The player discards a card even if they pay {1}."

## Audit — 2026-04-02 21:03

**Oracle text source**: Scryfall API (https://scryfall.com/card/isd/57/frightful-delusion), cached 2026-04-01
**Oracle text**: Counter target spell unless its controller pays {1}. That player discards a card.
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

- Card data (name, mana cost {2}{U}, type Instant, oracle text) all match Scryfall exactly.
- `TargetRequirement::Spell` and `is_valid_target` correctly restrict to objects on the stack, satisfying the ruling that you must target a spell to cast this.
- `on_resolve` checks `mana_pool.total() >= 1`; if payable, presents `PayOrNot` choice; if not, auto-counters.
- The `PayOrNot` handler in `engine.rs` (lines ~1957-1994) executes the discard in both branches (pay and don't-pay), correctly implementing the ruling: "The player discards a card even if they pay {1}."
- "That player" is correctly resolved to the targeted spell's `.controller` field.
- Empty hand: no discard triggered (correct). Single card: auto-discards without choice prompt (acceptable optimization). Multiple cards: presents `ChooseCardFromHand` choice.
- Fizzle: if the target spell leaves the stack before resolution, the engine's `resolve_top_of_stack` fizzles Frightful Delusion entirely (no discard occurs). This is correct because "That player" refers back to the target, so if the target is illegal, no effects apply.

### Tricky interactions checked (min 3)
1. **Discard on pay**: Opponent pays {1} but must still discard. Verified in both code (engine.rs PayOrNot handler always proceeds to discard) and test (`frightful_delusion_discard_on_pay`).
2. **Fizzle when target leaves stack**: If the target spell resolves or is otherwise removed before Frightful Delusion resolves, the spell fizzles and no discard happens. Verified via `resolve_top_of_stack` in `stack.rs` which checks target legality before calling `on_resolve`.
3. **Auto-counter when opponent has no mana**: When `mana_pool.total() == 0`, skips the `PayOrNot` choice entirely and auto-counters, then forces the discard. Verified in test `frightful_delusion_auto_counters_without_mana`.
4. **Cannot cast without a spell target**: `TargetRequirement::Spell` enforced at cast time; `is_valid_target` rejects `Target::Player`. Matches ruling: "You can't cast it without a legal target just to make a player discard a card."

### Test coverage
- `frightful_delusion_counters_and_discards` (tier2_spells.rs): Basic counter + discard, no mana, 1 card in hand.
- `frightful_delusion_choice_when_opponent_has_mana` (card_mechanics.rs): Pay/don't-pay choice presented; opponent declines, spell countered.
- `frightful_delusion_auto_counters_without_mana` (card_mechanics.rs): Auto-counter, no choice when 0 mana.
- `frightful_delusion_discard_on_pay` (card_fixes.rs): Discard occurs even after paying {1}.
