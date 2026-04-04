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

## Audit — 2026-04-02 20:41
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: At the beginning of your upkeep, return an instant or sorcery card at random from your graveyard to your hand.
Whenever you cast an instant or sorcery spell, this creature gets +4/+0 until end of turn.
**Type line**: Creature — Devil
**Status**: PASS

### Code issues
1. **Minor oracle_text field mismatch (cosmetic only):** The `oracle_text` field in `card_data()` says `"Charmbreaker Devils gets +4/+0 until end of turn"` but the current Scryfall oracle text says `"this creature gets +4/+0 until end of turn"`. This does not affect gameplay behavior since the logic is implemented in `on_spell_cast`, not derived from the text string. The old wording with the card name was used on earlier printings; Scryfall's current oracle text uses "this creature".

### Tricky interactions checked (min 3)
1. **Random selection happens at resolution, not on trigger:** The `on_upkeep` method selects the card when it executes (i.e., at resolution time). Per ruling #1, cards added to the graveyard in response can be eligible -- this is correctly handled because `on_upkeep` reads the graveyard at resolution time.
2. **No targeting:** The ability does not target (no target selection step). Per ruling #2, this is correct -- the implementation simply filters and randomly selects, with no targeting involved.
3. **Spell type filtering at dispatch layer:** The `on_spell_cast` handler does not check if the spell is an instant/sorcery. This is correct because `triggers.rs` (lines 645-650) already gates `SpellCast` events to only fire for instant/sorcery spells before invoking `on_spell_cast`.
4. **Multiple triggers in one turn:** If multiple instants/sorceries are cast, each pushes a separate `UntilEndOfTurnEffect`, so the bonuses stack correctly (+4/+0 per spell).
5. **Empty graveyard:** If no instants/sorceries exist in the graveyard, the upkeep ability does nothing (guarded by `if !candidates.is_empty()`).

### Test coverage
- `charmbreaker_devils_plus4_on_spell_cast` in `tier7_cards.rs` -- tests the +4/+0 trigger, verifying power goes from 4 to 8.
- **Missing:** No test for the upkeep ability (return random instant/sorcery from graveyard to hand).
