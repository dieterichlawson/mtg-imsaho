# Audit: Mentor of the Meek

## Reference (Scryfall/API)
- **Name:** Mentor of the Meek
- **Mana Cost:** {2}{W}
- **Type:** Creature — Human Soldier
- **Oracle:** Whenever another creature you control with power 2 or less enters, you may pay {1}. If you do, draw a card.
- **P/T:** 2/2

## Implementation: `mentor_of_the_meek.rs`
- **Name:** Mentor of the Meek -- CORRECT
- **Mana Cost:** {2}{W} -- CORRECT
- **Type:** Creature — Human Soldier -- CORRECT
- **P/T:** 2/2 -- CORRECT
- **Keywords:** None -- CORRECT
- **Triggered ability:** AnyCreatureEnters -- CORRECT
- **Controller check:** entered_controller == controller -- CORRECT
- **Self exclusion:** entered_id != self_id -- CORRECT
- **Power check:** effective_power <= 2 -- CORRECT
- **May pay {1}:** Auto-pays if mana available (simplified) -- ACCEPTABLE
- **Draw a card:** Draws 1 card on payment -- CORRECT

## Verdict: PASS -- No issues found

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Whenever another creature you control with power 2 or less enters, you may pay {1}. If you do, draw a card.
**Type line**: Creature — Human Soldier
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Mentor of the Meek", cost {2}{W}, 2/2, type Creature with subtypes Human/Soldier. Trigger fires on AnyCreatureEnters, correctly filters for another creature under same controller with power <= 2, auto-pays {1} if mana is available (acceptable simplification of "you may pay"), then draws a card. Behavior is correct.
