# Audit: Moldgraf Monstrosity

## Reference (Scryfall/API)
- **Name:** Moldgraf Monstrosity
- **Mana Cost:** {4}{G}{G}{G}
- **Type:** Creature — Insect
- **Oracle:** Trample / When this creature dies, exile it, then return two creature cards at random from your graveyard to the battlefield.
- **P/T:** 8/8

## Implementation: `moldgraf_monstrosity.rs`
- **Name:** Moldgraf Monstrosity -- CORRECT
- **Mana Cost:** {4}{G}{G}{G} -- CORRECT
- **Type:** Creature — Insect -- CORRECT
- **P/T:** 8/8 -- CORRECT
- **Keywords:** Trample -- CORRECT
- **Triggered ability:** SelfDies -- CORRECT
- **Exile self:** Moves to Zone::Exile -- CORRECT
- **Find creatures in graveyard:** Filters by power.is_some() and excludes self -- CORRECT
- **Random selection:** Shuffles list, takes up to 2 -- CORRECT
- **Return to battlefield:** Moves selected creatures to battlefield with controller = owner -- CORRECT

## Verdict: PASS -- No issues found

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Trample\nWhen this creature dies, exile it, then return two creature cards at random from your graveyard to the battlefield.
**Type line**: Creature — Insect
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Moldgraf Monstrosity", cost {4}{G}{G}{G}, 8/8, type Creature with subtype Insect, keyword Trample. On death trigger exiles itself then returns two random creature cards from graveyard to the battlefield. Behavior is correct.
