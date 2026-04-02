# Audit: Armored Skaab

## Reference (Scryfall/API)
- **Name:** Armored Skaab
- **Mana Cost:** {2}{U}
- **Type:** Creature — Zombie Warrior
- **Oracle:** When Armored Skaab enters the battlefield, mill four cards.
- **P/T:** 1/4

## Implementation: `armored_skaab.rs`
- **Name:** Armored Skaab -- CORRECT
- **Mana Cost:** {2}{U} -- CORRECT
- **Type:** Creature — Zombie Warrior -- CORRECT
- **Subtypes:** ["Zombie", "Warrior"] -- CORRECT
- **P/T:** 1/4 -- CORRECT
- **Triggered ability:** EntersBattlefield, mills 4 cards -- CORRECT

## Verdict: PASS -- No issues found

## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: When this creature enters, mill four cards.
**Type line**: Creature — Zombie Warrior
**Status**: ISSUE
### Code issues
1. **Oracle text wording mismatch (cosmetic)**: Oracle says `"When this creature enters, mill four cards."` but code oracle_text field says `"When Armored Skaab enters the battlefield, mill four cards."` The code uses the old ETB template instead of the updated "this creature enters" template.
   - Code: `"When Armored Skaab enters the battlefield, mill four cards."`
   - Oracle: `"When this creature enters, mill four cards."`

Behavior is otherwise correct: triggered ability on EntersBattlefield calls mill_cards(state, controller, 4). Stats (1/4), cost ({2}{U}), types (Creature — Zombie Warrior) all match.

## Re-audit — 2026-04-02
**Status**: PASS
Oracle text updated to match Scryfall: "When this creature enters, mill four cards." (was "When Armored Skaab enters the battlefield, mill four cards."). Doc comment updated. Behavior unchanged.
