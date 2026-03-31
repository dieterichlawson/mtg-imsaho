# Audit: Trepanation Blade

## Scryfall Reference
- **Name:** Trepanation Blade
- **Cost:** {3}
- **Type:** Artifact — Equipment
- **Oracle:** Whenever equipped creature attacks, defending player reveals cards from the top of their library until they reveal a land card. The creature gets +1/+0 until end of turn for each card revealed this way. That player puts the revealed cards into their graveyard. Equip {2}
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/trepanation_blade.rs`
- Name: "Trepanation Blade" -- MATCH
- Cost: {3} -- MATCH
- Types: Artifact -- MATCH
- Subtypes: ["Equipment"] -- MATCH
- Equip: {2} -- MATCH
- Trigger: Attacks (on equipped creature) -- MATCH
- is_equipment set on resolve -- MATCH
- Mill logic: reveals until land, counts all cards revealed (including land) -- MATCH
- Grants +1/+0 per card revealed until EOT -- MATCH

## Verdict
**PASS** — Equipment and mill trigger correctly implemented.
