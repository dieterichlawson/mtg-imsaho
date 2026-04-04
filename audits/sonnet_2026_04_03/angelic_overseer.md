## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
As long as you control a Human, this creature has hexproof and indestructible.
**Type line**: Creature — Angel
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- "As long as" continuous evaluation: PASS — `YouControlSubtype` condition is checked every time `has_keyword` is called, ensuring continuous re-evaluation
- Simultaneous destruction with Humans: PASS — When mass destruction occurs, indestructible is checked at the moment `try_destroy` is called, while Humans are still on battlefield
- Lethal damage timing with Human loss: PASS — Damage remains marked on creature; if indestructible is lost later in turn, next SBA check will destroy the creature
- Hexproof targeting prevention: PASS — `ConditionalKeyword` system correctly grants hexproof when condition is met
- Subtype checking (tokens vs printed): PASS — `YouControlSubtype` checks both `o.subtypes` (runtime/tokens) and `registry.card_data().subtypes` (printed)
- Controller-specific Human check: PASS — `YouControlSubtype` filters by `o.controller == controller`, only checking your Humans, not opponent's

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic flying keyword: `tier12_cards.rs:563` (angelic_overseer_has_flying)
- Hexproof/indestructible gained with Human: `tier12_cards.rs:574` (angelic_overseer_hexproof_indestructible_with_human)
- Hexproof/indestructible lost when Human leaves: `tier12_cards.rs:574` (same test, removes Human and verifies loss)
- Indestructible prevents destruction: `tier12_cards.rs:605` (angelic_overseer_survives_destroy_with_human)
- Simultaneous destruction ruling (Humans + Overseer destroyed together): NOT TESTED
- Lethal damage + Human loss timing interaction: NOT TESTED

Sources:
- [MTG : Angelic Overseer : Hexproof and Indestructible](https://cantrip.ru/en/mtg-cards/Angelic-Overseer.shtml)
- [Angelic Overseer rulings - MTG Assist](https://www.mtgassist.com/cards/Innistrad/Angelic-Overseer/rulings/)
- [Angelic Overseer MTG - Innistrad #3 (English) | Magic: The Gathering](https://gatherer.wizards.com/pages/card/Details.aspx?multiverseid=220370)
- [Angelic Overseer | Innistrad | Modern | Card Kingdom](https://www.cardkingdom.com/mtg/innistrad/angelic-overseer)
- [Angelic Overseer (Innistrad)](https://aetherhub.com/Card/ISD/Angelic-Overseer/3)