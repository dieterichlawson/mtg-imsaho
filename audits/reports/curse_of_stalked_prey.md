# Audit: Curse of Stalked Prey

## Scryfall Reference
- **Name:** Curse of Stalked Prey
- **Cost:** {1}{R}
- **Type:** Enchantment -- Aura Curse
- **Oracle:** Enchant player. Whenever a creature deals combat damage to enchanted player, put a +1/+1 counter on that creature.
- **P/T:** N/A
- **Keywords:** Enchant

## Implementation: `curse_of_stalked_prey.rs`
- **Name:** Curse of Stalked Prey -- CORRECT
- **Cost:** {1}{R} -- CORRECT
- **Type:** Enchantment -- CORRECT
- **Subtypes:** ["Aura", "Curse"] -- CORRECT
- **P/T:** N/A -- CORRECT
- **Target:** TargetRequirement::PlayerOnly -- CORRECT
- **Trigger:** AnyCombatDamageToPlayer -- CORRECT
- **Behavior:** Adds +1/+1 counter when creature deals combat damage to enchanted player -- CORRECT

## Issues
None

---

## Full Audit (2026-04-02)

### Oracle Text (Scryfall, cached 2026-04-01)
> Enchant player
> Whenever a creature deals combat damage to enchanted player, put a +1/+1 counter on that creature.

**Mana Cost:** {1}{R}
**Type Line:** Enchantment — Aura Curse
**Ruling (2011-09-22):** The ability will trigger when any creature deals combat damage to the enchanted player, including one controlled by another opponent or even by the enchanted player (if combat damage gets redirected somehow).

### Implementation: `mtg-engine/src/cards/isd/curse_of_stalked_prey.rs`

### Detailed Checklist

#### Card Data
- [x] Name: `"Curse of Stalked Prey"` -- correct
- [x] Mana cost: `Generic(1), Colored(Red)` -- matches {1}{R}
- [x] Card type: `Enchantment` -- correct
- [x] Subtypes: `["Aura", "Curse"]` -- correct
- [x] Oracle text string: matches Scryfall verbatim
- [x] Power/toughness: None -- correct (not a creature)

#### Enchant Player / Aura Curse
- [x] `target_requirement` returns `PlayerOnly` -- correct for "enchant player"
- [x] `on_resolve` calls `helpers::resolve_curse`, which attaches to a target player and moves to battlefield -- correct

#### Triggered Ability
- [x] Uses `TriggerKind::AnyCombatDamageToPlayer` -- correct; oracle says "whenever a creature deals combat damage", not restricted to controller's creatures
- [x] Engine dispatches via `PendingTrigger::CombatDamageWatch` to `on_any_combat_damage_to_player` -- correct plumbing
- [x] Checks `attached_to_player == Some(damaged_player)` -- correctly filters to enchanted player only
- [x] Checks source is still on the battlefield before adding counter -- correct (creature may have died in combat)
- [x] Adds exactly 1 `PlusOnePlusOne` counter to the damage source -- matches "put a +1/+1 counter on that creature"

#### Anti-Pattern Check
- [x] No controller restriction on which creatures trigger it -- correct per oracle and ruling
- [x] No creature-type restriction -- correct, oracle says "a creature" generically
- [x] Does not independently verify source is a creature type -- acceptable because `AnyCombatDamageToPlayer` in the engine is only dispatched for combat damage events which originate from creatures
- [x] `_amount` unused -- correct, the oracle effect does not scale with damage amount

#### Test Coverage
- `mtg-engine/tests/tier15_cards.rs`: `curse_of_stalked_prey_gives_counter_on_combat_damage` -- verifies curse attached to P1, creature deals combat damage to P1, attacker receives +1/+1 counter

#### LLM Integration
- No references in `mtg-player/src/llm.rs` -- not required for this card

### Verdict
**PASS** -- No issues found. Implementation correctly matches oracle text.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Enchant player\nWhenever a creature deals combat damage to enchanted player, put a +1/+1 counter on that creature.
**Type line**: Enchantment — Aura Curse
**Status**: PASS

### Code issues
No issues found. Card data matches: name, cost {1}{R}, subtypes Aura Curse, oracle text. Trigger AnyCombatDamageToPlayer correctly checks that the damaged player is the cursed player (attached_to_player). Adds a +1/+1 counter to the source creature if still on the battlefield. Any creature (not just controller's) triggers this per the ruling, and the implementation correctly does not filter by creature controller.

## Audit — 2026-04-02 20:45

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Enchant player
Whenever a creature deals combat damage to enchanted player, put a +1/+1 counter on that creature.
**Type line**: Enchantment — Aura Curse
**Status**: PASS

### Code issues
None found.

- Name "Curse of Stalked Prey", cost {1}{R}, types Enchantment with subtypes ["Aura", "Curse"] all match oracle.
- `TargetRequirement::PlayerOnly` correctly implements "Enchant player".
- `on_resolve` delegates to `helpers::resolve_curse` which attaches the aura to the target player on the battlefield.
- Trigger uses `TriggerKind::AnyCombatDamageToPlayer`, dispatched via `PendingTrigger::CombatDamageWatch` in the engine. This fires for any creature dealing combat damage to any player, which is correct -- the card does not restrict by creature controller.
- `on_any_combat_damage_to_player` checks `attached_to_player == Some(damaged_player)` to filter only for the enchanted player. Correct.
- Verifies source creature is still on the battlefield before adding counter. Correct edge-case handling.
- Adds exactly 1 `PlusOnePlusOne` counter. Matches oracle.
- `_amount` is unused, correct since the effect does not scale with damage.

### Tricky interactions checked (min 3)
1. **Creature dies during combat before trigger resolves**: Code checks `source_id` is on the battlefield (`zone == Battlefield`) before adding counter -- correctly does nothing if creature died.
2. **Multiple creatures dealing combat damage simultaneously**: Each creature generates a separate `CombatDamageWatch` trigger, each calling `on_any_combat_damage_to_player` with a different `source_id` -- each gets its own +1/+1 counter independently. Correct.
3. **Damage to a non-enchanted player**: The `cursed_player != Some(damaged_player)` guard returns early, so no counter is placed. Correct.
4. **Opponent's creature dealing combat damage to enchanted player**: No controller filter exists in the implementation, matching the 2011-09-22 ruling that any creature (including another opponent's or the enchanted player's own) triggers the ability.

### Test coverage
- `mtg-engine/tests/tier15_cards.rs::curse_of_stalked_prey_gives_counter_on_combat_damage` -- Attaches curse to P1, simulates a creature dealing 2 combat damage to P1, asserts attacker receives exactly 1 +1/+1 counter. Test passes.

## Audit — 2026-04-10 00:00

**Oracle text source**: Oracle cache (Scryfall API), cached 2026-04-01
**Oracle text**:
Enchant player
Whenever a creature deals combat damage to enchanted player, put a +1/+1 counter on that creature.
**Type line**: Enchantment — Aura Curse
**Status**: PASS

### Code issues
No issues found.

Verification:
- Mana cost `{1}{R}`: matches (`ManaSymbol::Generic(1)`, `ManaSymbol::Colored(Color::Red)`).
- Card types: `Enchantment`; subtypes `Aura`, `Curse`. Matches type line "Enchantment — Aura Curse".
- Oracle text field: matches fetched text verbatim.
- Target requirement: `TargetRequirement::PlayerOnly` — matches "Enchant player".
- `on_resolve` delegates to `resolve_curse` helper, which attaches the curse to the chosen player and moves it to the battlefield. Correct for an Aura Curse.
- Trigger: declared with `TriggerKind::AnyCombatDamageToPlayer`. The triggers dispatcher (mtg-engine/src/triggers.rs:551) fires `CombatDamageWatch` for every permanent on the battlefield when a creature deals combat damage to a player, calling `on_any_combat_damage_to_player` (triggers.rs:964).
- `on_any_combat_damage_to_player` filter: `cursed_player != Some(damaged_player)` — correctly fires only when the enchanted player is damaged, regardless of who controls the source creature (matches the 2011-09-22 ruling: "any creature ... including one controlled by another opponent or even by the enchanted player").
- Counter placement guarded by checking the source is still on the battlefield. Uses `state.add_counters(source_id, CounterType::PlusOnePlusOne, 1)` which is the correct pipeline.
- Triggered-ability declaration matches the implemented hook (`AnyCombatDamageToPlayer` ↔ `on_any_combat_damage_to_player`).

### Tricky interactions checked
- Damage dealt by opponent's creature (not owner of curse): handled correctly — code does not restrict on controller of source. PASS.
- Damage dealt by a creature controlled by the enchanted player (e.g., via combat damage redirection): also handled — code only checks `damaged_player`. PASS.
- Source creature dies to lethal combat damage before trigger resolution: counter placement guarded by battlefield check on `source_id`. PASS (no counter placed, consistent with "on that creature" having no valid target).
- Curse source itself removed before trigger resolves: `self_id`/`attached_to_player` lookup returns None/non-battlefield, early return. PASS.
- Enchanted player leaves the game (SBA): handled in mtg-engine/src/sba.rs (player-attached curses are exempt from aura-falls-off check that would apply; curse is dealt with elsewhere). Not a card-level concern.
- Non-combat damage: ignored correctly — only `CombatDamageDealt` dispatches `AnyCombatDamageToPlayer`. PASS.

### Test coverage
- Core ability (counter placed on creature that dealt combat damage to enchanted player): `mtg-engine/tests/tier15_cards.rs:23` (`curse_of_stalked_prey_gives_counter_on_combat_damage`). This test calls the hook directly and verifies a +1/+1 counter appears.
- Ruling — damage from creature controlled by another opponent (or enchanted player): NOT TESTED explicitly (only tests damage from curse controller's creature).
- Trigger dispatched through the full trigger pipeline (not direct hook call): NOT TESTED.
- Does not trigger when non-enchanted player takes combat damage: NOT TESTED.
- Source creature no longer on battlefield when trigger resolves: NOT TESTED.
- Non-combat damage does not trigger: NOT TESTED.
