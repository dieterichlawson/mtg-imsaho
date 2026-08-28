## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/148/infernal-plunge?utm_source=api
**Type line**: `Sorcery` — {R}
**Oracle text**:
```
As an additional cost to cast this spell, sacrifice a creature.
Add {R}{R}{R}.
```
**Status**: PASS

### Code issues
No issues found.

Adds {R}{R}{R} to the pool. Sacrifice is an additional cost paid at cast time.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/148/infernal-plunge?utm_source=api
**Type line**: `Sorcery` — {R}
**Oracle text**:
```
As an additional cost to cast this spell, sacrifice a creature.
Add {R}{R}{R}.
```

**Status**: PASS

### Code issues
No issues found.

Same additional-cost shape as Altar's Reap, same two rulings, same engine
path. `on_resolve` adds {R}{R}{R} to the controller's pool. A sorcery, so the
mana arrives during a main phase with an empty stack — the usual ritual timing.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_sacrifice_and_additional_costs.rs` — sacrifice at cast, three red mana on resolution.

## Audit — 2026-08-28 19:40

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: As an additional cost to cast this spell, sacrifice a creature.
Add {R}{R}{R}.
**Type line**: Sorcery
**Status**: ISSUE

### Code issues
Card data (`mtg-engine/src/cards/isd/infernal_plunge.rs`) matches oracle exactly: {R}, Sorcery, `AdditionalCost::SacrificeCreature`, adds three red on resolution. One consistency issue found and fixed:

- **The mana addition was silent** (`infernal_plunge.rs:31`).
  - Oracle text says: `Add {R}{R}{R}.` — mana added by a resolving spell is mana added like any other (CR 106.4); the engine announces additions with `GameEvent::ManaAdded`.
  - Code did: `state.get_player_mut(controller).mana_pool.add(ManaType::Red, 3);` — pushed into the pool directly, bypassing the `ManaAdded` event that mana abilities emit (`engine/mana_sources.rs`). The event log/UI never saw the addition, and any future "whenever a player adds mana" watcher would miss it.
  - Fix: new `GameState::add_mana(player, mana_type, amount)` (adds + emits), used by both `mana_sources.rs` and the card. No mana source can skip the event now.

### Tricky interactions checked
- Ruling: "You must sacrifice exactly one creature; you cannot cast it without sacrificing a creature": offer side requires a creature (`cannot_cast_without_creature`); submit side validated by `additional_cost_is_payable` (CR 601.2h, added during the Altar's Reap audit — the named sacrifice must be exactly one creature the caster controls). PASS
- Ruling: "Players can only respond once this spell has been cast and all its costs have been paid": sacrifice happens in `cast_spell`, before anyone gets priority; `sacrifice_at_cast_time` shows the creature in the graveyard while the spell is on the stack. PASS
- This is a mana-producing SPELL, not a mana ability — it uses the stack and can be responded to. Correctly implemented as a normal sorcery cast; not offered by the auto-tap planner as a mana source (`gather_mana_sources` only walks battlefield permanents). PASS
- Is the {R}{R}{R} actually usable? Pools empty only in `advance_step` and cleanup (CR 500.4), so mana added mid-main-phase persists for the rest of that phase. PASS
- Sorcery goes to graveyard after resolution via the engine (`stack::resolve_spell`), no self-cleanup in card code. PASS

### Test coverage
- Main effect (sacrifice + RRR): `mtg-engine/tests/cards_sacrifice_and_additional_costs.rs` `infernal_plunge_sacrifices_and_adds_rrr`
- Ruling 1 offer side: `cards_sacrifice_and_additional_costs.rs` `cannot_cast_without_creature` / `can_cast_with_creature`; submit side: `submitted_targets.rs` `a_sacrifice_cost_cannot_be_paid_with_an_opponents_creature` (engine-generic)
- Ruling 2 (cost paid at cast): `cards_sacrifice_and_additional_costs.rs` `sacrifice_at_cast_time`
- Mana added on resolution + announced: `cards_sacrifice_and_additional_costs.rs` `adds_three_red_mana` (now also asserts the `ManaAdded` event)
- One cast action per eligible sacrifice: `cards_sacrifice_and_additional_costs.rs` `one_action_per_sacrifice_target`

Mutation check: reverting the card to a direct `mana_pool.add` (silent) fails `adds_three_red_mana` on the event assertion. Bites.
