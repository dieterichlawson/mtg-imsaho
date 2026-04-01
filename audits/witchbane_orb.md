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

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: When this artifact enters, destroy all Curses attached to you. You have hexproof. (You can't be the target of spells or abilities your opponents control, including Aura spells.)
**Scryfall type line**: Artifact
**Status**: PASS

Previous player hexproof issue has been fixed. The implementation now has `fn grants_player_hexproof(&self) -> bool { true }` which the engine uses to grant hexproof to the controller.

Verified correct:
- Mana cost: {4} -- matches
- Types: Artifact -- matches
- ETB trigger: destroys all Curses attached to controller via `try_destroy` -- correct
- Player hexproof: granted via `grants_player_hexproof()` returning `true` -- correct
- `triggered_abilities`: EntersBattlefield -- correct
- Note: oracle text in code uses older templating ("When Witchbane Orb enters the battlefield" vs Scryfall's "When this artifact enters") -- cosmetic only, no functional impact
- Note: `continuous_effects` is empty but hexproof is handled via a dedicated trait method, which is an acceptable pattern
- No anti-patterns detected
- Tests found in `mtg-engine/tests/witchbane_orb.rs` and `mtg-engine/tests/innistrad_simple_cards.rs`

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch
**Oracle text**: When this artifact enters, destroy all Curses attached to you. You have hexproof. (You can't be the target of spells or abilities your opponents control, including Aura spells.)
**Type line**: Artifact
**Status**: PASS

Card data correct: name, mana cost ({4}), type (Artifact).

ETB trigger: finds all Curses attached to controller, destroys via try_destroy. Correct.

Player hexproof: grants_player_hexproof() returns true, used by engine's player_has_hexproof check. Correct.

Oracle text in code uses older "enters the battlefield" template vs current "enters" -- cosmetic only.

Tests in witchbane_orb.rs cover player hexproof grant, opponent targeting prevention, and self-targeting still allowed. Thorough. No anti-patterns found.
