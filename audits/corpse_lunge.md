## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/93/corpse-lunge?utm_source=api
**Type line**: `Instant` — {2}{B}
**Oracle text**:
```
As an additional cost to cast this spell, exile a creature card from your graveyard.
Corpse Lunge deals damage equal to the exiled card's power to target creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**As an additional cost to cast this spell**, exile a creature card from your
  graveyard" — paid on casting, so the card is already in exile while the spell
  is on the stack and countering the spell does not give it back: PASS
- "damage equal to the **exiled card's** power" — snapshotted at cast into
  `card_state`, because the card is in exile by resolution: PASS
- CR 109.1: "a creature **card** from your graveyard", so a token is not one:
  PASS
- Damage through `deal_damage`: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The additional cost and the snapshotted power: `cards_additional_costs.rs`, `cards_burn_and_damage.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/93/corpse-lunge?utm_source=api
**Type line**: `Instant` — {2}{B}
**Oracle text**:
```
As an additional cost to cast this spell, exile a creature card from your graveyard.
Corpse Lunge deals damage equal to the exiled card's power to target creature.
```

**Rulings fetched**:
- [2013-04-15] You must exile exactly one creature card from your graveyard to cast this spell; you cannot cast it without exiling a creature card, and you cannot exile additional creature cards.
- [2013-04-15] Players can only respond once this spell has been cast and all its costs have been paid. No one can try to otherwise remove the creature card you exiled in order to prevent you from casting this spell.

**Status**: ISSUE (fixed)

### Code issues

**1. The exiled card's power was read at cast time, in the wrong zone.**

- Oracle text says: `Corpse Lunge deals damage equal to the exiled card's power to target creature.`
- `engine/costs.rs::pay_exile_creatures` did:
  ```rust
  if let Some(&first) = to_exile.first() {
      let power = state.effective_power(first, registry).unwrap_or(0);
      if let Some(obj) = state.get_object_mut(spell) {
          obj.card_state.insert("exiled_power".into(), ObjectId(u64::try_from(power).unwrap_or(0)));
      }
  }
  for id in &to_exile {
      state.move_object(*id, Zone::Exile, registry);
  ```
  — the power read **before** the loop that exiles the card, and the card read
  it back with `.map_or(0, |id| i32::try_from(id.0)...)`.

"The exiled card's power" is that card's power where it now is, asked when the
spell resolves. Two things were wrong with a snapshot, and Boneyard Wurm shows
both: "power and toughness are each equal to the number of creature cards in
your graveyard", and a characteristic-defining ability functions in every zone
(CR 604.3).

*Wrong zone.* The snapshot was taken while the Wurm was still in the graveyard,
so it counted itself. Exiling it to pay the cost takes it out of the graveyard,
where it stops counting: with two other creature cards down there it is a 2/2
in exile, and the spell dealt 3.

*Wrong moment.* Corpse Lunge is an instant, so there is a priority window
between the cost being paid and the spell resolving. Creature cards reaching
the graveyard in that window raise the exiled Wurm's power, and the damage with
it. A snapshot cannot see them.

The limit case falls out of the same rule and is worth stating: a Boneyard Wurm
that is the *only* creature card in the graveyard pays the cost by leaving it,
is a 0/0 in exile with nothing left to count, and Corpse Lunge deals nothing.

**2. The engine's cost machinery was answering one card's question.**

`exiled_power` existed for Corpse Lunge alone — the four other
`ExileCreaturesFromGraveyard` cards (Makeshift Mauler, Stitched Drake, Skaab
Goliath, Skaab Ruinator) never read it — and it was a power stored in an
`ObjectId` field, so a negative power silently became 0. What the cost actually
did is that *these cards were exiled*; what any card wants to know about them
is that card's own business, asked when it resolves.

Replaced with `cards::EXILED_TO_COST` / `exiled_to_cost_key(i)`: the engine
records the exiled cards themselves as `exiled_to_cost_0`, `exiled_to_cost_1`,
… and Corpse Lunge asks `state.effective_power` of `exiled_to_cost_0` at
resolution. Both defects above are the same defect, and this is the one fix for
them.

**3. A test that pinned the behaviour of a cast the engine never offers.**

`corpse_lunge_no_graveyard_creature_deals_no_damage` force-submitted a
`CastSpell` action past `legal_actions` with an empty graveyard and asserted
the damage such a cast happens to do is 0. The ruling says otherwise:

> "You must exile exactly one creature card from your graveyard to cast this
> spell; you cannot cast it without exiling a creature card, and you cannot
> exile additional creature cards."

The engine gets this right — `legal/casting.rs` gates on
`additional_cost_plan(..).payable`, and I confirmed the spell is not offered
with an empty graveyard — but nothing tested it, while a test sat next to it
describing an illegal cast as though it were the rule. Replaced with
`corpse_lunge_cannot_be_cast_without_a_creature_card_to_exile`, which also
checks that a noncreature card in the graveyard is not fuel.

### Card data

`{2}{B}` Instant, `AdditionalCost::ExileCreaturesFromGraveyard(1)`,
`TargetRequirement::Creature` for "target creature" — all matching, and cost
and type line pinned pool-wide by `card_data_invariants.rs`. The damage source
is `object_id`, the spell itself, which is what "Corpse Lunge deals damage"
says. `DamageKind::NonCombat` through `damage::deal_damage`, so protection,
`damaged_by` and the rest of the pipeline apply.

### Tricky interactions checked

- Boneyard Wurm's power read in the graveyard rather than in exile: **was
  broken, fixed**.
- The power read at cast rather than at resolution: **was broken, fixed**.
- A lone Boneyard Wurm exiled — 0 damage, and 0 damage is not damage
  (CR 120.8): pass.
- Ruling 1, exactly one creature card, and the spell uncastable without one:
  pass (engine-level; now tested).
- Ruling 2, "Players can only respond once this spell has been cast and all its
  costs have been paid": pass — the cost is paid inside the cast, before
  priority; the second new test relies on exactly that, asserting the Wurm is
  already in exile before the response window.
- Negative power: `power.max(0)`, and `deal_damage` returns early at 0.
- Target removed in response: the engine fizzles the spell (CR 608.2b), and the
  card is not consulted.

### Test coverage

- damage equals the exiled card's power:
  `cards_sacrifice_and_additional_costs.rs::corpse_lunge_deals_damage_equal_to_exiled_power`
- read in the zone the card is now in:
  `…::corpse_lunge_reads_the_exiled_cards_power_where_it_now_is` (new)
- read when the spell resolves:
  `…::corpse_lunge_reads_the_exiled_cards_power_when_it_resolves` (new)
- the lone-Wurm limit case:
  `…::corpse_lunge_exiling_a_lone_boneyard_wurm_deals_nothing` (moved here from
  `xcost_mana.rs` and corrected — it asserted 1, which was the snapshot's
  answer, not the rule's 0)
- ruling 1, uncastable without a creature card:
  `…::corpse_lunge_cannot_be_cast_without_a_creature_card_to_exile` (new,
  replacing the test that enshrined an illegal cast)
- the cost records the card it exiled:
  `spell_costs.rs::auto_paying_an_exile_cost_picks_the_strongest_creature_card`
  (updated from asserting a stored power)

### Mutations run

- Restore the pre-exile power snapshot: **fails** both new power tests (3 vs 2,
  3 vs 4).
- Snapshot the power *after* the exile, isolating timing from zone: the zone
  test passes (2 is right) and the resolution test **fails** (2 vs 4) — the two
  tests separate the two halves cleanly.
- Drop `if !additional.payable { continue; }` from the hand-cast path in
  `legal/casting.rs`: **fails** the castability test.
- The first version of the resolution test put one creature card into the
  graveyard in response, and the pre-exile snapshot passed it — "in the
  graveyard counting itself with N others" and "in exile with N+1 others" are
  the same number. It uses two latecomers now.

Suite: 1510 passing, exit 0, `cargo check --workspace --all-targets` clean.
