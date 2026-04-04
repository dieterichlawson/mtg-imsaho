## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Return target exiled card with flashback you own to your hand.
**Type line**: Sorcery
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked

- **"you own" ownership filter**: The `is_valid_target` check at `runic_repetition.rs:39` includes `o.owner == caster`, correctly restricting targeting to cards the caster owns. At resolution, `move_object(*target_id, Zone::Hand)` places the card in `Zone::Hand`; since `objects_in_zone(Zone::Hand, player)` filters by `obj.owner == player` (state.rs:603), the card automatically appears in the caster's hand. Because the target must be owned by the caster (checked at cast time), and ownership never changes, this is always correct. PASS

- **"with flashback" targeting filter — native vs. granted**: `is_valid_target` checks `registry.card_data(o.card_id).map(|d| d.flashback_cost.is_some())` (runic_repetition.rs:40-42), which only reads the card's native, permanent flashback cost from the registry. It does NOT check `state.until_end_of_turn_flashback` (the Snapcaster Mage temporary grant). Per the ruling "An effect that gives flashback to an instant or sorcery card in your graveyard stops applying once that card has left the stack. The card won't have flashback while exiled and can't be the target of Runic Repetition (unless it naturally has flashback)." — this behavior is correct. PASS

- **"any reason" exile (not just cast-with-flashback)**: The targeting check looks only at `o.zone == Zone::Exile`, not at `o.cast_with_flashback`. A card with native flashback that was exiled by Sever the Bloodline, Fiend Hunter, etc. is correctly targetable. Matches ruling "The card could have been exiled for any reason, not just because it was cast using flashback." PASS

- **Fizzle if target leaves exile**: `is_target_legal` in `stack.rs:32` checks `TargetRequirement::ExileCard => obj.zone == Zone::Exile`. If the target card is no longer in exile at resolution time, the spell correctly fizzles per CR 608.2b. PASS

- **`move_spell_after_resolve` cleanup**: `on_resolve` (runic_repetition.rs:57) correctly calls `state.move_spell_after_resolve(object_id)`, so Runic Repetition itself goes to graveyard (or exile if cast via its own flashback — but Runic Repetition has no flashback, so always graveyard). PASS

- **Face-down exile**: Per the ruling "A card that's exiled face down doesn't have any characteristics or abilities, so it can't be the target of Runic Repetition." The engine has no face-down exile mechanic (no `face_down` field on `GameObject`), and no Innistrad card in the current registry exiles cards face-down. This ruling has no applicable scenario in the current implementation — not an issue. PASS

- **Target must be a card (not a token in exile)**: The `registry.card_data(o.card_id)` lookup returns `None` for tokens (which have `card_id = CardId(0)`, a sentinel with no registry entry), causing `flashback_cost.is_some()` to return `false` via `.unwrap_or(false)`. Tokens therefore can never be valid targets for Runic Repetition, which is correct. PASS

- **Snapcaster Mage exiled card**: If Snapcaster Mage grants flashback to, say, Divination (no native flashback), and Divination is cast with that temporary flashback and then exiled, Runic Repetition cannot target the exiled Divination because `registry.card_data(divination_id).map(|d| d.flashback_cost.is_some())` returns `false`. The `until_end_of_turn_flashback` entry still exists until end of turn but is not consulted by Runic Repetition's targeting — correct per ruling. PASS

### Test coverage

- Card type and mana cost (Sorcery, {2}{U}): `innistrad_simple_cards.rs:543` — TESTED
- Returns exiled card with flashback to hand: `innistrad_simple_cards.rs:552` — TESTED
- Non-flashback card in exile cannot be targeted: NOT TESTED
- Fizzle if target leaves exile before resolution: NOT TESTED
- Card exiled by means other than flashback casting (any-reason exile): PARTIALLY TESTED (test manually places Think Twice in Zone::Exile, covering this case)
- Card granted flashback by Snapcaster then exiled cannot be targeted: NOT TESTED
- Face-down exile (no native flashback characteristics): NOT TESTED (not applicable in current engine)
- Token in exile cannot be targeted: NOT TESTED
