# Audit: Gavony Township

## Oracle Reference (Scryfall)
- Cost: (none, land)
- Type: Land
- Oracle: "{T}: Add {C}.
  {2}{G}{W}, {T}: Put a +1/+1 counter on each creature you control."

## Implementation: gavony_township.rs

## Issues Found

No issues found. Name, type, oracle text, mana ability ({T}: Add {C}), and activated ability ({2}{G}{W}, {T}: +1/+1 counter on each creature you control) all match. The activated ability correctly requires tap and the right mana cost. Counters are correctly applied as PlusOnePlusOne.

## Verdict: PASS
