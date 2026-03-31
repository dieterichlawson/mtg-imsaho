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
