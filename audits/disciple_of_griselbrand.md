## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/98/disciple-of-griselbrand?utm_source=api
**Type line**: `Creature — Human Cleric` — {1}{B}, 1/1
**Oracle text**:
```
{1}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "The amount of life you gain is equal to the toughness of the creature
  **as it last existed on the battlefield**, not its toughness in the
  graveyard." Read from the `CreatureDied` event's `last_known_toughness`, which
  `death_event` builds before the zone change (CR 608.2g): PASS
- "Sacrifice a creature" can be the Disciple itself: PASS
- A negative toughness gains 0 life, not negative life: PASS
- The life gain emits `LifeChanged`: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Life equal to the sacrificed creature's toughness: `cards_sacrifice_and_additional_costs.rs:disciple_of_griselbrand_gains_life`
- Sacrificing itself: `sacrifice_choice.rs:disciple_of_griselbrand_can_sacrifice_itself`
- The player picks the fodder: `sacrifice_choice.rs:disciple_of_griselbrand_player_picks_highest_toughness_sacrifice`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/98/disciple-of-griselbrand?utm_source=api
**Type line**: `Creature — Human Cleric` — {1}{B}, 1/1
**Oracle text**:
```
{1}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.
```

**Rulings fetched**:
- [2011-09-22] The amount of life you gain is equal to the toughness of the creature as it last existed on the battlefield, not its toughness in the graveyard.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/98/disciple-of-griselbrand
**Oracle text**: {1}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.
**Type line**: Creature — Human Cleric
**Mana cost**: {1}{B} — **P/T**: 1/1
**Rulings** (1, 2011-09-22): "The amount of life you gain is equal to the toughness of the creature as it last existed on the battlefield, not its toughness in the graveyard."

**Status**: ISSUE (fixed) — a real gameplay bug.

### Card data
Matches the fetched text: `{1}{B}`, `card_types: [Creature]`,
`subtypes: ["Human", "Cleric"]` (both), 1/1, oracle text verbatim, no keywords.
The ability is `{1}` plus `SacrificeCost::SacrificeCreature` — the whole printed
cost, with `requires_tap: false`, which is right.

### Code issues

1. **The card read whatever died last, not the creature that paid its cost**
   (`disciple_of_griselbrand.rs:47`, plus the engine plumbing to make the right
   answer available). **This is a real gameplay bug, not a test gap.**
   - Oracle text says: `You gain life equal to the sacrificed creature's toughness.`
   - The code said:
     ```
     let toughness = state.events.iter().rev()
         .find_map(|e| match e {
             GameEvent::CreatureDied { last_known_toughness, .. } => Some(*last_known_toughness),
             _ => None,
         })
     ```
     — the **most recent** creature death, whichever creature that was.
   - The cost is paid when the ability is activated (CR 601.2h); the ability
     resolves later, and players get priority in between. Anything that dies in
     that window becomes the most recent death.
   - Verified against the built engine: activate Disciple sacrificing a 1/1,
     destroy a 5/9 in response, resolve — **9 life gained**. After the fix, 1.
   - The card had no way to know which creature paid: the engine took the
     `sacrifice` field off the `ActivateAbility` action, sacrificed that
     creature, and dropped it. So the fix is engine-side, following the pattern
     already there for `x_value`: `StackEntry::Ability` gains a `sacrificed`
     field, threaded through `state.last_activated_sacrifice` — set when the
     cost is paid, carried on the stack entry, restored on resolution. The card
     keys its `CreatureDied` lookup on that id.
   - `SacrificeThis` records the source itself, so the same handle is available
     to any future card that needs it.

2. **The card's only ruling had no test**
   (`sacrifice_choice.rs`, test added).
   - Ruling says: `the toughness of the creature as it last existed on the battlefield, not its toughness in the graveyard`
   - The card honours it by reading `CreatureDied`'s `last_known_toughness`,
     but replacing that with the graveyard object's own `toughness` produced
     zero failures — every existing test sacrifices an unmodified creature,
     where the two numbers are the same.
   - Added `disciple_of_griselbrand_gains_the_toughness_it_had_on_the_battlefield`:
     a Walking Corpse with three +1/+1 counters is a 5/5 on the battlefield and
     a printed 2/2 card in the graveyard (CR 400.7 makes it a new object), and
     five is what you gain.

### Tricky interactions checked
- The bug above, as a regression:
  `disciple_of_griselbrand_reads_the_creature_that_paid_not_the_last_one_to_die`
  — **added this audit**.
- The ruling (battlefield toughness, not graveyard): **added this audit**.
- The player picks which creature to sacrifice, one action per candidate:
  PASS — `disciple_of_griselbrand_player_picks_highest_toughness_sacrifice`.
- Disciple may sacrifice **itself** ("a creature", not "another creature"):
  PASS — `disciple_of_griselbrand_can_sacrifice_itself`. The fix records
  `Some(object_id)` for `SacrificeThis` and the chosen id otherwise, so this
  path keeps working.
- The `{1}` must actually be payable: PASS —
  `disciple_appears_when_mana_is_already_in_the_pool` and
  `disciple_does_not_appear_with_only_untapped_lands_and_no_floating_mana`.
- Life gain goes through `change_life`, so `LifeChanged` is emitted: guarded by
  `only_change_life_writes_a_life_total`.
- Sacrifice bypasses indestructible (CR 701.16a): the cost goes through
  `destruction::sacrifice`, the sacrifice pipeline, not `try_destroy`.
- A 0-toughness creature cannot be on the battlefield (SBA), and negative
  toughness is clamped by `.max(0)`, so no life is lost.
- Self-cleanup: none; this is a permanent.

### UI presentation
Ability description: "{1}, Sacrifice a creature: Gain life equal to its
toughness". The log line names the source and the amount.

### Test coverage
- Reads the creature that paid, not the last to die: `sacrifice_choice.rs`
  (`disciple_of_griselbrand_reads_the_creature_that_paid_not_the_last_one_to_die`)
  — **added this audit**.
- The ruling: (`disciple_of_griselbrand_gains_the_toughness_it_had_on_the_battlefield`)
  — **added this audit**.
- The sacrifice is the player's choice: `disciple_of_griselbrand_player_picks_highest_toughness_sacrifice`.
- May sacrifice itself: `disciple_of_griselbrand_can_sacrifice_itself`.
- Mana availability gating: two tests in the same file.

### Mutations run
| mutation | result |
| --- | --- |
| revert to "the most recent `CreatureDied`" | fails the new regression test |
| read the graveyard object's `toughness` instead of `last_known_toughness` | fails the new ruling test (before it: **nothing at all**) |

Suite after: 1466 passing, exit 0, zero warnings.

