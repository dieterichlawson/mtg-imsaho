# Architecture review

Written 2026-08-24, after closing the 116-ticket audit backlog. The question
asked: is the engine properly factored, as simple as possible but no simpler,
and a pleasure to work in? Where is it not, and what do other engines do?

Everything below is measured against the tree at that date: 33.5k lines of
engine source (13.3k engine proper, 20.3k in `cards/`), 39.3k lines of test,
270 cards.

## What is already right

Worth stating first, because the rest of this document is a list of problems
and that gives a skewed impression.

- **The characteristics layer.** One authoritative reader for "what is this
  object", with `characteristics_invariant.rs` as a guard. This was the root
  cause of ~15 duplicate bug reports before it existed.
- **`engine_knows_no_cards.rs`.** A build-failing guard that no card name
  appears in engine code, with a staleness check. Very few codebases have an
  executable statement of their own layering rule. Keep it.
- **The unified destruction pipeline** (`destruction.rs`) and the
  cost-legality predicates (`can_pay_tap_cost`, `can_pay_with_sources`) — each
  is one place where a rule lives.
- **`submit_action(&state, …) -> GameState`.** A pure transition function over
  a cloned state. This costs a full `GameState` clone per action, which is a
  real price, but it buys trivial undo, trivial search for AI, and no aliasing
  bugs. Deliberate and defensible; don't "optimise" it away without a profile.

## Findings, ranked

### 1. `CardBehavior` is the expression problem, in its worst configuration

60 methods, 270 implementors. Usage is a long tail:

| implementors | methods |
|---|---|
| ≥16 | 9 (`card_data`, `on_resolve`, `target_requirement`, `activated_abilities`, …) |
| 3–15 | 20 |
| 1–2 | 26 |
| 0 | 5 (`trigger_zones`, `on_end_combat`, `on_any_creature_attacks`, `on_equipment_block_trigger`, `grants_player_protection_from`) |

A trait is cheap to add *cases* to (a new card) and expensive to add
*operations* to (a new question the engine asks). Magic adds operations
constantly, so every mechanic that doesn't fit an existing hook becomes a new
trait method **plus** a new engine call site that must remember to call it. I
added four such methods in the last session alone. Twenty-six hooks exist for
one or two cards each; five are now dead surface.

The failure mode isn't ugliness, it's *silent omission*: a hook the engine
forgets to call at one of its several call sites is a bug with no compile
error. That is exactly what the CR 608.2b target re-check bug was.

**The counter-model.** Forge inverts this: a card is a declarative text
script, and effects are a closed vocabulary of ~200 `*Effect` classes
(`DrawEffect`, `DestroyEffect`, `ChangeZoneEffect`, `CountersPutEffect`, …).
That vocabulary covers ~30,000 cards. We use 270 hand-written Rust files for
270 cards — a 1:1 ratio where Forge runs 150:1.

We already have the right pattern in-tree and it works: `resolve_card_effect(
source, key, target)` with a card-opaque `key`, used by Divine Reckoning and
Bitterheart Witch to run multi-step choice chains without the engine knowing
anything. That is the seed of a generic vocabulary.

**Recommendation.** Not a rewrite. Two bounded moves:
- Delete the 5 dead hooks now.
- Adopt a rule: a new mechanic gets a trait method only if a *second* card
  would plausibly use it; otherwise it goes through `resolve_card_effect`.
  Then collapse the existing 1-implementor tail into it opportunistically.

### 2. No CR 613 layer system — a correctness ceiling, not a present bug

There is no mention of layers, timestamps, or dependency anywhere in the
engine. `effective_power` composes in a fixed order: base (incl. the CDA from
`dynamic_pt`) → continuous `ModifyPT` → ±1/+1 counters → until-end-of-turn
`ModifyPT`.

That order is *arithmetically equivalent* to CR 613.4 today, because every
P/T effect in the pool is additive and addition commutes. There is no
`SetPT`, no "becomes a 1/1", no type-changing effect. So this is not causing
wrong answers right now.

It becomes wrong the moment any of these enters the pool:
- a set-P/T effect (Turn to Frog, Humility, most Clone variants);
- a type-changing effect (layer 4) that a later effect keys off;
- two continuous effects where the correct order depends on timestamp, or on
  dependency (CR 613.8).

XMage models this properly: `ContinuousEffects.apply()` walks layers 1→7 with
sublayers, assigns a monotone `order` as effects become active, and defers
effects via `isDependentTo`/`waitingEffects` until their dependencies are
applied.

There is a miniature violation today worth noting as the canary: Tree of
Redemption's exchange writes `obj.toughness` directly — a base-value write for
what is really a layer-7 effect. It is correct only because nothing else sets
P/T.

**Recommendation.** Do **not** build this speculatively. It's the largest
single gap versus XMage and also the one whose value depends entirely on a
question I can't answer: *is the card pool going to grow past Innistrad?* If
yes, do it before adding the cards, because retrofitting a layer system under
a populated pool is much worse than building it first. If the pool stays ISD,
the fixed order is fine and this stays a documented limitation.

### 3. Replacement effects: six mechanisms for one rules concept

CR 614 is one idea — an event is replaced before it happens. We express it
seven different ways:

| mechanism | engine call sites |
|---|---|
| `replacement_effects() -> Vec<ReplacementEffect>` (closed 3-variant enum) | 4 |
| `enters_tapped` | 1 |
| `entering_with_counters` | 1 |
| `modify_creature_entering_counters` | 1 |
| `entering_modifier_zones` | 1 |
| `replace_combat_damage_to_player` | 1 |
| `enters_as_copy` | 0 (dispatched via the enum) |

`ReplacementEffect` being a closed enum in the engine is precisely the shape
we removed from `PendingEffect` last pass — adding a replacement effect means
editing an engine enum, which is the thing `engine_knows_no_cards.rs` exists to
prevent.

XMage has exactly one: `replaceEvent(GameEvent)`, consulted from a single
place, with player choice when several apply (CR 616).

**Recommendation.** This is the highest-value generalisation that is also
bounded. One `fn replace_event(&self, state, self_id, event: &GameEvent) ->
Option<GameEvent>` consulted at one place, and the six bespoke hooks collapse
into it. ~3 cards use the enum today, so the migration is small.

### 4. Trigger dispatch duplicates itself along "me vs. everyone"

`PendingTrigger` has 20 variants, and they pair up:

```
SelfDies              / DeathWatch
EnteredBattlefield    / EnterWatch
CombatDamageToPlayer  / CombatDamageWatch
AttacksTrigger        / AttackWatch
```

with the matching trait methods paired the same way (`on_dies` /
`on_any_creature_dies`, `on_enter_battlefield` / `on_any_creature_enters`,
`on_combat_damage_to_player` / `on_any_combat_damage_to_player`, `on_attacks` /
`on_any_creature_attacks`). The only difference is whether the watcher is the
subject.

`TriggerScope { Each, Your, AttachedPlayer }` already exists for step
triggers. Extending that idea — one variant carrying `subject: SelfOnly |
Others | Any` — collapses roughly 8 enum variants and 8 trait methods into 4
and 4.

`ContinuousEffect` has the same shape in miniature: `ConditionalKeyword`,
`ConditionalModifyPT`, `ConditionalPreventAttack`, `ConditionalPreventBlock`
are four variants that differ from their unconditional twins by one
`EffectCondition` field. An `Option<EffectCondition>` on the base variants
removes four.

### 5. Two 1,200-line functions

| function | lines |
|---|---|
| `engine::submit_action` | 1,277 (14 `Action` arms, 12 nested choice arms) |
| `engine::legal_actions` | 1,128 |
| `triggers::collect_triggers` | 722 |
| `sba::check_state_based_actions` | 381 |

`engine.rs` is 4,576 lines and over half of it is those two functions. This is
the single biggest drag on "a pleasure to work in": every change to any action
means scrolling through a function longer than most whole modules, and the
match arms share mutable local state in ways that make each arm hard to read in
isolation.

There is no design question here — one function per `Action` variant, one per
`ResolutionChoiceKind`, in an `actions/` submodule. Pure mechanical
refactor, no behavior change, fully covered by the existing 1,396 tests.

### 6. Card data is ~1,500 lines of `vec![]`

Across 270 card files: 237 × `continuous_effects: vec![]`, 225 ×
`keywords: vec![]`, 262 × `flashback_cost: None`, 281 × `additional_cost:
None`, 194 × `triggered_abilities: vec![]`, 277 × `supertypes: vec![]`.

A vanilla creature currently costs 25 lines, of which 7 say "nothing here".
`CardData: Default` plus `..Default::default()`, or a small builder, removes
most of it. 34 cards are `card_data` and nothing else; 91 are ≤45 lines.

Trivial to do, no risk, and it is the change most visible every single time
someone opens a card file.

### 7. Test suite: organised by how bugs were found, not by behavior

39.3k lines, which is healthy. But `audit_*` (14 files, 7.8k lines) and
`pipeline_bugs_*` (10 files, 1.9k lines) are named after the *process that
discovered* the bug rather than the rule under test, so there is no way to
find "the tests for replacement effects" except by grepping. `tier*_cards`
(11 files, 8.1k lines) is likewise an artifact of implementation order.

Also: `fn registry()` is defined 86 times across test files, and 127 files
carry local helpers that mostly duplicate `common/`.

Low priority — but worth folding in opportunistically, and worth a rule that
new tests are named for the rule, as the recent ones are
(`tap_cost_legality.rs`, `simultaneous_events.rs`, `control_durations.rs`).

## Concepts other engines have that we lack entirely

From XMage's `ContinuousEffects`, which keeps nine typed collections:

| XMage concept | our equivalent |
|---|---|
| `layeredEffects` + layers/sublayers/timestamps/dependency | none (see §2) |
| `replacementEffects` + `replaceEvent` | six ad-hoc hooks (§3) |
| `asThoughEffects` — "as though it had haste", "as though it were in your hand", "as though it weren't tapped" | none; `can_cast_from_graveyard` is a bespoke 1-card hook |
| `costModificationEffects` | `ReduceCost` / `AlternativeCost` / `modified_cost`, scattered |
| `requirementEffects` / `restrictionEffects` | `ForceAttack` / `PreventAttack` / `PreventBlock` — coherent, fine as-is |
| `continuousRuleModifyingEffects` | `PreventCastingNamed`, `PreventArtifactAbilities` — same idea, ad hoc |

`asThough` is the interesting absent one. It is a single generic concept
("treat X as if Y for the purpose of Z") that we would otherwise meet as a
stream of one-card trait methods — which is how `can_cast_from_graveyard`
arrived.

## What I would actually do next

In order, and with honest sizing:

1. **Decompose `submit_action` / `legal_actions` / `collect_triggers`.**
   Mechanical, no behavior change, biggest daily quality-of-life win. ~1 pass.
2. **`CardData::default()` + delete the 5 dead hooks.** ~1 short pass, removes
   ~1,500 lines of noise.
3. **Unify replacement effects behind one `replace_event` hook.** Real
   correctness value (six mechanisms means six places to forget), bounded
   because only ~3 cards use the enum. ~1 pass.
4. **Collapse the self/other trigger pairs and the `Conditional*` continuous
   effect twins.** Removes ~12 enum variants and ~8 trait methods. ~1 pass.
5. **Adopt the "second card or it goes through `resolve_card_effect`" rule**
   for new mechanics. No work now; prevents the tail regrowing.

**Deliberately not recommending yet:**

- **The CR 613 layer system.** Correct, and the biggest gap, but currently
  buys no observable correctness because every P/T effect in the pool is
  additive. Its value is entirely a bet on the card pool growing. Decide that
  question first — if the answer is yes, this jumps to #1, because it is far
  cheaper to build before the pool grows than after.
- **Migrating cards to a declarative script format** (the Forge model). It is
  the right end state for a 30,000-card pool and clearly wrong for a 270-card
  one; the crossover is somewhere in the low thousands. Revisit only if the
  pool is going to grow by an order of magnitude.
- **Anything about the per-action `GameState` clone.** It's a real cost but
  it buys real properties, and nobody has profiled it.
