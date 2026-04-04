# Audit: Mindshrieker

## Reference (Scryfall/API)
- **Name:** Mindshrieker
- **Mana Cost:** {1}{U}
- **Type:** Creature — Spirit Bird
- **Oracle:** Flying / {2}: Target player mills a card. This creature gets +X/+X until end of turn, where X is the milled card's mana value.
- **P/T:** 1/1

## Implementation: `mindshrieker.rs`
- **Name:** Mindshrieker -- CORRECT
- **Mana Cost:** {1}{U} -- CORRECT
- **Type:** Creature — Spirit Bird -- CORRECT
- **P/T:** 1/1 -- CORRECT
- **Keywords:** Flying -- CORRECT
- **Activated ability:** cost {2}, requires_tap=false, target=PlayerOnly -- CORRECT
- **Mill:** Removes top card from target player's library, moves to graveyard -- CORRECT
- **+X/+X:** Looks up milled card's mana_value, applies UntilEndOfTurnEffect with power_mod and toughness_mod -- CORRECT

## Verdict: PASS -- No issues found

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Flying\n{2}: Target player mills a card. This creature gets +X/+X until end of turn, where X is the milled card's mana value.
**Type line**: Creature — Spirit Bird
**Status**: PASS
### Code issues
None. Card data matches oracle: name "Mindshrieker", cost {1}{U}, 1/1, type Creature with subtypes Spirit/Bird, keyword Flying. Activated ability costs {2}, targets a player, mills one card, then applies +X/+X until end of turn where X is the milled card's mana value. Behavior is correct.
