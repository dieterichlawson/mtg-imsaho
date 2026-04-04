## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {1}{R}: This creature gains first strike until end of turn.
**Type line**: Creature — Wolf
**Status**: ISSUE

### Code issues
- Oracle text mismatch in card_data (line 23)
  - Oracle text says: `{1}{R}: This creature gains first strike until end of turn.`
  - Code does: `oracle_text: "{1}{R}: Kessig Wolf gains first strike until end of turn.".into()`

### Tricky interactions checked
- Activated ability mana cost and targeting: pass - correctly requires {1}{R} cost and no targeting
- "This creature" self-reference: pass - uses object_id correctly in on_activate_ability
- "Until end of turn" timing: pass - uses UntilEndOfTurnKeyword system which is properly cleaned up in engine.rs cleanup step
- First strike keyword grant: pass - correctly adds Keyword::FirstStrike to until_end_of_turn_keywords vector
- Ability availability: pass - activated_abilities only returns the ability when object is on battlefield
- Combat timing interactions: pass - activated ability can be used at instant speed, allowing proper combat timing

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic stats (3/1 Wolf): `activated_abilities.rs:83-90` / TESTED
- Activated ability mana cost: `activated_abilities.rs:93-114` / TESTED  
- First strike keyword grant: `activated_abilities.rs:93-114` / TESTED
- Until end of turn cleanup: NOT TESTED
- Combat timing interactions: NOT TESTED
- Multiple activations same turn: NOT TESTED

### Sources
- [Kessig Wolf MTG - Innistrad #151 (English) | Magic: The Gathering](https://gatherer.wizards.com/ISD/en-us/151/kessig-wolf)
- [Kessig Wolf · Innistrad (ISD) #151 - Scryfall](https://scryfall.com/card/isd/151/kessig-wolf)
- [Combat phase and Kessig Wolfrun - Magic Rulings Archives - Magic Rulings - Magic Fundamentals - MTG Salvation Forums - MTG Salvation](https://www.mtgsalvation.com/forums/magic-fundamentals/magic-rulings/magic-rulings-archives/297451-combat-phase-and-kessig-wolfrun)
- [Kessig Wolf from Innistrad Spoiler](https://www.magicspoiler.com/mtg-spoiler/kessig-wolf/)
