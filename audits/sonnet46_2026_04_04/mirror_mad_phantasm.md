## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
{1}{U}: This creature's owner shuffles it into their library. If that player does, they reveal cards from the top of that library until a card named Mirror-Mad Phantasm is revealed. The player puts that card onto the battlefield and all other cards revealed this way into their graveyard.
**Type line**: Creature — Spirit
**Status**: ISSUE

### Code issues

- **Reveal loop uses `draw_top_card()` which sets `has_drawn_from_empty = true` on library exhaustion, causing the player to incorrectly lose the game**
  - File: `mtg-engine/src/cards/isd/mirror_mad_phantasm.rs`, lines 83–97 (loop), exercising `mtg-engine/src/state.rs` line 1315
  - Oracle text says: `"they reveal cards from the top of that library until a card named Mirror-Mad Phantasm is revealed. The player puts that card onto the battlefield and all other cards revealed this way into their graveyard."` — Ruling clarifies: `"If no card named Mirror-Mad Phantasm is revealed (possibly because it was a card copying Mirror-Mad Phantasm or it was a token), all cards from that library will be put into their owner's graveyard."` (game continues, player does not lose)
  - Code does: `state.get_player_mut(owner).draw_top_card()` is called in the reveal loop. `draw_top_card()` in `state.rs:1314-1316` contains `if self.library_order.is_empty() { self.has_drawn_from_empty = true; None }`. When the library is exhausted without finding Mirror-Mad Phantasm, `has_drawn_from_empty` is set to `true`. SBAs in `sba.rs:37-46` then mark the player as having lost: `if !lost && drawn_empty { state.players[i].lost = true; ... }`. This turns a legal game event (milling your entire library without finding the card) into an erroneous loss condition. The ruling explicitly states all milled cards go to graveyard and the game continues. This affects any scenario where the library is exhausted: e.g., Necrotic Ooze gains the ability with no actual Mirror-Mad Phantasm in the library.

- **Token copy of Mirror-Mad Phantasm activating the ability is incorrectly found in the reveal loop and enters the battlefield**
  - File: `mtg-engine/src/cards/isd/mirror_mad_phantasm.rs`, lines 69–115; SBA check in `mtg-engine/src/sba.rs` line 307–314 runs only after the ability resolves
  - Oracle text ruling says: `"If no card named Mirror-Mad Phantasm is revealed (possibly because it was a card copying Mirror-Mad Phantasm or it was a token), all cards from that library will be put into their owner's graveyard."` — when a token is the source, all cards should be milled
  - Code does: `state.move_object(object_id, Zone::Library)` moves the token to Zone::Library (line 69). SBAs do not run until after `on_activate_ability` returns. During the loop, `draw_top_card()` returns the token's id (its `name` field is `"Mirror-Mad Phantasm"`), the condition `if name == "Mirror-Mad Phantasm"` (line 88) is true, and `found = Some(card_id)` is set. The token is then moved to the battlefield (line 106). SBAs at rule 704.5d (`o.is_token && o.zone != Zone::Battlefield` → `state.objects.remove`) do not fire because the token has already re-entered the battlefield. Per the ruling the token should have ceased to exist when it left the battlefield, the loop should find no Mirror-Mad Phantasm, and all library cards should be milled.

### Tricky interactions checked

- **Owner vs. controller for library operations**: The code correctly reads `o.owner` (not `o.controller`) for all library-manipulation operations (shuffling into owner's library, revealing from owner's library, putting found card on battlefield under owner's control). The engine restricts ability activation to the controlling player via `objects_in_zone(Zone::Battlefield, player)`. PASS.
- **"If that player does" conditional clause**: The shuffle always succeeds in this engine (no shuffle-prevention mechanics). Code unconditionally executes the reveal loop, which is correct for all reachable game states. PASS.
- **Sorcery-speed restriction**: The oracle text has no sorcery-speed restriction. `sorcery_speed_only: false` is correctly set. PASS.
- **Tap cost**: Not required by the oracle text. `requires_tap: false` is correctly set. PASS.
- **Once-per-turn restriction**: Not present in the oracle text. `once_per_turn: false` is correctly set. PASS.
- **Library zone change + library_order consistency**: `move_object(phantasm, Library)` changes zone only; the card then manually pushes to `library_order` and shuffles. The two-step approach is internally consistent. PASS.
- **ETB re-entry triggers**: When the phantasm re-enters the battlefield via `move_object(phantasm, Battlefield)`, a `GameEvent::EnteredBattlefield` is emitted. `collect_triggers` re-reads the object's zone and controller from live state (not from the event struct), so the corrected `controller = owner` is used. Mirror-Mad Phantasm has no `triggered_abilities` in `card_data()`, so no spurious ETB effects fire. PASS.
- **`has_drawn_from_empty` for normal case (MMP finds itself)**: In normal gameplay where Mirror-Mad Phantasm is the source, it is always present in the library after the shuffle. The reveal loop finds it before the library is exhausted. `draw_top_card()` is never called on an empty library. `has_drawn_from_empty` is never set. The bug is latent and only triggers in edge cases. PASS (normal case only).
- **`has_drawn_from_empty` for exhaustion case**: When the library is exhausted without finding Mirror-Mad Phantasm (e.g., Necrotic Ooze has the ability and no actual Mirror-Mad Phantasm card exists in the library), `draw_top_card()` at line 84 sets `has_drawn_from_empty = true`, causing the player to lose via SBAs after the ability resolves. Per the ruling, all cards should go to graveyard and the game continues. FAIL — flagged as Issue 1 above.
- **Token copy activating the ability**: A token copy of Mirror-Mad Phantasm (e.g., created by Cackling Counterpart) shares the card's `card_id` and therefore has the same `activated_abilities`. When it activates, `move_object(token, Library)` runs but SBAs have not yet removed the token. The reveal loop finds the token by name and puts it on the battlefield instead of milling everything. FAIL — flagged as Issue 2 above.
- **Milled cards' graveyard ownership**: Cards revealed and milled are owned by the same player whose library was searched (the owner). `move_object(card_id, Zone::Graveyard)` leaves `obj.owner` unchanged, so they go to the correct graveyard. PASS.
- **Zone change count / new-object identity**: Each call to `move_object` increments `zone_change_count`, correctly making each zone change produce a new "object" identity per MTG rules. PASS.
- **`summoning_sick` on re-entry**: `move_object(phantasm, Battlefield)` at line 503 of `state.rs` sets `summoning_sick = true` for the re-entering creature. Mirror-Mad Phantasm gets summoning sickness upon re-entering. Per MTG rules, a creature entering the battlefield without haste has summoning sickness. PASS.
- **Ruling — Necrotic Ooze gains ability, looks for "Mirror-Mad Phantasm" by name**: The reveal loop checks `if name == "Mirror-Mad Phantasm"` (line 88), which is name-based regardless of which card actually has the ability. This correctly matches the ruling: "its owner reveals cards until they reveal a card named Mirror-Mad Phantasm." PASS (conditional on Issue 1 not triggering first).
- **Card data fields**: mana cost `{3}{U}{U}` matches `[Generic(3), Blue, Blue]`; P/T 5/1; type `Creature`; subtype `Spirit`; keyword `Flying`. All correct. PASS.

### Test coverage

- Normal activation (MMP finds itself): `mtg-engine/tests/tier15_cards.rs:2553` — TESTED (but bypasses engine, does not go through `submit_action` + SBA loop; mana payment not verified)
- Library-exhausted case (no MMP found, all cards milled, game continues): NOT TESTED
- Token copy activating the ability (all cards milled, not found): NOT TESTED
- Necrotic Ooze or other creature gaining the ability: NOT TESTED
- Controlling-player-is-not-owner scenario (opponent controls MMP, activates it): NOT TESTED
- ETB triggers firing on re-entry: NOT TESTED
- Cards below the found MMP remain in library (not graveyard): NOT TESTED (only assertions on "graveyard or library" due to shuffle randomness, not a strict check)
