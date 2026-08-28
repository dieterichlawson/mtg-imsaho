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
| 7 | Card-specific one-offs | 28 left | in progress |

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

### Cluster 7 so far — the "one-offs" keep turning out to be clusters

Working the tail, the tickets have mostly collapsed into shared root causes
rather than needing individual handling:

- **`TriggerScope::AttachedPlayer`** — all three upkeep Curses said "at the
  beginning of ENCHANTED PLAYER's upkeep", which neither `Each` nor `Your`
  describes, so each fell back to `Each` and wrote the same early-return in its
  handler. This also replaced the `should_trigger` workaround Curse of the
  Pierced Heart had been given: one mechanism instead of the same condition
  written three times.
- **Equip re-targeting (CR 702.6a)** — target generation excluded the creature
  already wearing the equipment. Re-equipping to the current host is a real
  play whenever the equip COST is the point, and with one creature on the
  battlefield the filter removed the ability entirely. 3 tickets, one line.
- **Flashback (CR 702.33)** — a card can hold several instances of flashback at
  once and the player may pay any; the generator picked one winner and dropped
  the rest. Not just a missing choice: an unaffordable granted cost hid a
  payable printed one. Plus CR 702.33a — a card with no mana cost gains no
  flashback, where three sites substituted a free cost and made it castable for
  {0}. 5 tickets.
- **CR 400.7 identity reset** — `move_object` reset the obvious battlefield
  state but left a copy's `card_id`, an exchanged base toughness, a Curse's
  `attached_to_player`, and the display name. 4 tickets.

**Two more fragile test fixtures found**, both the same shape as before: a test
hand-set a fake name onto `CardId(200)` (really Hanweir Watchkeep) and then
identified its objects by that name, so the correct name reset made it find
nothing; and the Mask of Avacyn test asserted "no duplicate equip actions" by
counting actions, which conflated duplicates with target count. Both now
assert what they mean.

### Cluster 7, continued — more general hooks

- **Copy effects (CR 115.1 / 614.12 / 707.2)** — three rules, one card family.
  Choosing is not targeting, so Evil Twin's copy candidates must not be
  filtered by hexproof or protection (`creature_choices_except`). A permanent
  entering as a copy enters bearing the copied characteristics, so the COPIED
  creature's ETB abilities trigger — they were never raised at all. And a
  generic token's printed keywords live on the object, so copying a flying
  Spirit token dropped its flying (`printed_keywords_of` completes the
  characteristics accessors). 5 tickets, two of which were "no test coverage".
- **`should_trigger_on_damage_to_player`** — Curiosity's "enchanted creature
  deals damage to an opponent" was checked at resolution, so the ability went
  on the stack every time any permanent damaged any player.
- **CR 113.7a** — the EndStepTrigger arm required its source still be on the
  battlefield, unlike the Upkeep and ETB arms beside it, so killing Reaper from
  the Abyss in response silently cancelled its destruction.
- **CR 603.10a** — the simultaneous-death list was keyed on `CreatureDied`,
  which `destroy` only emits for things with power, so a non-creature watcher
  (Gutter Grime is an enchantment) destroyed alongside the creature it watches
  lost its trigger entirely. Keyed on `LeftBattlefield` now.
- **CR 121.1** — `add_counters` wrote to permanents that had left the
  battlefield, so a Gutter Grime destroyed in response to its own trigger still
  gained a slime counter and its Ooze came in 1/1 instead of 0/0.
- **`draw_cards` returns a count** and is `#[must_use]`. It returned `()`, so
  "draw a card. If you do, discard a card" checked the hand instead — a player
  with an empty library discarded a card they never drew. The must_use flagged
  13 other call sites; all are plain "draw N" and now say so.

### Cluster 7, continued — tap costs and "card" vs "token"

- **One tap pays one cost (CR 302.6 / 602.2h)** — the `{T}` symbol has the
  same three requirements on every permanent in the game (on the battlefield,
  untapped, and for a creature past summoning sickness unless hasty), and card
  code was re-deriving them. Two of the twenty-odd cards with a `{T}` ability
  spelled out the summoning-sickness half and **both forgot haste**; the other
  nineteen forgot summoning sickness entirely, so a mana creature tapped for
  mana the turn it arrived. Skirsdag High Priest's copy also *shadowed* the
  engine's correct check by returning an empty ability list before
  `legal_actions` could apply it, making a hasty Priest unactivatable. Now
  `GameState::can_pay_tap_cost`, applied centrally by the new
  `engine::available_mana_abilities` — every caller of
  `CardBehavior::mana_abilities` goes through it. Card code states only what is
  particular to the ability. Second half: a permanent paying an ability's `{T}`
  cost cannot also be tapped for mana toward that same ability, which the
  autotap source pool only enforced for "sacrifice this" costs — so the five
  ISD utility lands credited their own `{T}: Add {C}` toward their
  `{cost}, {T}:` ability and were offered one land short. 2 tickets, 21 files
  simplified.
- **"card" in oracle text excludes tokens (CR 109.1)** — Unbreathing Horde's
  "each Zombie card in your graveyard" and Splinterfright's "creature cards in
  your graveyard" had no `is_token` check. The window is real: CR 704.5e
  sweeps a token out of the graveyard as a *state-based action*, a discrete
  pass rather than something that happens the instant the token arrives, so
  anything reading the graveyard mid-resolution sees tokens on their way out.
  Same guard for Gnaw to the Bone, Past in Flames and Runechanter's Pike.
  Unbreathing Horde's battlefield half says "each other Zombie you control",
  with no "card", so that one still counts tokens — the distinction is the
  point. `GameState::is_card` gives the rule one spelling. 2 tickets.
- **Tree of Redemption** — an activated ability stays on the stack when its
  source leaves the battlefield; "exchange your life total with *this
  creature's* toughness" then has nothing to exchange with. The handler
  checked only that the object still existed, which it does in the graveyard.
- **Mindshrieker** — moved the milled card by hand instead of through
  `mill_one`, so no `CreatureCardMilled` event was emitted and Undead
  Alchemist never saw it.

### Cluster 7, continued — "at once", and reading state at the wrong moment

- **Simultaneous destruction (CR 700.2c)** — "destroy all creatures" is one
  event, not a sequence of them, and the difference is observable. Angelic
  Overseer is "indestructible as long as you control a Human"; when a sweeper
  catches the Overseer and its controller's last Human together, the Human is
  still on the battlefield at the moment destruction happens, so the Overseer
  survives. A loop over `try_destroy` got that wrong whenever it reached the
  Human first — and gave a different answer depending on list order.
  `try_destroy_all` decides for every permanent against the state before any of
  them died, and captures each death's last known information there too
  (CR 608.2g) rather than after earlier deaths changed the board. Divine
  Reckoning, Paraselene and Witchbane Orb go through it.
- **Simultaneous discard (CR 101.4)** — Liliana's +1 discarded as each player
  chose, so control returned to the game loop — which collects triggers — with
  one card already in the graveyard and the next player not yet asked. A
  discard watcher could see it mid-choice. `ChooseCardFromHand` now carries
  `discard_immediately`; Liliana collects and discards together. Her queue of
  remaining players was also packed by joining player ids with commas and
  parsing the result **as a u64**, silently 0 with more than one player left.
- **`GameState::discard_card`** — "move to graveyard, then push Discarded" was
  written out at thirteen sites, and a site that forgot the event silently
  broke every discard watcher.
- **"You may search" (CR 701.19)** — an optional search skipped the question
  entirely when nothing qualified and shuffled anyway. The decision is the
  player's, and the unconditional shuffle leaked that there had been nothing to
  find. Fixed in the shared helper, not in Ghost Quarter.
- **"You may pay" (CR 608.2g)** — Frightful Delusion offered its choice only to
  a player who already had {1} floating and silently countered everyone else.
  There is no priority window mid-resolution, so the engine now does what it
  does for every other cost: works out a tap plan and runs it on "yes"
  (`can_pay_with_sources` / `pay_cost_with_sources`). The handler also ignored
  whether payment succeeded, so answering "pay" with an empty pool saved the
  spell for free.
- **Moldgraf Monstrosity** — "exile it" ran unconditionally, so the second of
  two simultaneous death triggers exiled a creature the first had already
  returned to the battlefield.

### Cluster 7, continued — triggers reading the board at the wrong time

- **A trigger is about what was true when it went on the stack.** `AttacksTrigger`
  now carries the attacking creature and the defending player, snapshotted at
  declaration (CR 508.1), and `on_attacks` receives them as `AttackInfo`.
  Trepanation Blade read the Blade's *current* `attached_to` instead: killing
  the equipped creature in response made the handler return before doing
  anything, so the mandatory mill was skipped along with the buff; and
  re-equipping before resolution moved the buff onto a creature that never
  attacked.
- **Death triggers fire after their watcher died.** Selhoff Occultist and Rage
  Thrower required themselves to still be on the battlefield, which made them
  no-ops in exactly the board-wipe case they exist for. Neither effect touches
  the source — one mills a player, the other deals damage, which a source that
  has left the battlefield still does from last known information (CR 608.2h).
  The audit also flagged Lumberknot, Unruly Mob and Village Cannibals; those
  guards are *correct*, because their triggers put a counter on themselves and
  a permanent off the battlefield cannot receive counters (CR 121.1). The
  distinction is written down beside the code now.
- **Twelve identical werewolves.** Each carried a byte-identical private copy
  of the transform condition, and every copy had the same invented
  `&& !state.is_first_turn` — nowhere in the oracle text. One helper now.
- **`ManaCost::without_x` / `has_x`** — Back from the Brink's "pay its mana
  cost" is not casting, so X is 0 and there is no announcement (CR 107.3e); the
  printed {X} was reaching the engine's X-funding prompt. The new methods also
  collapse eight hand-rolled X filters in the engine.

### Cluster 7, finished — the five that each needed a new mechanism

Every remaining ticket turned out to need something the engine did not have,
rather than a local fix. All five are general mechanisms now, each with the
card that motivated it reduced to a single call.

- **`CreatureFilter::Attacking`** (`instigator_gang-02`) — "attacking creatures
  you control get +X/+0" was an `AnyCreatureAttacks` trigger pushing an
  until-end-of-turn P/T modifier. A one-shot buff applied at declaration
  diverges from a static ability three ways, all reachable: it outlived the
  combat, it missed creatures put onto the battlefield attacking, and it missed
  attackers already declared when the Gang arrived. As a static
  `ModifyPT` over `And([ControlledByYou, Attacking])` all three fall out, and
  so does "stops when the Gang leaves".
- **`ActivatedAbilityDef::counter_cost`** (`grimoire_of_the_dead-02`) — "remove
  three study counters ... and sacrifice it" is two cost actions in a fixed
  order (CR 601.2h), and the second clears every counter at once. The removal
  never happened as a discrete action, and the card hand-rolled the "do I have
  three?" check. Plus `GameState::remove_counters`, deliberately *not*
  restricted to the battlefield — unlike `add_counters` — because it runs while
  the permanent is on its way out.
- **Player protection from a color** (`bitterheart_witch-01`) — hexproof stops
  a player being targeted; protection additionally stops them being enchanted
  by an Aura of that color (CR 702.16b), so a player can be a legal target and
  still not be somewhere the Curse can go. `player_has_protection_from` /
  `player_can_be_enchanted_by`, with `grants_player_protection_from` beside the
  existing `grants_player_hexproof`, and a CR 303.4h re-check at attach time.
  No Innistrad card grants it, so the test registers one alongside the real set
  rather than asserting against a hypothetical.
- **`GameState::control_effects`** (`olivia_voldaren-02`) — CR 611.2b: a "for as
  long as" duration ends the moment its condition stops being true. Olivia kept
  her own list of what she had stolen (player ids packed into `ObjectId`s) and
  unwound it from `on_leave_battlefield`, so the effect ended in exactly one
  way; an Act of Treason on Olivia left the stolen Vampires where they were.
  `expire_control_effects` runs first in the SBA loop, so a permanent that has
  changed hands is back with its owner before anything else asks who controls
  what.
- **`ManaAbilityDef::cost`** (`shimmering_grotto-01`) — "{1}, {T}: Add one mana
  of any color" is a mana ability (CR 605.1a) but lived in
  `activated_abilities`, which the auto-tap planner never reads: three Plains
  and a Grotto produced no CastSpell action for a {2}{G} spell. A filter is net
  zero, so the planner counts its cost as extra generic demand, never uses one
  to pay a generic requirement, and orders cost-bearing abilities last so the
  mana that pays for them is already floating.

## Where this landed

**116 fixed / 0 open** (was 2 / 114 at the start of this pass).

Two root causes accounted for most of the backlog, and both are now structural
rather than a matter of remembering:

1. **Characteristics had no single authoritative reader**, so every call site
   improvised. The layer in `state.rs` is the answer, and
   `characteristics_invariant.rs` is the guard.
2. **Card rules lived in engine match arms**, because `PendingEffect` was a
   closed enum. `CardBehavior` hooks replaced them, and
   `engine_knows_no_cards.rs` fails the build if a card name reappears in the
   engine — including a staleness check, which has already caught one primitive
   becoming dead code.

The third, visible right through the tail of this list: **card code
re-deriving something the engine already checks.** Twelve identical werewolf
transform conditions, twenty-one hand-rolled `{T}`-cost checks, thirteen
copies of "move to graveyard, then push Discarded", eight X-symbol filters.
Every duplicate was a place where one copy could drift, and in each family at
least one had. Reach for the shared hook first:

- `should_trigger` — any intervening-if condition (CR 603.4).
- `can_pay_tap_cost` — any `{T}` cost. `can_pay_with_sources` — any "you may
  pay", including tapping for it (CR 608.2g).
- `try_destroy_all` — any effect that destroys more than one thing (CR 700.2c).
- `search_library` — anything that searches, so the "may" and the shuffle stay
  right.
- `discard_card` — any discard.
- `AttackInfo` — attack triggers, instead of re-reading `attached_to` or combat
  state at resolution.
- `gain_control_while_source_controlled` — any "for as long as" control effect.
- `ManaAbilityDef` (with its `cost`) rather than `activated_abilities`, for
  anything that makes mana.

### Verification

`ANTHROPIC_API_KEY=dummy cargo test --workspace` — 1396 tests across 137
suites, zero failures, zero compiler warnings. The env var matters: six
`llm_conversation` tests otherwise fail at `mtg-player/src/llm.rs:487` looking
for a key.

Every fix in this pass was verified red-before / green-after by reverting the
production change and re-running its regression test.
