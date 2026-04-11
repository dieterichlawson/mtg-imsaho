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

### 🟡 Engine Bug H: Into the Maw of Hell first-target filter is dropped — model can pick a creature as the "land" target
**Severity:** HIGH — actively cost Seat 7 their Rakish Heir + a 6-mana spell
**File:** `mtg-engine/src/engine.rs:1496` and `mtg-engine/src/engine.rs:1609`

Into the Maw of Hell is "Destroy target land. Into the Maw of Hell deals
13 damage to target creature." Engine target requirement:
```rust
TargetRequirement::TwoTargets(
    Box::new(TargetRequirement::PermanentWithFilter(
        TargetFilter::HasCardType(vec![CardType::Land]),
    )),
    Box::new(TargetRequirement::Creature),
)
```

But `valid_targets_for_req` and the spell-side `generate_cast_actions_with_targets`
both pattern-match `TargetRequirement::PermanentWithFilter(_)` and **discard**
the filter:
```rust
TargetRequirement::PermanentWithFilter(_) => {
    // Target any permanent on the battlefield matching a filter.
    // Actual filtering is done by the card's is_valid_target.
    state.all_objects_in_zone(Zone::Battlefield).iter()
        .filter(|o| can_be_targeted_by(...))
        .filter(|t| behavior.is_valid_target(state, caster, t, registry))
        .collect()
}
```
The comment says "actual filtering is done by `is_valid_target`", but Maw of
Hell's `is_valid_target` accepts BOTH lands and creatures (because the same
function handles both target slots in `TwoTargets` and can't differentiate).

**Audit log evidence (lines 46455-46630, Seat 7 Round 1):** the model cast
Maw of Hell intending to remove opp's Makeshift Mauler. The first-target
prompt offered:
```
0: Makeshift Mauler (opponent's), 1: Island, 2: Island, 3: Island,
4: Plains, 5: Plains, 6: Plains, 7: Ashmouth Hound (your),
8: Rakish Heir (your), 9: Ghoulraiser (your), 10: Swamp ...
```
The model picked `0` (Mauler) because the prompt presented Mauler as a
valid first target. The engine then resolved the spell with `targets[0] =
Mauler`, called `try_destroy(Mauler)` (which works on creatures), and dealt
13 damage to `targets[1] = Rakish Heir` — Seat 7's *own* creature.

Net effect: lost a Rakish Heir and a 6-mana spell to remove a single
opponent creature. The model's THOUGHT was *"I am selecting a target as
prompted, although this choice appears to be restricted to my own creatures
despite the card requiring a land target"* — the model knew the engine was
asking the wrong question but had no way to refuse.

**Two underlying problems:**
1. `PermanentWithFilter(_)` ignores the filter — should call a helper like
   `matches_target_filter` (which already exists at line 1935 for ability
   target enumeration) and consult the registry when `obj.card_types` is
   empty.
2. The first-target prompt's land options have no controller label
   (`1: Island, 2: Island, 3: Island, 4: Plains, ...` — whose Plains?).
   Even if the filter were correct, the model couldn't easily pick an
   *opponent's* land for mana denial.

**Proposed fix:**
- In `valid_targets_for_req` and `generate_cast_actions_with_targets`,
  match `PermanentWithFilter(filter)` and apply the filter via
  `matches_target_filter` (or a registry-aware version).
- In `format_combat_creature_list` (or wherever target labels are
  generated), label lands with `(your)` / `(opponent's)` the same way
  creatures are labeled.

This affects every spell using `PermanentWithFilter`. Search for
`PermanentWithFilter(TargetFilter::` to enumerate the cards that hit
this bug. Maw of Hell is the most obvious in-set offender.

---

### 🟡 Engine Bug I: X-cost flashback compute_autotap fails for `{X}{R}{R}{R}` etc.
**Severity:** HIGH — Devil's Play (the only X-cost flashback card in ISD) can never be flashbacked via auto-tap
**File:** `mtg-engine/src/engine.rs:1121` (flashback path)

The normal-cast code path for X-cost spells has special handling at
line ~778 that strips the X symbol from the cost and taps ALL mana sources
to maximize X. The flashback code path at line 1121 just calls
`mana::compute_autotap(fb_cost, ...)` directly with the full X-cost — and
`compute_autotap` doesn't know how to handle the X variable. It tries to
match the X symbol literally, can't, returns None, and the engine
`continue`s, dropping the flashback action entirely.

**Audit log evidence:** Seat 7 wanted to flashback Devil's Play multiple
times. The prompts say `Flashback available: Devil's Play (flashback {X}{R}{R}{R})`
in the hint section but the action list does NOT contain a Flashback
Devil's Play option even when the player has 8+ untapped lands. Seat 7
worked around this by manually tapping mountains/swamps one at a time
to prefill the mana pool, then casting (the prefilled-pool path doesn't
need autotap). Lines 49989, 50117, 50143 show six consecutive priority
passes to assemble {2}{R}{R}{R} for X=2 lethal damage. Inefficient but
functional.

**Proposed fix:** copy the X-cost handling from the normal-cast code path
into the flashback code path. Strip X from `fb_cost` before computing
autotap, then build a tap plan that taps all mana sources.

This bug ALSO means the cast label doesn't show what X will be — the
model has to mentally compute "I have N mana, the non-X part of cost is K,
so X = N - K" for every Devil's Play cast.

---

### 🟡 Harness Bug H8: X-cost spell labels don't show X
**Severity:** medium
**File:** `mtg-player/src/llm.rs:2069` (only handles `exile_x_from_gy_max`)

For Harvest Pyre (`ExileXFromGraveyard`), the cast label shows
`Cast Harvest Pyre X=2 (2 damage)` — the LLM player computes the effective
X from `exile_x_from_gy_max` and shows it.

But for ANY OTHER X-cost spell (Devil's Play, Heretic's Punishment if drafted,
Brimstone Volley uses fixed cost so it's fine, etc.) the label is just
`Cast Devil's Play (tap Swamp, Mountain)` with no X value at all. The model
has to mentally compute X based on the tap plan size. This works most of
the time but is a constant cognitive burden and source of errors.

The model also can't choose a smaller X — the engine pre-picks max X for
spells that DO get autotapped, and there's no way to express "I want X=2
not X=4 because I want to leave mana up". The collapsed cast option always
goes for max X.

**Proposed fix:** when the cast spell has an X-cost, compute the effective
X (`mana_value - non_x_cost`) from the tap plan and show it in the label,
the same way Harvest Pyre's `exile_x_from_gy_max` is shown. Optionally,
expand into one option per X value the player might want (this gets large
for many lands but can be capped).

---

### 🟡 Engine Bug J: Harvest Pyre cast options collapse to a single max-X choice
**Severity:** low (most uses of Harvest Pyre want max X)
**File:** `mtg-player/src/llm.rs` action collapsing

The engine generates one cast action per (X, subset of graveyard cards) for
Harvest Pyre, but the LLM player's `seen_spell_objects` deduplication
collapses them all into a single "Cast Harvest Pyre X=N (N damage)" entry,
showing only the maximum X. For Boneyard Wurm / Spider Spawning / Splinterfright
graveyard-care decks the player might want to deal less damage to preserve
graveyard creatures. Currently impossible.

Related to Engine Bug F (sacrifice/exile auto-pick) and Bug H8 (X-cost
labels). All three need a richer cast-action representation.

---

### 🟡 Engine Bug K: SacrificeThis abilities also got the no-autotap restriction (regression from Bug C fix)
**Severity:** medium
**File:** `mtg-engine/src/engine.rs:572-573` (the Bug C fix)

When fixing Bug C (sacrifice-cost abilities autopicking the wrong creature),
I added a blanket restriction:
```rust
let ability_has_sac_cost = !matches!(ab.sacrifice_cost, SacrificeCost::None);
let ability_tap_plan: Vec<(ObjectId, usize)> = if ability_has_sac_cost {
    // No auto-tap for sacrifice abilities — require mana already in the pool.
    ...
}
```

This is too aggressive: it includes `SacrificeCost::SacrificeThis`, where the
source permanent sacrifices itself and there is no creature-choice conflict
to worry about. Cards affected:
- Selfless Cathar (`{1}{W}, sacrifice this: +1/+1 to creatures`)
- Traveler's Amulet (`{1}, sacrifice this: search for basic land`)
- Brain Weevil (no mana cost — unaffected)
- Full Moon's Rise (no mana cost — unaffected)
- Ghost Quarter (no mana cost — unaffected)
- Selfless Cathar / Silverchase Fox (have mana costs — affected)
- Grimoire of the Dead (`{4}, T, discard: study counter`) — has discard, not strict sacrifice
- Skirsdag High Priest (uses tap-creature cost, different mechanism)

For SacrificeThis, the autotap can't conflict with the sacrifice because the
source is the only thing being sacrificed (it's not chosen from a list).
The exception is if the source itself is a mana source (Cellar Door, etc.)
but those are rare and not in this set.

**Did NOT fire in the audit** because the audit log was generated before
the fix landed. But after the fix, Selfless Cathar's ability won't appear
in the action list unless the player has manually pre-tapped {1}{W}, which
is a regression from how it would behave under just-the-Bug-A fix.

**Proposed fix:** narrow the autotap restriction to
`SacrificeCreature | SacrificeAnotherCreature` only:
```rust
let ability_has_creature_choice_sac = matches!(
    ab.sacrifice_cost,
    SacrificeCost::SacrificeCreature | SacrificeCost::SacrificeAnotherCreature
);
```

---

### 🟡 Engine Bug G: cosmetic — duplicate `Step: Upkeep` AUTO-PASS entries
**Severity:** cosmetic

Throughout the audit log every turn transition produces *two* consecutive
`AUTO-PASS [SeatN] Step: Upkeep, active: pX` entries. Suggests the engine is
double-iterating the upkeep step somewhere. Not a gameplay bug, just noise in
the logs. Worth a glance from whoever knows the priority loop.

---

### 🟡 Engine Bug L: Charmbreaker Devils triggers on every spell cast, not just instants/sorceries
**Severity:** medium — gives the model phantom +4/+0 turns
**File:** `mtg-engine/src/cards/isd/charmbreaker_devils.rs:75-92`

Oracle: "Whenever you cast an instant or sorcery spell, this creature gets
+4/+0 until end of turn." The handler filters by `caster == controller` but
**not** by spell type:
```rust
fn on_spell_cast(&self, state: &mut GameState, self_id: ObjectId,
                 caster: PlayerId, _spell_id: ObjectId, ...) {
    ...
    if caster != controller { return; }
    state.until_end_of_turn.push(crate::state::TemporaryEffect::ModifyPT {
        target: self_id, power_mod: 4, toughness_mod: 0,
    });
}
```
The dispatcher (`triggers.rs:727`) explicitly says
"Dispatch SpellCast triggers for ALL spell types... Individual card handlers
can filter by spell type if needed" — Charmbreaker Devils doesn't.

The audit only triggered Charmbreaker on actual sorceries (Devil's Play)
because Seat 7 was on a burn deck and didn't cast creatures while Charmbreaker
was on the battlefield. Latent bug — would manifest in any game where the
controller casts a creature spell while Charmbreaker is in play.

**Proposed fix:** look up the spell's card type via `state.get_object(spell_id)`
+ registry, gate the +4/+0 push behind
`is_instant_or_sorcery(spell_id)`. Compare with Burning Vengeance which DOES
filter (`cast_with_flashback`).

---

### 🟡 Engine Bug O: Memory's Journey accepts targets from any graveyard, not just the targeted player's
**Severity:** medium — latent (Memory's Journey was drafted but not cast in audit)
**File:** `mtg-engine/src/cards/isd/memorys_journey.rs:37-41`

Oracle: "Target player shuffles up to three target cards from **their**
graveyard into their library." The implementation uses
`TargetRequirement::TwoTargets(PlayerOnly, UpToTargets(3, GraveyardCard))` —
but `GraveyardCard` returns cards from ALL graveyards (any owner), not just
the targeted player's graveyard. So the model can target opponent and then
shuffle cards from its own graveyard.

In addition, the resolution loop puts each card back into ITS OWNER's library
(not the targeted player's), so even if you target opponent and pick your own
cards, the cards return to your library and only opponent's library gets
shuffled. Net: you can use Memory's Journey to shuffle just your own graveyard
back into your library while still triggering opponent's mandatory library
shuffle. Probably not exploitable but it's wrong.

**Proposed fix:** introduce `TargetRequirement::GraveyardCardOf(target_index)`
that filters to a graveyard owned by the player named in another target
slot. Or, simpler, validate at resolve time: filter the
graveyard-card targets to only those whose `owner == target_player`.

---

### 🟡 Engine Bug N: APNAP simultaneous-trigger ordering choice is missing
**Severity:** low (most ordering choices don't matter)
**File:** `mtg-engine/src/triggers.rs:946-951`

CR 603.3b: "If multiple triggered abilities triggered at the same time, the
active player puts all of theirs on the stack in any order, then each other
player in turn order does the same."

Current code:
```rust
for t in ap_triggers {
    state.stack.push(StackEntry::Trigger(t));
}
for t in nap_triggers {
    state.stack.push(StackEntry::Trigger(t));
}
```
This pushes triggers in collection order (essentially arbitrary), without
ever asking the player. There is NO ordering prompt anywhere in the engine.
For most ISD interactions this is fine, but the order of e.g. multiple
Falkenrath Noble drain triggers vs Reaper from the Abyss vs Bloodgift Demon
upkeep triggers could matter for race math.

(My earlier H7 description mentioned a "trigger ordering prompt" — that was
inaccurate; the engine simply doesn't have one. The ACTUAL bug is just
opaque trigger-resolution target prompts, which is what H7 documents.)

**Proposed fix:** when there is more than one ap_trigger, present the
controller of the active player with an ordering choice (a permutation
selection). Same for nap_triggers. Skip the prompt when there's only one
or when the order is provably equivalent (e.g. multiple identical triggers
from the same source — they're functionally interchangeable).

---

### 🟡 Engine Bug P: Caravan Vigil auto-picks the first basic land in library order
**Severity:** low — affects splash decks
**File:** `mtg-engine/src/cards/isd/caravan_vigil.rs:38-50`

Oracle: "Search your library for a basic land card, reveal it, put it into
your hand". Implementation uses `library_order.iter().find(...)` which returns
the first basic land in library order — no choice given to the player. If
the deck contains multiple basic land types (a 2-color deck splashing a third
for one card), the player can never specifically tutor the splash color.

Same shape as Bug C/D (cost-time auto-pick) but for a search effect instead
of a sacrifice. The fix is the same family of changes: enumerate the
possible choices and present them to the player via
`AwaitingAction::ResolutionChoice`.

**Did NOT fire** — Caravan Vigil wasn't cast in the audit log.

---

### 🟡 Engine Bug T: Skirsdag Cultist and Rolling Temblor don't push damage source to `damaged_by`
**Severity:** low (no in-set deathtouch interaction)
**Files:**
- `mtg-engine/src/cards/isd/skirsdag_cultist.rs:56-58`
- `mtg-engine/src/cards/isd/rolling_temblor.rs:38-39`

Most damage sources in `mtg-engine/src/cards/isd/*.rs` push the damage source
into the target's `damaged_by` vector when they apply damage. These two skip
that step. The `damaged_by` data is consulted by SBA 704.5h (deathtouch
destruction) and by death-watch triggers that care about who killed what
(none in ISD use this, but the data hygiene is worth fixing).

Compare to Daybreak Ranger, Olivia Voldaren, Heretic's Punishment, Garruk
Relentless, Blasphemous Act, Into the Maw of Hell, etc. — they all do
push to `damaged_by`.

**Proposed fix:** add `obj.damaged_by.push(self_id);` (or equivalent) next
to the `obj.damage_marked += amount;` line in both files.

---

### 🟡 Engine Bug U: X-cost activated abilities use whatever's in the mana pool, with no choice of X
**Severity:** low — only affects Kessig Wolf Run; can be worked around by manual tapping
**File:** `mtg-engine/src/engine.rs:588-599` (legal_actions) and 2288-2305 (apply)

Kessig Wolf Run's `{X}{R}{G}, {T}` ability is offered as a single legal
action whose effective X is determined at apply time by emptying the mana
pool. The player can manually pre-tap to control the X value, but there's
no way to express "I want X=2 not X=4" inline in the action label, and
there's no enumeration of possible X values the way there is for spells.

This is the activated-ability variant of Bug I (X-cost flashback) and Bug
H8 (X-cost spell labels). Same fix family: either enumerate one action per
plausible X, or set up a follow-on prompt for X selection. Wolf Run is the
only ISD card affected.

**Did NOT obviously fire** in the audit — Wolf Run was drafted but the
single Wolf Run activation I sampled used max-X via auto-tap, which was
the right choice.

---

### 🟡 Engine Bug Q: Dearly Departed implemented as a triggered ability instead of a static replacement
**Severity:** low — affects ETB-trigger ordering with Champion of the Parish
**File:** `mtg-engine/src/cards/isd/dearly_departed.rs:30-69`

Oracle: "As long as Dearly Departed is in your graveyard, each Human creature
you control enters with an additional +1/+1 counter on it." This is a static
replacement effect (CR 614.1c, "enters with X counters"). The current
implementation uses `TriggerKind::AnyCreatureEnters` and adds the counter
in `on_any_creature_enters`, AFTER the creature has entered.

Functional differences:
- ETB triggers from the entering creature (e.g. Champion of the Parish's
  "+1/+1 counter when a Human enters" trigger) and Dearly Departed's
  trigger are simultaneous. Resolution order matters but the current code
  doesn't enforce one.
- Effects that examine the creature *as it enters* (via replacement
  effects of other permanents) won't see the +1/+1 counter.
- Festerhide Boar's analogous "enters with two counters" is correctly
  implemented as on_resolve placement (in
  `mtg-engine/src/cards/isd/festerhide_boar.rs:34-43`); Dearly Departed
  should follow that pattern, just dispatched from the OWNER of the
  graveyard rather than from the entering card.

**Proposed fix:** instead of a triggered ability, have the engine consult
"enters-with-counters" replacement effects when a creature enters the
battlefield. Walk all graveyards/battlefield permanents that could grant
counters to the entering creature and apply them as part of the entry
event. Same-shape change for Mayor of Avabruck's continuous +1/+1 to
Humans (which IS implemented correctly via ContinuousEffect, but uses a
DIFFERENT mechanism).

**Did NOT fire** — no Dearly Departed + Champion-of-the-Parish interaction
sampled in the audit.

---

### 🟡 Engine Bug W: Legend rule keeps a nondeterministic permanent and never asks the player
**Severity:** medium — wrong rules behavior, fortunately latent
**File:** `mtg-engine/src/sba.rs:251-270`

CR 704.5j (current legend rule, since M14): "If a player controls two or
more legendary permanents with the same name, that player chooses one of
them, and the rest are put into their owners' graveyards."

The current SBA code:
```rust
let mut legend_groups: Map<(PlayerId, String), Vec<ObjectId>> = Map::new();
for obj in state.objects.values() { ... }
for (_, ids) in legend_groups {
    if ids.len() > 1 {
        // Keep the first (oldest), remove the rest.
        for &id in &ids[1..] {
            state.move_object(id, Zone::Graveyard, registry);
            took_action = true;
        }
    }
}
```
Two problems:
1. **No player choice** — the rule says the player chooses; the code picks
   automatically.
2. **Nondeterministic order** — `ids` comes from iterating
   `state.objects.values()` which is a HashMap iteration. The "kept"
   permanent is whichever the HashMap happens to surface first. Comment
   even says "keep the newest" but code keeps `ids[0]`.

The only legendary permanents in ISD are Geist of Saint Traft, Olivia
Voldaren, Bloodline Keeper (no, not legendary), Mikaeus the Lunarch, and
the planeswalker Garruk Relentless. None of them showed up twice in any
audit-log game, so the bug is latent.

**Proposed fix:**
1. Stable iteration order — sort `legend_groups` by `(controller, name)`
   and the inner Vec by ObjectId.
2. Present a `ResolutionChoice::ChooseTarget` listing the legend group
   members so the controller picks which one to keep. Skip the prompt
   when there's only one (the common case).

---

### 🟡 Engine Bug AD: Unburial Rites can reanimate creatures from any graveyard (including opponent's)
**Severity:** HIGH if it fires — completely changes the card's value
**File:** `mtg-engine/src/cards/isd/unburial_rites.rs:30-32` and `mtg-engine/src/engine.rs:1670-1683`

Oracle: "Return target creature card from **your** graveyard to the
battlefield." Unburial Rites declares
`TargetRequirement::GraveyardCreature` which the engine resolves as ALL
creature cards in ALL graveyards (any owner). Unburial Rites does not
override `is_valid_target` to filter by owner. Result: the model can
target opp's graveyard, and the engine reanimates opp's creature under
the spell controller's control (creature controller becomes the
reanimating player, not the owner).

**Did fire in audit at line 30188-30189:** Seat 1 cast Unburial Rites
and the prompt offered:
```
Unburial Rites: select a target:
0: Selfless Cathar, 1: Avacynian Priest
```
No owner labels — the model can't tell which graveyard. The model
picked Avacynian Priest. Whether either creature was actually in
opponent's graveyard at that moment is unclear without tracing object
IDs back through the log, but the engine would have allowed it either
way.

Same shape applies to Memory's Journey (Bug O), and likely to other
cards using `GraveyardCreature` / `GraveyardCard` with "your" in the
oracle text.

**Proposed fix:** Either
1. Override `is_valid_target` in each affected card to filter by
   `obj.owner == caster`, OR
2. Add a `TargetRequirement::GraveyardCreatureOwnedByCaster` variant
   parallel to the existing `GraveyardCardOwnedByCaster` (engine.rs:1703)
   and use it in Unburial Rites and similar cards.

Affected ISD cards (cast-time targeting from graveyard):
- Unburial Rites — confirmed fired in audit
- Memory's Journey (Bug O — already documented)
- Skaab Ruinator's exile additional cost — need to check
- Makeshift Mauler's exile additional cost — uses additional_cost path
- Stitched Drake's exile additional cost — uses additional_cost path

---

### 🟡 Engine Bug AC: Unbreathing Horde under-counts when reanimated from graveyard
**Severity:** low — only affects Unbreathing Horde + reanimation interaction
**File:** `mtg-engine/src/cards/isd/unbreathing_horde.rs:94-103`

Per Scryfall ruling: "If Unbreathing Horde enters the battlefield from a
graveyard, it counts itself for its enter-with-counters ability." This is
because "enters with X counters" is a replacement effect (CR 614.1c) and
the Horde is technically still in the graveyard zone at the moment of
entry timing.

Current implementation runs `add_zombie_counters` from
`on_enter_battlefield`, which fires AFTER the move to battlefield. By
then `count_zombies_on_battlefield` excludes the Horde (via
`exclude == object_id`) and `count_zombies_in_graveyard` doesn't include
it either (it's no longer in graveyard). Net: reanimated Unbreathing
Horde enters with one fewer counter than it should.

The cast path (`on_resolve`) computes counts BEFORE the move and is
correct, since the Horde is on the stack (not on battlefield, not in
graveyard) and the count of "other Zombies you control" + "Zombie cards
in graveyard" naturally excludes itself.

**Proposed fix:** in the on_enter_battlefield path (reanimation only),
add 1 to the total count to compensate for the Horde counting itself.
Or, for cleanliness, special-case "if entering from graveyard, +1".

**Did NOT fire** — Unbreathing Horde was drafted exactly once and not
cast in any reanimation scenario.

---

### ✅ Engine Bug AJ: Equipment equip-ability appears twice in legal_actions and the wrong one misroutes attach
**Commit:** `136de64` — "Fix Bug AJ: gate equipment activated_abilities on power.is_none()"
**Severity:** medium — fires only after Bug A's autotap fix (so latent in old audit)
**Files fixed:** Cobbled Wings, Mask of Avacyn, Silver-Inlaid Dagger,
Butcher's Cleaver, Sharpened Pitchfork, Wooden Stake (all in
`mtg-engine/src/cards/isd/*.rs`). Inquisitor's Flail, Trepanation Blade,
Runechanter's Pike, Blazing Torch, Demonmail Hauberk already had the
correct `power.is_none()` gating.

**Confirmed empirically** by writing a regression test that found the
duplicate before the fix:
```
equip-to-b count: 2
  ActivateAbility { object_id: ObjectId(1), ability_index: 0, targets: [Object(ObjectId(2))], ... }  // bears_a (wrong)
  ActivateAbility { object_id: ObjectId(3), ability_index: 0, targets: [Object(ObjectId(2))], ... }  // wings (correct)
```

The engine collects activated abilities for a permanent in two passes
(`mtg-engine/src/engine.rs:528-559`):
1. The permanent's own behavior — `behavior.activated_abilities(state, obj_id, registry)`
2. Behaviors of objects attached TO the permanent (auras and equipment),
   passing the same `obj_id` as the call site

For aura-granted abilities (like Skeletal Grimace's `{B}: Regenerate`)
this is correct: the aura's ability_index applies to the enchanted
creature. But for equipment, the "equip {N}" ability belongs to the
equipment itself, not the attached creature. When the engine iterates
the attached creature and asks the equipment "what activated abilities
do you have?", a buggy equipment card returns its equip ability anyway,
so the engine produces an `Action::ActivateAbility { object_id: creature_id, ... }`
in addition to the correct `{ object_id: equipment_id, ... }`.

Cards that DO filter (Inquisitor's Flail, etc.):
```rust
if obj.zone == Zone::Battlefield && obj.power.is_none() {
    vec![ActivatedAbilityDef { ... }]
```
Cards that DON'T filter:
```rust
if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
    vec![ActivatedAbilityDef { ... }]
```

Worse, the second variant gets routed to
`on_activate_ability(object_id = creature_id)`, and the handler does
`state.get_object_mut(object_id).attached_to = Some(*target_id)`, which
mutates the **creature's** `attached_to` field, not the equipment's.

**Did NOT fire** in the audit log because Bug A meant equipment
activations never appeared. After the Bug A fix landed, the LLM player
will see two near-identical "Activate <name> (Equip {N})" entries in
equipment-heavy positions and may pick the wrong one.

**Proposed fix:** add `&& obj.power.is_none()` to the activated_abilities
gating check in Cobbled Wings, Mask of Avacyn, Silver-Inlaid Dagger,
Butcher's Cleaver, Sharpened Pitchfork, and Wooden Stake. (Or, more
robustly, fix the engine's attached-iteration loop to skip equipment
since equip abilities are never granted to the attached creature.)

---

### 🟡 Engine Bug AE: Undead Alchemist "instead" damage replacement implemented as a trigger
**Severity:** medium — wrong order of operations vs other on-damage triggers
**File:** `mtg-engine/src/cards/isd/undead_alchemist.rs:45-106`

Oracle: "If a Zombie you control would deal combat damage to a player,
**instead** that player mills that many cards." This is a damage
replacement effect (CR 614), not a triggered ability. The Zombie should
mill the player INSTEAD of dealing damage; the player should never take
the damage.

The current implementation triggers on `AnyCombatDamageToPlayer` (a
post-damage trigger), then restores the lost life and mills:
```rust
state.get_player_mut(damaged_player).life = current_life + amount as i32;
// Mill that many cards.
```

Functional problems:
1. **Other on-damage triggers fire first.** Curse of Stalked Prey,
   Falkenrath Noble drain, Curiosity, Sturmgeist's "draw on combat damage"
   all fire on the actual damage event before Undead Alchemist's "trigger"
   undoes it. They shouldn't have triggered at all.
2. **Lethal damage** kills the player before the restoration runs. SBA
   704.5a fires when life ≤ 0; Undead Alchemist's trigger is too late
   to save them.

**Did NOT fire in audit** — Undead Alchemist was drafted exactly once
and never made it to the battlefield in a damage-dealing context.

**Proposed fix:** introduce a `ReplacementEffect::ReplaceCombatDamageWithMill`
that the engine consults during the combat damage step, producing a mill
event instead of a damage event when the source matches. Same shape as
`DoubleCombatDamage` for Inquisitor's Flail.

---

### 🟡 Engine Bug AT: registry-only subtype filters miss tokens (Slayer of the Wicked, Vampiric Fury, Village Cannibals)
**Severity:** medium — multiple cards affected
**Files:**
- `mtg-engine/src/cards/isd/slayer_of_the_wicked.rs:42-46` (ETB destroy V/W/Z)
- `mtg-engine/src/cards/isd/vampiric_fury.rs:42-47` (Vampire +2/+0 anthem)
- `mtg-engine/src/cards/isd/village_cannibals.rs:39-42` (death-trigger Human counter)

```rust
.filter(|o| {
    registry.card_data(o.card_id)
        .map(|d| d.subtypes.iter().any(|s| s == "Vampire" || s == "Werewolf" || s == "Zombie"))
        .unwrap_or(false)
})
```
The filter only checks `registry.card_data(o.card_id).subtypes`. Tokens
have `card_id: CardId(0)` (sentinel — see `state.rs:341-356`), so the
registry lookup returns None and the filter returns false. Slayer of
the Wicked therefore CANNOT target:
- Bloodline Keeper's 2/2 Vampire tokens
- Endless Ranks of the Dead's 2/2 Zombie tokens
- Cellar Door's 2/2 Zombie tokens
- Moan of the Unhallowed's 2/2 Zombie tokens
- Army of the Damned's thirteen 2/2 Zombie tokens

**Proposed fix:** also check `o.subtypes` (instance level), the same
pattern Avacynian Priest's "tap target non-Human" filter uses:
```rust
.filter(|o| {
    let from_registry = registry.card_data(o.card_id)
        .map(|d| d.subtypes.iter().any(|s| s == "Vampire" || s == "Werewolf" || s == "Zombie"))
        .unwrap_or(false);
    let from_instance = o.subtypes.iter().any(|s| s == "Vampire" || s == "Werewolf" || s == "Zombie");
    from_registry || from_instance
})
```

**Did NOT fire in audit** for Slayer of the Wicked — it was cast
multiple times but never against a board with V/W/Z tokens.

**Vampiric Fury** has the same registry-only filter for "Vampire
creatures you control" — Bloodline Keeper's Vampire tokens would NOT
get the +2/+0 first strike buff. (Audit log line 134244+: Vampiric
Fury was repeatedly cast and the model said "I have no Vampires" —
it has no Vampires anyway, so this didn't fire in practice.)

**Village Cannibals** has the same registry-only filter for "Human
creature dying" — if a Human-typed token (none in ISD) died, the
counter wouldn't accumulate. Latent.

The buggy pattern is recognizable as a registry-only subtype check
without an `|| o.subtypes.iter().any(...)` follow-up. Counter-examples
that already do it right: Avacynian Priest, Reaper from the Abyss,
Endless Ranks of the Dead, Hamlet Captain, Bloodline Keeper, Wooden
Stake, Elder Cathar.

---

### 🟡 Engine Bug AU: Moonmist Human filter breaks for creatures Olivia bit
**Severity:** low — requires Olivia + Moonmist interaction (Olivia not drafted in audit)
**File:** `mtg-engine/src/cards/isd/moonmist.rs:43-56` and `mtg-engine/src/cards/isd/olivia_voldaren.rs:107-110`

Moonmist's Human filter:
```rust
let has_human_subtype = if !o.subtypes.is_empty() {
    o.subtypes.iter().any(|s| s == "Human")
} else if o.is_transformed {
    // back face data
} else {
    // front face data via registry
};
```
The `if !o.subtypes.is_empty()` branch checks ONLY instance subtypes
when they're populated, completely ignoring the registry. That's
correct for transformed DFCs (which fully replace `obj.subtypes` via
`apply_transform`) but WRONG for partial subtype additions.

Olivia Voldaren's "deal 1 damage, becomes a Vampire" ability:
```rust
if !obj.subtypes.contains(&"Vampire".to_string()) {
    obj.subtypes.push("Vampire".to_string());
}
```
This pushes "Vampire" onto an empty `obj.subtypes` vector. Avacyn's
Pilgrim (Human Monk per registry) bitten by Olivia ends up with
`obj.subtypes = ["Vampire"]` — the original Human and Monk subtypes
are NOT in the instance vector (they're in the registry).

Now Moonmist runs, sees `obj.subtypes = ["Vampire"]` (non-empty), takes
the first branch, asks "is Human in [Vampire]?", returns false. The
Pilgrim does NOT get transformed by Moonmist even though it's still
demonstrably a Human (the "becomes a Vampire **in addition to its
other types**" oracle text means Human is preserved).

This same shape of bug applies to ANY card that grants subtypes via
`obj.subtypes.push` without first copying the registry's subtypes
into the instance vector. Olivia is the only ISD source of this
pattern.

**Did NOT fire** in audit — Olivia Voldaren wasn't drafted.

**Proposed fix (two options):**
1. In Moonmist (and any other filter that has the
   "if !o.subtypes.is_empty()" pattern): always check BOTH instance
   subtypes AND registry subtypes, taking the union. The
   "is_transformed" case should still use back_face_data instead of
   front-face registry, but the union logic still applies.
2. In Olivia (and any other "creature becomes a <subtype>" effect):
   when first pushing the granted subtype, ALSO copy the registry
   subtypes into `obj.subtypes` so the instance vector is the
   complete authoritative list. This is the "instance subtypes are
   authoritative when non-empty" contract that Moonmist's filter is
   already assuming.

Option 2 is cleaner — it makes the contract uniform across all
subtype-checking code.

---

### 🟡 Engine Bug AW: Prey Upon's TwoTargets ignores YouControl/YouDontControl filters
**Severity:** medium — model could target two of its own creatures (or two of opp's)
**File:** `mtg-engine/src/cards/isd/prey_upon.rs:28-33` and `mtg-engine/src/engine.rs:1408-1424` (TwoTargets path)

Prey Upon's `target_requirement`:
```rust
TargetRequirement::TwoTargets(
    Box::new(TargetRequirement::CreatureWithFilter(TargetFilter::YouControl)),
    Box::new(TargetRequirement::CreatureWithFilter(TargetFilter::YouDontControl)),
)
```

The engine's `valid_targets_for_req` for `Creature | CreatureWithFilter(_)`
ignores the inner filter (line 1408-1424) — it returns ALL battlefield
creatures, then defers to `behavior.is_valid_target` for refinement.
Prey Upon doesn't override `is_valid_target`, so the default (always
true) applies. Result: the legal_actions list contains every (any
creature, any other creature) pair, including:
- (your creature, your creature)
- (opp's creature, opp's creature)
- (opp's creature, your creature) — wrong order

The on_resolve does a swap based on which target is the caster's:
```rust
let a_mine = caster.and_then(|c| state.get_object(*a).map(|o| o.controller == c)).unwrap_or(false);
let (my_creature, their_creature) = if a_mine { (*a, *b) } else { (*b, *a) };
crate::combat::fight(state, my_creature, their_creature, registry);
```
But this doesn't enforce the constraint that one target is yours and
one is opp's. If both are yours, `a_mine = true`, my=a, their=b, and
combat::fight is called with two creatures you control. They fight
each other.

Same shape of bug as Bug H (Maw of Hell PermanentWithFilter dropped).
Different code path: this is the `CreatureWithFilter` path in
`valid_targets_for_req`, not `PermanentWithFilter`.

**Did NOT fire** in audit — Prey Upon was cast with sensible targeting
in every observed instance because the model's combat logic naturally
picked one of each.

Affected cards: Prey Upon. Daybreak Ranger and Skirsdag Cultist use
single targets so they're fine. Cackling Counterpart's `YouControl`
filter for the single-target path (`CreatureWithFilter(YouControl)`)
is ALSO not enforced — but the LLM would never accidentally make a
token copy of an opponent's creature, so latent.

**Proposed fix:** in the `Creature | CreatureWithFilter(_)` arm of
`valid_targets_for_req`, destructure the inner filter and apply
`matches_target_filter` (the same helper that activated abilities use
at line 1907-1913). This requires `matches_target_filter` to handle
`YouControl` and `YouDontControl` filters, which it currently doesn't
(line 1945 falls through to `_ => true`).

---

### 🟡 Engine Bug BT: Burning Vengeance logs "deals 2 damage to opponent" before the target is chosen
**Severity:** very low — log message accuracy only
**File:** `mtg-engine/src/cards/isd/burning_vengeance.rs:55-69`

The on_spell_cast handler presents a target-choice via
`present_target_choice` (line 57) and then immediately logs
`"Burning Vengeance deals 2 damage to opponent (flashback spell cast)"`
(line 67-68). The log fires BEFORE the player picks a target. The
player might pick:
- An opponent (the log is correct)
- A creature (the log is wrong — it's a creature, not "opponent")
- A planeswalker (Bug BQ blocks this anyway)
- A creature you control (the log is wrong — it's your own creature)
- Yourself (the log is wrong — it's "you", not "opponent")

The actual damage application happens later via the `PendingEffect::DealDamage`
that the choice resolves to. That path logs correctly. The Burning
Vengeance log line is just stale.

**Did NOT fire** in audit because Burning Vengeance was drafted but
not put on the battlefield in any sampled game.

**Proposed fix:** delete the line 67-68 `state.log(...)` call —
present_target_choice's effect handler already logs the damage
correctly when it resolves. (And if it doesn't, log THERE, not in
the trigger handler.)

A related but separate concern is the same one Snapcaster Mage hits
(Bug M): the target should be chosen WHEN THE TRIGGER GOES ON THE
STACK, not at resolution time. Burning Vengeance's
`on_spell_cast` is called from `resolve_next_trigger` (i.e. at
resolution), so opponents have no priority window to respond to
"Burning Vengeance with target X" before X is locked in. Same general
issue as Snapcaster, same proposed fix family.

---

### 🟡 Engine Bug BS: `cast_with_flashback` flag persists after Runic Repetition returns the card to hand
**Severity:** low — Runic Repetition + flashback card interaction only
**Files:**
- `mtg-engine/src/state.rs:485-503` (move_object — doesn't reset cast_with_flashback)
- `mtg-engine/src/engine.rs:2188-2199` (cast handler — only sets the flag, never clears)
- `mtg-engine/src/state.rs:1292-1300` (move_spell_after_resolve — uses the flag to choose exile vs graveyard)
- `mtg-engine/src/cards/isd/runic_repetition.rs:50-58`

The cast handler sets `obj.cast_with_flashback = true` only when `is_flashback`
is true (line 2193); it doesn't reset the flag on a normal cast. `move_object`
clears battlefield-related fields when leaving the battlefield but doesn't
touch `cast_with_flashback`. Result:

1. Player flashbacks Devil's Play. Spell resolves with cast_with_flashback=true,
   goes to exile.
2. Player casts Runic Repetition targeting Devil's Play in exile.
3. Devil's Play returns to hand, but `obj.cast_with_flashback` is still true.
4. Player casts Devil's Play normally (from hand). The cast handler doesn't
   set is_flashback, so the flag is unchanged (still true).
5. Spell resolves and `move_spell_after_resolve` checks the flag — still true —
   sends Devil's Play to exile instead of graveyard.

The card ends up in exile after a NORMAL cast just because it was previously
cast via flashback. This means Runic Repetition can't actually "reuse" a
flashback spell via the normal-cast path: the spell still gets exiled.

Same shape applies to any flashback card returned from exile to hand by
Runic Repetition (or future "return from exile" effects).

**Did NOT fire** in audit — Runic Repetition was drafted but not cast.

**Proposed fix:** in `move_object`, when transitioning from any zone to
hand/library/stack (i.e., the card is no longer "the resolved spell"),
reset `obj.cast_with_flashback = false`. Or, more conservatively, reset
it specifically when the card moves from Exile to a non-resolution zone.
Or in the cast handler, ALWAYS set `obj.cast_with_flashback = is_flashback`
unconditionally, instead of only setting when true.

---

### 🟡 Engine Bug BF: Traveler's Amulet doesn't shuffle the library after the search
**Severity:** low — Traveler's Amulet only, related to Bug BC's auto-pick
**File:** `mtg-engine/src/cards/isd/travelers_amulet.rs:51-83`

Oracle: "Search your library for a basic land card, reveal it, put it
into your hand, **then shuffle**." The current implementation removes
the searched land from `library_order` and moves it to hand, then
returns. There is no shuffle call. The comment at line 83 says
"Shuffle (no-op in our engine, library is treated as ordered for
gameplay)" — but other tutors (Caravan Vigil at line 99, Ghost Quarter
at line 100, Bitterheart Witch at line 101, Garruk -1 at lines 56,67)
DO call `library_order.shuffle(&mut rand::thread_rng())`. Traveler's
Amulet was missed.

This isn't strictly observable through gameplay (the engine doesn't
let the player peek at the library order), but it leaves the
library in a non-shuffled state for any subsequent reveal effect
(Mindshrieker, Cellar Door, Trepanation Blade, Moan of the
Unhallowed's flashback search) and makes the game state diverge from
a Magic Online reference.

**Did NOT fire** in audit — Traveler's Amulet was drafted but the
shuffle's absence didn't have a measurable effect on the games sampled.

**Proposed fix:** add the standard shuffle call after the search,
matching the pattern in Caravan Vigil:
```rust
use rand::seq::SliceRandom;
let mut rng = rand::thread_rng();
state.get_player_mut(controller).library_order.shuffle(&mut rng);
```

---

### 🟡 Engine Bug BE: Garruk Relentless dies before transforming when damage takes him from 3+ loyalty straight to 0
**Severity:** medium — only affects Garruk, latent in audit (not drafted)
**File:** `mtg-engine/src/sba.rs:184-244` (planeswalker zero-loyalty SBA + state trigger)
**Note:** originally numbered Bug AZ → renamed to BB → renamed to BE to deconflict
with Agent A's Bug BB (Ludevic hatchling counters) which landed concurrently
on top of the AZ→BB rename.

The SBA loop processes planeswalker zero-loyalty BEFORE the Garruk
state trigger. If Garruk takes damage that drops his loyalty from 3+
straight to 0, the planeswalker-zero-loyalty SBA fires first and
moves Garruk to the graveyard. Then the Garruk state trigger checks
`o.zone == Battlefield && loyalty <= 2 && !is_transformed` — Garruk
isn't on the battlefield, the trigger doesn't fire, Garruk is dead
without transforming.

Per CR 603.8 + 704.5j, the state-triggered transform should preempt
zero-loyalty destruction: the state trigger condition was true (loyalty
hit ≤2 the moment damage was applied), so it should have been queued
before the zero-loyalty SBA had a chance to graveyard him.

**Workaround case:** If Garruk takes damage that drops his loyalty
to ≤2 but >0 (e.g., 3→2 from a 1-power attacker), the state trigger
fires and Garruk transforms correctly. The bug only manifests when
damage goes straight to 0.

**Did NOT fire** in audit — Garruk Relentless wasn't drafted.

**Proposed fix:** check the state trigger condition BEFORE the
planeswalker-zero-loyalty SBA in the same pass, or run state trigger
processing immediately after damage is applied (before SBA checks
zero-loyalty death). Cleanest: add a "state trigger watch" pass at
the top of the SBA loop that queues triggers, then process queued
triggers between SBA passes.

---

### 🟡 Engine Bug BD: setup_game doesn't initialize obj.subtypes from registry data
**Severity:** HIGH — root cause of Bug AX, contributes to Bug AT/AU/AY family
**File:** `mtg-engine/src/engine.rs:3450-3462` (setup_game)
**Note:** originally numbered Bug AY → renamed to BA to deconflict with Agent A's
Bug AY (HasSubtype filter) → renamed to BD to deconflict with Agent A's Bug BA
(Skirsdag {:?} label) which landed concurrently. The two AY entries share the
same root cause: `obj.subtypes` is empty for normal cards. Fixing BD would let
Agent A's AY become a no-op.

When setting up a game, each card object is created with empty
`obj.subtypes`:
```rust
let obj_id = state.create_object(card_id, player_id, Zone::Library, card_data.power, card_data.toughness);
let obj = state.get_object_mut(obj_id).expect("...");
obj.colors = colors.clone();
obj.name = card_name.clone();
obj.keywords = card_data.keywords.clone();
obj.card_types = card_data.card_types.clone();
// obj.subtypes is NEVER initialized — stays Vec::new() from create_object
```
`obj.colors`, `obj.name`, `obj.keywords`, and `obj.card_types` are all
copied from registry data, but `obj.subtypes` is not. So every normal
card object starts the game with `obj.subtypes = []`.

This is the **root cause of Bug AX**: dual lands check
`o.subtypes.iter().any(|s| s == "Swamp")` against an empty vector and
always return false. It also makes Bug AT (registry-only filters miss
tokens) more pervasive than it needs to be — many of those filters
would have been correct if `obj.subtypes` were populated up front.

**Proposed fix:** add `obj.subtypes = card_data.subtypes.clone();` to
setup_game alongside the other field initializations. This makes
`obj.subtypes` the authoritative subtype list for non-token objects
and matches the contract that token creation already follows
(`create_token_internal` sets `subtypes` directly on the new object).

After this fix, every "instance subtypes only" check (Woodland
Cemetery et al, Moonmist's first branch) would work correctly. Cards
that check both registry and instance still work — they just become
mildly redundant.

Note: this would interact with Olivia Voldaren's "becomes a Vampire"
ability differently. Currently Olivia pushes "Vampire" onto an empty
vector. After the fix, Olivia would push onto a vector that already
contains the original subtypes ("Vampire" gets appended), which is
the correct behavior per CR 205.3 ("becomes a Vampire **in addition
to its other types**"). Bug AU goes away as a free side effect.

---

### 🟡 Engine Bug AX: Four ISD dual lands always enter tapped (Woodland Cemetery, Sulfur Falls, Clifftop Retreat, Isolated Chapel)
**Severity:** HIGH (mana fixing fundamentally broken) — but latent in audit (no dual lands drafted)
**Files:**
- `mtg-engine/src/cards/isd/woodland_cemetery.rs:21-22`
- `mtg-engine/src/cards/isd/sulfur_falls.rs:21-22`
- `mtg-engine/src/cards/isd/clifftop_retreat.rs:21-22`
- `mtg-engine/src/cards/isd/isolated_chapel.rs:21-22`

Each of these lands says "this land enters tapped unless you control a
{basic1} or a {basic2}". They check by iterating other permanents and
testing instance subtypes:
```rust
o.subtypes.iter().any(|s| s == "Swamp")
    || o.subtypes.iter().any(|s| s == "Forest")
```
Basic lands are created via `state.create_object` which initializes
`subtypes: Vec::new()` (state.rs:255). The basic Swamp's subtype lives
in `registry.card_data(card_id).subtypes`, NOT on the instance. The
check therefore always returns false, and these dual lands always
enter tapped — completely defeating their purpose as fixers.

`Hinterland Harbor` (Forest/Island) has the correct pattern (checks
both instance and registry via a closure), so it's the model for the
fix:
```rust
let has_subtype = |name: &str| {
    o.subtypes.iter().any(|s| s == name)
        || registry.card_data(o.card_id)
            .map_or(false, |d| d.subtypes.iter().any(|s| s == name))
};
has_subtype("Forest") || has_subtype("Island")
```

**Did NOT fire** in audit — none of the dual lands were drafted past
the initial pool listing. But if they were, the model would have spent
mana fixing on lands that always enter tapped.

**Proposed fix:** apply the Hinterland Harbor pattern to all four
broken duals.

---

### 🟡 Engine Bug AV: create_token_copy doesn't preserve dynamic P/T (Cackling Counterpart and Back from the Brink break for */* creatures)
**Severity:** medium — affects Cackling Counterpart and Back from the Brink
**File:** `mtg-engine/src/state.rs:404-440` (create_token_copy)
**Callers:** `cards/isd/cackling_counterpart.rs` and `cards/isd/back_from_the_brink.rs`

```rust
let (name, power, toughness, card_id) = match source {
    Some(o) => (o.name.clone(), o.power, o.toughness, o.card_id),
    None => return ObjectId(0),
};
let (colors, keywords, card_types, subtypes) = registry.card_data(card_id)
    .map(|d| { ... });
let id = self.create_token_with_subtypes(
    &name, owner,
    power.unwrap_or(0),
    toughness.unwrap_or(0),
    ...
);
```
The token is created with `power` and `toughness` taken from the SOURCE
object's stored fields. For `*/*` creatures with characteristic-defining
abilities (CDAs):
- Geist-Honored Monk (P/T = creatures you control)
- Splinterfright (P/T = creature cards in graveyard)
- Boneyard Wurm (P/T = creature cards in graveyard)
- Sturmgeist (P/T = cards in your hand)
- Mikaeus the Lunarch (P/T from +1/+1 counters; X-cost ETB)

The source's `obj.power` is the BASE printed value (0 for Sturmgeist,
Geist-Honored Monk, Splinterfright, Boneyard Wurm; >0 for Mikaeus
because of the X-cost counter ETB). The token is created with that
base value and `card_id: CardId(0)` (sentinel — see
`state.rs:341-356`), so the registry can't look up the card's
behavior to evaluate the CDA.

Result: a Cackling Counterpart token-copy of Geist-Honored Monk (or
Splinterfright/Boneyard Wurm/Sturmgeist) is a 0/0 instead of the
characteristic-defining value, and dies immediately to SBA 704.5f.

Per CR 706.2 the copy is "an exact copy of the original… all the
characteristics of the original are copied," which includes the CDA
itself. The token should compute its own dynamic P/T based on its own
controller's state.

**Did NOT fire** in audit — Cackling Counterpart was drafted but not
cast.

**Proposed fix:** store the source's `card_id` on the token (use a
new `is_copy_of: Option<CardId>` field, or set the token's `card_id`
to the source's), then have `effective_power` consult the registry
when computing the token's P/T. Same trick the engine already uses
for transformed DFCs (face-aware lookup). Or, simpler: snapshot the
effective P/T at copy time:
```rust
let power = state.effective_power(source_id, registry).unwrap_or(0);
let toughness = state.effective_toughness(source_id, registry).unwrap_or(0);
```
This loses the live-recomputation property — the token's P/T would
be frozen at creation rather than tracking the controller's
graveyard/hand/etc — but it gives a non-zero starting value.

---

### 🟡 Engine Bug AP: Global "creatures get +N/+N until end of turn" effects snapshot at resolution
**Severity:** medium — affects every global anthem/debuff in ISD
**Files:**
- `mtg-engine/src/cards/isd/rally_the_peasants.rs:30-51`
- `mtg-engine/src/cards/isd/vampiric_fury.rs:29-65`
- `mtg-engine/src/cards/isd/hysterical_blindness.rs:29-50`
- (Likely also moment_of_heroism.rs and similar — they all use the same pattern)

These cards say "Creatures you control get +N/+N until end of turn" or
"Creatures your opponents control get -N/-N until end of turn" — an
anthem effect that applies to ALL relevant creatures at any point during
the turn, including ones cast/created AFTER the anthem resolves.

The current implementation:
```rust
let creature_ids: Vec<ObjectId> = state.objects.values()
    .filter(|obj| obj.zone == Zone::Battlefield && obj.controller == controller && obj.power.is_some())
    .map(|o| o.id)
    .collect();
for id in creature_ids {
    state.until_end_of_turn.push(TemporaryEffect::ModifyPT { target: id, ... });
}
```
iterates the creatures at the moment of resolution and pushes one
per-target ModifyPT effect. New creatures that come into play later in
the turn (Bloodline Keeper's vampire token activation after Vampiric
Fury, a Civilized Scholar transformed into Homicidal Brute after Rally
the Peasants, a Mausoleum Guard death-trigger spirit after Hysterical
Blindness) won't get the bonus.

**Did NOT fire in audit** — these spells are typically cast immediately
before a combat phase and the model rarely creates more creatures
between casting and combat. But the bug is structurally present.

**Proposed fix:** add a `TemporaryEffect::GlobalAnthem { filter: CreatureFilter, power_mod, toughness_mod }`
variant that the effective_power/effective_toughness machinery walks
when computing P/T. The filter would let Vampiric Fury target
"creatures you control with subtype Vampire", Hysterical Blindness target
"creatures your opponents control", etc. Same architecture as the
existing `ContinuousEffect::ModifyPT { scope: EffectScope::Global(...) }`
pattern but with an until-end-of-turn duration.

---

### 🟡 Engine Bug AO: combat::get_subtypes is not face-aware for transformed DFCs
**Severity:** low — no in-set repro, fortuitous because all ISD werewolf back faces are also Werewolves
**File:** `mtg-engine/src/combat.rs:402-415`

```rust
fn get_subtypes(state: &GameState, creature_id: ObjectId, registry: &CardRegistry) -> Vec<String> {
    let mut subtypes = Vec::new();
    if let Some(obj) = state.get_object(creature_id) {
        subtypes.extend(obj.subtypes.iter().cloned());
        if let Some(data) = registry.card_data(obj.card_id) {
            for s in &data.subtypes {
                if !subtypes.contains(s) {
                    subtypes.push(s.clone());
                }
            }
        }
    }
    subtypes
}
```
For a transformed creature, this combines the back-face subtypes (from
`obj.subtypes`) with the FRONT-face subtypes (from
`registry.card_data(obj.card_id).subtypes`). The combined list is the
union of both faces. For a DFC where the back face DROPS a subtype,
this falsely reports the dropped subtype as still active.

For ISD this happens to be safe — Tormented Pariah, Mayor of Avabruck,
Hanweir Watchkeep, Daybreak Ranger, Villagers of Estwald all keep the
"Werewolf" subtype on both faces — but Civilized Scholar → Homicidal
Brute drops "Advisor" and Cloistered Youth → Unholy Fiend drops "Human".

The function is consulted by `is_non_wolf_damage_prevented` (Moonmist's
combat-damage prevention) and possibly elsewhere. For Moonmist's check
this means a transformed Cloistered Youth (Unholy Fiend) could be
treated as still having Human, etc., but Moonmist only cares about
Werewolf/Wolf so it doesn't manifest.

**Proposed fix:** check `obj.is_transformed` and use back-face data
when transformed:
```rust
if obj.is_transformed {
    if let Some(behavior) = registry.get(obj.card_id) {
        if let Some(back) = behavior.back_face_data() {
            return back.subtypes.clone();
        }
    }
}
```
And mirror this fix in any other "subtype lookup" helper in `combat.rs`
or `state.rs` that doesn't already use the face-aware pattern from
`matches_filter::HasSubtype` (state.rs:692).

---

### 🟡 Engine Bug Y: pay-mana-during-resolution checks the mana pool only — never offered when pool is empty
**Severity:** medium — multiple cards affected
**Files:**
- `mtg-engine/src/cards/isd/screeching_bat.rs:89-93` (upkeep transform `{2}{B}{B}`)
- `mtg-engine/src/cards/isd/mentor_of_the_meek.rs:73-88` (ETB pay `{1}` to draw)
- `mtg-engine/src/cards/isd/frightful_delusion.rs:50` (target may pay `{1}`)

These cards present "you may pay {N}" choices during trigger/spell
resolution. Each one checks `state.get_player(controller).mana_pool.get(...)`
or `mana_pool.total() >= 1` — i.e. it only succeeds if the player has
already floated mana into the pool.

The problem: mana pools empty between phases and steps (CR 106.4). For
upkeep triggers (Screeching Bat) the pool is GUARANTEED to be empty when
the trigger resolves, so the prompt never appears. For ETB triggers
(Mentor of the Meek) the pool depends on whatever the player floated
before casting. For instant resolution (Frightful Delusion) the targeted
player has a priority window to manually float mana — but the engine
doesn't surface this as part of the choice, the player has to know to
tap lands defensively.

Mentor and Screeching Bat additionally don't even check `can_pay` before
presenting the Yes/No prompt — Screeching Bat does (line 91) and skips
the prompt when it can't pay, so the upkeep transform option is just
silently never offered in any plausible game state.

**Did NOT fire** in audit — Screeching Bat was sideboard-only,
Mentor of the Meek was a single draft-pick, Frightful Delusion never cast.

**Proposed fix:** present the Yes/No choice with an autotap-resolved tap
plan attached. When the player picks "Yes", the engine taps the lands in
the plan and pays. Same shape as Bug A's autotap-for-activated-abilities
fix, except for ResolutionChoice rather than legal_actions.

---

### 🟡 Engine Bug X (suspected): aura-granted activated abilities collide with creature-native ability_index
**Severity:** low — only Skeletal Grimace in ISD grants an activated ability via aura
**File:** `mtg-engine/src/engine.rs:2257-2284` (apply path)

Skeletal Grimace's `{B}: Regenerate` is implemented in
`mtg-engine/src/cards/isd/skeletal_grimace.rs` as an `activated_abilities`
method that returns the granted ability with `ability_index: 0`. The
engine collects activated abilities by walking the creature's own behavior
and ALL attached auras, then bundles them into `Action::ActivateAbility`
entries that carry only `(object_id, ability_index)`.

The apply path then uses this lookup chain to find the ability:
1. Try the creature's own `activated_abilities` first.
2. If not found, try Evil Twin override.
3. If not found, try attached auras.

Step 1 short-circuits — if the creature itself has an activated ability at
`ability_index: 0`, the lookup returns *that* one and never reaches the
aura. For example, if Skeletal Grimace is attached to Daybreak Ranger
(which has `{T}: Deal 2 damage to flying` at index 0), the engine thinks
"Activate <Ranger> ability 0" means the Ranger's tap-damage ability, not
the regenerate. The action label might also be wrong.

Even when there's no native ability at index 0, the LLM-player display
collapses by `(object_id, ability_index)` (`mtg-player/src/llm.rs:2086`),
so two distinct abilities sharing the same index would appear as one.

**Did NOT fire confirmed in audit** — Skeletal Grimace was attached to
Spectral Rider in Seat 2's deck, and Spectral Rider has no activated
abilities, so the lookup correctly fell through to the aura. But the
broken case is reachable.

**Proposed fix:** add a `source_card_id: Option<CardId>` field to
`Action::ActivateAbility` that records WHICH card the ability is granted
by, so apply can disambiguate without scanning. Same for the LLM player's
`seen_ability_keys` tuple — make it `(object_id, source_card_id, ability_index)`.

---

### 🟡 Engine Bug M (rules-shortcut): Snapcaster Mage chooses target on resolve, not on cast
**Severity:** low — opponents can't respond to the Snapcaster choice
**File:** `mtg-engine/src/cards/isd/snapcaster_mage.rs:43-87`

Snapcaster's ETB trigger says "When this creature enters, target instant or
sorcery card in your graveyard gains flashback...". Per CR, the target must
be chosen WHEN THE TRIGGER GOES ON THE STACK — opponents then have priority
to respond before the trigger resolves. The current implementation defers the
target choice to ETB resolution (`on_enter_battlefield` builds the
choice list and calls `present_target_choice`), so:
- The opponent never gets a window to respond *between* "Snapcaster trigger
  goes on stack" and "trigger resolves" with the target locked in.
- A spell that exiles the targeted card in response (Purify the Grave,
  Surgical Extraction) can't fizzle the trigger.

This is a general engine pattern (many ETB triggers in this codebase pick
targets at resolve time). Not specific to Snapcaster but worth noting since
Snapcaster is one of the highest-profile cards affected.

**Did NOT fire** — Snapcaster wasn't drafted in the audit log.

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

### 🟡 Harness Bug H10: board display uses comma as both keyword and creature separator
**Severity:** medium — model has to infer creature boundaries from capitalization
**File:** `mtg-player/src/llm.rs:1488-1585` (format_perms_compact) and 2366-2385 (format_keywords)

`format_keywords` joins keywords with `", "` and `format_perms_compact`
joins creatures with `", "` — same separator. So a board with multiple
keyworded creatures renders ambiguously:

```
Your board: 3x Forest, 2x Mountain, Terror of Kruin Pass 3/3 double strike, menace, Ashmouth Hound 2/1, Gatstaf Howler 3/3 intimidate, menace, Howlpack of Estwald 4/6 menace
```
(audit log line 30173)

The model has to figure out which `menace` belongs to which creature by
matching the creature-name-then-P/T pattern. For Terror of Kruin Pass
("Werewolves you control have menace") all four werewolves on this
board correctly have menace, but the format makes it look like the
keyword might belong to the next creature in the list.

**Did NOT cause an obvious wrong play in audit** — Gemini 3.1 Flash
Lite seemed able to disambiguate by capitalization, but it's a fragile
signal and the prompt costs cognitive bandwidth.

**Proposed fix:** use a stronger separator between creatures, e.g.
`" | "` instead of `", "`, or wrap each creature's keyword list in
brackets: `Terror of Kruin Pass 3/3 [double strike, menace]`. The
combat-creature labels in `format_combat_creature` are similarly
affected.

---

### 🟡 Harness Bug H9: deck-builder validator doesn't help the model converge
**Severity:** medium — wastes API calls, occasionally gets stuck (Seat 0 had 9 attempts)
**File:** `mtg-draft/src/deckbuilding.rs:60-73`
**Audit evidence:** 21 deck-builder validation failures in v5 audit log lines
8380-11200, including:
- Seat 0: 9 attempts to build a 40-card deck, kept submitting 32-33 cards
- Seat 6: 5 attempts, all submitting 31 cards
- Seat 3: 1 valid deck-too-small + 2 "missing or empty lands object" errors
- Seat 5: 2 "missing or empty lands object" errors
- Seat 4, Seat 7: 1 attempt each at deck-too-small

Two failure modes:

**(a) "missing or empty lands object" with all-zero counts:**
The parser at line 60-66 only adds entries with `count > 0`:
```rust
for (name, count) in lmap {
    if let Some(n) = count.as_u64() {
        if n > 0 {
            lands.insert(name.clone(), n as u32);
        }
    }
}
```
So `"lands": {"Plains": 0, "Swamp": 0}` parses to an empty `lands`
HashMap and triggers the "missing or empty" error at line 71. The error
message is misleading — the lands object IS present, just all values are
0. The model interprets the message as "you forgot the lands key" and
re-submits the same shape.

**(b) deck too small + slow convergence:**
Seat 0 went from 32→33→32→32→33→32→32→32→32 cards across 9 attempts. The
model is reducing Plains count by 1 and adding 1 maindeck card each
attempt — net +1 card per try. The error message just says the count
("Deck has 32 cards (need at least 40). Add more cards or lands.") but
doesn't show what the model gave it last time, doesn't suggest a specific
count to add, and doesn't auto-complete with lands when the model can't
converge. The example in the prompt template
(`{"Plains": 0, "Island": 9, "Swamp": 8, "Mountain": 0, "Forest": 0}`)
also confuses some seats — Seat 0 is W/R Boros but the example only
mentions Island/Swamp non-zero, so the model omits Mountain entirely
from its response and only adjusts Plains.

**Proposed fix:**
1. Better error message for the all-zero case: "Your `lands` object has
   no non-zero values. Specify the count of each basic land you want
   (e.g. `\"Mountain\": 9`)."
2. Echo the deck composition back to the model in the retry prompt:
   "Your previous attempt: 23 maindeck cards + 9 lands = 32 cards
   total. You need 8 more cards. Add basic lands or include more cards
   from your pool."
3. After N failed attempts, auto-complete with the most-needed basic
   lands (compute from cost symbols of the maindeck) so the game can
   start instead of looping until the LLM gives up.

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
- (Earlier write-up referenced "trigger ordering" prompts here. After
  further investigation: the engine doesn't actually have ordering prompts
  — see Bug N. The model's confusion came from cases where the choice
  prompt had no source label and the model conflated it with the stack
  state shown in the prompt header.)
- **Confirmed in audit at line 116530-116531** (Bitterheart Witch curse-search
  YesNo prompt): the prompt is literally just
  ```
  Available actions:
  0: Yes, 1: No
  ```
  with no header. The engine HAS the description string in
  `ResolutionChoiceKind::YesNo::description` ("Bitterheart Witch: search
  your library for a Curse card?") but the LLM player's `format_action`
  for `Action::ResolveChoice` only formats individual actions ("Yes",
  "No") and never surfaces the description.

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

**Audit-log decisions sampled (Passes 1-3):** ~80 of ~3,766 model thoughts.

**Cards inspected by reading source code (Pass 3):** equipment (Silver-Inlaid
Dagger, Butcher's Cleaver, Cobbled Wings, Trepanation Blade, Inquisitor's
Flail, Mask of Avacyn, Demonmail Hauberk, Wooden Stake, Sharpened Pitchfork,
Blazing Torch); transform creatures (Daybreak Ranger, Cloistered Youth,
Civilized Scholar, Mayor of Avabruck, Tormented Pariah, Ulvenwald Mystics,
Villagers of Estwald, Screeching Bat, Bloodline Keeper, Mikaeus the Lunarch,
Garruk Relentless); sacrifice abilities (Skirsdag Cultist, Disciple of
Griselbrand, Stitcher's Apprentice, Brain Weevil, Demonmail Hauberk);
target-choice spells (Sever the Bloodline, Memory's Journey, Bramblecrush,
Naturalize, Curse of Death's Hold, Curse of the Pierced Heart, Curse of
Stalked Prey, Brimstone Volley, Geistflame, Heretic's Punishment); morbid
(Caravan Vigil, Festerhide Boar, Somberwald Spider, Reaper from the Abyss,
Morkrut Banshee, Skirsdag High Priest, Hollowhenge Scavenger); ETB+choice
(Snapcaster Mage, Mentor of the Meek, Bitterheart Witch, Olivia Voldaren);
graveyard care (Spider Spawning, Endless Ranks of the Dead, Splinterfright,
Boneyard Wurm, Wreath of Geists, Mulch, Mirror-Mad Phantasm, Back from the
Brink); auras (Bonds of Faith, Skeletal Grimace, Curiosity, Spectral Flight,
Dead Weight, Dearly Departed); flashback (Forbidden Alchemy, Cackling
Counterpart, Burning Vengeance, Devil's Play, Travel Preparations); lands
(Stensia Bloodhall, Kessig Wolf Run, Cellar Door); planeswalker (Garruk
Relentless); legendary handling (Geist of Saint Traft, Olivia, Mikaeus);
counterspells (Frightful Delusion); damage tracking (Abattoir Ghoul, Falkenrath
Noble, Lumberknot, Charmbreaker Devils); state-tracking (creature_died_this_turn,
num_spells_cast_last_turn, legend rule, autotap, X-cost handling).

**Audit decisions checked:** Pass 1 covered ~50 random samples; Pass 2 added
hypothesis-driven sweeps for Bug A (equipment activation), Bug C (sacrifice
fizzle), Bug H7 (opaque target prompts), Bug H5 (Yes/No prompts), Bug H
(Maw of Hell two-target filter), Bug I (X-cost flashback compute_autotap);
Pass 3 sampled Devil's Play / Bitterheart Witch / Bonds of Faith / Skeletal
Grimace prompt formats and trigger-target prompts. All confirmed.

**Status of bug families found:**
- ✅ Fixed: A (autotap), B (Human snapshot), C (sacrifice choice), H1 (combat
  disambiguation), P1-P5 (model prompt fixes).
- 🟡 Documented but not fixed: D, E, F, G, H, I, J, K, L, M, N, O, P, Q, T,
  U, W, X, Y, plus harness bugs H2, H3, H5, H6, H7, H8.

**Areas not yet checked:**
- Most round 2 / round 3 audit decisions
- Werewolf transform timing edge cases with Moonmist mid-resolution
- Mulligan bottoming decisions (only the keep/mull side checked)
- Long-thought decisions (~hundreds of candidates)
- Activated abilities of creatures I haven't spotted yet (Geistcatcher's Rig,
  Cellar Door, etc.)

Mining will continue.

---

### 🟡 Engine Bug AY: `TargetFilter::HasSubtype` checks instance subtypes only — Olivia Voldaren's `{3}{B}{B}` can't target any registry-only Vampire
**Severity:** medium — latent (Olivia was not drafted in audit) but erases one of her two activated abilities for most realistic targets
**File:** `mtg-engine/src/engine.rs:1808-1810` (matches_ability_target_filter) and `mtg-engine/src/engine.rs:1944` (matches_target_filter)

Same structural failure as Bug AT, but on the **engine-side target
enumeration** for activated abilities rather than a card-specific filter.
Both `HasSubtype` branches in `engine.rs` consult only `obj.subtypes`:

```rust
// engine.rs:1808 (matches_ability_target_filter, activated abilities path)
TargetFilter::HasSubtype(subtype) => {
    obj.subtypes.contains(subtype)
}

// engine.rs:1944 (matches_target_filter, general ability helper)
TargetFilter::HasSubtype(subtype) => obj.subtypes.iter().any(|s| s == subtype),
```

Neither falls back to `registry.card_data(obj.card_id).subtypes`. As
documented in Bug AU and Bug AV, `obj.subtypes` is initialised to
`Vec::new()` in `GameObject::new` (`state.rs:255`) and is only written in
three narrow situations:
1. Enter-as-copy via `state.rs:593` (Evil Twin / Cackling Counterpart /
   Essence of the Wild).
2. `apply_transform` / Moonmist transform flips
   (`cards/helpers.rs:253,261`, `cards/isd/moonmist.rs:80,94`).
3. Card-specific "becomes a <subtype>" on_activate hooks (Olivia's first
   ability pushes "Vampire", Grimoire of the Dead pushes "Zombie").

**Regular creatures cast from hand enter with `obj.subtypes = Vec::new()`.**
Their subtypes live exclusively in the registry.

**In-set impact — Olivia Voldaren's `{3}{B}{B}: Gain control of target
Vampire`:**

```rust
// cards/isd/olivia_voldaren.rs:75
target_requirement: Some(TargetRequirement::CreatureWithFilter(
    TargetFilter::HasSubtype("Vampire".into()))),
```

`generate_ability_targets` for a `CreatureWithFilter` applies
`matches_ability_target_filter` (engine.rs:1855), which routes through
the buggy `HasSubtype` arm. As a result, the only Vampires Olivia's
second ability can target are:
- Bloodline Keeper's 2/2 tokens (tokens get their subtypes written into
  `obj.subtypes` at creation via `create_token_with_subtypes`).
- Creatures that Olivia has already bitten with her first ability
  (`obj.subtypes.push("Vampire")` at `olivia_voldaren.rs:109`).
- Transformed back-faces that ended up Vampires (none in ISD).

Regular registry-only Vampires — Crossway Vampire, Markov Patrician,
Stromkirk Noble, Bloodcrazed Neonate, Rakish Heir, Vampire Interloper,
Falkenrath Noble, Falkenrath Marauders, Bloodline Keeper front face —
cannot be targeted at all. The action never even appears in the
legal-actions list because `generate_ability_targets` returns an empty
vector, so no `ActivateAbility` action is emitted for that (source,
ability_index) pair.

**Did NOT fire in audit** — Olivia was not drafted (only appears in the
deck-building card knowledge dump at log line 829-832). But she's a
mythic rare and this bug zeroes out half her text box in most realistic
board states.

**Proposed fix:** mirror the pattern Victim of Night already uses
(`cards/isd/victim_of_night.rs:43-47`). Make both filter branches fall
through to the registry:

```rust
TargetFilter::HasSubtype(subtype) => {
    if obj.subtypes.iter().any(|s| s == subtype) {
        return true;
    }
    registry.card_data(obj.card_id)
        .map(|d| d.subtypes.iter().any(|s| s == subtype))
        .unwrap_or(false)
}
```

`matches_ability_target_filter` already has `registry` in scope
(engine.rs:1778), so that caller is a one-line change.
`matches_target_filter` (engine.rs:1935) doesn't currently take
`registry` — callers need to thread it through, or inline the check at
the call site.

**Related bugs:**
- Bug AT is the mirror failure on the card-specific-filter side
  (registry-only, misses tokens). This bug is instance-only, misses
  cards. The two together mean every "filter by subtype" in the engine
  should always check both sources.
- Bug AU's Option 2 (copy registry subtypes into `obj.subtypes` when
  first modifying them) would partially mitigate this bug for creatures
  that have been touched by subtype-mutation effects, but not for
  untouched creatures freshly cast from hand.
- The `NotSubtypes` filter at engine.rs:1798 has the same shape
  (instance-only). Victim of Night papers over it with its own
  `is_valid_target` override, but a generic fix on the filter side
  would remove the need.

**Proposed fix:** one-line change at each of the two HasSubtype arms
to consult the registry on fallback.

---

### 🟡 Engine Bug AZ: Spare from Evil's protection anthem is snapshotted at resolution (Bug AP sibling for `GrantProtection`)
**Severity:** low — affects Spare from Evil only, latent in audit
**File:** `mtg-engine/src/cards/isd/spare_from_evil.rs:33-55`

Same structural defect as Bug AP ("snapshot anthems"), but for a
different `TemporaryEffect` variant. Spare from Evil's oracle:
"Creatures you control gain protection from non-Human creatures until
end of turn."

```rust
let creature_ids: Vec<ObjectId> = state.objects.values()
    .filter(|obj| obj.zone == Zone::Battlefield
        && obj.controller == controller && obj.power.is_some())
    .map(|obj| obj.id)
    .collect();
let filter = CreatureFilter::Not(Box::new(
    CreatureFilter::HasSubtype("Human".into())));
for id in &creature_ids {
    state.until_end_of_turn.push(
        crate::state::TemporaryEffect::GrantProtection {
            target: *id,
            filter: filter.clone(),
        }
    );
}
```

The creatures-you-control list is snapshotted at resolution, then one
`GrantProtection` push per creature. Any creature entering the
battlefield under your control AFTER Spare from Evil resolves (Mausoleum
Guard death-trigger spirits, a Doomed Traveler's spirit token from the
same combat, Mayor of Avabruck transforming a werewolf mid-turn, etc.)
will NOT have the protection applied, contrary to the continuous
wording of the anthem.

Bug AP already called out rally_the_peasants / vampiric_fury /
hysterical_blindness for the same snapshot pattern with
`TemporaryEffect::ModifyPT`. This is the analogous case for
`TemporaryEffect::GrantProtection`. The fix family is the same — an
until-end-of-turn global-scoped continuous effect that the relevant
query helpers (effective_power for ModifyPT, `has_protection` for
GrantProtection) consult when evaluating a creature — but it needs a
second variant (`TemporaryEffect::GlobalGrantProtection { filter }` or
similar).

**Did NOT fire** in audit — Spare from Evil was not cast in any game I
sampled. The card is niche (fog-ish white instant) and rarely appears.

**Proposed fix:** add `TemporaryEffect::GlobalProtection { scope:
CreatureFilter, filter: CreatureFilter }` (scope = "who gets the
protection", filter = "what they're protected from"), consulted by the
same code path that handles `TemporaryEffect::GrantProtection` today.
Spare from Evil uses scope=ControlledByYou, filter=Not(HasSubtype(Human)).
This mirrors the proposed fix shape for Bug AP.

<!-- Reserving letters BJ-BN for this branch (5 letters past BE on master). -->

---

### 🟡 Engine Bug BJ: Evil Twin enters as a 0/0 and dies to SBA before its ETB copy trigger resolves
**Severity:** HIGH (card is non-functional when cast) — latent (Evil Twin not drafted in audit)
**File:** `mtg-engine/src/cards/isd/evil_twin.rs:43-61` and `mtg-engine/src/engine.rs:3146-3180` (CopyCreature PendingEffect)

Oracle: "You may have Evil Twin enter the battlefield as a copy of any
creature on the battlefield, except it has '{U}{B}, {T}: Destroy target
creature with the same name as this creature.'" This is a *replacement
effect* (CR 614.1d / "enters the battlefield as..."), not an ETB trigger
— the copy decision happens *as* Evil Twin enters, and Evil Twin never
exists on the battlefield in its 0/0 native form.

The current implementation models it as an ETB *triggered* ability with
`power: Some(0), toughness: Some(0)` and an EntersBattlefield trigger
that calls `present_optional_target_choice(... PendingEffect::CopyCreature ...)`.

**But state-based actions fire before the ETB trigger resolves.** The
priority loop at `engine.rs:4085-4096` runs:
```rust
loop {
    let sba = check_state_based_actions(state, registry);
    if !sba { break; }
    any_work = true;
}
if triggers::collect_triggers(state, registry) {
    any_work = true;
}
```
SBAs run *first*, and CR 704.5f destroys creatures with 0 toughness
directly to the graveyard (`mtg-engine/src/sba.rs:53-92`). Evil Twin
enters with the card_data-declared `power: 0, toughness: 0`, its
`effective_toughness` is 0 (Evil Twin has no `dynamic_pt` override),
and SBA 704.5f kills it *before* `collect_triggers` ever puts the ETB
copy trigger on the stack.

By the time the ETB trigger does resolve, Evil Twin is already in the
graveyard. The `CopyCreature` PendingEffect handler at
`engine.rs:3161-3177` still mutates `obj.name / power / toughness /
card_id / subtypes / keywords / colors` via
`state.get_object_mut(source_id)` — but the source is now a graveyard
object, not a battlefield permanent. The mutation silently writes the
copied creature's characteristics onto a dead Evil Twin sitting in the
graveyard. The "copy" never actually enters the battlefield.

Compare with Geist-Honored Monk (also declared 0/0 in card_data): it
has a `dynamic_pt` override that returns `(creature_count,
creature_count)`, so when SBA calls `effective_toughness` the CDA
kicks in and Monk survives (it counts itself, so ≥1 even when it's
the only creature). Evil Twin has no CDA and no similar escape valve.

**Did NOT fire** in audit — Evil Twin was not drafted in
verify-draft-8seat-high-v5.log. Any draft where the model picks Evil
Twin would be unable to cast it productively.

**Proposed fix (cleanest):** implement Evil Twin's copy as a
replacement effect alongside `ReplacementEffect::EnterAsCopy` (Essence
of the Wild, already implemented in `state.rs:530-560` via
`apply_entering_copy_replacement`). Evil Twin needs a *choice-driven*
variant — the player picks the copy target — which requires stopping
the entry to run a choice prompt, applying the chosen copy to the
entering object, then continuing entry. This would also correctly
prevent Evil Twin from being visible as a 0/0 on the battlefield.

**Workaround fix (simpler):** give Evil Twin a `dynamic_pt` override
that returns `Some((0, 1))` while Evil Twin's `card_state` lacks the
`is_evil_twin` marker. This lets Evil Twin survive the first SBA pass
so its ETB trigger can go on the stack, resolve, and apply
`CopyCreature`. Once `CopyCreature` applies, the marker is set and
the override no-ops, letting the copied creature's base P/T take
over. Not quite correct rules-wise — opponents see a 0/1 Evil Twin
on the battlefield during the window between entry and trigger
resolution, and can respond to the ETB trigger knowing Evil Twin
exists as itself — but it makes the card playable without a full
replacement-effect rewrite. Related to Bug AV (create_token_copy
doesn't preserve dynamic P/T): Evil Twin is the non-token counterpart,
using `CopyCreature` / direct-mutation rather than
`create_token_copy`.

---

### 🟡 Engine Bug BK: Instigator Gang's static "attacking creatures you control get +1/+0" anthem is implemented as a per-attack snapshot trigger
**Severity:** medium — latent in audit (Instigator Gang drafted but not cast into a matching scenario)
**File:** `mtg-engine/src/cards/isd/instigator_gang.rs:40-50, 89-110`

Oracle (verified via `scripts/oracle_lookup.py lookup "Instigator Gang"`):
- Instigator Gang front face: "Attacking creatures you control get +1/+0."
- Wildblood Pack back face: "Attacking creatures you control get +3/+0."

Both are **static continuous abilities** per CR 611 / 604.1 — they
are in effect continuously as long as Instigator Gang / Wildblood
Pack is on the battlefield, and they modify P/T of any creature that
is *currently* attacking.

The current implementation declares the buff as a triggered ability
on `TriggerKind::AnyCreatureAttacks` and resolves it by pushing an
until-end-of-turn `TemporaryEffect::ModifyPT` per attacker:
```rust
triggered_abilities: vec![
    TriggeredAbilityDef {
        kind: TriggerKind::Upkeep, // transform
        description: "transform".into(),
    },
    TriggeredAbilityDef {
        kind: TriggerKind::AnyCreatureAttacks,
        description: "attacking creatures you control get +1/+0".into(),
    },
],
...
fn on_any_creature_attacks(&self, state: &mut GameState, self_id: ObjectId,
                           attacker_id: ObjectId, attacker_controller: PlayerId, ...) {
    let (controller, is_transformed) = ...;
    if attacker_controller != controller { return; }
    let bonus = if is_transformed { 3 } else { 1 };
    state.until_end_of_turn.push(
        crate::state::TemporaryEffect::ModifyPT {
            target: attacker_id,
            power_mod: bonus,
            toughness_mod: 0,
        }
    );
}
```

This snapshots the buff once per attacker at the declare-attackers
step. The buff then sits in `state.until_end_of_turn` until end of
turn. Wrong in several ways:

1. **Buff persists after Instigator Gang leaves the battlefield.**
   If Instigator Gang is destroyed by first-strike damage before the
   normal combat damage step, the attackers keep their +1/+0 through
   normal damage. Per oracle, the buff should turn off the moment
   Instigator Gang dies — it's a static ability depending on its
   source being on the battlefield. Same problem if Instigator Gang
   is removed mid-combat by Silent Departure, Blasphemous Act
   resolving post-attack-declaration, a targeted removal instant, etc.
   — attackers keep the bonus instead of losing it. This is a direct
   parallel to Bug B (Human bonus is a snapshot) at a different scope:
   Bug B is an attached-equipment static effect; this is a global
   static effect emitted by a permanent-on-the-battlefield.
2. **Creatures entering combat after the trigger fired don't get the
   buff.** No in-set way for creatures to enter attacking after the
   `DeclareAttackers` step in ISD. But Cackling Counterpart (which
   was in the audit pool) producing a token copy of an attacking
   creature mid-combat would NOT trigger this handler for the token
   — the token didn't go through `DeclareAttackers` — so the copy
   wouldn't get the +1/+0 even though it's "a creature you control
   that is attacking" per oracle.
3. **Instigator Gang entering the battlefield mid-combat** (via any
   flash or surprise effect) doesn't retroactively buff already-declared
   attackers. Latent in ISD (no flash enchant/creature that does this).

Compare with **Full Moon's Rise** (`cards/isd/full_moons_rise.rs:28-44`),
which correctly implements its static "Werewolf creatures you control
get +1/+0 and have trample" via `continuous_effects` +
`ContinuousEffect::ModifyPT` with `EffectScope::Global(CreatureFilter::And([
ControlledByYou, HasSubtype("Werewolf")]))`. Full Moon's Rise's buff
is continuous — it correctly turns off the moment the enchantment
leaves, and correctly applies to creatures entering mid-turn.
Instigator Gang should follow the same shape with a different filter.

Note: the current implementation gives the bonus to Instigator Gang /
Wildblood Pack *itself* when it attacks (`on_any_creature_attacks`
fires with `attacker_id == self_id` without being excluded). Oracle
agrees with this ("attacking creatures you control" includes
Instigator Gang when it's attacking), so the value is correct, but
for the wrong reason (the trigger fires on self-attack rather than
the continuous filter evaluating "self is currently attacking").

**Did NOT fire** in audit — Instigator Gang appeared in a drafter's
pool but I did not find a game sample where the snapshot-vs-continuous
distinction changed a combat outcome. Latent but structurally present.

**Proposed fix:** delete the `AnyCreatureAttacks` triggered ability
and the `on_any_creature_attacks` handler. Replace with a continuous
effect in `card_data()`:
```rust
continuous_effects: vec![
    ContinuousEffect::ModifyPT {
        power: 1, toughness: 0,
        scope: EffectScope::Global(CreatureFilter::And(vec![
            CreatureFilter::ControlledByYou,
            CreatureFilter::Attacking,
        ])),
    },
],
```
and override the back-face's `continuous_effects` in `back_face_data`
to use `power: 3`. The face-aware continuous-effect machinery would
need to pick up the back-face's `continuous_effects` when
`obj.is_transformed` — check whether that already works for DFC
back-face continuous effects (a separate machinery question from
this card).

This requires a new `CreatureFilter::Attacking` variant in
`matches_filter` (`state.rs:689-790`). Implementation:
`state.combat_state.as_ref().map(|c| c.attacker_ids.contains(&obj.id))`
or the equivalent — there's already an attackers list on the combat
state that tracks live attackers.

Instigator Gang is the only card in ISD with a "creatures you control
that are attacking get +X/+X" static anthem. Adding
`CreatureFilter::Attacking` would unblock this card alone for the ISD
set; other sets would reuse the predicate.

<!-- Reserving letters BP-BT for this branch (5 letters past BK on master). -->

---

### 🟡 Engine Bug BP: Forced-attack effects ignore "can't attack" continuous effects (Furor of the Bitten + Bonds of Faith interaction)
**Severity:** medium — latent in audit (no overlap between forced-attack sources and PreventAttack sources in the sampled games)
**File:** `mtg-engine/src/engine.rs:2381-2407` (forced attacker enumeration in `DeclareAttackers` handler)

After attackers are declared, the engine walks the active player's
creatures and collects any that should be "forced to attack" via a
continuous effect with `ContinuousEffect::ForceAttack`:

```rust
for creature in new_state.objects.values() {
    if creature.zone != Zone::Battlefield || creature.controller != active
        || creature.power.is_none() || creature.tapped || creature.summoning_sick {
        continue;
    }
    if new_state.combat.as_ref().map(|c| c.attackers.contains_key(&creature.id)).unwrap_or(false) {
        continue; // already attacking
    }
    // Check for Defender — can't be forced to attack.
    if new_state.has_keyword(creature.id, crate::types::Keyword::Defender, registry) {
        continue;
    }
    // Check for forced attack effects (e.g., Furor of the Bitten).
    let must_attack = new_state.has_continuous_effect(creature.id, ...ForceAttack...);
    if must_attack {
        forced.push(creature.id);
    }
}
```

The exclusion list is: zone≠Battlefield, not a creature, wrong
controller, tapped, summoning sick, already attacking, has Defender.
It does **not** call `state.can_attack(creature_id, registry)` or
otherwise consult `PreventAttack` / `ConditionalPreventAttack`
continuous effects. Those are the effects used by Bonds of Faith
(`cards/isd/bonds_of_faith.rs:37-45`, `ConditionalPreventAttack` when
attached creature is non-Human) and potentially Pacifism-shaped
effects in other sets.

**Consequence:** a creature enchanted with Bonds of Faith that is NOT
a Human AND is ALSO the target of Furor of the Bitten, Curse of the
Nightly Hunt, or Galvanic Juggernaut's forced-attack effect will be
forced to attack despite Bonds of Faith's rules text saying "it can't
attack or block." The engine inserts the creature into
`combat.attackers` and taps it, producing a nonsense attack that the
opponent can block freely. The "if able" clause of Furor's oracle
text means the force should not apply when the creature can't attack.

A concrete repro recipe in ISD:
1. Opponent controls a Werewolf (non-Human, transformed).
2. You cast Bonds of Faith on the Werewolf (it's non-Human, so Bonds
   applies the PreventAttack clause — "it can't attack or block").
3. You cast Furor of the Bitten on the same Werewolf (under your
   control? No — Furor is "enchanted creature", so opponent's
   Werewolf is enchanted by the aura you cast on it; Furor makes it
   "attack each combat if able").
4. On opponent's attack step, the engine's forced-attack loop skips
   the creature's Defender check (not a defender), skips the tapped
   check (not tapped yet), skips the summoning-sick check, and sees
   the ForceAttack continuous effect is active. It inserts the
   creature into `combat.attackers` even though Bonds of Faith's
   `ConditionalPreventAttack` should lock it down.

**Did NOT fire** in audit — Furor of the Bitten, Curse of the
Nightly Hunt, and Galvanic Juggernaut appear in the v5 audit log
only in the deck-builder card knowledge dump; none were cast. Bonds
of Faith was cast many times but never in combination with a force-
attack source. Pure latent bug.

**Cards affected by the bug pattern (ISD):**
- `cards/isd/furor_of_the_bitten.rs:27` (Attached)
- `cards/isd/curse_of_the_nightly_hunt.rs:30` (Global)
- `cards/isd/galvanic_juggernaut.rs:28` (OnSelf — Juggernaut forces
  itself to attack; lockable by Bonds of Faith only if Juggernaut
  were somehow enchanted, which isn't normal)
- `cards/isd/hanweir_watchkeep.rs:62` (OnSelf — back-face Wildblood
  Pack)
- `cards/isd/bloodcrazed_neonate.rs:28` (OnSelf)

The OnSelf cases are lower-impact — a creature forced to attack
*itself* via OnSelf can only conflict with a PreventAttack on the
same creature, which is rare. The broader-scope cases (Furor of the
Bitten on Attached, Curse of the Nightly Hunt on Global) are where
the bug most naturally manifests.

**Proposed fix:** add a `state.can_attack(creature.id, registry)`
check next to the Defender exclusion:
```rust
if new_state.has_keyword(creature.id, Keyword::Defender, registry)
    || !new_state.can_attack(creature.id, registry) {
    continue;
}
```
`can_attack` already consults both `PreventAttack` and
`ConditionalPreventAttack` (see `state.rs:989-1005`) and returns
false when either applies. The same logic should gate the
"legal_attackers" enumeration earlier in the declare-attackers code
path so that manually-declared attackers respect the same "if able"
semantics — but that path already uses `can_attack`, only the
forced-attack path is missing it.

---

### 🟡 Engine Bug BQ: "Any target" damage cannot target planeswalkers — affects Brimstone Volley, Devil's Play, Geistflame, Skirsdag Cultist, Blazing Torch, Heretic's Punishment
**Severity:** medium — latent in audit (no planeswalker drafted) but structural
**File:** `mtg-engine/src/engine.rs:1890-1906` (valid_targets_for_req / generate_ability_targets for `TargetRequirement::AnyTarget`) and `mtg-engine/src/cards/helpers.rs:49-80` (resolve_damage)

Per CR 115.4a, "any target" is the modern oracle phrasing that means
"any creature, player, planeswalker, or battle." Every ISD damage
card that deals "N damage to any target" should be able to point its
damage at a planeswalker.

The engine's target enumeration for `TargetRequirement::AnyTarget`:

```rust
TargetRequirement::AnyTarget => {
    let mut targets: Vec<Target> = state.all_objects_in_zone(Zone::Battlefield).iter()
        .filter(|o| o.power.is_some())  // <-- this filter excludes planeswalkers
        .filter(|o| can_be_targeted(state, o.id, controller, registry))
        .map(|o| Target::Object(o.id))
        .filter(|t| behavior.is_valid_target(state, controller, t, registry))
        .collect();
    for p in &state.players { ... }
    targets
}
```

Planeswalkers have `obj.power = None` in the registry, so `o.power.is_some()`
filters them out. The resulting target list contains only creatures and
players. ISD cards using `AnyTarget`:

- `cards/isd/brimstone_volley.rs:31` — `Brimstone Volley` "deals 3 damage to any target" (oracle: "any target")
- `cards/isd/devils_play.rs` — `Devil's Play` "deals X damage to any target"
- `cards/isd/geistflame.rs` — `Geistflame` "deals 1 damage to any target"
- `cards/isd/skirsdag_cultist.rs:40` — Cultist ability "deals 2 damage to any target"
- `cards/isd/blazing_torch.rs` — Torch activated ability "deals 2 damage to any target"
- `cards/isd/heretics_punishment.rs` — "deals 5 damage to any target" per discard

Garruk Relentless and Liliana of the Veil are the two ISD
planeswalkers. Neither is targetable by any of the six cards above
in the current implementation; the "any target" prompt only lists
creatures + players.

**Related (likely deserves its own entry, tracked here as BQ-2):**
the damage-application helpers also don't correctly handle
planeswalker targets even if they were enumerated. `resolve_damage`
(`cards/helpers.rs:49-80`) blindly does `obj.damage_marked += amount`
for any `Target::Object`, and `obj.damaged_by.push(spell_id)`. For a
planeswalker this writes to `damage_marked` instead of decrementing
the `CounterType::Loyalty` counters. The engine DOES have a
planeswalker-aware branch at `engine.rs:2856-2867` (used by the
central `PendingEffect::DealDamage` path), but `resolve_damage`
bypasses it. Skirsdag Cultist's `on_activate_ability` has its own
inline damage path (`cards/isd/skirsdag_cultist.rs:49-81`) which
also bypasses the central damage helper and directly writes
`damage_marked`, with the same bug.

**Did NOT fire** in audit — Garruk Relentless and Liliana were
listed in the ISD card database (log lines ~3300, ~5000) but
neither was drafted, so the "any target" → planeswalker path was
never exercised.

**Proposed fix (two-part):**
1. In `valid_targets_for_req`'s `AnyTarget` arm, replace the
   `o.power.is_some()` filter with:
   ```rust
   let is_damageable = o.power.is_some()  // creature
       || o.card_types.contains(&CardType::Planeswalker)
       || registry.card_data(o.card_id)
           .map(|d| d.card_types.contains(&CardType::Planeswalker))
           .unwrap_or(false);
   ```
   Apply the same change to the `generate_ability_targets` copy at
   `engine.rs:1890`.
2. In `cards/helpers.rs::resolve_damage`, gate the
   `obj.damage_marked +=` branch on creature-vs-planeswalker:
   ```rust
   let is_planeswalker = registry.card_data(obj.card_id)
       .map(|d| d.card_types.contains(&CardType::Planeswalker))
       .unwrap_or(false) || obj.card_types.contains(&CardType::Planeswalker);
   if is_planeswalker {
       let loyalty = obj.counters.entry(CounterType::Loyalty).or_insert(0);
       *loyalty = loyalty.saturating_sub(amount);
   } else {
       obj.damage_marked += amount;
   }
   ```
3. Skirsdag Cultist's inline damage path should be refactored to
   call `resolve_damage` / the central damage helper once the
   helper is planeswalker-aware. Same for Olivia Voldaren's first
   ability (Bug AJ-adjacent — inline `obj.damage_marked += 1` at
   `olivia_voldaren.rs:104-106`), Curse of the Pierced Heart's
   direct life-subtract (`curse_of_the_pierced_heart.rs:72-78`), and
   anywhere else that bypasses the central helper.

---

### 🟡 Engine Bug BR: Olivia Voldaren's +1/+1-bite and Curse of the Pierced Heart's damage bypass the central damage helper
**Severity:** low-medium — silently wrong vs lifelink / protection / replacement / double-damage effects
**Files:** `mtg-engine/src/cards/isd/olivia_voldaren.rs:104-123` and `mtg-engine/src/cards/isd/curse_of_the_pierced_heart.rs:70-82`

Both cards deal damage as part of an ability effect, but bypass the
central `PendingEffect::DealDamage` path (which lives at
`engine.rs:2854-2877` and handles protection, planeswalker
redirection, life-subtract, and `damaged_by` bookkeeping). Instead
they inline:

- **Olivia {1}{R}**: `obj.damage_marked += 1; obj.damaged_by.push(self_id);`
  Doesn't check protection, doesn't handle planeswalker loyalty
  (Olivia's target-filter also doesn't exclude planeswalkers via
  `Another`, so in theory she can "deal 1 damage" to an opposing
  Garruk Relentless and increment `damage_marked` instead of
  decrementing loyalty). Doesn't fire lifelink if Olivia somehow
  has lifelink via Bonds of Faith / Mentor of the Meek / etc.
- **Curse of the Pierced Heart**: `state.get_player_mut(cursed_player).life = old - 1;`
  Directly subtracts life without going through the damage pipeline.
  Missing: protection checks, prevention effects (none in ISD, but
  structurally wrong), damage-replacement effects, and any
  lifelink-from-source interaction.

The problems are mostly latent in ISD because:
- Nothing in ISD gives Olivia or an enchantment a temporary
  lifelink.
- There are no in-set damage-doubling effects that would multiply
  the 1 damage.
- The only planeswalkers are Garruk and Liliana, and neither is
  likely to be on opp's side when Olivia activates.

**Proposed fix:** route both cards through the central damage
helper. Olivia's first ability should queue a `PendingEffect::DealDamage`
with `source = olivia_id`, `target = creature_id`, `amount = 1`.
Curse of the Pierced Heart should do the same, queuing against
`Target::Player(cursed_player)` or the chosen planeswalker. The
helper at `engine.rs:2854+` already handles both shapes.

Related to Bug BQ (damage helper needs planeswalker branch) — the
fix for Olivia is blocked on BQ being resolved first for the
non-Olivia cards, but Olivia's own bite should still route through
the unified helper regardless.

---

### 🟡 Engine Bug BT: `on_any_creature_dies` handlers zone-gate on Battlefield, silently dropping triggers when the watcher dies simultaneously with its target
**Severity:** medium — *confirmed fired in audit* (Abattoir Ghoul in mutual-first-strike trades at log lines 17678-17681 and 18086-18089, both with no life gain logged)
**Files:**
- `mtg-engine/src/cards/isd/abattoir_ghoul.rs:38-43`
- `mtg-engine/src/cards/isd/murder_of_crows.rs:38-43`
- `mtg-engine/src/cards/isd/rage_thrower.rs:38-43`
- `mtg-engine/src/cards/isd/selhoff_occultist.rs:47-54`

Four `AnyCreatureDies` watcher handlers begin with:
```rust
fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, ...) {
    let controller = match state.get_object(self_id) {
        Some(o) if o.zone == Zone::Battlefield => o.controller,
        _ => return,
    };
    ...
}
```
This pattern early-returns (silently) whenever the watcher is no
longer on the battlefield at trigger-resolution time. For effects
whose payload operates on the CONTROLLER (gain life, draw/discard,
mill a player, deal damage to a target) rather than on the source
itself, the zone gate is **wrong**: per CR 603.6d / 603.10c, a
triggered ability that was already queued continues to resolve even
if the source has since left the battlefield.

Collected triggers in `triggers.rs:472-512` explicitly include
"simultaneously dead" watchers (`state.objects.values().filter(|o|
o.id != dead_id && (o.zone == Zone::Battlefield ||
simultaneously_dead.contains(&o.id)))`). So the dispatcher correctly
queues the DeathWatch trigger even for a watcher that is
dying-at-the-same-time. But the handler at resolution kills itself
off because of the zone gate, dropping the effect.

**Confirmed firing in audit (log lines 17670-17690 and 18080-18106):**
Seat 0's Voiceless Spirit (2/1 **first strike**) blocks opp's
Abattoir Ghoul (3/2 **first strike**) on turn 10. Both have first
strike, so the first-strike damage step kills both simultaneously.
Oracle semantics:

1. First-strike damage resolves. Abattoir Ghoul deals 3 to
   Voiceless Spirit. Voiceless Spirit deals 2 to Abattoir Ghoul.
2. Abattoir Ghoul gets appended to Voiceless Spirit's `damaged_by`.
   SBA check: both die (Voiceless Spirit's 1 toughness vs 3 damage,
   Abattoir Ghoul's 2 toughness vs 2 damage).
3. Trigger: "a creature Abattoir Ghoul dealt damage to this turn
   died" → Voiceless Spirit just died, Abattoir Ghoul had damaged it.
4. DeathWatch trigger queued via the `simultaneously_dead` branch
   in `triggers.rs:482-485`.
5. Trigger resolves — Abattoir Ghoul's controller should gain 1
   life (Voiceless Spirit's toughness).
6. Handler checks zone → Ghoul is in graveyard → returns early.
7. Opp's life is unchanged.

Log excerpt from line 18086-18089:
```
Abattoir Ghoul died
Voiceless Spirit died
Typhoid Rats died
Selfless Cathar died
```
No "Abattoir Ghoul: gained N life from creature death" message
follows, even though the same message IS logged at lines 81072,
81379, 95374 when Abattoir Ghoul is alive at trigger-resolution
time (i.e., when the dying creature was blocked/attacked by Ghoul
without Ghoul itself dying).

**Other affected cards (patterns, latent in audit):**

- **Murder of Crows** ({3}{U}{U}, "Whenever another creature dies,
  you may draw a card. If you do, discard a card."). If Murder of
  Crows dies in the same combat as another creature (unblocked
  attacker trading with a 4-power blocker), the trigger is
  collected but the zone-gated handler returns early. The
  controller never gets the "may draw" prompt — no YesNo
  awaiting_action is set, so the game just proceeds. Not drafted
  into a relevant scenario in this audit.
- **Rage Thrower** ({5}{R}, "Whenever another creature dies, this
  creature deals 2 damage to target player or planeswalker."). If
  Rage Thrower is blocked and dies to the blocker, the blocker's
  death also triggers Rage Thrower's ability. Under current code,
  the handler early-returns because Rage Thrower is in graveyard.
  Per rules the 2 damage should still be dealt — the damage source
  is Rage Thrower-as-last-known-object (graveyard-object
  characteristics are preserved for damage attribution per CR 112.7a).
  Not cast in audit.
- **Selhoff Occultist** ({2}{U}, "Whenever this creature or another
  creature dies, target player mills a card."). Selhoff has two
  separate handlers: `on_dies` (correct, does not zone-gate) and
  `on_any_creature_dies` (zone-gated, BUGGY). If Selhoff dies at
  the same time as another creature, the SelfDies branch fires
  correctly via `on_dies` → 1 mill. The AnyCreatureDies branch
  (from the other creature's death) fires separately and the
  zone-gated handler drops it → missing 1 mill. Per rules, BOTH
  triggers should resolve (they're two separate triggered
  abilities, both firing on simultaneous deaths). Not drafted in a
  relevant scenario.

**Counter-example (correct implementation):** Falkenrath Noble's
`on_any_creature_dies` at `falkenrath_noble.rs:47-54` does NOT
zone-gate — it reads `o.controller` unconditionally and calls
`drain()`. This is the correct pattern for effects whose payload
operates on the controller.

**Counter-examples where zone-gating IS correct:** Lumberknot
(+1/+1 counter on self), Gutter Grime (slime counter on self),
Galvanic Juggernaut (untap self), Unruly Mob (+1/+1 counter on
self), Thraben Sentry (transform trigger), Village Cannibals
(+1/+1 counter on self). All of these either mutate `self_id`
directly or need the source on the battlefield for the effect to
mean anything — early-returning is fine (and often necessary to
avoid no-op counter adds to graveyard objects).

**Not a bug in the trigger dispatcher** — the dispatcher's
`simultaneously_dead` logic is correct per CR 603.6c. The bug lives
entirely in the card-level handlers.

**Proposed fix:** drop the `if o.zone == Zone::Battlefield` gate
in the four affected handlers, mirroring Falkenrath Noble:
```rust
fn on_any_creature_dies(&self, state: &mut GameState, self_id: ObjectId, ...) {
    let controller = match state.get_object(self_id) {
        Some(o) => o.controller,
        None => return,
    };
    ...
}
```
For Abattoir Ghoul specifically, the effect also needs to check
`dead_damaged_by.contains(&self_id)` (which it already does) — the
"dealt damage by this creature" check uses the pre-captured
damaged_by list, not the current battlefield state, so it's safe
even when Ghoul is in the graveyard.

No trigger-dispatcher or engine-level changes needed — just
per-card handler tweaks.

**Audit impact:** Bug BT fired at least twice in the audit log
(lines 17678-17681 and 18086-18089, both Abattoir Ghoul mutual
first-strike trades). Each instance cost opp 1 missed life gain.
Small enough to not have swung the games, but it's a silent
correctness bug the model cannot see.
