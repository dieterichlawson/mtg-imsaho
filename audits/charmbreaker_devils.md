# Audit: Charmbreaker Devils

## Oracle Text (Scryfall, cached 2026-04-01)

> At the beginning of your upkeep, return an instant or sorcery card at random from your graveyard to your hand.
> Whenever you cast an instant or sorcery spell, this creature gets +4/+0 until end of turn.

## Rulings (2017-11-17)

1. The instant or sorcery card returned to your hand is chosen at random as the first ability resolves. If any player responds to the ability, that player won't yet know what card will be returned.
2. Because the first ability doesn't target, any instants or sorceries put into your graveyard in response may be returned.
3. All players get to see which card you chose at random as it's returned to your hand.

## Card Data Check

| Field     | Oracle     | Implementation | Verdict |
|-----------|------------|----------------|---------|
| Name      | Charmbreaker Devils | "Charmbreaker Devils" | CORRECT |
| Cost      | {5}{R}     | Generic(5), Red | CORRECT |
| Type      | Creature -- Devil | Creature, subtypes=["Devil"] | CORRECT |
| P/T       | 4/4        | 4/4            | CORRECT |
| Keywords  | (none)     | vec![]         | CORRECT |

## Triggered Abilities Declaration

- `TriggerKind::Upkeep` -- declared in `triggered_abilities` -- CORRECT
- `TriggerKind::SpellCast` -- declared in `triggered_abilities` -- CORRECT

## Behavior: `on_upkeep` (return random instant/sorcery)

- Checks self is on the battlefield -- CORRECT
- Checks `state.active_player == controller` (only on your upkeep) -- CORRECT
- Filters graveyard for `CardType::Instant | CardType::Sorcery` -- CORRECT
- Uses `rand::thread_rng()` + `shuffle` for random selection -- CORRECT (truly random, not player choice)
- Moves chosen card to `Zone::Hand` -- CORRECT
- Logs the returned card name -- CORRECT
- Does nothing if no candidates exist (empty graveyard check) -- CORRECT
- Per ruling #2, selection happens on resolution which is how `on_upkeep` works (called at resolution time) -- CORRECT

## Behavior: `on_spell_cast` (+4/+0)

- Checks self is on the battlefield -- CORRECT
- Checks `caster == controller` (only your own spells) -- CORRECT
- Applies `+4/+0` via `until_end_of_turn_effects` -- CORRECT
- Does NOT re-check if spell is instant/sorcery -- NOT A BUG. The trigger framework in `triggers.rs` (lines 628-632) pre-filters `SpellCast` events to only dispatch for instant/sorcery spells before calling `on_spell_cast`. The handler is only ever invoked for qualifying spells.

## Anti-Patterns Check

- No hardcoded player IDs -- PASS
- No missing zone checks -- PASS
- No off-by-one errors -- PASS
- Random selection uses standard library RNG, not deterministic -- PASS

## Test Coverage

- `charmbreaker_devils_plus4_on_spell_cast` in `tier7_cards.rs` -- tests the +4/+0 trigger. Verifies power goes from 4 to 8.
- **MISSING:** No test for the upkeep ability (returning a random instant/sorcery from graveyard to hand).

## Issues

1. **MISSING TEST (low severity):** No test exercises the upkeep trigger that returns a random instant or sorcery from graveyard to hand. A test should verify: (a) an instant/sorcery is moved from graveyard to hand, (b) non-instant/sorcery cards are not eligible, and (c) if no instants/sorceries exist in graveyard, nothing happens.

## Previous Audit Correction

The prior audit flagged `on_spell_cast` for not checking whether the cast spell is an instant or sorcery. This was a **false positive**. The trigger dispatch system in `triggers.rs` already gates `SpellCastWatch` events on the spell being an instant or sorcery (lines 628-632), so the per-card handler does not need to duplicate this check.

## Verdict

**PASS** -- Implementation is correct. One missing test for the upkeep ability.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: At the beginning of your upkeep, return an instant or sorcery card at random from your graveyard to your hand.
Whenever you cast an instant or sorcery spell, this creature gets +4/+0 until end of turn.
**Type line**: Creature — Devil
**Status**: PASS

### Code issues
No issues found.
