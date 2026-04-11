# Audit-found bugs

Tracking file for bugs surfaced by mining `verify-draft-8seat-high-v5.log` (the
8-seat best-of-3 Innistrad draft tournament played by `gemini-3.1-flash-lite-preview`
at `high:high` thinking).

## Status legend
- ✅ FIXED — landed in a commit
- 🟡 SURVEYED — known bug, not yet fixed
- ⚪ INFORMATIONAL — model/prompt weakness, not an engine bug

---

## Engine bugs

### ✅ Engine Bug A: mana-cost activated abilities never offered
**Commit:** `b9ef64e` — "Auto-tap mana sources for activated abilities"

`legal_actions` only checked activated-ability mana costs against the player's
*current floating mana pool* via `mana::can_pay`. It did not call
`compute_autotap` against untapped lands the way spell casting does. As a
result, equipment with `Equip {N}` (Silver-Inlaid Dagger, Butcher's Cleaver,
Cobbled Wings, Blazing Torch, Sharpened Pitchfork, Wooden Stake, Mask of Avacyn,
Trepanation Blade, Inquisitor's Flail) and Elder of Laurels' `{3}{G}` ability
were *never* offered as legal actions. The LLM's auto-pass logic skipped right
past main phase whenever its only nominal options were Pass / Tap land / Cast
spell, so the model never had a chance to manually pre-tap and activate.

Across the entire 120k-line audit log, zero "Activate <equipment>" actions
appear for any of these cards.

**Regression test:** `mtg-engine/tests/equipment_autotap.rs` (17 tests).

---

### ✅ Engine Bug B: Silver-Inlaid Dagger / Butcher's Cleaver Human bonus is a snapshot
**Commit:** `ec3d3f1` — "Make Silver-Inlaid Dagger and Butcher's Cleaver Human bonuses continuous"

Both cards say "as long as equipped creature is a Human". Earlier
implementations took a snapshot of `is_human` in an `update_effects` helper
at equip time and wrote a fixed `instance_continuous_effects` vector. If the
equipped creature later transformed (e.g. a Human Werewolf flipped into its
non-Human back face via Moonmist or via the no-spells-last-turn upkeep
trigger), the bonus stayed put — wrong both ways.

Fix: switched to `ContinuousEffect::ConditionalModifyPT` and
`ContinuousEffect::ConditionalKeyword` (the same shape Bonds of Faith already
uses). The engine re-evaluates these effects every time `effective_power` /
`effective_toughness` / `has_keyword` is called.

Was masked by Bug A — the dagger never got equipped in the audit log so the
follow-on bug never fired. Would have fired the moment Bug A was fixed.

**Regression test:** `mtg-engine/tests/equipment_human_conditional.rs` (10 tests).

---

### ✅ Engine Bug C: SacrificeCost auto-picks the wrong creature, fizzles equip-sacrifice
**Commit:** `3126526` — "Sacrifice-cost activated abilities: player chooses the sacrifice"

The audit log directly captured this. Seat 0 R1 turn 13 (around log line
34561): the model activated Demonmail Hauberk targeting Crossway Vampire,
intending to sacrifice a Spirit. The engine's apply path called
`.find(|o| o.power.is_some())` on the battlefield zone — i.e. "first creature,
any creature" — without consulting the targets vector. It picked the Vampire,
sacrificed the Vampire, then on_activate_ability tried to attach the Hauberk
to a now-dead target. The attach silently failed via state-based-action
cleanup. Player lost the Vampire AND the equip never happened. The model
tried again on the next turn, sacrificed a Spirit, and again the Hauberk
ended up unattached.

Same root-cause bug applied to Disciple of Griselbrand (sac for life) and
Skirsdag Cultist (sac to deal damage). Both used `SacrificeCost::SacrificeCreature`
with the same auto-pick.

A `// TODO: Present choice to player when there are multiple options` comment
had been sitting in the apply path for some time.

**Fix:** legal_actions now enumerates one `Action::ActivateAbility` per
(target, sacrifice) combo, mirroring how `CastSpell` already handles its
sacrifice cost. Pairs where target == sacrifice are filtered out so the
player can never accidentally pick a fizzling combo. The apply path uses the
explicit sacrifice. Activated abilities with both a mana cost AND a sacrifice
cost no longer auto-tap (they require manual mana) to avoid the planner
deciding to tap a creature mana source and then sacrificing that same creature.

**Regression tests:** `mtg-engine/tests/sacrifice_choice.rs` (11 tests).

---

### 🟡 Engine Bug D: Moorland Haunt auto-picks creature to exile from graveyard
**File:** `mtg-engine/src/cards/isd/moorland_haunt.rs:85`

Activation cost is `Exile a creature card from your graveyard`. Code:
```rust
// Exile a creature card from graveyard (auto-pick the first one).
let creature_in_gy = state.objects_in_zone(Zone::Graveyard, controller)
    .iter()
    .filter(|o| o.power.is_some() && !o.is_token)
    .map(|o| o.id)
    .next();
```
Player should choose. Same shape of bug as Bug C but with exile-from-graveyard
instead of sacrifice. Did NOT fire in the audit (Moorland Haunt was P1P1 by
Seat 5 but they didn't end up in UW so it was never on the battlefield).

**Proposed fix:** add an `exile_from_graveyard: Option<ObjectId>` field to
`Action::ActivateAbility` and enumerate combos in legal_actions, the same way
the SacrificeCreature fix worked. Or generalize to a single `cost_choice` /
`cost_choices` field that captures any payment-time choice.

---

### 🟡 Engine Bug E: Nevermore peeks at opponent's hand and auto-picks
**File:** `mtg-engine/src/cards/isd/nevermore.rs:42-67`

Doubly broken:
1. **Information leak**: Nevermore is supposed to let you name *any* card name
   (the player just says one out loud). The current `on_enter_battlefield`
   handler iterates `state.objects.values()` for objects in `Zone::Hand` owned
   by the opponent and picks one. The controller learns what's in the
   opponent's hand. That's not how Nevermore works at all.
2. **Auto-pick**: even if the info leak weren't there, the player should be
   the one choosing the name.
3. Hard-coded fallback: `.unwrap_or_else(|| "Lightning Bolt".into())` if
   nothing matches in the opp's hand.

Did NOT fire in the audit (Nevermore wasn't drafted), but the implementation
is wrong on multiple counts.

**Proposed fix:** introduce a `ResolutionChoiceKind::ChooseCardName` variant
that the player resolves with a string. Stop reading from opp's hand entirely
— the choice is independent of board state.

---

### 🟡 Engine Bug F: ExileCreaturesFromGraveyard for spells auto-picks highest power
**File:** `mtg-engine/src/engine.rs:2122-2147`

Affects every spell with `AdditionalCost::ExileCreaturesFromGraveyard`:
- Stitched Drake (1)
- Skaab Ruinator (3)
- Skaab Goliath (2)
- Makeshift Mauler (1)
- Corpse Lunge (1)

The engine auto-picks "highest power first". For Corpse Lunge that happens to
be correct (damage = exiled creature's power, so max is best). For the
others the player should choose — they might want to preserve specific
creatures for graveyard effects (Boneyard Wurm, Spider Spawning,
Splinterfright, Mulch enablement).

**Did fire in the audit log** (Makeshift Mauler ×N, Stitched Drake ×N) but in
every case I traced, the auto-pick happened to be the only legal option, so
no incorrect behavior was observed in this particular tournament. Latent bug.

**Proposed fix:** same shape as Bug C — enumerate one `Action::CastSpell` per
choice subset, store the chosen IDs in the action, use them in apply.

---

### 🟡 Engine Bug G: cosmetic — duplicate `Step: Upkeep` AUTO-PASS entries
**Severity:** cosmetic

Throughout the audit log every turn transition produces *two* consecutive
`AUTO-PASS [SeatN] Step: Upkeep, active: pX` entries. Suggests the engine is
double-iterating the upkeep step somewhere. Not a gameplay bug, just noise in
the logs. Worth a glance from whoever knows the priority loop.

---

## Harness / display bugs

### ✅ Harness Bug H1: combat-prompt labels don't disambiguate identical permanents
**Commit:** `236572d` — "LLM prompt: combat math, keyword grounding, name disambiguation"

Seat 7 R3 (audit log line 107513) had two `Rakish Heir 4/2 first strike`
creatures, one wearing Bonds of Faith and locked down. The action list
collapsed the un-locked one to `0:Rakish Heir 4/2 first strike` with no
disambiguator. The model said "I will assume index 0 represents the
un-enchanted Rakish Heir" and got lucky.

**Fix:** new `format_combat_creature_list` helper that appends `#1`, `#2`,
... when names + P/T + keywords would otherwise collide, and surfaces any
attached aura/equipment inline (`Rakish Heir 4/2 #2 [+Bonds of Faith]`).
Wired into both the attacker and blocker prompt builders. 5 unit tests in
`llm::tests`.

---

### 🟡 Harness Bug H2: mid-resolution choice prompts have no clear marker
**File:** `mtg-player/src/llm.rs` (prompt header generation)

When the player needs to make a mid-resolution choice (e.g. Forbidden
Alchemy: pick one of these 4 cards to put into hand), the prompt header
just says `Turn N - <step> (your turn)` with no indication that this is a
*choice* prompt. The model has to figure it out from the available actions
list and recent events. Forbidden Alchemy works because the action labels
are the card names ("0: Island, 1: Plains, 2: Silent Departure, ..."), but
**Yes/No prompts are completely opaque** — see Bug H5 below.

**Proposed fix:** when the legal_actions are a `ResolveChoice`-style list,
prefix the prompt with `[CHOICE: <description>]` so the model knows what
it's picking and why.

---

### 🟡 Harness Bug H5: Yes/No prompts have no description of what's being asked
**File:** `mtg-player/src/llm.rs` (action label generation for ResolveChoice)

The audit caught this one bad: Bitterheart Witch's "may search library for
a Curse card" trigger renders as just:

```
Available actions:
0: Yes, 1: No
```

with no header explaining the choice. The model has to infer from
"Recent events" (which only said "Bitterheart Witch died") that this is
the Bitterheart Witch search trigger. **Seat 1 declined the search at log
line 116434**, saying *"Since I do not have any Curse cards in my deck,
I will decline"* — but **its deck DOES contain Curse of Death's Hold**
(verified at line 9347, the deck construction). The model hallucinated
its own decklist (Bug M1 below) and the harness gave it no help.

This pattern affects every "may" trigger: Bitterheart Witch (search for
Curse), Delver of Secrets (may reveal top), Civilized Scholar (may pay
to draw), Frightful Delusion (target may pay 1), Mentor of the Meek (may
pay 1), and any "you may" ETB or activated ability.

**Did fire in the audit log** — Seat 1's missed Curse of Death's Hold tutor
is the textbook example. Searching would have:
1. Tutored Curse of Death's Hold for free (saving the {3}{B}{B} cast cost)
2. Attached it to opponent immediately (saving turns of waiting to draw it)
3. Effectively given Seat 1 a free 5-mana spell + a tempo boost

Instead the model declined and cast Curse of Death's Hold normally several
turns later from its hand.

**Proposed fix:** wrap each `Action::ResolveChoice` with a description
sourced from the underlying `ResolutionChoiceKind`. For "may search
library for X" triggers, also enumerate the matching cards in the
controller's library so the model can see what's available without
needing to remember its decklist. (E.g. "Search for a Curse? Available:
Curse of Death's Hold, Curse of Oblivion. Yes / No.")

---

### 🟡 Harness Bug H6: BeginCombat / DeclareAttackers prompts confuse the model
**File:** `mtg-player/src/llm.rs` GAME_RULES (prompt clarity)

Multiple Seat 7 thoughts say *"I would like to attack with X, but there is
no explicit action to declare attackers in the provided list"*. The model
doesn't realize that passing through `[BEGIN COMBAT]` will take it to a
separate Declare Attackers prompt. This is a model misunderstanding of the
phase flow, but it's also a documentation gap in `GAME_RULES`.

**Proposed fix:** add an explicit note to GAME_RULES: "If you see
`[BEGIN COMBAT]` and want to attack, pass priority — the engine will
automatically take you to the Declare Attackers prompt, where the action
list switches to creature indices."

---

### 🟡 Harness Bug H7: target-choice and trigger-ordering prompts use the same opaque format
**File:** `mtg-player/src/llm.rs` (action label generation for ResolveChoice)
**Severity:** HIGH — actively cost Seat 2 multiple games of life total in the audit

This is the worst harness finding so far. Two completely different choice
types render the same way:

- Falkenrath Noble's "target player loses 1 life, you gain 1 life" target
  choice renders as `0: Opponent, 1: You`.
- Trigger ordering when the controller has multiple same-time triggers
  (e.g. Unruly Mob's "+1/+1 counter" trigger and Falkenrath Noble's drain
  trigger from the same death event) renders as `0: <option>, 1: <option>`.

In the audit (Seat 2, multiple games), the model received a Falkenrath
Noble target prompt while there were ALSO Unruly Mob triggers waiting on
the stack. The model THOUGHT it was being asked to order/resolve the
Unruly Mob trigger first ("I need to resolve the remaining trigger for my
Unruly Mob..."), and picked action 1 — which in the *target* prompt
meant `You`, self-targeting Falkenrath Noble's drain.

Net effect: instead of `Opp loses 1, You gain 1` (a +2 life swing), the
model got `You lose 1, You gain 1` (zero swing). Across the audit log
this happened **at least 5 times** (lines 62363, 62608, 64525, 65710,
67404 — every "You lost 1 life, You gained 1 life" line).

The model did not "fail at combat math". It literally thought it was
answering a different question.

**Proposed fix:** every prompt that comes from a `ResolutionChoice`
must include a header that names the source object and the choice text.
For example, instead of:
```
Available actions:
0: Opponent, 1: You
```
the prompt should read:
```
[CHOICE] Falkenrath Noble: choose target player to lose 1 life
0: Opponent, 1: You
```
And similarly for trigger-ordering prompts:
```
[CHOICE] Order your triggered abilities (top of stack resolves first)
0: Falkenrath Noble's drain trigger
1: Unruly Mob's +1/+1 counter trigger
```
This is the same fix as Bug H5 (Yes/No prompts) — every mid-resolution
choice prompt needs an explicit `[CHOICE: <description>]` header that
sources its description from the `ResolutionChoiceKind`.

---

### 🟡 Harness Bug H3: spells with additional costs don't show the cost in the action label
**File:** `mtg-player/src/llm.rs` (Cast label generation) and the underlying engine

Stitched Drake's cast option was rendered as `Cast Stitched Drake (tap 2x Island, Plains)`
with no mention that an additional creature card from the graveyard would be
exiled. Same for Corpse Lunge (which exiles for damage), Makeshift Mauler,
Skaab Ruinator, Skaab Goliath. Harvest Pyre (X-cost exile) at least surfaces
X in the label, but doesn't show *which* cards.

The model knew the cost from card knowledge ("I have Delver of Secrets in
graveyard to exile"), but the label should make it explicit.

**Proposed fix:** include the additional cost in the cast label, e.g.
`Cast Stitched Drake (tap 2x Island, Plains; exile Delver of Secrets from graveyard)`.
This is mostly cosmetic but interacts with Bug F — once F is fixed and the
player picks the exile target, the label can show the chosen target.

---

### ⚪ Harness Issue H4: Begin-Combat prompts confuse the model about how to declare attackers
**Severity:** model-side, prompt clarity (covered by H6 above)

Documented in Bug H6.

---

## Prompt-fixable issues (model behavior)

### ✅ P1: combat math — multi-blocker damage assignment
**Commit:** `236572d`

Pattern: model treats blocker toughness as a shared pool. Seat 1 line 61172:
*"my blockers have enough combined toughness (6) to survive the 4 damage it
deals"*. Combined toughness isn't shared — damage is assigned per-blocker
in lethal-first order.

**Fix:** added a worked example in `GAME_RULES` showing how a 4/2 trample
attacker double-blocked by a 1/4 and a 2/2 results in *one* dead blocker,
not "both blockers absorb the damage".

### ✅ P2: hallucinated keywords (trample, first strike, flying)
**Commit:** `236572d`

Seat 0 line 22492: claimed Rampaging Werewolf 8/4 with Rally the Peasants
deals "7 trample damage" — Rampaging Werewolf has no native trample and
there was no Full Moon's Rise on the board. Several similar misattributions
where the buff was actually from Vampiric Fury / Rally the Peasants etc.

**Fix:** strengthened the anti-hallucination instruction in both
`GEMINI_RESPONSE_FORMAT` and `ANTHROPIC_RESPONSE_FORMAT`: "if a keyword
isn't printed after the creature's P/T in the prompt, the creature does
not have it." Listed the specific failure modes.

### ✅ P3: identical-permanent disambiguation
**Commit:** `236572d` — same as harness bug H1.

### ✅ P4: chump-block heuristic
**Commit:** `236572d`

Seat 4 line 87499: chose to chump-block a 2/1 spirit token over a 3/3
creature, taking 1 extra damage to "remove a creature from the board".

**Fix:** added a chump-block heuristic to GAME_RULES — chump the
highest-power attacker, not the smallest one.

### ✅ P5: morbid timing
**Commit:** `236572d`

Seat 5 line 84846: stumbled through morbid Brimstone Volley arithmetic
even though it picked the right action ("they are at 7 health, I can just
cast it for 3, or if a creature dies, the extra 2 damage will definitely
finish them off" — wrong math, right line).

**Fix:** added a worked example showing pass-on-Declare-Blockers → let
combat damage resolve → cast morbid spell with bonus active.

---

## Model capability issues (informational only)

### ⚪ M1: model forgets its own decklist
Seat 1 declined the Bitterheart Witch curse-tutor trigger because it
thought it had no Curse cards in its deck — but the deck contained Curse
of Death's Hold (verified at line 9347). The decklist is in the system
prompt but apparently doesn't always inform mid-game decisions. This is
partly a model capability issue and partly a prompt clarity issue (see
H5 above — if the harness enumerated tutorable cards, the model wouldn't
need to remember).

### ⚪ M2: model claims spells are "illegal" when they have no targets
Seat 7 repeatedly said *"Vampiric Fury is illegal because I have no
Vampires"* (lines 133947, 134244, 134320, 134358, 134502, 134825, 135216,
135317). Vampiric Fury is a global +N/+0 buff with no targets — it's
always *legal* to cast, just useless when there are no Vampires. The
model conflates "useless" with "illegal". The action was correct (don't
waste mana) but the rules reasoning was wrong. Add a clarifying note to
GAME_RULES: "spells without targets are always legal to cast even if
they would have no effect."

### ⚪ M3: arithmetic slips on lethal damage thresholds
Seat 5 line 22665 / Daybreak Ranger: *"dealing 2 damage is insufficient
... 2 toughness ... insufficient"*. 2 damage to a 2-toughness creature
IS lethal. Two turns later (line 22801) the model corrected itself:
*"will kill it (as it has 2 toughness)"*. So the model has inconsistent
arithmetic on the same creature within the same game.

### ⚪ M4: model misremembers card text in passing
Seat 6 line 39108: *"Hysterical Blindness ... provides a -3/-0 debuff
(and in many versions acts as a board-wide effect)"*. Actual oracle:
"Creatures your opponents control get -4/-0 until end of turn." The
model is wrong on both magnitude (-4 not -3) AND board-wideness ("in
many versions" — it's always board-wide). The action was still correct
(cast Hysterical Blindness for the fog) but the reasoning is shaky.

---

## Coverage

Decisions sampled in passes 1-2: ~50 of ~3,766 thoughts (~1.3%).
Cards/interactions checked in depth: equipment, sacrifice abilities,
Forbidden Alchemy choice resolution, Spider Spawning, Endless Ranks of the
Dead, Curse of Death's Hold, Stitched Drake / Makeshift Mauler exile cost,
APNAP trigger ordering (3 cases sampled, all correct), Bonds of Faith
behavior, transform triggers (Villagers ↔ Howlpack), Rally the Peasants math,
Tribute to Hunger sacrifice direction, Moonmist usage, lethal/race math.

Not yet checked in depth:
- Most of round 2 / round 3 decisions
- Death triggers (Doomed Traveler, Mausoleum Guard) — verify the spirit tokens
  are created with the right keywords
- Werewolf transform timing edge cases (Moonmist resolution interactions
  with attack restrictions)
- Triggered ability ordering on more complex stacks (Selhoff Occultist mill
  triggers, Falkenrath Noble drain triggers)
- Deck-building decisions and how they constrained later play
- Mulligan bottoming decisions (only checked the keep/mull side)
- Decisions where the model had a long thought (~hundreds of long-thought
  candidates I haven't sampled)
- Seat 6 deeply (Mirror-Mad Phantasm controller's seat, only sampled twice)
- Activated abilities of creatures I haven't spotted yet (Daybreak Ranger,
  Cellar Door, etc.)

Mining will continue.
