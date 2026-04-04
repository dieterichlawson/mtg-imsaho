## Audit — 2026-04-04 11:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: If a Zombie you control would deal combat damage to a player, instead that player mills that many cards. Whenever a creature card is put into an opponent's graveyard from their library, exile that card and create a 2/2 black Zombie creature token.
**Type line**: Creature — Zombie
**Status**: ISSUE

### Code issues

- **Second triggered ability only fires from Undead Alchemist's own mill, not from all sources** (`mtg-engine/src/cards/isd/undead_alchemist.rs:82-99`)
  - Oracle text says: `"Whenever a creature card is put into an opponent's graveyard from their library, exile that card and create a 2/2 black Zombie creature token."`
  - Code does: The exile-and-create-token logic is implemented inline inside `on_any_combat_damage_to_player` (lines 82–99), which is only called when a Zombie deals combat damage. There is no separate triggered ability watching for library-to-graveyard movement from any source. The `TriggerKind` enum has no variant for "creature card put into graveyard from library," and `triggers.rs` handles no such event. If another mill source (e.g., `Curse of the Bloody Tome`, `mill_cards` called from any other effect) moves creature cards from an opponent's library to their graveyard, the second ability of Undead Alchemist never fires.

- **Multiple Undead Alchemists cause incorrect life restoration (net life gain) and double milling** (`mtg-engine/src/cards/isd/undead_alchemist.rs:63-99`)
  - Oracle text says (ruling 2011-09-22): `"If you control multiple Undead Alchemists, the multiple replacement abilities will have no added effect. Combat damage dealt to a player by a Zombie you control will be replaced only once with cards being put into that player's graveyard."`
  - Code does: Each Alchemist independently registers as a `TriggerKind::AnyCombatDamageToPlayer` watcher. When a Zombie deals X damage, triggers.rs creates one `CombatDamageWatch` trigger per Alchemist on the battlefield. Each trigger resolves `on_any_combat_damage_to_player` independently. With two Alchemists and a player at life L before damage: (1) damage reduces life to L−X; (2) Alchemist 1's trigger fires: reads `current_life = L−X`, restores to `L−X+X = L`, mills X cards; (3) Alchemist 2's trigger fires: reads `current_life = L` (already restored), restores to `L+X` (net life gain of X), mills X more cards. Net result: opponent gains X life and is milled 2X cards — both wrong per the ruling.

- **First-strike Zombie dealing lethal combat damage causes player loss before Alchemist trigger fires** (`mtg-engine/src/combat.rs:146-153`, `mtg-engine/src/cards/isd/undead_alchemist.rs:45-105`)
  - Oracle text says: `"If a Zombie you control would deal combat damage to a player, instead that player mills that many cards."`
  - Code does: In `combat.rs::deal_combat_damage` (line 146–147), after the first-strike damage step, SBAs are run synchronously (`while crate::sba::check_state_based_actions_with_registry(state, Some(registry)) {}`) with no trigger processing between them. If a Zombie with First Strike deals lethal combat damage to a player (reducing their life to ≤ 0), `sba.rs` sets `player.lost = true` and `state.result = Some(Winner(...))`. Control then returns to `engine.rs`, which calls `triggers::process_triggers` — the Alchemist's `CombatDamageWatch` trigger fires and restores the life, but `player.lost` is already `true` and `state.result` is already set. The player incorrectly loses the game. The replacement effect should have prevented any life loss from the first-strike damage, but because the replacement is modeled as a post-damage trigger, the SBA check sees 0 life first.

- **Lifelink on the Zombie source incorrectly grants life when Undead Alchemist's replacement applies** (`mtg-engine/src/combat.rs:539-549`, `mtg-engine/src/cards/isd/undead_alchemist.rs:45-105`)
  - Oracle text says: `"If a Zombie you control would deal combat damage to a player, instead that player mills that many cards."` — no damage is dealt; the event is replaced.
  - Code does: `deal_damage_to_player` applies lifelink gain immediately when damage is dealt (lines 539–549 of `combat.rs`), before any trigger fires. Because the Alchemist's replacement is modeled as a trigger rather than a true replacement effect, the Zombie's controller gains life equal to the combat damage amount (via lifelink) even though per oracle text no damage was actually dealt. The Alchemist trigger later restores the damaged player's life, but it does not reverse the lifelink gain on the Zombie controller's side.

### Tricky interactions checked

- **Replacement effect modeled as trigger (fundamental design issue)**: FAIL — the first ability is a replacement effect ("instead"), but is implemented as a `TriggerKind::AnyCombatDamageToPlayer` watcher that fires after damage is applied, then "undoes" the life loss. This causes the four downstream issues listed above.
- **Multiple Alchemists — replacement applies only once per ruling**: FAIL — each Alchemist adds an independent trigger, resulting in multiple mills and net life gain (see Issue 2).
- **First-strike Zombie + lethal combat damage**: FAIL — SBAs fire before Alchemist trigger in the first-strike damage path, killing the player before the life can be restored (see Issue 3).
- **Lifelink Zombie + Undead Alchemist**: FAIL — lifelink gain applies even though no damage is dealt per oracle text (see Issue 4).
- **Second ability triggering from non-combat mill sources**: FAIL — the triggered ability is not a standalone engine trigger and does not fire when other effects (Curse of the Bloody Tome, etc.) mill creature cards from an opponent's library (see Issue 1).
- **Ruling: subsequent Alchemist triggers create tokens even if creature already exiled**: NOT TESTED — the ruling states that if multiple Alchemists trigger on the same creature card reaching the graveyard, later triggers still create a token even if the first already exiled the card. The current bundled implementation does not distinguish these cases.
- **Source Zombie's subtype check covers both registry and object subtypes**: PASS — the code checks `registry.card_data(source.card_id).map(|d| d.subtypes...)` OR `source.subtypes.iter().any(...)` (lines 55–58), correctly covering both registry-backed cards and token Zombies.
- **Alchemist leaves battlefield before trigger resolves**: PASS — the handler checks `o.zone == Zone::Battlefield` for `self_id` at line 47, returning early if the Alchemist is no longer on the battlefield.
- **Library-order convention consistent with engine mill**: PASS — both the card code (`library_order[..mill_count]` + `remove(0)`) and `engine::mill_cards` (`library_order.remove(0)`) take from index 0, consistent with the "top of library" convention.
- **Milled non-creature cards stay in graveyard**: PASS — the creature check at lines 83–89 only exiles cards that are creatures; non-creature cards remain in the graveyard as expected.
- **"Opponent" scope of the mill**: PASS (for two-player games) — the combat damage event requires an attacker to deal combat damage to the defending player, who is necessarily an opponent.
- **Normal (no first-strike) path: life is restored before SBAs fire**: PASS — in the non-first-strike path, SBAs are NOT run inside `deal_combat_damage`; they run only after `process_triggers` in `engine.rs`. So the Alchemist trigger fires and restores life before SBAs check the player's total.

### Test coverage

- **Mill-instead-of-damage basic case**: `mtg-engine/tests/tier15_cards.rs:416` (`undead_alchemist_mills_instead_of_damage`) — TESTED (single Alchemist, non-creature cards are NOT present, normal damage path only)
- **Creature cards exiled and tokens created**: `mtg-engine/tests/tier15_cards.rs:416` — TESTED (single Alchemist, two Kalonian Tuskers in library)
- **Second ability triggers from non-combat mill source**: NOT TESTED
- **Multiple Undead Alchemists (no added effect on replacement)**: NOT TESTED
- **Multiple Undead Alchemists (each creates a token per ruling)**: NOT TESTED
- **First-strike Zombie + Undead Alchemist (lethal damage)**: NOT TESTED
- **Lifelink Zombie + Undead Alchemist (no lifelink gain)**: NOT TESTED
- **Ruling: subsequent trigger creates token even if card already exiled**: NOT TESTED
- **Alchemist leaves battlefield before trigger resolves**: NOT TESTED
