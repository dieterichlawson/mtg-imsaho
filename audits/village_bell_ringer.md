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

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Flash (You may cast this spell any time you could cast an instant.) / When this creature enters, untap all creatures you control.
**Mana cost**: {2}{W}
**Type line**: Creature — Human Scout
**P/T**: 1/4
**Status**: ISSUE
### Code issues
1. **Oracle text string mismatch**: Oracle says `"When this creature enters, untap all creatures you control."` but code has `"When Village Bell-Ringer enters the battlefield, untap all creatures you control."`. The oracle template was updated to use "this creature enters" instead of the old "enters the battlefield" wording.
### Behavior
Behavior is correct: Flash keyword present, ETB untaps all creatures controlled by the controller (filters by zone == Battlefield, power.is_some(), and tapped). Logic is sound.

## Re-audit — 2026-04-02
**Status**: PASS
Oracle text updated to match Scryfall: "Flash (You may cast this spell any time you could cast an instant.)\nWhen this creature enters, untap all creatures you control." (was "Flash\nWhen Village Bell-Ringer enters the battlefield..."). Doc comment updated. Behavior unchanged.
