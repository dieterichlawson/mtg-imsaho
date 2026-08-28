## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/117/skirsdag-high-priest?utm_source=api
**Type line**: `Creature — Human Cleric` — {1}{B}, 1/2
**Oracle text**:
```
Morbid — {T}, Tap two untapped creatures you control: Create a 5/5 black Demon creature token with flying. Activate only if a creature died this turn.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Morbid — ... **Activate only if a creature died this turn**" is an activation
  restriction, not an intervening-if: the ability is simply not offered: PASS
- "{T}, **Tap two untapped creatures you control**" — a cost the
  `ActivatedAbilityDef` cannot express, so it is paid in `pay_activation_cost`
  (CR 601.2h) and the two creatures stay tapped even if the ability is countered:
  PASS
- The two tapped creatures may be summoning sick — tapping as a cost is not the
  {T} symbol (CR 302.6): PASS
- The 5/5 Demon token carries its subtype and flying: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The cost paid at activation and the token on resolution: `activated_no_stack.rs:skirsdag_high_priests_tap_cost_is_paid_at_activation`, `:skirsdag_high_priest_makes_its_demon_on_resolution`, `:skirsdag_summoning_sick_creature_can_be_tapped`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/117/skirsdag-high-priest?utm_source=api
**Type line**: `Creature — Human Cleric` — {1}{B}, 1/2
**Oracle text**:
```
Morbid — {T}, Tap two untapped creatures you control: Create a 5/5 black Demon creature token with flying. Activate only if a creature died this turn.
```

**Rulings fetched**:
- [2020-08-07] Unlike Skirsdag High Priest itself, the two other creatures you tap to activate its ability aren't required to have been under your control continuously since the beginning of your most recent turn.

**Status**: ISSUE (1 in this card; 7 across the shape it shares)

### Code issues found and fixed

**One bug in this card, shared with six others.**

`resolve_activated_ability` read the controller off the Priest:

```rust
let controller = match state.get_object(object_id) {
    Some(o) => o.controller,
    None => return,
};
```

- Oracle text says: `Morbid — {T}, Tap two untapped creatures you control:
  Create a 5/5 black Demon creature token with flying. Activate only if a
  creature died this turn.`
- Code does: gives the token to whoever controls the Priest at resolution, and
  gives no token at all if the Priest has left.

CR 602.2a: an activated ability's controller is the player who *activated* it.
The engine already records the activator on the stack entry, and
`helpers::ability_controller` reads it back, falling through to the source's
last known controller (CR 608.2g). So an opponent who took the Priest in
response — Traitorous Blood is in this set — collected the 5/5 Demon. And the
`None => return` threw the effect away if the source had left, which CR 113.7a
says it must not.

Fixed on this card and on the six others with the identical shape:

| card | what the activator would have lost |
|---|---|
| Back from the Brink | the creature it returns |
| Bloodline Keeper | the Vampire token |
| Cellar Door | the Zombie token |
| Civilized Scholar | the card drawn |
| Heretic's Punishment | the damage's controller |
| Mikaeus, the Lunarch | the +1/+1 counters on "each other creature you control" |
| Skirsdag High Priest | the 5/5 Demon |

Eleven cards were converted to `ability_controller` when it was introduced and
these seven were missed, because nothing failed the build over it. New guard
`card_data_invariants.rs::no_card_reads_its_controller_off_the_source_when_an_ability_resolves`
does, for a `o.controller` read off the source inside
`resolve_activated_ability`. It ignores a read off some other object — Ghost
Quarter reads its *target land's* controller, which is that land's business —
and a comparison against a controller already in hand, which is what Olivia
Voldaren does against the recorded activator.

`pay_activation_cost` is left reading the current controller: costs are paid at
activation, when the activator *is* the controller, and `ability_controller`'s
recorded activator is not set until the ability resolves.

### Card data checked against the fetched text

| field | oracle | code |
|---|---|---|
| cost | `{1}{B}` | `Generic(1), Colored(Black)` OK |
| type | `Creature - Human Cleric` | `Creature`, `["Human", "Cleric"]` OK |
| P/T | 1/2 | `Some(1)/Some(2)` OK |
| keywords | Morbid | none declared - correct, morbid is an ability word (CR 207.2c) and not a keyword ability |
| oracle text | verbatim match | OK |
| token | "a 5/5 black Demon creature token with flying" | `create_token_with_subtypes("", .., 5, 5, [Black], [Creature], [Flying], ["Demon"])` OK |

### Tricky interactions checked

- **Ruling 2020-08-07: "Unlike Skirsdag High Priest itself, the two other
  creatures you tap to activate its ability aren't required to have been under
  your control continuously since the beginning of your most recent turn."**
  **Pass** — the candidate filter is `!o.tapped` and creature-ness, with no
  summoning-sickness test. Summoning sickness (CR 302.6) restricts the {T}
  *symbol* in a creature's own cost; being tapped to pay someone else's cost is
  not that. Was untested; now is.
- **The Priest cannot be one of the two.** **Pass** — `o.id != object_id`. Its
  own {T} already taps it, and the same permanent cannot pay two tap costs at
  once; the ruling's wording ("the two *other* creatures") says the same.
- **The Priest's own {T} and summoning sickness** — left to the engine, which
  knows about haste. **Pass**, tested in `tap_cost_legality.rs`.
- **"Activate only if a creature died this turn" is an activation restriction
  (CR 602.5), not an intervening-if.** It is checked when the ability is
  activated and *not* re-checked on resolution — and the code correctly does
  not re-check. Worth stating because Reaper from the Abyss's morbid, on a
  *trigger*, is an intervening-if (CR 603.4) and must be re-checked. Not
  directly testable here: `creature_died_this_turn` only clears at the turn
  boundary, so there is no in-game way to lose the condition mid-turn with the
  ability on the stack.
- **Control of the Priest changes in response.** **Was broken, now fixed.**
- **The C(n,2) enumeration of which pair to tap.** The card offers one
  `ActivatedAbilityDef` per pair, so the player picks the pair up front rather
  than the engine choosing (CR 601.2b). This is the same shape the engine uses
  for sacrifice costs, which enumerate one action per (target, sacrifice)
  combination. Not a shortcut. **Pass.**
- **Token characteristics.** **Pass**; the existing test asserted size and
  flying but not colour or creature type, so it now asserts all four.

### Test coverage

- offered only with morbid and two other untapped creatures, all three rows,
  and the token's size, colour, type and flying:
  `cards_activated_abilities.rs::skirsdag_high_priest_is_offered_only_with_morbid_and_two_helpers`
  (colour and subtype assertions added)
- the Priest's own {T} respects summoning sickness and haste:
  `tap_cost_legality.rs::skirsdag_high_priest_with_haste_can_activate_while_summoning_sick`
- the ability goes on the stack rather than resolving immediately, and the two
  creatures are tapped as a cost: `activated_no_stack.rs:235`, `:259`
- every pair is offered rather than the engine picking one: `auto_pick.rs:895`
- **the ruling — summoning-sick helpers can be tapped**:
  `cards_activated_abilities.rs::skirsdag_high_priests_helpers_may_be_summoning_sick` (new)
- **control change in response**:
  `cards_activated_abilities.rs::skirsdag_high_priests_demon_goes_to_whoever_activated_it` (new)

Both new tests and the new guard mutation-checked: reverting the
`ability_controller` change fails the control-change test and the guard, and
filtering summoning-sick creatures out of the candidate list fails the ruling
test.
