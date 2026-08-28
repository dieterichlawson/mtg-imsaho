## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/moc/197/mikaeus-the-lunarch?utm_source=api
**Type line**: `Legendary Creature — Human Cleric` — {X}{W}, 0/0
**Oracle text**:
```
Mikaeus enters with X +1/+1 counters on it.
{T}: Put a +1/+1 counter on Mikaeus.
{T}, Remove a +1/+1 counter from Mikaeus: Put a +1/+1 counter on each other creature you control.
```

**Status**: ISSUE

### Code issues
See below.


- It arrived as a 0/0 and got its counters afterwards.
  - Oracle text says: `Mikaeus enters with X +1/+1 counters on it.`
  - Code did: `state.move_object(object_id, Zone::Battlefield, registry);` then
    `state.add_counters(object_id, CounterType::PlusOnePlusOne, x);`
  - CR 614.1c makes that a replacement effect, applied as the permanent enters.
    `move_object` emits `EnteredBattlefield` as part of the move, so every ETB
    watcher saw a 0/0 Mikaeus — and a 0/0 creature is one state-based-action
    check from the graveyard (CR 704.5f). Converted to `replace_event` with the
    `enters_with_counters` helper the set's other "enters with counters" cards
    already use. The same override also re-did the trait default's "move a
    permanent to the battlefield" and its `is_legendary` stamping; a guard now
    fails the build on a card that moves itself.

### Tricky interactions checked
- "{X}{W}" with X read from the object's `x_value`, so X = 0 makes a 0/0 that
  dies immediately — correct, not a special case: PASS
- "{T}, Remove a +1/+1 counter from Mikaeus: Put a +1/+1 counter on **each
  other** creature you control" — the removal is a cost, and Mikaeus excludes
  itself from the beneficiaries: PASS
- Legendary, so the legend rule applies: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Entering with X counters, and both activated abilities: `cards_complex_creatures.rs`, `counter_costs.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/moc/197/mikaeus-the-lunarch?utm_source=api
**Type line**: `Legendary Creature — Human Cleric` — {X}{W}, 0/0
**Oracle text**:
```
Mikaeus enters with X +1/+1 counters on it.
{T}: Put a +1/+1 counter on Mikaeus.
{T}, Remove a +1/+1 counter from Mikaeus: Put a +1/+1 counter on each other creature you control.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (1, fixed in the engine)

**Note on the source**: the oracle cache's entry for this card is keyed to the
Commander Masters printing (`scryfall.com/card/moc/197`). Scryfall's
`oracle_text`, type line and mana cost are printing-independent, so this is the
same text the Innistrad printing has; the P/T and everything compared below
come from that record.

**Rulings fetched**: none are published for this card.

### Code issues found and fixed

**One, and it is in the engine rather than the card: `x_value` survived a
permanent leaving the battlefield.**

- Oracle text says: `Mikaeus enters with X +1/+1 counters on it.`
- CR 107.3b: X is 0 for a permanent that was not cast — and CR 400.7 makes a
  permanent that leaves and comes back a new object, one that was never cast at
  all.
- Code did: nothing cleared `obj.x_value` on a zone change, so a Mikaeus cast
  for X=5, killed, and reanimated read the *old* cast's X in its
  enters-with-counters replacement and came back a 5/5.

Confirmed before fixing rather than argued: a Mikaeus with `x_value = Some(5)`
moved battlefield -> graveyard -> battlefield came back with five +1/+1
counters.

The fix belongs where the other CR 400.7 resets live, in `state.rs`'s
`move_object`. X means something in exactly two places — on the stack while
that cast is resolving, and on the permanent that cast produced — so it is
cleared on a move to anywhere else:

```rust
if !matches!(to, Zone::Battlefield | Zone::Stack) {
    obj.x_value = None;
}
```

Three cards read `x_value`: this one, Devil's Play (from its own spell object
on the stack), and the X-cost planeswalker path in `cards/mod.rs`. Only a
*permanent* can carry a stale one across zones, which is why Mikaeus is where
it showed up — it is the set's one creature card with {X}.

### Card data checked against the fetched text

| field | oracle | code |
|---|---|---|
| cost | `{X}{W}` | `ManaSymbol::X, Colored(White)` OK |
| type | `Legendary Creature - Human Cleric` | `Creature` + `Supertype::Legendary`, `["Human", "Cleric"]` OK |
| P/T | 0/0 | `Some(0)/Some(0)` OK |
| keywords | none | none OK |
| oracle text | verbatim match | OK |
| enters with counters | "enters with X +1/+1 counters on it" | `replace_event` -> `helpers::enters_with_counters` OK, a replacement effect (CR 614.1c) rather than something done afterwards |
| ability 0 | `{T}: Put a +1/+1 counter on Mikaeus.` | `requires_tap`, free, adds one to itself OK |
| ability 1 | `{T}, Remove a +1/+1 counter from Mikaeus: Put a +1/+1 counter on each other creature you control.` | `requires_tap` + `counter_cost: Some((PlusOnePlusOne, 1))`, effect on other creatures only OK |

### Tricky interactions checked

- **A reanimated Mikaeus does not remember its X.** **Was broken, now fixed.**
- **"each OTHER creature"** — Mikaeus does not give itself one, so paying the
  cost leaves it down net one counter. **Pass**; the existing test asserted the
  count but not the reason, and now says so.
- **"each other creature YOU control"** — an opponent's creatures get nothing.
  **Pass**; was untested, and the existing test had no opponent creature on the
  board at all. Now it does.
- **Paying the last counter kills it mid-ability.** Removing the counter is a
  cost (CR 602.2b), so a Mikaeus at one counter becomes a 0/0 and dies to
  state-based action (CR 704.5f) with its ability still on the stack — which
  resolves anyway (CR 113.7a) and gives the rest of the team their counters.
  **Pass**, now tested.
- **CR 601.2h** — with no counter to remove, only the first ability is offered.
  **Pass**, now tested; the engine enforces it from the declared `counter_cost`
  rather than the card hiding the ability itself.
- **CR 602.2h, one tap pays one cost** — both abilities need {T}, so using
  either takes the other off the table until it untaps. **Pass**, now tested.
- **X=0** — Mikaeus enters as a 0/0 and dies. Correct and unavoidable;
  `unwrap_or(0)` gives the same answer for a permanent put onto the battlefield
  without being cast.
- **Back from the Brink casting it from exile** — X is 0 when a mana cost is
  paid other than by casting (CR 107.3e), so the offered ability must not carry
  {X}. **Pass**, tested in `trigger_snapshots.rs`.
- **The legend rule** — handled generally by `state.is_legendary` off the
  active face, not stamped by this card.
- **`activated_abilities` re-checks the zone.** Redundant —
  `legal/abilities.rs` only asks battlefield permanents — but inert, and left
  alone.

### Test coverage

- enters with X counters, from a hand-built stack object:
  `cards_transforming_permanents.rs::mikaeus_enters_with_x_counters`
- and end to end through a real cast:
  `x_cost_spells.rs::the_announced_x_is_the_x_the_spell_resolves_with`
- X is the mana left after the printed cost:
  `x_cost_spells.rs::x_is_what_is_left_after_the_printed_cost_is_paid`
- X is 0 when cast other than by paying its mana cost:
  `trigger_snapshots.rs::x_cost_creature_activation_costs_only_non_x_portion`
- distributes counters to other creatures:
  `cards_transforming_permanents.rs::mikaeus_distributes_counters` (now also
  asserts an opponent's creature gets nothing, and says why Mikaeus is down one)
- **a reanimated Mikaeus has X = 0**:
  `x_cost_spells.rs::a_reanimated_x_creature_does_not_remember_the_x_it_was_cast_for` (new)
- **paying the last counter and dying before it resolves**:
  `cards_transforming_permanents.rs::mikaeus_can_pay_its_last_counter_and_die_before_the_ability_resolves` (new)
- **the second ability needs a counter**:
  `cards_transforming_permanents.rs::mikaeus_second_ability_needs_a_counter_to_remove` (new)
- **one tap, one ability per untap**:
  `cards_transforming_permanents.rs::mikaeus_only_gets_one_of_its_two_tap_abilities_per_untap` (new)

Mutation-checked: keeping `x_value` across zone changes fails the reanimation
test; dropping the `!= object_id` filter fails the distribution test; and
pointing the distribution at the opponent's creatures fails it too.
