## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/158/rakish-heir?utm_source=api
**Type line**: `Creature — Vampire` — {2}{R}, 2/2
**Oracle text**:
```
Whenever a Vampire you control deals combat damage to a player, put a +1/+1 counter on it.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "a Vampire **you control**" — the controller check is present, so an
  opponent's Vampire connecting gives nothing: PASS
- "a Vampire", not "another", so the Heir's own combat damage grows it: PASS
- The counter goes on the Vampire that dealt the damage, not on the Heir: PASS
- CR 113.7a: the Heir trading with a blocker in the same combat damage step does
  not counter the trigger: PASS
- `has_subtype`, so a creature Olivia Voldaren turned into a Vampire counts: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The controller filter and the counter placement: `combat_rules.rs`, `subtype.rs`

## Audit — 2026-08-28 17:15

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Rakish Heir"`, https://scryfall.com/card/isd/158/rakish-heir
**Oracle text**:
```
Whenever a Vampire you control deals combat damage to a player, put a +1/+1 counter on it.
```
**Type line**: Creature — Vampire
**Mana cost**: {2}{R}   **P/T**: 2/2
**Rulings**: none on Scryfall for this card.
**Status**: ISSUE (one engine bug, found here, fixed across cards)

### Code issues

**1. Engine — a trigger whose watcher or dealer died to the same combat damage was never
collected.** Found on this card, fixed in `triggers/collect/damage.rs` for every card.

- Oracle text says: `Whenever a Vampire you control deals combat damage to a player, put a
  +1/+1 counter on it.`
- Code did: `triggers/collect/damage.rs` gated the whole combat-damage-to-player block on
  `if obj.zone == Zone::Battlefield` for the damage source, and filtered watchers with
  `.filter(|o| o.zone == Zone::Battlefield)` — both in the present tense, at collection time.

CR 603.2 has the ability trigger when its event happens; CR 704.3 has the game perform
state-based actions — noticing the deaths that damage caused — before putting any triggered
ability on the stack. So by the time the collector runs, everything the damage killed is
already in a graveyard, and its abilities triggered anyway (CR 113.7a).

Both halves are reachable in this pool:
- *Watcher*: the Heir attacks into a block and trades while another Vampire gets through. The
  Heir was on the battlefield when the damage was dealt; the other Vampire gets its counter.
  It got nothing.
- *Dealer*: Balefire Dragon with trample (Kessig Wolf Run) tramples over its blocker and dies
  to it. "It deals that much damage to each creature that player controls" is visible on the
  board rather than on a creature that is already dead — the sweep simply did not happen.

The fix is one shared `collect::was_on_the_battlefield(state, events, id)`: still there, or
gone somewhere in this same batch of events. The death-watch collector in `zones.rs` had its
own inline copy of exactly this idea (`simultaneously_dead`) and now shares the one.

The dealer's controller now comes from `helpers::controller_of` rather than a raw
`o.controller` read, which resets to the owner on leaving the battlefield (CR 400.7, CR
608.2g). **Not falsifiable by any test in this pool**: the value feeds APNAP bucketing, and
every card hook that needs "you" already calls `controller_of` on its own source.

**2. Card — a second copy of CR 121.1.** `on_any_combat_damage_to_player` guarded
`if state.get_object(source_id).is_some_and(|o| o.zone == Zone::Battlefield)` before
`add_counters`. `GameState::add_counters` already refuses anything not on the battlefield, for
every card at once. Removed; mutation-checking it proved it changed nothing.

Everything else matches: `{2}{R}`, `Creature — Vampire`, 2/2, oracle text verbatim,
`TriggerKind::AnyCombatDamageToPlayer` matching the implemented hook, `target_requirement:
None` (nothing is targeted).

### Tricky interactions checked
- **"a Vampire you control" includes the Heir itself**: PASS. The condition is not
  self-excluding, and the collector's watcher loop includes the damage source.
- **"you control", not "a Vampire"**: PASS. `should_trigger_on_damage_to_player` compares the
  dealer's controller against the Heir's *last known* controller, and answers at damage time
  (CR 603.2) rather than putting a trigger on the stack that does nothing.
- **"it" is the dealer, not the Heir**: PASS.
- **The dealer died dealing the damage**: PASS — the trigger is collected (after fix 1) and
  the counter lands nowhere (CR 121.1, in `add_counters`).
- **The Heir died in the same combat damage step**: PASS after fix 1 — was broken at
  collection.
- **Two Heirs**: each is a separate watcher in the collector's scan; two counters. Not tested.
- **Non-combat damage to a player**: PASS — a different `TriggerKind`, and the Heir declares
  only `AnyCombatDamageToPlayer`.
- **Combat damage to a creature or a planeswalker**: not a player, no trigger. (Planeswalker
  combat is unimplemented engine-wide; recorded, not this card's.)
- **A Vampire token**: PASS. `has_subtype` reads `obj.subtypes` before the registry, so a token
  with the subtype counts.

### Test coverage
- counter goes on the other Vampire, and none on the Heir:
  `cards_combat_damage_triggers.rs:322 rakish_heir_counter_on_other_vampire_combat_damage`
  (tightened from `>= 2` to an exact count, plus the Heir's own count)
- an opponent's Vampire gets nothing:
  `cards_combat_damage_triggers.rs:346 rakish_heir_ignores_a_vampire_an_opponent_controls` (NEW)
- the Heir counters itself, as one of the Vampires you control:
  `cards_combat_damage_triggers.rs:369 rakish_heir_counters_itself_as_one_of_the_vampires_you_control` (NEW)
- a non-Vampire gets nothing: `cards_combat_damage_triggers.rs:~390 rakish_heir_no_counter_on_non_vampire`
- CR 121.1, a dealer that died gets no counter:
  `cards_combat_damage_triggers.rs:~400 rakish_heir_gives_nothing_to_a_vampire_that_died_dealing_the_damage` (NEW)
- the Heir dying in the same combat damage step, at *collection*:
  `trigger_source_independence.rs:752 rakish_heir_triggers_for_a_vampire_it_died_beside` (NEW)
- the same for a dealer's own trigger:
  `trigger_source_independence.rs:775 a_dealers_own_combat_damage_trigger_survives_the_dealer` (NEW)
- the Heir dying with the trigger already on the stack, at *dispatch*:
  `trigger_source_independence.rs:87 rakish_heir_gives_its_counter_after_trading_in_combat`
- two Heirs at once: NOT TESTED

Mutation-checked: dropping "you control", dropping the Vampire check, and countering the Heir
instead of the dealer each kill exactly one test. The CR 121.1 test was vacuous before fix 1
(no trigger was collected at all) and kills the `add_counters` guard after it.

### Changes made
- `rakish_heir.rs`: dropped the card's own copy of CR 121.1.
- `triggers/collect/mod.rs`, `damage.rs`, `zones.rs`: the collector fix above.
- `cards_combat_damage_triggers.rs`: three new tests, one tightened, and Rakish Heir taken out
  of the `SELF_COUNTER_ON_COMBAT_DAMAGE` table — it does counter itself, but not for the reason
  the table is about ("whenever *this creature* deals combat damage"), and the table's own
  coverage check never included it.
- `trigger_source_independence.rs`: the two collection-time tests.
