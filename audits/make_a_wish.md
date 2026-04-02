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

## Audit - 2026-04-02

### Oracle Reference
- **Name:** Make a Wish
- **Cost:** {3}{G}
- **Type:** Sorcery
- **Oracle Text:** Return two cards at random from your graveyard to your hand.

### Card Data Checks
- [x] Name: "Make a Wish" — correct
- [x] Cost: {3}{G} — correct
- [x] Types: Sorcery — correct
- [x] Oracle text matches — correct

### Behavior Checks
- [x] Gets cards from controller's graveyard — correct
- [x] Excludes self (Make a Wish itself) from candidates — correct
- [x] Excludes tokens — correct
- [x] Shuffles and takes up to 2 at random — correct
- [x] Returns 1 card if only 1 available (per ruling) — correct
- [x] Handles empty graveyard gracefully — correct
- [x] Spell moves to graveyard after resolve — correct

### Result: PASS
