# Audit: Creepy Doll

## Oracle Text (Scryfall, cached 2026-04-01)
> Indestructible
> Whenever this creature deals combat damage to a creature, flip a coin. If you win the flip, destroy that creature.

**Card Data:** {5}, Artifact Creature — Construct, 1/1

## Official Rulings (2011-09-22)
1. You don't flip the coin until the ability resolves. If you want to respond to the ability, perhaps by regenerating the damaged creature, you'll have to do so before you know the outcome of the flip.
2. If the combat damage Creepy Doll deals to a creature is lethal, you'll still flip a coin. If the creature is still on the battlefield (perhaps because it regenerated), it could be destroyed a second time, depending on the coin flip.

## Implementation File
`mtg-engine/src/cards/isd/creepy_doll.rs`

## Audit Checklist

### Card Data
- [x] Name: "Creepy Doll" — correct
- [x] Mana cost: `Generic(5)` — correct ({5})
- [x] Card types: `[Artifact, Creature]` — correct
- [x] Subtypes: `["Construct"]` — correct
- [x] Power/Toughness: 1/1 — correct
- [x] Keywords: `[Indestructible]` — correct
- [x] Oracle text string matches Scryfall text — correct

### Triggered Ability
- [x] `triggered_abilities` declares `TriggerKind::DealsCombatDamageToCreature` — correct (combat damage to a creature only, not to a player)
- [x] `on_deals_combat_damage_to_creature` handler is implemented — correct
- [x] Zone check: handler verifies self is on the battlefield before proceeding — correct
- [x] Coin flip: uses `rand::thread_rng().gen_bool(0.5)` — functionally correct (50/50)
- [x] Destroy: calls `crate::destruction::try_destroy()` — correct; this respects indestructible and regeneration on the target, which is the proper behavior for a "destroy" effect
- [x] Logs both win and loss outcomes — correct

### Anti-Pattern Check
- [x] Does NOT use `destroy()` directly (would bypass indestructible) — correct, uses `try_destroy`
- [x] Does NOT use `sacrifice()` (would bypass indestructible) — correct
- [x] Does NOT trigger on non-combat damage — correct, uses the combat-damage-specific hook
- [x] Does NOT trigger on damage to players — correct, `DealsCombatDamageToCreature` is creature-only

### Test Coverage (`mtg-engine/tests/creepy_doll.rs`)
- [x] `has_correct_trigger_kind` — verifies `DealsCombatDamageToCreature` and absence of `Blocks`/`BecomesBlocked`
- [x] `has_indestructible` — verifies keyword present
- [x] `trigger_fires_on_combat_damage_to_creature` — verifies trigger goes on stack
- [x] `trigger_does_not_fire_on_combat_damage_to_player` — verifies no trigger for player damage
- [x] `on_deals_combat_damage_to_creature_calls_destroy` — verifies `try_destroy` is called and can destroy target (randomized, runs 50 iterations)

## Potential Improvements (non-blocking)
- **Coin flip API:** The implementation uses `rand::thread_rng().gen_bool(0.5)` directly. If the engine ever adds a centralized coin-flip API (e.g., for logging, for "Krark's Thumb" effects that modify coin flips, or for deterministic testing), this should be updated to use it. Currently there is no such API in the codebase, so this is acceptable.

## Verdict
**PASS** — No mismatches found. The implementation correctly models all aspects of Creepy Doll: card data, indestructible keyword, combat-damage-to-creature trigger, coin flip mechanic, and proper use of `try_destroy` for the destroy effect. Test coverage is thorough.
