## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/144/geistflame?utm_source=api
**Type line**: `Instant` — {R}
**Oracle text**:
```
Geistflame deals 1 damage to any target.
Flashback {3}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "any target", 1 damage, through the damage pipeline: PASS
- Flashback {3}{R} and exile after the flashback resolution: PASS
- Ruling: "You must still follow any timing restrictions and permissions" — an
  instant's flashback can be cast at instant speed: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The damage and the flashback: `cards_flashback.rs`, `cards_burn_and_damage.rs`

## Audit — 2026-08-28 17:29

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Geistflame"`, https://scryfall.com/card/isd/144/geistflame
**Oracle text**:
```
Geistflame deals 1 damage to any target.
Flashback {3}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Type line**: Instant
**Mana cost**: {R}   **Keywords**: Flashback
**Rulings**: six, all the generic flashback ones.
**Status**: ISSUE (one engine bug found here, fixed; card itself correct)

### Code issues
The card is correct. `mtg-engine/src/cards/isd/geistflame.rs` is `{R}`, `CardType::Instant`,
`flashback_cost: Some({3}{R})`, oracle text verbatim, `TargetRequirement::AnyTarget`, and one
call to the shared `helpers::resolve_damage(state, object_id, targets, 1, registry)` — which
routes through the damage pipeline, so the event is `NonCombatDamageDealt` and `damaged_by`
records the spell.

**Engine — `is_target_legal` accepted a player for every requirement.** Found by mutating this
card's `AnyTarget` to `Creature` and watching it still deal its damage to a player.

- Code did: the `Target::Player` arm was `if matches!(inner_req, OpponentOnly) && *pid ==
  caster { return false } ; can_target_player(...)`. Every other requirement — `Creature`,
  `CreatureWithFilter`, `PermanentWithFilter`, `Spell`, the five graveyard ones, `ExileCard` —
  fell through to "yes, if that player is targetable at all".
- CR 115.4 / CR 601.2c: what a spell may target is decided by what it says. "Destroy target
  attacking creature" cannot be pointed at a player.

The offer lists never produce such a target, which is why it survived: it is reachable only
through a *submitted* one, and neither client picks a whole offered action — both assemble one
from per-slot choices. Same family as the three submit paths fixed earlier this session, and it
also sat in the CR 608.2b re-check on resolution.

The arm now names every requirement, so a new one has to answer the question rather than
inherit an answer.

### Tricky interactions checked
- **"Any target" is creature, player, or planeswalker (CR 115.4a)**: PASS. The `AnyTarget`
  offering arm collects creatures and planeswalkers on the battlefield plus every targetable
  player.
- **1 damage is damage, not life loss**: PASS — `resolve_damage` goes through the damage
  pipeline, so prevention and protection apply and watchers see it (contrast Bump in the Night).
- **Damage to a planeswalker removes loyalty**: PASS, in the shared pipeline.
- **`damaged_by` records the spell, not a creature**: PASS. Abattoir Ghoul's "dealt damage by
  this creature" does not match a spell, which is correct.
- **Flashback**: PASS, all engine-side — cast from the graveyard, the alternative cost, exile
  instead of graveyard whether it resolves, is countered, or fizzles.
- **Fizzle**: PASS. A flashback Geistflame whose target has left is countered by game rules and
  still exiled.

### Test coverage
- 1 damage to a creature: `cards_removal_and_bounce.rs:33 geistflame_deals_1_damage`
- 1 damage to a **player**: `spells.rs:53 direct_damage_spells_drain_player_life` (NEW row)
- the requirement matches the printed words, for every "any target" card in the set:
  `card_data_invariants.rs:1944` — a registry-wide sweep, and the test that killed the
  `AnyTarget` → `Creature` mutation
- cast from the graveyard by flashback, then exiled: `flashback.rs:110`, `flashback.rs:127`
- countered on the stack, still exiled: `flashback.rs:169`
- fizzled, still exiled: `fizzle.rs:373 a_fizzled_flashback_spell_is_still_exiled`
- flashback cost distinct from mana cost: `flashback_multiple_instances.rs:45`
- noncombat damage marks `damage_marked` past "prevent all combat damage":
  `cards_morbid_and_ltb.rs:658`
- damage to a planeswalker: `damage_helper.rs:119`, `damage_pipeline.rs:100` (pipeline-level)
- a player is not a legal target for a creature-only spell:
  `submitted_targets.rs:135 a_player_is_not_a_legal_target_for_a_spell_that_wants_a_creature` (NEW)

Mutation-checked: 1 damage → 2 kills the player row; `AnyTarget` → `Creature` kills the
registry sweep; restoring the old player arm kills the new targeting test.

### Changes made
- `stack.rs`: the player-target fix above.
- `submitted_targets.rs`: the test for it.
- `spells.rs`: Geistflame added to the player-damage table, and the doc comment that called it
  "creature-only" corrected — it reads "any target" and always did.
