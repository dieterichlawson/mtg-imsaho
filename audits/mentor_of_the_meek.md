## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/21/mentor-of-the-meek?utm_source=api
**Type line**: `Creature — Human Soldier` — {2}{W}, 2/2
**Oracle text**:
```
Whenever another creature you control with power 2 or less enters, you may pay {1}. If you do, draw a card.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**another** creature **you control** with power 2 or less" — all three
  conditions, and the power is read as the creature *enters* (CR 603.2) rather
  than at resolution, so a creature pumped in response still drew the card: PASS
- "you **may** pay {1}. If you do, draw a card" — an optional cost, declined
  without drawing: PASS
- `effective_power` rather than printed power, so a token or a buffed creature
  is judged by what it actually is: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The power threshold at entry time: `trigger_snapshots.rs`, `cards_complex_creatures.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/21/mentor-of-the-meek?utm_source=api
**Type line**: `Creature — Human Soldier` — {2}{W}, 2/2
**Oracle text**:
```
Whenever another creature you control with power 2 or less enters, you may pay {1}. If you do, draw a card.
```

**Rulings fetched**:
- [2025-01-24] Mentor of the Meek’s ability checks the power of the other creature only as it enters. If that creature’s power is 2 or less, the ability will trigger. Once the ability triggers, raising that creature’s power above 2 won’t affect that ability. Similarly, reducing the creature’s power to 2 or less after it enters won’t cause the ability to trigger.
- [2025-01-24] If a creature enters with +1/+1 counters or a continuous effect such as that of Wedding Festivity will apply to the creature on the battlefield, those effects apply when checking to see if Mentor of the Meek’s ability will trigger.
- [2025-01-24] While resolving the triggered ability of Mentor of the Meek, you can’t pay {1} multiple times to draw more than one card.

**Status**: ISSUE (1, fixed)

### Code issues found and fixed

**One: the card paid its {1} by walking the mana pool by hand, so it could not
tap a land.**

- Oracle text says: `Whenever another creature you control with power 2 or less
  enters, you may pay {1}. If you do, draw a card.`
- Code did:
  ```rust
  if player.mana_pool.get(ManaType::Colorless) >= 1 {
      *player.mana_pool.mana.entry(ManaType::Colorless).or_insert(0) -= 1;
      paid = true;
  } else {
      for mt in &[White, Blue, Black, Red, Green] { ... }
  }
  ```

Paying a cost may involve tapping lands for the mana (CR 601.2g, reached
through CR 608.2g when a resolving ability asks for a payment) — which is what
`engine::pay_cost_with_sources` does. This spent a floating unit if it found
one and silently did nothing otherwise. Confirmed before changing anything: a
Mentor controller with an untapped Plains, an empty pool, and cards in library,
answering "yes", drew nothing.

```
hand before=0 after=0 (an untapped Plains was available)
```

Screeching Bat, the set's other "you may pay", has always gone through the
engine (`plan_autotap_for_cost` + `execute_tap_plan_and_pay`). Mentor was the
only card in the set reaching into a mana pool's map, and it is now
`pay_cost_with_sources`, with the failure branch logging rather than passing
silently. New guard
(`card_data_invariants.rs::no_card_spends_mana_out_of_the_pool_by_hand`) fails
the build on `mana_pool.mana`. It deliberately leaves `mana_pool.add` alone —
that is how a mana ability produces mana, and Infernal Plunge does exactly
that.

Also removed `let _ = (entered_controller, entered_id, registry);` — three
parameters discarded in a line, now named `_entered_id` and so on in the
signature where the reason is a doc comment rather than a statement that does
nothing.

### Card data checked against the fetched text

| field | oracle | code |
|---|---|---|
| cost | `{2}{W}` | `Generic(2), Colored(White)` OK |
| type | `Creature - Human Soldier` | `Creature`, `["Human", "Soldier"]` OK |
| P/T | 2/2 | `Some(2)/Some(2)` OK |
| keywords | none | none OK |
| oracle text | verbatim match | OK |
| trigger | another creature you control, power 2 or less, enters | `TriggerKind::AnyCreatureEnters` with a `should_trigger_on_creature_enters` carrying all three conditions OK |

### Tricky interactions checked

- **Ruling: "checks the power of the other creature only as it enters...
  raising that creature's power above 2 won't affect that ability. Similarly,
  reducing the creature's power to 2 or less after it enters won't cause the
  ability to trigger."** **Pass**, in both directions, and the card says so:
  the condition lives in `should_trigger_on_creature_enters` (CR 603.2, read at
  event time) and the resolution handler deliberately does not re-check.
  Already tested both ways.
- **Ruling: "If a creature enters with +1/+1 counters or a continuous effect...
  those effects apply when checking to see if Mentor of the Meek's ability will
  trigger."** **Pass** — the check is `state.effective_power`, which includes
  counters and continuous effects, not the printed value. Tested via Festerhide
  Boar.
- **Ruling: "you can't pay {1} multiple times to draw more than one card."**
  **Pass** — one prompt, one payment, one draw. Was untested; now is, with five
  Plains on the board.
- **"another"** — Mentor does not trigger off itself. **Pass**, tested.
- **"you control"** — an opponent's creature does not trigger it. **Pass**,
  tested.
- **"you may"** — declining draws nothing and taps nothing. Was untested; now
  is.
- **Saying yes with no mana available** — "if you do" is not satisfied, so no
  draw. Was untested; now is, and drawing regardless fails it.
- **The Mentor destroyed in response to its own trigger** — the prompt is still
  presented (CR 113.7a). **Pass**, tested in
  `trigger_source_independence.rs`.
- **A copy of Mentor** — tested in `copy_effects.rs`.

### Test coverage

- triggers for a small creature, and not for a big one, an opponent's, or
  itself: `enter_trigger_conditions.rs::mentor_triggers_for_a_small_creature`
  and the three beside it
- pumping or shrinking after entry changes nothing:
  `::pumping_after_entry_does_not_undo_the_trigger`,
  `::shrinking_after_entry_does_not_create_a_trigger`
- counters on the entering creature count:
  `cards_shortcuts_taken.rs:356`
- the prompt survives the Mentor's death:
  `trigger_source_independence.rs::mentor_of_the_meek_offers_its_payment_after_dying`
- **paying taps a land**: `enter_trigger_conditions.rs::mentor_taps_lands_to_pay_its_one` (new)
- **one card however much mana is available (the ruling)**:
  `::mentor_draws_one_card_however_much_mana_is_available` (new)
- **declining costs nothing**: `::mentor_declining_costs_nothing_and_draws_nothing` (new)
- **yes with no mana draws nothing**: `::mentor_saying_yes_with_no_mana_available_draws_nothing` (new)

Mutation-checked: paying only from the floating pool fails the land test, and
drawing without checking that the cost was paid fails the no-mana test.
