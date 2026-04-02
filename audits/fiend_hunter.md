# Audit: Fiend Hunter

## Reference (Scryfall)
- **Name:** Fiend Hunter
- **Cost:** {1}{W}{W}
- **Type:** Creature -- Human Cleric
- **Oracle:** When Fiend Hunter enters the battlefield, you may exile another target creature. When Fiend Hunter leaves the battlefield, return the exiled card to the battlefield under its owner's control.
- **P/T:** 1/3

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({1}{W}{W})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Human, Cleric)
- Oracle text: CORRECT
- P/T: CORRECT (1/3)
- ETB trigger: CORRECT (TriggerKind::EntersBattlefield)
- LTB trigger: CORRECT (TriggerKind::LeavesBattlefield)
- "You may" optional exile: CORRECT (uses present_optional_target_choice)
- "Another target creature": CORRECT (creature_targets_except excludes self)
- Returns exiled card on LTB: CORRECT (checks Zone::Exile, moves to Battlefield)

## Issues
None found.

## Audit — 2026-04-01 12:00

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: When this creature enters, you may exile another target creature.\nWhen this creature leaves the battlefield, return the exiled card to the battlefield under its owner's control.
**Type line**: Creature — Human Cleric
**Status**: ISSUE

### Code issues

1. **LLM card knowledge is inaccurate** (`mtg-player/src/llm.rs:102`)
   - Oracle text says: `"you may exile another target creature"` (any creature, not just opponent's; and it's optional)
   - LLM knowledge says: `"When it enters, exiles an opponent's creature."` — this is wrong in two ways: (a) it says "opponent's creature" but the card can target any creature including your own, and (b) it omits the "you may" optional nature.

2. **Oracle text uses old templating** (`fiend_hunter.rs:27`)
   - Current oracle text: `"When this creature enters, you may exile another target creature. When this creature leaves the battlefield, return the exiled card to the battlefield under its owner's control."`
   - Code oracle_text: `"When Fiend Hunter enters the battlefield, you may exile another target creature. When Fiend Hunter leaves the battlefield, return the exiled card to the battlefield under its owner's control."`
   - This is a cosmetic difference from the 2023 templating update. Not a functional issue.

### Tricky interactions checked
- "Another" excludes self: PASS (`creature_targets_except(state, object_id)` excludes Fiend Hunter)
- "You may" is optional: PASS (uses `present_optional_target_choice`)
- Can target own creatures: PASS (`creature_targets_except` includes all creatures on battlefield)
- LTB returns exiled creature: PASS (line 57-65, checks `card_state["exiled_creature"]` and verifies zone is Exile)
- Oblivion Ring trick (LTB before ETB resolves): PASS (if LTB fires before ETB, `card_state` has no "exiled_creature" key, so LTB does nothing; then ETB exiles permanently)
- Token handling per ruling: PASS (tokens in exile are removed by SBA before LTB would typically fire in normal gameplay)
- ETB trigger kind: PASS (TriggerKind::EntersBattlefield)
- LTB trigger kind: PASS (TriggerKind::LeavesBattlefield)

### Test coverage
- ETB exiles a creature: `tier3_cards.rs:211` (fiend_hunter_exiles_on_etb)
- LTB returns exiled creature: `card_mechanics.rs:127` (fiend_hunter_returns_exiled_on_death)
- Can target own creatures: `card_fixes.rs:30` (fiend_hunter_can_target_own_creature)
- Presents choice with multiple targets: `card_fixes.rs:60` (fiend_hunter_presents_choice_with_multiple_targets)
- Declining to exile (choosing "no"): NOT TESTED
- Oblivion Ring trick (LTB before ETB): NOT TESTED
- Token exiled doesn't return: NOT TESTED
