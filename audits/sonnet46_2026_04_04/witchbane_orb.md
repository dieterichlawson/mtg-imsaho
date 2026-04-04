## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: When this artifact enters, destroy all Curses attached to you.
You have hexproof. (You can't be the target of spells or abilities your opponents control, including Aura spells.)
**Type line**: Artifact
**Status**: ISSUE

### Code issues

- **ETB trigger suppressed when Witchbane Orb leaves the battlefield before trigger resolves** — `mtg-engine/src/triggers.rs` lines 893–898
  - Oracle text says: `"When this artifact enters, destroy all Curses attached to you."`
  - Code does: `if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) { behavior.on_enter_battlefield(state, object_id, registry); }` — the "destroy Curses" effect is silently skipped if the Orb is no longer on the battlefield when the trigger resolves. Per CR 603.6a–e, an ETB trigger resolves regardless of whether its source subsequently left the battlefield. If an opponent destroys Witchbane Orb in response to the ETB trigger, the Curses are never destroyed.

- **Hexproof not re-validated at spell resolution for player targets** — `mtg-engine/src/stack.rs` line 39
  - Oracle text says: `"You have hexproof. (You can't be the target of spells or abilities your opponents control, including Aura spells.)"`
  - Code does: `Target::Player(_) => true` — `is_target_legal` unconditionally considers every player target legal without checking hexproof. Per CR 608.2b, if a target becomes illegal between when a spell was cast and when it resolves, the spell is countered. A Curse Aura spell that was cast before Witchbane Orb entered (targeting the player) will still resolve and attach even though the player now has hexproof, because the legality check at resolution never inspects `player_has_hexproof`. This is acknowledged as a known engine deficiency in `tests/spell_fizzle.rs` lines 186–225 (creature hexproof case), but the player hexproof case is similarly broken.

### Tricky interactions checked

- **ETB trigger fires for non-creature artifact**: PASS — `GameEvent::EnteredBattlefield` is emitted by `state.move_object` for any zone transition to Battlefield (state.rs lines 503–514), and the trigger collector in `triggers.rs` lines 344–364 fires for all registered cards (`registry.get(card_id).is_some()`), not just creatures.
- **Hexproof prevents opponent targeting at cast time**: PASS — `can_target_player` in `engine.rs` line 772–777 calls `state.player_has_hexproof`, which scans all battlefield permanents for `grants_player_hexproof() == true` (state.rs lines 1143–1152). Curses and player-targeting spells are correctly excluded from legal actions when the player controls Witchbane Orb.
- **Hexproof covers activated abilities**: PASS — `generate_ability_targets` in `engine.rs` lines 1279–1383 also calls `can_target_player` for `PlayerOnly`, `PlayerOrPlaneswalker`, and `AnyTarget` requirement types, covering activated ability targeting.
- **Hexproof covers Aura spells**: PASS at cast time — Curse Auras declare `TargetRequirement::PlayerOnly` and use the same `can_target_player` path; a hexproof player cannot be targeted when the Curse is cast. FAIL at resolution — see Issue 2 above.
- **ETB trigger won't fire if source left battlefield before resolution**: FAIL — see Issue 1 above.
- **Controller lookup in on_enter_battlefield if Orb is gone**: FAIL (secondary) — `on_enter_battlefield` in `witchbane_orb.rs` line 40 resolves the controller via `state.get_object(object_id)`, which returns `PlayerId(0)` if the Orb is gone; the engine guard in triggers.rs prevents this from ever executing, masking this secondary defect.
- **Curse subtype check is registry-only (misses tokens)**: NOT AN ISSUE in practice — `witchbane_orb.rs` lines 47–49 check only `registry.card_data(o.card_id).map(|d| d.subtypes.iter().any(|s| s == "Curse"))`. There are no Curse tokens in the current card pool; all Curses are registered cards with "Curse" in their registry subtypes. The pattern matches the analogous check in `bitterheart_witch.rs` line 87.
- **try_destroy respects indestructible**: PASS — `try_destroy` in `destruction.rs` line 33 checks `has_keyword(id, Keyword::Indestructible)` before destroying. The oracle text uses "destroy," which is correctly blocked by indestructible.
- **"you" means the controller, not a fixed player**: PASS — `on_enter_battlefield` in `witchbane_orb.rs` line 40 looks up the controller of the Orb at trigger resolution time, not a hardcoded player.
- **Self can target self despite hexproof**: PASS — `can_target_player` in `engine.rs` line 773 checks `target_player != caster` before blocking; the controlling player can still target themselves.

### Test coverage

- Hexproof granted to controlling player: `tests/witchbane_orb.rs:20` (`grants_player_hexproof`)
- Opponent cannot target hexproof player with spells at cast time: `tests/witchbane_orb.rs:34` (`opponent_cannot_target_hexproof_player`)
- Player can target themselves with hexproof: `tests/witchbane_orb.rs:62` (`can_target_self_with_hexproof`)
- ETB trigger destroys existing Curses: NOT TESTED
- Hexproof causes Curse spell to fizzle at resolution (target gained hexproof after cast): NOT TESTED (acknowledged bug documented for creature case at `tests/spell_fizzle.rs:192`)
- ETB trigger resolves after Orb leaves battlefield: NOT TESTED
- Basic card data (type, cost): `tests/innistrad_simple_cards.rs:653` (`witchbane_orb_card_data`)
