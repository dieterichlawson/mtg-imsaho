## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Deathtouch
When this creature dies, you may search your library for a Curse card, put it onto the battlefield attached to target player, then shuffle.
**Type line**: Creature — Human Shaman
**Status**: PASS
### Code issues
No issues found.

## Audit — 2026-04-02 (full-reaudit)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 20:33

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Deathtouch\nWhen this creature dies, you may search your library for a Curse card, put it onto the battlefield attached to target player, then shuffle.
**Type line**: Creature — Human Shaman
**Status**: PASS

### Code issues
No issues found.

Note: The `oracle_text` field in code (line 47) uses "When Bitterheart Witch dies" instead of the current Scryfall templating "When this creature dies". This is cosmetic only — both are functionally identical and older printings used the card name. The target player is chosen during resolution rather than at trigger time, which is an engine-wide simplification affecting all targeted triggers, not a card-specific bug.

### Tricky interactions checked
- "You may" opt-out skips search and shuffle correctly: PASS (lines 76-78 return early without shuffling; this is correct since "then shuffle" is part of the search action)
- No Curses in library: PASS (lines 93-101 log the failure and still shuffle, which is correct since you chose to search)
- Single Curse auto-selects, multiple Curses present choice: PASS (lines 103-125 handle both paths)
- Curse attached to any player (including self): PASS (engine lines 2598-2614 move curse to battlefield with `attached_to_player` set)
- Curse search checks subtype "Curse" correctly: PASS (line 87 filters by subtype matching "Curse", consistent with how Curse cards are registered)
- Library is shuffled after attaching curse: PASS (engine line 2612-2613 shuffles after attaching)

### Test coverage
- Death trigger presents YesNo choice then attaches curse to opponent: `tier15_cards.rs:176`
- Curse can be attached to self (controller): `tier15_cards.rs:217`
- Declining search leaves curse in library: `tier15_cards.rs:252`
- Multiple Curses in library (player picks which): NOT TESTED
- No Curses in library (shuffle only): NOT TESTED

## Audit — 2026-04-02 21:23

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Deathtouch
When this creature dies, you may search your library for a Curse card, put it onto the battlefield attached to target player, then shuffle.
**Type line**: Creature — Human Shaman
**Status**: PASS

### Code issues
No issues found.

The `oracle_text` field in code (line 47) uses "When Bitterheart Witch dies" instead of "When this creature dies". This is cosmetic only -- both are functionally identical and older printings used the card name.

Target player is chosen during resolution rather than at trigger time (MTG rules technically require targets to be chosen when the ability goes on the stack). This is an engine-wide simplification affecting all targeted triggers, not a card-specific bug.

### Tricky interactions checked (min 3)
1. **"You may" opt-out**: Declining search returns immediately without shuffling (lines 76-78). Correct -- "then shuffle" is part of the search action, so no search means no shuffle.
2. **No Curses in library**: If player says yes but no Curses exist, the library is still shuffled (lines 93-101). Correct -- the player searched (looked through the library) even though nothing was found.
3. **Curse identification**: Searches for cards with subtype "Curse" (line 87). All Curse cards in the engine (Curse of the Pierced Heart, Curse of Death's Hold, Curse of Oblivion, Curse of the Bloody Tome, Curse of the Nightly Hunt, Curse of Stalked Prey) register "Curse" as a subtype. Correct.
4. **Curse bypasses the stack**: Per rulings, the Curse is put directly onto the battlefield (not cast), so it cannot be countered. The implementation does this correctly via `move_object` to Zone::Battlefield (engine line 2603) rather than casting. Correct.
5. **Attach to any player including self**: Player can choose any player including themselves. Verified by test at tier15_cards.rs:217. Correct.
6. **Multiple Curses**: When multiple Curses exist in library, player is presented a choice (lines 107-125). Single Curse auto-selects (lines 103-106). Both paths then present the player target choice. Correct.

### Test coverage
- Death trigger presents YesNo choice then attaches curse to opponent: `tier15_cards.rs:176`
- Curse can be attached to self (controller): `tier15_cards.rs:217`
- Declining search leaves curse in library: `tier15_cards.rs:252`
- Multiple Curses in library (player picks which): NOT TESTED
- No Curses in library (shuffle only): NOT TESTED
