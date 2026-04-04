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

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Indestructible
Whenever this creature deals combat damage to a creature, flip a coin. If you win the flip, destroy that creature.
**Type line**: Artifact Creature — Construct
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 20:45
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Indestructible
Whenever this creature deals combat damage to a creature, flip a coin. If you win the flip, destroy that creature.
**Type line**: Artifact Creature — Construct
**Status**: ISSUE (minor)

### Code issues
1. **Oracle text string mismatch (cosmetic)**: The `oracle_text` field in `card_data()` uses the card's name instead of "this creature".
   - Implementation (line 24): `"Whenever Creepy Doll deals combat damage to a creature, flip a coin. If you win the flip, destroy that creature."`
   - Scryfall oracle text: `"Whenever this creature deals combat damage to a creature, flip a coin. If you win the flip, destroy that creature."`
   - This does not affect gameplay behavior, only the displayed oracle text string.

### Tricky interactions checked (min 3)
1. **Indestructible target**: If Creepy Doll wins the flip against an indestructible creature, `try_destroy` correctly returns `DestroyResult::Indestructible` and does not move the creature to the graveyard. Correct per rules.
2. **Regeneration interaction (ruling #1)**: The ability uses a triggered ability that goes on the stack (`DealsCombatDamageToCreature` trigger in `triggers.rs`), giving opponents a window to respond (e.g., regenerate). At resolution, `try_destroy` checks regeneration shields. Both aspects are correct.
3. **Lethal damage + coin flip (ruling #2)**: The trigger fires on combat damage dealt regardless of lethality. If the creature has already left the battlefield by resolution (e.g., died to SBAs), `try_destroy` fails gracefully. If it regenerated and is still on the battlefield, the destroy from the coin flip can attempt to destroy it again. Correct.
4. **Self leaves battlefield**: Both the trigger resolution in `triggers.rs` (line 927) and the handler itself (line 39) check that Creepy Doll is still on the battlefield before resolving. Correct.
5. **Damage to players**: Trigger kind is `DealsCombatDamageToCreature`, not `CombatDamageToPlayer`. Test `trigger_does_not_fire_on_combat_damage_to_player` confirms this. Correct.

### Test coverage
- `has_correct_trigger_kind` -- verifies DealsCombatDamageToCreature, absence of Blocks/BecomesBlocked
- `has_indestructible` -- verifies Indestructible keyword
- `trigger_fires_on_combat_damage_to_creature` -- verifies trigger goes on stack
- `trigger_does_not_fire_on_combat_damage_to_player` -- verifies no trigger for player damage
- `on_deals_combat_damage_to_creature_calls_destroy` -- verifies destroy can happen (50 iterations for randomness)
- `creepy_doll_is_indestructible` (in `tier15_cards.rs`) -- verifies `has_keyword` returns true for Indestructible
