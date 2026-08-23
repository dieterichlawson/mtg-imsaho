# Bug triage progress log

Running record of work against the `new_pipeline/tickets/` backlog. Append to
this file as clusters land — it is the resumable state for any agent or device.

**Companion doc:** `docs/plans/SESSION_HANDOFF.md` holds the original cluster
analysis and rationale. This file holds *what has actually been done since*.

**Branch:** `claude/bug-triage-fixes-7516rt`

## How to run the tests here

`cargo test --workspace` reports 6 failures in `mtg-player`'s
`llm_conversation` suite in any sandbox without an API key —
`mtg-player/src/llm.rs:487` panics on a missing `ANTHROPIC_API_KEY`. These are
environment noise, not regressions. Run:

```
ANTHROPIC_API_KEY=dummy cargo test --workspace
```

With that set, all 16 llm_conversation tests pass. Per CLAUDE.md: check the
exit code and read the full output for both `FAILED` and `could not compile`;
never filter through `grep FAILED`.

## The root-cause refactor (read this first)

Cluster 4 was a symptom. The disease was that **a permanent's characteristics
had no single authoritative reader**, so ~55 call sites each improvised. Three
things made that inevitable; all three are now fixed, and the rule is enforced
by `mtg-engine/tests/characteristics_invariant.rs`.

**The rule:** an object's characteristics = its **active face** UNION its
**runtime grants**. `face_data` is the printed half (back face when
transformed); the object-level vectors are grants only (Olivia's "Vampire",
Grimoire's "Zombie"). Tokens are the exception — no registry face, so their
object fields carry their printed characteristics.

What was wrong:

1. **The composition rule contradicted itself.** `card_types_of` and
   `colors_of` returned the object's vector *instead of* the face's when
   non-empty; `subtypes_of` unioned. Same question, different answer depending
   on the field. All three union now.
2. **The fields were populated on one code path and not the other.**
   `setup_game` copied each card's data onto its library object; `create_object`
   (every test helper, token, reanimation) left them empty. Proven directly:
   Avacyn's Pilgrim came out `card_types=[Creature] subtypes=["Human","Monk"]`
   from `setup_game` and `[] []` from the test helper. **So card code reading
   the raw field worked in a real game and silently did nothing under test** —
   the bug was invisible to the tests written to catch it, which is why one
   defect was re-reported ~15 times under 15 card names. `setup_game` no
   longer copies.
3. **Nothing enforced the rule.** It was documented in `state.rs` and violated
   in ~55 places. Now a test fails the build, naming file, line and the
   accessor to use. Its allowlist has five entries that implement the layer
   rather than consume it, and a second test rejects any entry that stops
   needing its exemption (it caught two dead ones immediately).

Also removed: `apply_transform` copying the new face's name/keywords/subtypes
onto the object (a second source of truth whose only job was to agree with the
first — and which a card hand-rolling its own transform left disagreeing), and
the copy path's redundant duplicates.

**Two real bugs this surfaced**, both invisible before because tests and
production disagreed:
- **Runechanter's Pike** counted instants/sorceries in the graveyard off raw
  `o.card_types` with no fallback — it worked *only* because `setup_game`
  populated that field, and was already broken in every test. (`dynamic_pt`
  now takes a registry; a characteristic-defining ability needs one.)
- **Champion of the Parish** used `registry.card_data`, which is always the
  FRONT face, so it counted a transformed werewolf as a Human — the same
  defect `hamlet_captain-01` was filed for.

**Nine tests were re-based** from asserting the duplication to asserting the
guarantee. The sharpest case: `audit_subtype_family`'s Bug BD demanded that
`setup_game` populate `obj.subtypes` *too*, and named itself the root cause of
three sibling bugs. That prescription was the disease — it asks for more
duplication across the two paths. Replaced by its inverse, plus a test pinning
the property whose absence hid everything: an object built by `create_object`
and one built by `setup_game` must agree on their characteristics.

`new_pipeline/prompts/auditor-insights.md` — which every audit agent reads
before starting — recorded this backwards ("always empty for non-token
permanents"). Corrected, with the two superseded entries marked RESOLVED in
place so a top-to-bottom read cannot act on them.

## Second root-cause refactor: card logic in the engine

**The rule: no card-specific logic in engine modules.** Enforced by
`mtg-engine/tests/engine_knows_no_cards.rs`. It had leaked in by two routes.

**By name.** `engine.rs` called `registry.get_id_by_name("Evil Twin")` — twice —
to re-find a copy's granted ability, because the copy overwrites `card_id` and
the granting card's identity was otherwise lost. `sba.rs` looked up
"Garruk Relentless" to run his state-triggered ability, with the "two or fewer
loyalty" threshold and trigger text written into state-based actions. Both are
now generic: `GameObject::copy_grantor` (CR 706.2 — a copy effect may add
abilities) and `CardBehavior::state_trigger_condition` /
`state_trigger_description` (CR 603.8).

**By enum variant.** `PendingEffect` is a closed engine enum, so a card needing
a deferred resolution had to add a variant *and* an engine match arm to execute
it. That is structurally guaranteed to pull card rules into the engine, and it
had: Ghost Quarter's library search, Moorland Haunt's Spirit token, Curse of
Oblivion's exile loop, Elder Cathar's Human bonus, Graveyard Shovel's 2 life,
Tribute to Hunger's toughness gain, Grimgrin's counter, Bloodgift Demon's
draw-and-lose-1, Fiend Hunter's bookkeeping, Night Terrors' exile, Bitterheart
Witch's two-step Curse search, Garruk's -1 tutor, and ~70 lines of Divine
Reckoning's entire choice chain — each with the card's name in the engine's log
strings.

`PendingEffect::CardEffect { source_id, key }` replaces them: the engine routes
the chosen target back to the source card's `CardBehavior::resolve_card_effect`
and does nothing else. Chain state travels in `key`, which the engine treats as
opaque — the shape of a card's intermediate state is not the engine's business.
**PendingEffect went from 29 variants to 16**, and every remaining one is a
general primitive.

Also generalised: `damage.rs` had `is_non_wolf_damage_prevented`, hardcoding
Moonmist's subtype list behind a `TemporaryEffect` variant named after it. Now
`PreventCombatDamageExcept { filter }` — which is what the card actually says —
with Moonmist supplying the filter. And `PendingEffect::AddCounters` carried a
`human_bonus: bool`, i.e. Elder Cathar's rule as a flag on a shared effect; the
card had a *second* copy of the same check for its auto-pick path, so the rule
was written twice and could drift.

Added `GameState::change_life` / `gain_life` / `lose_life`: every site was
hand-rolling read-life, write-life, push `LifeChanged`, so a site that forgot
the event would silently break any life-total watcher.

**The guard has two checks**, both verified by reintroducing a violation:
a literal card name passed to `get_id_by_name` in an engine module, and a
`PendingEffect` variant used by exactly one card. The second is a *signal*, not
proof — a general primitive can have one user today — so such variants must be
listed with the reason they are general. Six are (DebuffUntilEOT,
CantBlockThisTurn, Mill, DestroyCreature, ReturnToHand, CopyCreature); the
thirteen removed would all have failed, because their arms spelled out card
text rather than applying a general rule to card-supplied parameters. Stale
entries fail too.

**Falkenrath Noble** turned out never to need a deferred effect at all — its
target is locked in at CR 603.3d, so it was using `PendingEffect` as a plain
function call. It now applies its own drain directly.

## Cluster status

| # | Cluster | Tickets | Status |
|---|---------|--------:|--------|
| 2 | Protection-from-source in ability targeting | ~10 | **done** |
| 1 | Intervening-if triggers (CR 603.4) | 12 | **done** |
| — | Confirm-and-close already-fixed tickets | 19 | **done** |
| 3 | Enters-tapped replacement effects | 6 | **done** |
| 4 | Card code reading empty object-level fields | 7 | **done** |
| 5 | Control-on-entry ordering | 4 | **done** |
| 6 | Targeted trigger declared untargeted | 4 | **done** |
| 7 | Card-specific one-offs | ~45 | not started |

## Done

### Cluster 2 — protection from source during activated-ability targeting

**Root cause.** `generate_ability_targets` filtered candidates through a
`can_be_targeted` wrapper that hard-coded `source_id: None`. The
protection-from-source check inside `can_be_targeted_by` only runs when
`source_id` is `Some`, so with `None` it was silently skipped: every creature
with protection from the ability's source still showed up as a legal target
(CR 702.16b). The spell path (`valid_targets_for_req`) threaded the source
through correctly; the ability path never did.

**Fix** (`mtg-engine/src/engine.rs`):

- All five `can_be_targeted` call sites in `generate_ability_targets`
  (`Creature`, `CreatureWithFilter`, `PlayerOrPlaneswalker`, `AnyTarget`,
  `PermanentWithFilter`) now call
  `can_be_targeted_by(..., Some(source_id), ...)`.
- Found the same bug on the **spell** path while fixing it: the
  `PlayerOrPlaneswalker` branch of `valid_targets_for_req` (was engine.rs:1681)
  passed `None` rather than `Some(spell_id)`, so a planeswalker with protection
  from the spell was targetable. Fixed too.
- Deleted the `can_be_targeted` wrapper entirely, so the `None` default cannot
  be reintroduced by a future call site.

**Test.** `mtg-engine/tests/ability_target_protection.rs` — two tests covering
both target-requirement shapes (Avacynian Priest / `CreatureWithFilter`,
Elder of Laurels / `Creature`). Each asserts the unprotected creature *is*
offered as a target first, so the test cannot pass vacuously. Verified red
against the pre-fix engine and green after.

**Tickets closed:** avacynian_priest-01, cobbled_wings-02, daybreak_ranger-01,
elder_of_laurels-01, kessig_wolf_run-01, sharpened_pitchfork-01,
stensia_bloodhall-01, inquisitor_s_flail-01, skirsdag_cultist-03, evil_twin-06.

### Cluster 1 — intervening-if trigger conditions (CR 603.4)

**Root cause.** An intervening-if clause is checked when the ability *would*
trigger, not only when it resolves; a false condition means the ability never
goes on the stack. `collect_triggers` had no notion of this — it queued a
trigger for every permanent whose active face declared one and left the
condition to the resolution handler. The board ended up correct (which is why
no existing test caught it), but a phantom stack entry appeared and opened a
priority window the rules say shouldn't exist.

**Fix.** New `CardBehavior::should_trigger`, consulted at dispatch time on both
the step (upkeep / end step / end of combat) and ETB paths in
`collect_triggers`. It follows the existing `should_trigger_on_spell_cast` /
`_on_blocks` / `_on_becomes_blocked` gates, which solve the same problem for
CR 603.2 event conditions. Defaults to `true`. It takes the `TriggerKind`, so a
face with both a conditional and an unconditional trigger (Howlpack Alpha's
upkeep transform vs. its end-step Wolf token) gates only the conditional one.

Two `helpers` functions carry the shared conditions instead of duplicating
them across 15 cards: `werewolf_should_trigger` delegates to the card's own
`should_transform` — which is what stops dispatch-time and resolution-time
checks from ever disagreeing — and `morbid_should_trigger` reads
`creature_died_this_turn`.

**Deliberately excluded** (they read as intervening-if but aren't): Cloistered
Youth ("you may transform"), Screeching Bat ("you may pay {2}{B}{B}"), Delver
of Secrets (looks at the top card regardless, then transforms on what it finds).

**Test.** `mtg-engine/tests/intervening_if.rs`, 8 tests. They assert on the
stack after dispatch, not the resolved board, since the board was already
right. Every condition is covered in both directions across the whole family,
so the gate can't pass by suppressing everything. 5 of the 8 verified red
against the pre-fix engine.

**Bug found along the way.** The family test caught `instigator_gang-01`
independently: Wildblood Pack declared only its `AnyCreatureAttacks` ability,
and `face_trigger_description` reads the *visible* face's list — so the back
face had no upkeep trigger and could never turn back into a Human, no matter
how many spells were cast. Fixed in the same pass.

**Tickets closed:** daybreak_ranger-02, gatstaf_shepherd-01,
grizzled_outcasts-01, hanweir_watchkeep-01, instigator_gang-01, kruin_outlaw-01,
reckless_waif-01, tormented_pariah-01, ulvenwald_mystics-01,
village_ironsmith-01, villagers_of_estwald-01, woodland_sleuth-01.

**Correction to `SESSION_HANDOFF.md`'s grouping.** It listed
`mentor_of_the_meek-01` and `wooden_stake-01` in this cluster; reading the
tickets, neither is intervening-if. `mentor_of_the_meek-01` wants the entering
creature's power *snapshotted* into `PendingTrigger::EnterWatch` at trigger
time (CR 603.2 event condition). `wooden_stake-01` is a "blocks a Vampire"
event condition, which the existing `should_trigger_on_blocks` hook already
covers. Both are tracked as one-offs instead.

### Confirm-and-close pass — 19 tickets

`SESSION_HANDOFF.md` listed ~20 tickets believed fixed by the previous
session's engine work. Each was verified by reading the ticket's own claim and
then the current code at the path it names — not by trusting the list. All 19
are genuinely fixed, and each has regression-test coverage already in
`mtg-engine/tests/`. Per-ticket verification notes are in each ticket's
`fix_note`. The recurring shapes were:

- **Inline damage → `damage::deal_damage`** (blazing_torch-01,
  heretic_s_punishment-01, skirsdag_cultist-01, daybreak_ranger-03 via
  `combat::fight`). The central pipeline applies protection, the planeswalker
  loyalty branch, `PreventDamageRemoveCounter` and lifelink; each inline copy
  had missed some subset.
- **Characteristics layer** (ghost_quarter-01, maw_of_the_mire-01,
  into_the_maw_of_hell-01, blazing_torch-02, heretic_s_punishment-02,
  skirsdag_cultist-02, olivia_voldaren-03) — filters now go through
  `state.has_card_type` / `has_subtype` rather than reading the empty
  object-level vectors.
- **Dispatch-time trigger conditions** (charmbreaker_devils-01 via
  `should_trigger_on_spell_cast`, wooden_stake-01 via
  `should_trigger_on_blocks`, thraben_sentry-01 via
  `triggered_abilities_of`) — the same family as the new `should_trigger`.
- One-offs: fiend_hunter-01 (target locking), divine_reckoning-02 (engine-owned
  spell cleanup), runechanter_s_pike-01 (equipment `dynamic_pt` leaking into
  its own P/T), evil_twin-03 (`entering_copy_source` never cleared),
  blazing_torch-03 (attached-ability controller guard).

The one correction in the handoff held up: **unbreathing_horde-01 is still
open** and is not covered by the damage-pipeline work.

### Cluster 4 — characteristics reads in card and condition code

Three distinct root causes, not one:

1. **Empty vector read as "has no types".** Garruk Relentless' -3 filtered on
   `o.card_types.contains(Creature)`, false for every non-token creature, so
   the +X/+X and trample hit nothing. Curse of the Pierced Heart scanned the
   same field for Planeswalker, making "or a planeswalker that player
   controls" dead code. Both now use `state.is_creature` / `has_card_type`.
2. **Non-empty vector read as the whole truth.**
   `EffectCondition::AttachedHasSubtype` treated `obj.subtypes` as
   authoritative once anything was in it, so a Human that Olivia Voldaren had
   turned into a Vampire stopped counting as a Human — Silver-Inlaid Dagger's
   +1/+0 and Butcher's Cleaver's lifelink both vanished, and
   `AttachedLacksSubtype` inverted the wrong answer on top. Now goes through
   `has_subtype`, matching what `CreatureFilter::HasSubtype` already did.
3. **Front-face-only registry read.** `registry.card_data` always returns the
   front face, so Hamlet Captain buffed transformed werewolves as if still
   Human (CR 712.8d). Full Moon's Rise had the same read and was switched over
   with it (harmless today — werewolf front faces are Werewolves too).

Plus the mirror of (2) on the way out: CR 400.7 runtime grants — Olivia's
"Vampire", Grimoire of the Dead's "Zombie" and black — were surviving a zone
change, so a creature reanimated by Grimoire stayed a black Zombie in the
graveyard. `move_object` now clears `subtypes` and `colors` on leaving the
battlefield, **before** the CR 712.8a front-face revert (order matters: the
clear would otherwise wipe what that revert writes). Tokens are exempt —
their object-level fields *are* their printed characteristics.

**Deliberately not done:** clearing `card_types` on zone change. Neither
ticket asks for it, nothing in the set grants a card type at runtime (only the
copy-effect and token paths write that field), and clearing it broke test
fixtures that use `card_types` as a stand-in for a real card. That is a
separate behavioural change with its own blast radius.

**Test.** `mtg-engine/tests/characteristics_card_sweep.rs`, 6 tests; 5
verified red pre-fix, the 6th guards the token exemption.

**Two existing tests were re-based on real levers.** Both simulated "stops
being a Human" by overwriting `obj.subtypes`, which models nothing the engine
does — outside transform, subtypes are only ever added to. They now transform
a real DFC (Cloistered Youth → Unholy Fiend; Villagers of Estwald → Howlpack
of Estwald), which is the lever the equipment test file's own opening note
already described. Their assertions are unchanged in intent.

**Tickets closed:** garruk_relentless-01, curse_of_the_pierced_heart-01,
hamlet_captain-01, butcher_s_cleaver-02, silver_inlaid_dagger-01,
olivia_voldaren-01, grimoire_of_the_dead-01.

### Clusters 3, 5, 6 — general hooks, not special cases

Each of these was a *missing general mechanism*, and the cards were working
around its absence. Implementing the mechanism was the fix in every case.

**Cluster 3 — `CardBehavior::enters_tapped` (CR 614.1d).** The five check
lands modelled "enters tapped unless you control a Mountain" as a triggered
ability that tapped the land on resolution. Three observable bugs: the land
entered UNTAPPED and could be tapped for mana in response to its own trigger
(a free mana every time); the condition was read at resolution, so an opponent
could bounce the qualifying land in response; and when the condition was met a
trigger still went on the stack for nothing. The hook sits next to the
existing `entering_with_counters` (CR 614.1c) and applies at the same point in
`move_object` — after the permanent is placed, before `EnteredBattlefield`.

**Library search — `helpers::search_library` (CR 701.19).**
`ChooseFromLibrary` hardcoded "put it in the owner's hand", so any card
fetching onto the battlefield hand-rolled the whole shape; five did, and
drifted. The choice now carries `destination` and `tapped`, and one helper
implements find-candidates / none-one-many / move / shuffle. `optional` is
part of the mechanism: "you MAY search" is a real decision even with one
candidate, which Ghost Quarter's tests caught when the first version
auto-took. Also fixed a bug Caravan Vigil's own comment admitted — with 2+
lands it fell through to the engine's finisher and never offered morbid.

**Cluster 5 — `GameState::move_object_under_control` (CR 110.2).** Cards
called `move_object` then assigned `controller` on the next line, but the
event is emitted *during* the move, so it carried the previous controller and
every `AnyCreatureEnters` watcher read the wrong player. Not hypothetical:
`move_object` deliberately keeps `controller` on leaving the battlefield
(death triggers use it as last known information), so a creature that died
while stolen sits in its owner's graveyard marked as the thief's — which is
exactly where these three cards reanimate from. Splinterfright was the same
confusion inverted: it passed `controller` to `objects_in_zone`, which filters
graveyards by OWNER, so after a steal it counted the opponent's graveyard
(CR 112.8).

**Cluster 6 — no engine change needed at all.** The mechanism already existed;
three cards declared `target_requirement: None` and bypassed it, hand-rolling
a prompt at resolution. That broke CR 603.3b (targets chosen as the trigger
goes on the stack), 603.3c (no legal targets → never reaches the stack) and
608.2b (resolution-time legality re-check, which only runs on declared
targets) simultaneously — and Elder Cathar's hand-rolled list never called
`can_be_targeted_by`, so shroud didn't protect. Curse of the Pierced Heart
gates its "enchanted player's upkeep" through the `should_trigger` hook.

Two tests were found to be **passing vacuously** while writing this: moving a
creature to the graveyard is not *dying*, so the death trigger never fired and
"no trigger on the stack" was trivially true. They now destroy the creature.

## Next up

1. Clusters 3 → 5 → 6, then the one-off tail.
2. `should_trigger` is now the hook for any future intervening-if card — check
   for one whenever a ticket says "the condition is only evaluated at
   resolution".

**Backlog count: 64 fixed / 52 open**, plus the root-cause refactor above,
which removes the mechanism behind the whole characteristics bug family (was 2 / 114 at the start of this pass).
