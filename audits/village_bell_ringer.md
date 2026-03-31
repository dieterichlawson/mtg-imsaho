# Audit: Village Bell-Ringer

## Scryfall Reference
- **Name:** Village Bell-Ringer
- **Cost:** {2}{W}
- **Type:** Creature — Human Scout
- **Oracle:** Flash / When this creature enters, untap all creatures you control.
- **P/T:** 1/4

## Implementation: `mtg-engine/src/cards/village_bell_ringer.rs`
- Name: "Village Bell-Ringer" -- MATCH
- Cost: {2}{W} -- MATCH
- Types: Creature -- MATCH
- Subtypes: ["Human", "Scout"] -- MATCH
- P/T: 1/4 -- MATCH
- Keywords: [Flash] -- MATCH
- Trigger: EntersBattlefield -- MATCH
- on_enter_battlefield: Untaps all creatures (power.is_some()) controlled by controller -- MATCH

## Verdict
**PASS** — Flash creature with ETB untap correctly implemented.
