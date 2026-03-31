# Audit: Make a Wish

## Oracle (Official)
- **Name:** Make a Wish
- **Cost:** {3}{G}
- **Type:** Sorcery
- **Oracle:** Return two cards at random from your graveyard to your hand.
- **P/T:** N/A

## Implementation
- Name: "Make a Wish" -- CORRECT
- Cost: {3}{G} -- CORRECT
- Type: Sorcery -- CORRECT
- Oracle text matches -- CORRECT
- Shuffles graveyard cards and takes 2 at random -- CORRECT
- Excludes tokens from selection -- CORRECT
- Excludes self (the Make a Wish spell) from graveyard selection -- CORRECT
- Calls move_spell_after_resolve -- CORRECT

## Issues
None.

## Verdict: PASS
