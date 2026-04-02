# Audit: Traitorous Blood

## Scryfall Reference
- **Name:** Traitorous Blood
- **Cost:** {1}{R}{R}
- **Type:** Sorcery
- **Oracle:** Gain control of target creature until end of turn. Untap it. It gains trample and haste until end of turn.
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/traitorous_blood.rs`
- Name: "Traitorous Blood" -- MATCH
- Cost: {1}{R}{R} -- MATCH
- Types: Sorcery -- MATCH
- Target: Creature -- MATCH
- Behavior: Changes controller, untaps, grants haste + trample until EOT -- MATCH
- Stores original controller for revert at end of turn -- CORRECT

### Note
- Struct name is misspelled as `TraiterousBlood` (should be `TraitorousBlood`). This is a cosmetic code issue only; the card name string is correct.

## Verdict
**PASS** — Correctly implements the Threaten variant. Struct typo noted but harmless.

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Gain control of target creature until end of turn. Untap it. It gains trample and haste until end of turn.
**Type line**: Sorcery
**Mana Cost**: {1}{R}{R}
**Status**: ISSUE
### Code issues
1. **Oracle text wording mismatch**: Oracle says `"Untap it. It gains trample and haste until end of turn."` but code has `"Untap that creature. It gains haste and trample until end of turn."`. Two differences: "it" vs "that creature", and keyword order "trample and haste" vs "haste and trample".
2. **Struct name typo**: Struct is named `TraiterousBlood` (misspelled) instead of `TraitorousBlood`. This is cosmetic but incorrect.
### Behavior
Correct. on_resolve gains control of target creature, untaps it, grants haste and trample until end of turn. Control change is tracked for end-of-turn revert. All mechanical behavior matches oracle.

## Re-audit — 2026-04-02
**Status**: PASS
Oracle text updated to match Scryfall: "Untap it. It gains trample and haste until end of turn." (was "Untap that creature. It gains haste and trample until end of turn."). Doc comment updated. Behavior unchanged.
