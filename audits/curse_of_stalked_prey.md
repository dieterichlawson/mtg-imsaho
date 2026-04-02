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
