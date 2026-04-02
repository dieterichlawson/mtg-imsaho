# Audit: Champion of the Parish

## Scryfall Reference
- **Name:** Champion of the Parish
- **Cost:** {W}
- **Type:** Creature -- Human Soldier
- **Oracle:** Whenever another Human you control enters, put a +1/+1 counter on this creature.
- **P/T:** 1/1
- **Keywords:** none

## Implementation: `champion_of_the_parish.rs`
- **Name:** Champion of the Parish -- CORRECT
- **Cost:** {W} -- CORRECT
- **Type:** Creature -- CORRECT
- **Subtypes:** ["Human", "Soldier"] -- CORRECT
- **P/T:** 1/1 -- CORRECT
- **Keywords:** none -- CORRECT
- **Trigger:** AnyCreatureEnters -- CORRECT
- **Self-exclusion ("another"):** Handled by trigger infrastructure in `triggers.rs` line 352: `.filter(|o| o.zone == Zone::Battlefield && o.id != *object)` excludes the entering creature from the watcher list. No card-level check needed. -- CORRECT
- **Controller check ("you control"):** `entered_controller != controller` guard at line 43. -- CORRECT
- **Human subtype check:** Checks both registry `card_data.subtypes` and instance `obj.subtypes` (lines 48-54), covering both registered cards and tokens. -- CORRECT
- **+1/+1 counter:** `state.add_counters(self_id, CounterType::PlusOnePlusOne, 1)` at line 56. -- CORRECT
- **Zone check:** Confirms Champion is on the battlefield before triggering (line 39). -- CORRECT

## Tests (`mtg-engine/tests/tier6_cards.rs`)
- `champion_of_the_parish_counter_on_human_etb` -- triggers on friendly Human ETB. PASS
- `champion_of_the_parish_no_counter_on_non_human` -- does not trigger on non-Human. PASS
- `champion_of_the_parish_no_counter_on_opponent_human` -- does not trigger on opponent's Human. PASS
- No test for self-entering (Champion entering while another Champion is already on the battlefield) but infrastructure handles this correctly.

## LLM hint (`mtg-player/src/llm.rs`)
- "Champion of the Parish ({W} creature 1/1): Gets a +1/+1 counter whenever another Human enters under your control. Play Humans to grow it!" -- CORRECT, matches oracle semantics.

## Issues

### Issue 1: Oracle text field uses outdated wording (cosmetic)

**Scryfall oracle (current):**
> Whenever another Human you control enters, put a +1/+1 counter on this creature.

**Implementation `oracle_text` field (line 23):**
> "Whenever another Human creature enters the battlefield under your control, put a +1/+1 counter on Champion of the Parish."

Differences: (a) includes "creature" which was removed in 2023 oracle update, (b) includes "the battlefield" which was removed in 2023 oracle update, (c) says "Champion of the Parish" instead of "this creature". This is cosmetic only and does not affect game behavior since the trigger logic correctly implements the intended semantics.

### Issue 2: Comment in doc-string uses outdated wording (cosmetic)

**Line 7:**
> `/// Whenever another Human creature enters the battlefield under your control,`

Same outdated wording as the oracle_text field. Cosmetic only.

## Verdict
**No functional bugs.** The implementation correctly handles all aspects of the card: self-exclusion via trigger infrastructure, controller check, Human subtype check (both registry and instance), and +1/+1 counter placement. Two cosmetic issues with outdated oracle text wording in the `oracle_text` field and doc comment.
