## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/220/creepy-doll?utm_source=api
**Type line**: `Artifact Creature — Construct` — {5}, 1/1
**Oracle text**:
```
Indestructible
Whenever this creature deals combat damage to a creature, flip a coin. If you win the flip, destroy that creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "flip a coin. **If you win the flip**, destroy that creature" — a real 50/50,
  not an auto-win: PASS
- "deals **combat** damage to a creature", so a Geistflame does not set it off:
  PASS
- Indestructible on the Doll itself, so it survives what it kills: PASS
- `try_destroy`, so an indestructible victim survives the flip: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The coin flip and the destroy: `cards_complex_creatures.rs`, `combat_rules.rs`

## Audit — 2026-08-28 17:25

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Creepy Doll"`, https://scryfall.com/card/isd/220/creepy-doll
**Oracle text**:
```
Indestructible
Whenever this creature deals combat damage to a creature, flip a coin. If you win the flip, destroy that creature.
```
**Type line**: Artifact Creature — Construct
**Mana cost**: {5}   **P/T**: 1/1   **Keywords**: Indestructible
**Rulings** (2, both 2011-09-22):
- "You don't flip the coin until the ability resolves. If you want to respond to the ability,
  perhaps by regenerating the damaged creature, you'll have to do so before you know the
  outcome of the flip."
- "If the combat damage Creepy Doll deals to a creature is lethal, you'll still flip a coin. If
  the creature is still on the battlefield (perhaps because it regenerated), it could be
  destroyed a second time, depending on the coin flip."
**Status**: ISSUE (three fixed: one rules bug, one log bug, one engine-wide testability bug)

### Code issues

**1. The trigger refused to resolve without its source.**
- Oracle text says: `Whenever this creature deals combat damage to a creature, flip a coin.`
  Nothing in it is conditional on the Doll.
- Code did: `if state.get_object(self_id).is_none_or(|o| o.zone != Zone::Battlefield) { return; }`
  as the first line of `on_deals_combat_damage_to_creature`.

CR 113.7a: a triggered ability on the stack exists independently of its source. This is the
pattern `tests/trigger_source_independence.rs` exists to hold — a card re-implementing an
engine gate that was removed. Indestructible keeps the Doll out of most removal, but not out
of a sacrifice: Grimgrin's "Sacrifice another creature" is an activated ability at instant
speed, so the Doll can leave with its own trigger on the stack. Removed.

**2. It announced a kill it might not have got.**
- Ruling says: the destroy can fail — a creature that regenerated "could be destroyed a second
  time" only if the shield is there to be spent, and indestructible answers it outright.
- Code did: `state.log(..., format!("Creepy Doll won the coin flip! Destroying {name}"))` and
  *then* `try_destroy`, without looking at the result.

This is exactly what `destruction::try_destroy_by` was introduced for; its doc names the five
cards that did this and Creepy Doll was a sixth. Now the log line is the true one.

**3. Engine — nothing decided at random could be tested.** `helpers::flip_coin()` called
`rand::thread_rng()`, so the card's whole behaviour was untestable; its test ran the hook fifty
times and asserted at least one run destroyed something. Fixed engine-wide: `GameState` carries
a seed, and coin flips, "at random" choices and shuffles all draw from it. See the separate
commit — six other cards and four other tests were on the same footing.

Card data is otherwise correct: `{5}`, `Artifact Creature — Construct`, 1/1,
`keywords: vec![Keyword::Indestructible]`, oracle text verbatim,
`TriggerKind::DealsCombatDamageToCreature` matching the implemented hook.

### Tricky interactions checked
- **Flip on resolution, not on trigger**: PASS. The flip is in the resolution hook, so a
  response happens before the outcome is known.
- **Lethal damage does not skip the flip**: PASS. Nothing in the hook looks at the damage.
- **A creature that regenerated from the Doll's damage**: PASS. The trigger's destroy is a
  second, separate destruction, and a spent shield does not answer it.
- **Indestructible target**: PASS via `try_destroy` — no destruction, and the log says so.
- **The Doll leaves before the trigger resolves**: PASS after fix 1.
- **The damaged creature already died**: `try_destroy_by` on a graveyard object destroys
  nothing; `has_keyword` and the pipeline both no-op. Not tested — nothing observable
  distinguishes it.
- **Damage to a player**: no trigger. `DealsCombatDamageToCreature` only.
- **First strike / two damage steps**: two damage events would be two triggers and two flips.
  Not reachable — nothing in the pool gives the Doll first strike.

### Test coverage
- wins the flip, destroys: `cards_complex_creatures.rs:2544 creepy_doll_destroys_the_creature_when_it_wins_the_flip` (NEW)
- loses the flip, destroys nothing: `cards_complex_creatures.rs:2565` (NEW)
- resolves after the Doll is sacrificed: `cards_complex_creatures.rs:2586 creepy_dolls_flip_happens_even_if_the_doll_is_gone` (NEW)
- the regeneration ruling: `cards_complex_creatures.rs:2617 creepy_doll_can_destroy_a_creature_that_regenerated_from_its_damage` (NEW)
- indestructible survives, and the log says so:
  `cards_complex_creatures.rs:2657 creepy_doll_cannot_destroy_an_indestructible_creature` (NEW)
- triggers on damage to a creature: `cards_complex_creatures.rs:2524 trigger_fires_on_combat_damage_to_creature`
- does not trigger on damage to a player: `cards_complex_creatures.rs:2550`
- the coin itself is the game's, not the thread's: `test_suite_guards.rs randomness_comes_from_the_game_state` (NEW)

Mutation-checked: inverting the flip kills all five outcome tests; putting the battlefield
guard back kills the source-independence test alone; going back from `try_destroy_by` to
`try_destroy` kills the indestructible test alone.

Two tests were deleted rather than kept: `on_deals_combat_damage_to_creature_calls_destroy`
(fifty runs, "at least one destroyed something") and `bug_creepy_doll_trigger_with_lethal_damage`
(twenty runs watching for a consumed regeneration shield). Both are now stated directly.

### Changes made
- `creepy_doll.rs`: dropped the battlefield guard, switched to `try_destroy_by`, and the flip
  now comes from the game state.
- `cards_complex_creatures.rs`: five deterministic tests replacing the two sampling ones.
- Engine-wide seeded randomness and its guard — see the two preceding commits.
