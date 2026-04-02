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

## Audit — 2026-04-02

**Oracle text (Scryfall, cached 2026-04-01)**:
> When this artifact enters, destroy all Curses attached to you.
> You have hexproof. (You can't be the target of spells or abilities your opponents control, including Aura spells.)

**Type line**: Artifact
**Status**: PASS

### Card Data
- Name: "Witchbane Orb" -- correct
- Cost: {4} (Generic 4) -- correct
- Type: Artifact -- correct
- No supertypes or subtypes -- correct

### ETB Trigger: Destroy All Curses
- `triggered_abilities` contains one `TriggerKind::EntersBattlefield` entry -- correct.
- `on_enter_battlefield` filters battlefield objects where `attached_to_player == Some(controller)` and subtype is `"Curse"`, then calls `try_destroy` on each -- correct.
- Curse subtype check matches how curses declare themselves (e.g., `subtypes: vec!["Aura".into(), "Curse".into()]`).

### Continuous Effect: Player Hexproof
- `grants_player_hexproof()` returns `true`.
- Engine's `player_has_hexproof` (state.rs:1070-1079) checks battlefield permanents controlled by the player.
- `can_target_player` (engine.rs:714) blocks opponent targeting of hexproof players; self-targeting is allowed -- correct per rules.

### Cosmetic Note
Code oracle text: "When Witchbane Orb enters the battlefield, destroy all Curses attached to you."
Scryfall oracle text: "When this artifact enters, destroy all Curses attached to you."
This is a WotC template modernization with no rules impact.

### Tests (all pass)
- `grants_player_hexproof` -- controller gains hexproof, opponent does not.
- `opponent_cannot_target_hexproof_player` -- opponent's player-targeting spell blocked.
- `can_target_self_with_hexproof` -- player can still target themselves.
- `witchbane_orb_card_data` -- card type and mana value.

### Verdict
No functional mismatches. Implementation is correct.

---

## Audit 2026-04-02

### Oracle Text (Scryfall, cached 2026-04-01)
```
When this artifact enters, destroy all Curses attached to you.
You have hexproof. (You can't be the target of spells or abilities your opponents control, including Aura spells.)
```

### Card Data
- Name: "Witchbane Orb" -- correct.
- Cost: `Generic(4)` -- correct ({4}).
- Type: `Artifact` -- correct.
- Supertypes/subtypes: empty -- correct.
- Power/toughness: None -- correct (not a creature).

### Oracle Text Mismatch (Cosmetic)
Code: `"When Witchbane Orb enters the battlefield, destroy all Curses attached to you."`
Scryfall: `"When this artifact enters, destroy all Curses attached to you."`
The current oracle template uses "this artifact enters" rather than the card name + "enters the battlefield". No functional impact.

### ETB Trigger: Destroy All Curses
- `triggered_abilities` contains one `TriggerKind::EntersBattlefield` entry -- correct.
- `on_enter_battlefield` determines the controller, then filters all battlefield objects where `attached_to_player == Some(controller)` and subtype contains `"Curse"`. Calls `try_destroy` on each match. This is correct behavior.
- Subtype check (`s == "Curse"`) matches how curse cards declare themselves (e.g., `subtypes: vec!["Aura".into(), "Curse".into()]`).
- Uses `try_destroy` (respects indestructible/regeneration) -- correct; the oracle says "destroy".

### Continuous Effect: Player Hexproof
- `grants_player_hexproof()` returns `true`.
- Engine's `player_has_hexproof` (state.rs:1071) checks all battlefield permanents controlled by the player for this trait -- correct.
- `can_target_player` (engine.rs:714) blocks opponent targeting of hexproof players; self-targeting still allowed -- correct per rules.
- Hexproof reminder text "(You can't be the target of spells or abilities your opponents control, including Aura spells.)" is not stored but does not affect gameplay logic.

### Test Coverage
- `grants_player_hexproof` -- controller gains hexproof, opponent does not.
- `opponent_cannot_target_hexproof_player` -- opponent's player-targeting spell blocked.
- `can_target_self_with_hexproof` -- player can still target themselves.
- `witchbane_orb_card_data` -- card type and mana value.
- **Missing**: No test for the ETB trigger that destroys curses attached to the controller.

### Verdict
No functional mismatches. Implementation is correct. One cosmetic oracle text difference (old "enters the battlefield" template vs current "enters"). Test gap: ETB curse destruction is untested.
