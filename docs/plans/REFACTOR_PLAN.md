# Refactor plan

Approved scope from `docs/ARCHITECTURE_REVIEW.md`, minus the CR 613 layer
system and the declarative-card migration, both explicitly out.

Seven phases. Each is one self-contained pass ending in a green tree, so any
of them can be the last one done without leaving the codebase mid-transition.

## How each phase is verified

Two categories, with different guarantees:

- **Mechanical phases** (0, 1, 6) must not change behavior. The guarantee is
  *the existing tests pass without being edited*. If a test needs changing,
  the phase did something it shouldn't have, and that is the signal to stop
  and look. Exception: phase 6 moves tests by definition — there the rule is
  that no assertion changes, only file and name.
- **Semantic phases** (2–5) collapse N mechanisms into 1. Behavior should be
  identical for everything that already worked, so the same rule applies —
  but each of these is *expected to uncover cases the old scattered version
  missed*, because that is the whole reason for unifying. Those get new tests,
  red-before/green-after, and are called out as bugs found rather than folded
  in silently.

Every phase also ends with `cargo check --workspace` at zero warnings and the
full suite under `ANTHROPIC_API_KEY=dummy`.

Where a phase introduces a new central mechanism, it also gets a **guard
test** in the style of `engine_knows_no_cards.rs` — a test that fails the
build if someone adds a variant without wiring it up. That is what keeps
these unifications from silently re-fragmenting.

## Ordering, and why

```
Phase 0  boilerplate + dead code     ── touches all 270 card files, nothing semantic
   │
Phase 1  decompose big functions     ── creates the seams everything else lands in
   │
   ├── Phase 2  replacement effects (incl. damage prevention)
   ├── Phase 3  triggers: self/other
   ├── Phase 4  continuous effects: conditional twins + restriction/requirement
   └── Phase 5  cost modification
   │
Phase 6  test reorganisation          ── last: phases 0–5 add and move tests
```

Phase 0 first because it edits every card file; doing it first means every
later diff is small and readable instead of buried in whitespace churn.

Phase 1 second is the load-bearing decision. The alternative — do the semantic
unifications first so the big functions shrink on their own — was rejected:
it means making every subtle rules change inside a 1,277-line function, where
it cannot be reviewed in isolation. Decomposing first is safe (see below) and
means phases 2–5 each land as a small diff in a small file.

Phases 2–5 are mutually independent and can be reordered or dropped
individually.

---

## Phase 0 — card data boilerplate and dead trait surface

**Problem.** Across 270 card files: 277 × `supertypes: vec![]`, 262 ×
`flashback_cost: None`, 281 × `additional_cost: None`, 237 ×
`continuous_effects: vec![]`, 225 × `keywords: vec![]`, 194 ×
`triggered_abilities: vec![]`. A vanilla creature costs 25 lines, 7 of which
say "nothing here". Separately, five `CardBehavior` methods have zero
implementors: `trigger_zones`, `on_end_combat`, `on_any_creature_attacks`,
`on_equipment_block_trigger`, `grants_player_protection_from`.

**Change.**
1. `impl Default for CardData`, and rewrite every card as
   ```rust
   CardData {
       name: "Walking Corpse".into(),
       cost: Some(ManaCost::new(vec![Generic(1), Colored(Black)])),
       card_types: vec![CardType::Creature],
       subtypes: vec!["Zombie".into()],
       power: Some(2), toughness: Some(2),
       ..Default::default()
   }
   ```
2. Delete `trigger_zones`, `on_end_combat`, `on_equipment_block_trigger` and
   their call sites.
3. `on_any_creature_attacks` is dead as of the Instigator Gang change — delete
   it and the `AttackWatch` machinery *if* phase 3 doesn't want it; otherwise
   leave it for phase 3 to fold in. Decide at the time.
4. Keep `grants_player_protection_from`: it has no card implementor but is
   exercised by `player_protection.rs`, which registers a card for it. Note
   this in the guard so it isn't deleted by a future sweep.

**Size.** ~1,500 lines deleted, 270 files touched, all mechanical.
**Risk.** Very low. `Default` on a struct with no card semantics can't change
behavior; a missed field is a compile error, not a silent default.
**Guard.** None needed.

---

## Phase 1 — decompose the three big functions

**Problem.**

| function | lines |
|---|---|
| `engine::submit_action` | 1,277 (14 `Action` arms + 12 nested choice arms) |
| `engine::legal_actions` | 1,128 |
| `triggers::collect_triggers` | 722 |

`engine.rs` is 4,576 lines; over half is those two functions.

**Change.** `engine/` becomes a directory module:

```
engine/mod.rs          — the loop, setup, shared helpers
engine/actions/        — one file per Action variant
      pass_priority.rs, play_land.rs, cast_spell.rs, activate_ability.rs,
      declare_attackers.rs, resolve_choice/ (one fn per ResolutionChoiceKind), …
engine/legal/          — mana_abilities.rs, activated.rs, casting.rs, combat.rs
triggers/collect/      — one fn per GameEvent variant
```

**Why this is safe rather than brave.** `submit_action` is
prologue (clone / clear events / reset index) → `match` → epilogue
(`finish_spell_resolution_if_idle`). The arms share **no mutable locals** —
each operates on `new_state`. So each arm lifts to
`fn apply_x(state: &mut GameState, …, registry: &CardRegistry)` by cut and
paste. Two arms recurse into `submit_action`; those keep doing so.

`legal_actions` is the one that needs care: it *does* share state across
sections — `early_mana_sources`, `prevent_artifact_abilities`, the
`AbilityGroup` accumulator. Those become an explicit `LegalActionsCtx` struct
passed to each section rather than closed-over locals. This is the only part
of phase 1 that is design rather than transcription, and it should be done as
its own commit.

**Size.** Large diff, near-zero semantic content. ~15 new files.
**Risk.** Low but not nil — the risk is transcription error, which the 1,396
existing tests cover well.
**Verification.** No test file may change. If one does, revert and find out why.

---

## Phase 2 — one replacement-effect mechanism

**Problem.** CR 614 is one idea, expressed seven ways:

| mechanism | engine call sites |
|---|---|
| `replacement_effects() -> Vec<ReplacementEffect>` (closed 3-variant enum) | 4 |
| `enters_tapped` | 1 |
| `entering_with_counters` | 1 |
| `modify_creature_entering_counters` | 1 |
| `entering_modifier_zones` | 1 |
| `replace_combat_damage_to_player` | 1 |
| `enters_as_copy` | 0 (routed via the enum) |

Plus three CR 615 damage prevention/replacement effects living as
`ContinuousEffect` variants: `PreventCombatDamage`, `DoubleCombatDamage`,
`PreventDamageRemoveCounter`. Those are the same rules concept and belong in
this phase, not phase 4.

`ReplacementEffect` being a closed enum *in the engine* is precisely the shape
removed from `PendingEffect` last pass — adding one means editing an engine
enum, which is what `engine_knows_no_cards.rs` exists to stop.

**Change.** One hook, consulted from one place:

```rust
fn replace_event(&self, state: &GameState, self_id: ObjectId, event: &GameEvent)
    -> Option<GameEvent> { None }
```

with an engine-side `replacement::apply(state, event, registry) -> GameEvent`
that walks candidate permanents, applies at most one replacement per effect
per event (CR 614.5), and asks the affected player to order them when several
apply (CR 616.1).

The seven hooks collapse into `replace_event` matching on the event:
`EnteringBattlefield` for tapped/counters/copy, `DamageWouldBeDealt` for the
combat-damage ones, `TokensWouldBeCreated` for Parallel Lives,
`WouldDraw` for Laboratory Maniac.

**What this is likely to uncover.** Each of the seven is consulted from
exactly one place today. That is fine when each is called at the only site
that matters and suspicious otherwise — for example `entering_with_counters`
is consulted on the battlefield-entry path but there is more than one way a
permanent enters. Expect one or two real bugs here; they get their own tests.

**Size.** Medium. Only ~3 cards use the enum and ~8 use the entry hooks.
**Risk.** Medium — this changes an ordering-sensitive path.
**Guard.** A test asserting every `GameEvent` variant that can be replaced is
reachable through `replacement::apply`, so a new event type can't silently
bypass replacement.

**Note on CR 616 ordering.** Choosing among several applicable replacements
is the affected player's choice. We have no such prompt today because no
board can produce two. Build the seam (`Vec<Candidate>` + a choice point) but
do not build the prompt until a second card needs it.

---

## Phase 3 — triggers: collapse "me" and "everyone"

**Problem.** `PendingTrigger` has 20 variants that pair up:

```
SelfDies             / DeathWatch
EnteredBattlefield   / EnterWatch
CombatDamageToPlayer / CombatDamageWatch
AttacksTrigger       / AttackWatch
```

with matching trait methods (`on_dies` / `on_any_creature_dies`,
`on_enter_battlefield` / `on_any_creature_enters`, `on_combat_damage_to_player`
/ `on_any_combat_damage_to_player`, `on_attacks` / `on_any_creature_attacks`).
The only difference is whether the watcher is the subject.

**Change.** One variant per event, carrying

```rust
subject: TriggerSubject   // SelfOnly | Others | Any
```

and one trait method per event taking the subject as a parameter, the way
`on_attacks` now takes `AttackInfo`. `TriggerScope { Each, Your,
AttachedPlayer }` already does this for step triggers — same idea, extended.

Roughly 8 enum variants and 8 trait methods become 4 and 4.

**What this is likely to uncover.** The pairs have drifted: the "self" and
"other" halves of the same event were written at different times and gate on
different things (this is exactly how `selhoff_occultist-02` happened — the
"other" half had a battlefield guard the "self" half didn't). Unifying forces
those to agree.

**Size.** Medium.
**Risk.** Medium — trigger dispatch is where several of last pass's bugs lived.
**Guard.** Extend `engine_knows_no_cards.rs` style: a test that every
`GameEvent` which can trigger has exactly one collection site.

### What was done (and where it diverged from the plan)

The duplication turned out not to be the self/watch pairing. It was that
all twenty variants restated the *same four facts about the source* —
object id, card id, controller, description — under four different naming
conventions (`watcher_id` / `object_id` / `dead_id` / `creature_id`, and the
matching `*_card_id`). That is why five accessors and `display_name` were
each a twenty-arm or-pattern over one field.

So the shape is `PendingTrigger { source: TriggerSource, event: TriggerEvent }`
rather than the planned `subject: TriggerSubject`. The self and watcher
events stay separate variants, deliberately: they dispatch to different
`CardBehavior` hooks and match different `TriggerKind`s, and a card can have
both — Blood Artist's "whenever this or another creature dies" has to fire
its watcher ability when it dies itself, which a `source.id == subject.id`
test would get wrong. What they no longer do is carry two independent
descriptions of the same source.

The predicted drift was real and worse than expected. It was not two
mismatched guards but ten: half the dispatch arms gated on the source still
being on the battlefield and half did not, and the split ran through matched
pairs (a creature's own combat-damage trigger resolved after it died, a
watcher's did not; same for attacks). CR 113.7a says a triggered ability on
the stack is independent of its source, so the ungated half was right — the
same rule already applied to the end-step arm for Reaper from the Abyss. The
gate is gone from the engine entirely.

Removing it exposed twenty-three cards that had written the gate themselves.
Sixteen are protective and stay (transform needs a battlefield permanent; a
card that puts a counter on itself must not counter a graveyard card). Seven
were bugs, fixed with `tests/trigger_source_independence.rs`: Rakish Heir,
Curse of Stalked Prey, Balefire Dragon, Burning Vengeance, Charmbreaker
Devils, Curiosity, and the three upkeep Curses. The Curses needed
`GameState::attached_player` — `move_object` cleared `attached_to_player`, so
a destroyed Curse no longer knew whom it had cursed (CR 608.2).

Two smaller things fell out: `chosen_targets` now lives on the source, so
every trigger can carry targets (seven of twenty variants had the field, so a
`target_requirement` declared on any other kind was silently dropped); and
`display_name` is transform-aware for every kind, not just upkeep.

Guard: `triggers_are_built_in_one_place` (a trigger can only be constructed
in `triggers/collect/`, plus the two the engine raises itself) and
`trigger_dispatch_does_not_gate_on_the_source_zone`.

---

## Phase 4 — continuous effects: conditional twins, and a real restriction/requirement category

**Problem, part one.** Four variants differ from their unconditional twins by
a single field: `ConditionalKeyword`, `ConditionalModifyPT`,
`ConditionalPreventAttack`, `ConditionalPreventBlock`.

**Change, part one.** `Option<EffectCondition>` on the base variants. Removes
four variants and the parallel handling in `continuous_pt_mods` and
`has_keyword`. Purely mechanical.

**Problem, part two.** The 12 restriction/requirement/rule-modifying effects
are queried by 20 scattered `has_continuous_effect` calls across four files
(`state.rs` 12, `engine.rs` 3, `combat.rs` 3, `damage.rs` 2). There is no way
to ask "what stops this creature attacking?" — only a series of individual
questions each caller has to know to ask.

**Change, part two.** Group them by what CR calls them, and give each category
one predicate:

```rust
state.attack_restrictions(id, registry) -> Vec<Restriction>   // CR 509.1b "can't"
state.attack_requirements(id, registry) -> Vec<Requirement>   // CR 508.1d "must"
state.can_activate(id, kind, registry) -> bool                // PreventArtifactAbilities, …
state.can_cast(card, player, registry) -> bool                // PreventCastingNamed
```

Damage-related ones (`PreventCombatDamage`, `DoubleCombatDamage`,
`PreventDamageRemoveCounter`) are **not** here — they moved to phase 2.

**Size.** Part one small, part two medium.
**Risk.** Low. Part one can't change behavior; part two is a query-shape
change with the same underlying data.

### What was done

Part one turned out to be the cause of part two, so they were one change.

The four `Conditional*` variants did not become `Option<EffectCondition>` on
their twins — that would have made every one of the ~40 unconditional
construction sites write `condition: None`, and would still have left only
four effects able to be conditional. Instead:

```rust
ContinuousEffect::When { condition, effect: Box<ContinuousEffect> }
```

Wrapping means "as long as" qualifies *any* effect, and only the seven cards
that were already conditional changed. Net: 22 variants become 19.

The reason part two existed is that a baked-in condition needs its own
reader. There were seven walks over the battlefield looking for continuous
effects — `has_continuous_effect`, `count_continuous_effect`,
`has_conditional_prevent`, `has_conditional_keyword`, the loop inside
`continuous_pt_mods`, and two more in `combat.rs` and `mana_sources.rs`. Two
of them had drifted: `combat.rs` missed back faces, `mana_sources.rs` missed
instance effects.

`GameState::walk_effects` is now the only one, with `has_effect`,
`count_effect`, `global_effects` and four payload-reading sites on top of it.
`effect_applies_to` went from public with seven callers to private with one.
The planned `attack_restrictions` / `attack_requirements` vectors were not
built: with one walk, `must_attack`, `cant_be_blocked` and `untaps_normally`
alongside the existing `can_attack` / `can_block` cover every caller, and a
`Vec<Restriction>` nobody enumerates would be ceremony.

One thing worth recording: `want` is tested against the unwrapped effect
*before* the condition, and that is load-bearing. Evaluating conditions
eagerly (say, inside `continuous_effects_of`) sends
`EffectCondition::SelfHasKeyword` back through `has_keyword` into itself.
The old code avoided this by accident, by filtering on the keyword first.

Two behaviour fixes fell out: a permanent that entered as a copy lost its
dynamic P/T (a token copy of Wreath of Geists), and `casting_banned` read
only instance effects.

`PreventCombatDamage`, `PreventDamageRemoveCounter` and `DoubleCombatDamage`
are still `ContinuousEffect` variants — phase 2 unified the *mechanism* that
applies replacements but left these three declared here, which is the right
place for "this permanent has this property"; `damage.rs` reads them through
the one walk.

---

## Phase 5 — cost modification

**Problem.** A spell's cost is adjusted in at least five unrelated ways:

| mechanism | users |
|---|---|
| `ContinuousEffect::ReduceCost` | 1 card |
| `ContinuousEffect::AlternativeCost` | 1 card |
| `CardBehavior::modified_cost` | 2 cards |
| `CardData::flashback_cost` | 27 cards |
| `CardData::additional_cost` | 8 cards |

`effective_spell_cost` handles reductions; `alternative_costs_from_effects`
handles alternatives; flashback is special-cased in `legal_actions` (which
already had a bug last pass where multiple flashback grants collapsed to one);
`additional_cost` is matched inline at six separate sites in `legal_actions`
and `submit_action`.

**Change.** One pipeline with an explicit order, matching CR 601.2f:

```rust
struct CostToPay {
    base: ManaCost,                  // printed, or the alternative that replaced it
    additional: Vec<AdditionalCost>, // sacrifice, exile, discard, …
    increases: Vec<ManaCost>,
    reductions: Vec<ManaCost>,
}

fn cost_to_cast(state, card, player, how: CastMethod, registry) -> CostToPay
```

where `CastMethod` is `Normal | Flashback(ManaCost) | Alternative(ManaCost)`.
Everything that wants to know what a spell costs calls this, and the six
inline `additional_cost` matches become one.

**What this is likely to uncover.** The order of reduce-then-alternative is
currently implicit and probably wrong in at least one direction (CR 601.2f:
alternative cost replaces the base, *then* increases, *then* reductions).
Cost reductions almost certainly aren't applied to flashback costs today.

**Size.** Medium. Touches `legal_actions` heavily — do it after phase 1.
**Risk.** Medium. Cost bugs surface as "the action isn't offered", which is
invisible unless tested for.
**Guard.** A property test in the shape of `mana_filters.rs`'s
`every_tap_plan_the_solver_returns_actually_pays_the_cost`: for a spread of
boards and cast methods, the cost the engine charges equals the cost
`cost_to_cast` reports.

### What was done

`engine::costs` with `cost_to_cast(state, registry, card, player, method)`,
as planned, but `CostToPay`'s four fields collapsed to two: `mana` and
`additional`. There are no cost increases in this pool, and keeping
`increases`/`reductions` as separate vectors nobody sums would be ceremony —
`cost_to_cast` applies CR 601.2f's order internally and returns the total.
The comment in `reduce` names where an increase would go.

`CastMethod` is `Normal | Alternative(ManaCost)`; a flashback cost is just an
alternative cost, so the planned third variant was unnecessary.

The predicted bug was there: cost reductions did not apply to flashback
costs, and the in-pool case is Skaab Ruinator (which is cast from the
graveyard for its printed cost, through the flashback path) under Heartless
Summoning. `modified_cost` also short-circuited reductions.

Two more, unpredicted:

  - The graveyard path checked one of the three `AdditionalCost` kinds and
    reported no sacrifice options or label at all.
  - The cast handler and the exile-choice handler each auto-picked which
    creature cards to exile, and disagreed: the cast handler ranked by
    `obj.power`, which is `None` for every non-token card, so all candidates
    scored 0 and it picked by object id. Corpse Lunge's damage is the exiled
    creature's power.

Both cost effects also read `card_data().continuous_effects` directly — the
same class of bug phase 4 removed from the rest of the engine — so they
ignored back faces and runtime grants. They go through
`state.effects_controlled_by` now.

The planned property test was not written. `spell_costs_are_determined_in_one_place`
does the same job structurally and cheaply: no call site outside
`engine::costs` may read `additional_cost` and act on it. A property test
over generated boards would restate what the six behaviour tests already
pin, since there is now only one function that can answer the question.

---

## Phase 6 — test reorganisation

**Problem.** 128 files, 38,959 lines. The big ones are named after the process
that found the bug, not the rule under test:

- `audit_*` — 14 files, 7,782 lines (largest: `audit_bugs.rs` at 2,116)
- `pipeline_bugs_*` — 10 files, 1,855 lines
- `tier*_cards` — 11 files, 8,080 lines (largest: `tier15_cards.rs` at 3,023)

There is no way to find "the tests for replacement effects" except by
grepping. Separately, `fn registry()` is defined 86 times, and 127 files carry
local helpers duplicating `common/`.

**Change.**
1. Move the 86 `registry()` copies and the common local helpers into
   `common/`. Mechanical.
2. Re-file `audit_*` / `pipeline_bugs_*` / `tier*_cards` tests by the rule they
   exercise, matching the naming the recent files use (`tap_cost_legality.rs`,
   `simultaneous_events.rs`, `control_durations.rs`, `trigger_snapshots.rs`).
   Card-specific tests that aren't really about a rule go to
   `cards/<card_name>.rs`.
3. Write `docs/TESTING.md`: one paragraph on where a new test goes.

**Rule.** No assertion may change. Only file, name, and the helper it calls.
Diff review is `git diff -M --stat` showing renames, not rewrites.

**Size.** Large diff, zero semantic content.
**Risk.** Very low.
**Do it last** because phases 0–5 will add tests and delete others.

---

## Explicitly out of scope

- **CR 613 layer system.** Declined. The fixed P/T order stays correct while
  every effect is additive; the moment a set-P/T or type-changing effect is
  added this becomes wrong, so `docs/ARCHITECTURE_REVIEW.md` §2 stays as the
  record of that limitation.
- **Declarative card scripts.** Declined.
- **`asThough` effects.** Not on the approved list. Flagging that
  `can_cast_from_graveyard` is the first instance of this shape; if a second
  arrives, revisit before adding a second bespoke hook.
- **The per-action `GameState` clone.** Unmeasured; leave alone.

## Open questions

1. **One PR per phase, or one branch for the lot?** Phases are independent
   after phase 1; I'd suggest a branch per phase so each is reviewable, but
   the remote can't delete branches in this environment (403 on ref deletion),
   so they will accumulate. Alternatively: one branch, one commit per phase.
2. **Phase 3 vs phase 0 item 3.** `on_any_creature_attacks` / `AttackWatch` is
   dead right now. Delete it in phase 0, or keep it and let phase 3 decide the
   unified shape? I lean delete — it's easier to re-add the general version
   than to refactor around a dead one.
3. **Stop-on-bug or note-and-continue?** Phases 2 and 5 are expected to find
   real bugs. Fix them inline as part of the phase (my default), or file them
   and keep the phase purely structural?
