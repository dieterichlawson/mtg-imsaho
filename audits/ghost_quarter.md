# Audit: Ghost Quarter

## Oracle Reference (Scryfall)
- Cost: (none, land)
- Type: Land
- Oracle: "{T}: Add {C}.
  {T}, Sacrifice Ghost Quarter: Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle."

## Implementation: ghost_quarter.rs

## Issues Found

No issues found. Name, type (Land), oracle text, mana ability, and activated ability all match. The sacrifice ability correctly requires tap, sacrifice self, and targets a land. The "may search" is auto-resolved (always searches), which is a reasonable AI simplification. Basic land search logic correctly checks for CardType::Land + Supertype::Basic.

## Verdict: PASS
