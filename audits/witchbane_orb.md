# Audit: Witchbane Orb

## Scryfall Reference
- **Name:** Witchbane Orb
- **Cost:** {4}
- **Type:** Artifact
- **Oracle:** When this artifact enters, destroy all Curses attached to you. You have hexproof.
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/witchbane_orb.rs`
- Name: "Witchbane Orb" -- MATCH
- Cost: {4} -- MATCH
- Types: Artifact -- MATCH
- Trigger: EntersBattlefield -- MATCH
- ETB: Finds all curses attached to controller, destroys them via try_destroy -- MATCH

### ISSUE: Player Hexproof Not Implemented
- Oracle grants "You have hexproof" as a static ability while this artifact is on the battlefield.
- The implementation does NOT implement player hexproof. Code comment acknowledges this as a known limitation.
- **BUG**: Opponents can still target you with spells and abilities while Witchbane Orb is on the battlefield.

## Verdict
**FAIL** — ETB curse destruction works, but the ongoing "You have hexproof" static ability is not implemented.
