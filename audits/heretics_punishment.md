# Audit: Heretic's Punishment

## Oracle Reference (Scryfall)
- Cost: {4}{R}
- Type: Enchantment
- Oracle: "{3}{R}: Choose target creature or player. Reveal the top three cards of your library. Heretic's Punishment deals damage to that creature or player equal to the highest mana value among the revealed cards. Put those cards on the bottom of your library in any order."

## Implementation: heretics_punishment.rs

## Issues Found

1. **ISSUE: Missing damaged_by tracking for creature targets** - Lines 82-85 mark damage on creature targets (`obj.damage_marked += max_mv`) but do NOT push to `obj.damaged_by`. This means effects tracking damage sources won't work correctly.

2. **ISSUE: Revealed cards go to graveyard instead of bottom of library** - Oracle says "Put those cards on the bottom of your library in any order." The implementation (lines 113-117) moves the revealed cards to the graveyard (Zone::Graveyard) instead of the bottom of the library. The comment on line 112 even says "Mill the revealed cards (move to graveyard per current Oracle errata)" but this is INCORRECT -- the current Oracle text still says bottom of library, not graveyard.

3. **ISSUE: Damage not dealt when max_mv is 0** - Line 78 checks `if max_mv > 0` before dealing damage, but the ability should still resolve even if all revealed cards have MV 0 (it would just deal 0 damage). While dealing 0 damage is functionally irrelevant for life totals, it matters for triggers that care about damage being dealt.

Otherwise correct: cost ({4}{R}), type (Enchantment), activated ability cost ({3}{R}), target requirement (AnyTarget), reveal mechanic.

## Verdict: ISSUES FOUND (3 issues, including critical library-vs-graveyard bug)
