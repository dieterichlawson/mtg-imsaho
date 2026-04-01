# Audit: Geist of Saint Traft

## Oracle Reference (Scryfall)
- Cost: {1}{W}{U}
- Type: Legendary Creature -- Spirit Cleric
- P/T: 2/2
- Oracle: "Hexproof
  Whenever Geist of Saint Traft attacks, create a 4/4 white Angel creature token with flying that's tapped and attacking. Exile that token at end of combat."

## Implementation: geist_of_saint_traft.rs

## Issues Found

1. **ISSUE: Token exiled at end step instead of end of combat** - Oracle says "Exile that token at end of combat." The implementation uses on_end_step (line 89), which is the end step, not end of combat. The comment on line 90 even acknowledges this: "Exile the angel token at end of combat (simplified: at end step)." This means the Angel token survives longer than it should -- it can block during the opponent's next combat if there's a way to untap it, and it persists through the second main phase.

2. **ISSUE: Triggered ability incorrectly uses EndStep TriggerKind** - The triggered_abilities list includes a TriggerKind::EndStep entry for the exile effect (line 35-38). The correct trigger timing is end of combat, not end step.

Otherwise correct: cost, types (Legendary Spirit Cleric), P/T (2/2), hexproof keyword, 4/4 white Angel token with flying, token enters tapped and attacking.

## Verdict: ISSUES FOUND (2 issues - timing)

## Audit — 2026-04-01 09:00

**Scryfall Oracle text**: Hexproof. Whenever Geist of Saint Traft attacks, create a 4/4 white Angel creature token with flying that's tapped and attacking. Exile that token at end of combat.
**Scryfall type line**: Legendary Creature -- Spirit Cleric
**Status**: PASS

Previous timing issues have been fixed. The implementation now uses `TriggerKind::EndCombat` and `on_end_combat` (instead of EndStep/on_end_step).

Verified correct:
- Mana cost: {1}{W}{U} -- matches
- Types: Legendary Creature -- matches
- Subtypes: Spirit, Cleric -- matches
- P/T: 2/2 -- matches
- Keywords: Hexproof -- matches
- `triggered_abilities`: Attacks + EndCombat -- correct
- `on_attacks`: creates 4/4 white Angel creature token with flying, tapped and attacking -- correct. Token has "Angel" subtype -- correct.
- `on_end_combat`: exiles the Angel token -- correct timing now
- `on_resolve`: moves to battlefield, sets `is_legendary` -- correct
- No anti-patterns detected
- Tests found in `mtg-engine/tests/tier15_cards.rs`

## Audit — 2026-04-01 10:00

**Oracle text source**: Scryfall card page via WebSearch
**Oracle text**: Hexproof (This creature can't be the target of spells or abilities your opponents control.) / Whenever Geist of Saint Traft attacks, create a 4/4 white Angel creature token with flying that's tapped and attacking. Exile that token at end of combat.
**Type line**: Legendary Creature — Spirit Cleric
**Status**: ISSUE

Card data correct: name, mana cost ({1}{W}{U}), supertypes (Legendary), subtypes (Spirit, Cleric), P/T (2/2), keywords (Hexproof).

triggered_abilities correctly declares Attacks and EndCombat triggers.

on_attacks creates a 4/4 white Angel token with flying, tapped and attacking. Token has correct subtypes.

on_end_combat correctly exiles the angel token.

Minor issues:
1. card_state.insert("angel_token", token_id) uses insert which overwrites previous values. If Geist attacks in multiple combat phases (extra combat steps), only the last angel token ID is tracked and earlier ones would not be exiled. Edge case but a potential bug.
2. Angel token always attacks state.opponent(controller) rather than allowing choice of defender. Per ruling: "You choose which player or planeswalker the Angel token is attacking." In a 2-player game this is functionally correct, but the implementation doesn't respect multiplayer choice.

Tests in tier15_cards.rs cover angel creation and exile at end of combat.
