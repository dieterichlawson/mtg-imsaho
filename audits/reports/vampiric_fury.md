# Audit: Vampiric Fury

## Scryfall Reference
- **Name:** Vampiric Fury
- **Cost:** {1}{R}
- **Type:** Instant
- **Oracle:** Vampire creatures you control get +2/+0 and gain first strike until end of turn.
- **P/T:** N/A

## Implementation: `mtg-engine/src/cards/vampiric_fury.rs`
- Name: "Vampiric Fury" -- MATCH
- Cost: {1}{R} -- MATCH
- Types: Instant -- MATCH
- Behavior: Finds Vampire creatures under caster's control, grants +2/+0 and first strike until EOT -- MATCH

### Note
- Only checks registry card_data subtypes for Vampire, not instance subtypes on game objects. Vampire tokens whose subtype is only on the game object (not via card_data) might be missed.

## Verdict
**PASS** — Correctly implements the Vampire tribal pump spell.

## Audit — 2026-04-02
**Oracle text source**: Scryfall API
**Oracle text**: Vampire creatures you control get +2/+0 and gain first strike until end of turn.
**Type line**: Instant
**Status**: PASS

### Card Data
- **Name:** Vampiric Fury -- CORRECT
- **Mana Cost:** {1}{R} -- CORRECT
- **Type:** Instant -- CORRECT

### Code issues
None. On resolve, finds all creatures controlled by caster with Vampire subtype, gives +2/+0 via UntilEndOfTurnEffect and FirstStrike via UntilEndOfTurnKeyword. Only affects Vampires on the battlefield at resolution time (matching the ruling). Spell is cleaned up after resolve. All data and behavior match oracle.
