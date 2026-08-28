## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/112/reaper-from-the-abyss
**Oracle text**:
```
Flying
Morbid — At the beginning of each end step, if a creature died this turn, destroy target non-Demon creature.
```
**Type line**: `Creature — Demon` — {3}{B}{B}{B}, 6/6, Keywords: Flying, Morbid
**Ruling [2011-09-22]**: The morbid ability is mandatory. If you control the only non-Demon creature when the ability triggers, you must choose it as the target.

**Status**: ISSUE (2 found, both fixed)

### Code issues

1. **Morbid enforced as target legality (CR 603.3c) rather than as an intervening-if (CR 603.4)** — `reaper_from_the_abyss.rs`, `is_valid_target`.
   - Oracle text says: `Morbid — At the beginning of each end step, if a creature died this turn, destroy target non-Demon creature.`
   - Code did: `fn is_valid_target(...) { if !state.creature_died_this_turn { return false; } ... }`, with the comment *"if no creature died this turn, no creature is a legal target, so the trigger is removed from the stack per CR 603.3c."*
   - Those are different rules. 603.4 says an ability with an unmet intervening-if **never triggers**; 603.3c says an ability with no legal targets **is put on the stack and then removed**. The board state came out the same, which is why the existing test passed — but the difference is player-visible in the log. Probed directly: with no creature dead, the engine emitted
     `Trigger removed: no legal targets (Reaper from the Abyss's end step trigger (if morbid, destroy target non-Demon creature))`
     for an ability that by 603.4 never triggered. "Each end step" means this fired twice a turn, every turn the Reaper survived without a death.
   - Fixed: morbid moved to `should_trigger` (the hook 16 other ISD cards use); `is_valid_target` now tests only properties of the target. The resolution-time re-check in `on_end_step` stays, per CR 603.4's second check.
   - Reproduction: extended `intervening_if.rs::reaper_from_the_abyss_end_step_trigger_respects_its_morbid_clause` to assert no "Trigger removed" line — failed before the fix with the exact message above.

2. **Hand-rolled half of the `is_creature` accessor** — same function.
   - Code did: `if obj.zone != Zone::Battlefield || obj.power.is_none() { return false; }`
   - `state.is_creature(id, registry)` is the accessor for this and is documented as `has_card_type(Creature) || obj.power.is_some()` — card types *plus* the object-level P/T sentinel that tokens and `*/*` creatures carry. Inlining one half is the `obj.power`-instead-of-registry anti-pattern from step 9.
   - Fixed: calls `state.is_creature`. Worth recording that my first attempt swapped in `has_card_type` alone and broke the positive arm of the existing test — the P/T sentinel is load-bearing, not redundant.

### Tricky interactions checked
- **Ruling (mandatory, may have to target your own creature)**: PASS. The filter is `NotSubtypes(["Demon"])` with no controller restriction, so the controller's own non-Demons are offered.
- **Trigger outliving its source (CR 113.7a)**: PASS. `on_end_step` deliberately ignores `self_id`; covered by `trigger_independence.rs:74`.
- **Target chosen at trigger time (CR 603.3d)**: PASS. `target_requirement` is declared on the `TriggeredAbilityDef`, so the engine locks the target as the trigger goes on the stack; `on_end_step` reads `chosen_targets` rather than re-picking.
- **Destroy vs indestructible**: PASS. Uses `PendingEffect::DestroyCreature` through `apply_pending_effect` → the `try_destroy` pipeline, so indestructible and regeneration apply. Oracle says "destroy", not "sacrifice".
- **"each end step", not "your end step"**: PASS. No `step_trigger_scope` override, so it defaults to `TriggerScope::Each`.
- **Morbid is an ability word, not a keyword**: correct to omit from `keywords` (only `Flying` is declared). Scryfall lists ability words in its `keywords` array; they confer nothing.
- **Demon targeting itself**: PASS. Reaper is a Demon, so the `NotSubtypes` filter excludes it — matching "target non-Demon creature" without needing a self-exclusion clause.

### Test coverage
- Morbid gates the trigger, both arms: `intervening_if.rs:236`
- No phantom "Trigger removed" log when morbid is unmet: `intervening_if.rs:251` — **added by this audit**
- Trigger resolves after the Reaper dies: `trigger_independence.rs:74`
- Ruling (must target your own creature when it is the only non-Demon): NOT TESTED
- Destroy respects indestructible: NOT TESTED for this card (the pipeline is covered generally in `state_based_actions.rs`)
- Non-Demon filter excludes another Demon: NOT TESTED
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/112/reaper-from-the-abyss?utm_source=api
**Type line**: `Creature — Demon` — {3}{B}{B}{B}, 6/6
**Oracle text**:
```
Flying
Morbid — At the beginning of each end step, if a creature died this turn, destroy target non-Demon creature.
```

**Rulings fetched**:
- [2011-09-22] The morbid ability is mandatory. If you control the only non-Demon creature when the ability triggers, you must choose it as the target.

**Status**: PASS (behaviour correct; a stale rules citation and two untested properties fixed)

### Code issues

**The card's behaviour is correct.** Everything it does matched the fetched
text and its one published ruling; what I found was a wrong rules citation and
two properties of the card that nothing asserted.

1. **Stale CR citation: 112.7a.** Rule 112 in the current Comprehensive Rules
   is *Spells*; abilities are rule 113, and "an ability on the stack exists
   independently of its source" is **113.7a**, which is what the other 57
   citations in this repo say. `reaper_from_the_abyss.rs` and
   `trigger_independence.rs` (3 sites) carried the pre-renumbering 112.7a, as
   did one line of `docs/plans/BUG_TRIAGE_PROGRESS.md`. All corrected.

2. **Dead binding.** `on_end_step` took `self_id` and immediately discarded it
   with `let _ = self_id;`. The parameter is genuinely unused — deliberately,
   since reading the source is the bug the comment is about — so it is now
   named `_self_id` in the signature and the reason is a doc comment on the
   method rather than a line of code that does nothing.

### Card data checked against the fetched text

| field | oracle | code |
|---|---|---|
| cost | `{3}{B}{B}{B}` | `Generic(3), Black, Black, Black` OK |
| type | `Creature - Demon` | `Creature`, `["Demon"]` OK |
| P/T | 6/6 | `Some(6)/Some(6)` OK |
| keywords | Flying, Morbid | `vec![Keyword::Flying]` OK - morbid is an ability word (CR 207.2c), not a keyword ability, so it is not a `Keyword` |
| oracle text | verbatim match | OK |
| trigger | end step, targets a non-Demon creature | `TriggerKind::EndStep` with `TargetRequirement::CreatureWithFilter(TargetFilter::NotSubtypes(["Demon"]))` OK |

### Tricky interactions checked

- **"At the beginning of EACH end step"** — fires on both players' end steps.
  **Pass**: `step_trigger_scope` defaults to `TriggerScope::Each` and the card
  correctly does not override it. Was untested; see below.
- **Ruling 2011-09-22, "The morbid ability is mandatory. If you control the
  only non-Demon creature when the ability triggers, you must choose it as the
  target."** **Pass**, and handled generally rather than by this card:
  `triggers.rs` prompts with `optional: false` and auto-takes the single legal
  target. Was untested; see below.
- **Morbid is an intervening-if (CR 603.4)**, checked at trigger time and
  again on resolution. **Pass**, both arms tested.
- **Morbid counts any creature death** — `creature_died_this_turn` is set by
  the SBA zero-toughness path, by `destroy`, and by `sacrifice` (which routes
  through `destroy`), and reset when the turn advances. **Pass.**
- **CR 113.7a, the Reaper destroyed in response.** **Pass**, tested.
- **CR 608.2b, the target became illegal.** **Pass**, tested in both halves —
  the generic one (left the battlefield) and the card's own restriction (the
  target gained the Demon subtype).
- **"non-Demon" excludes the Reaper itself and any other Demon.** **Pass**;
  now also asserted directly.
- **"destroy" respects indestructible and regeneration.** **Pass** — the
  effect goes through `destruction::try_destroy`, not a bare `move_object`.
- **Hexproof.** **Pass**, and generally: trigger targets come from
  `engine::valid_targets_for_req`, the same helper spell casting uses, given
  the trigger's controller — so the Reaper's controller can still target their
  own hexproof creature while an opponent's is filtered out.

### Coverage added, and a vacuous sweep fixed

Two properties of this card had no test, and closing the first one turned up a
weakness in an existing pair of sweeps.

`your_upkeep_scope.rs` swept every card whose step trigger is scoped `Your` and
checked both directions — fires on its controller's step, silent on the
opponent's. Nothing did the same for the `Each` cards, so I added
`an_each_step_trigger_fires_on_both_players_steps`.

That sweep does not discriminate on its own: scoping Reaper to `Your` as a
mutation simply moved it out of the `Each` list into the `Your` list, and both
sweeps stayed green. Each sweep only looks at the cards that already claim the
scope it is testing, so neither can catch a card scoped the wrong way.

The outside opinion is the card's own text, which
`card_data_invariants.rs::oracle_text_says_what_scryfall_says` already checks
against Scryfall. So `a_step_triggers_scope_is_the_one_its_oracle_text_states`
reads the expected scope out of the oracle text — "each upkeep" -> `Each`,
"your upkeep" -> `Your`, "enchanted player's upkeep" -> `AttachedPlayer` — and
checks the declaration against it, over both faces of every card, 40+ triggers.
That one does fail on the mutation, naming the card and both scopes.

### Test coverage

- morbid intervening-if, both arms, and no spurious "Trigger removed" log:
  `intervening_if.rs::reaper_from_the_abyss_end_step_trigger_respects_its_morbid_clause`
- the Reaper destroyed in response:
  `trigger_independence.rs::reapers_end_step_trigger_resolves_after_the_reaper_dies`
- target became illegal, both halves:
  `trigger_target_recheck.rs::a_trigger_whose_target_became_illegal_changes_nothing`
- the mandatory-target ruling, with the only legal target being your own:
  `trigger_targets_declared.rs::reapers_morbid_target_is_forced_even_when_it_is_your_own_creature` (new)
- "each end step" fires on both players' steps:
  `your_upkeep_scope.rs::an_each_step_trigger_fires_on_both_players_steps` (new)
- and the scope each card declares matches its printed text:
  `your_upkeep_scope.rs::a_step_triggers_scope_is_the_one_its_oracle_text_states` (new)

Both new invariants mutation-checked; the text-derived one names the card and
both scopes when it fails, and the forced-target one fails if the engine stops
auto-taking a single legal target.
