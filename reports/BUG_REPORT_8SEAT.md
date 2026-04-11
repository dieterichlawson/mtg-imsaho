# Bug Report: 8-Seat Verification Draft

Findings from `VERIFICATION_REPORT_8SEAT.md`, reformatted for a fix-it agent. Each bug has:

1. **Severity** and **summary**
2. **What was observed** (concrete log excerpts and counts)
3. **Why it's wrong** (with exact oracle text from `python3 scripts/oracle_lookup.py` and relevant CR references)
4. **Root cause** (file:line)
5. **Fix sketch** (code-level, not pseudocode)
6. **Verification** (unit tests + regression greps)

All line numbers below reference `verify-draft-8seat.log` in this repo unless otherwise noted.

Before fixing, read `VERIFICATION_REPORT_8SEAT.md` §4 for additional numeric context.

## General guidance

- **Always pull oracle text with `python3 scripts/oracle_lookup.py lookup "<Card Name>"`** before making any claim about what a card does. The 4-seat audit had false alarms due to stale memory. Every oracle quote in this document was pulled live from that tool — if you need to verify or extend any claim, re-run the lookup, don't trust your own memory.
- Run `cargo check && cargo test` after each bug fix.
- When in doubt about a rule, consult CR 603 (triggered abilities), 702 (keyword abilities), and 700 (general rules).
- Fix bugs one at a time and commit each independently. Most are mechanically independent.

---

## Bug A — Empty `dies` / `LTB` triggers pollute the stack

**Severity: HIGH** — correctness + LLM token waste. Fires in every game on every creature death.

### What was observed

Creatures (and **auras**, a new finding vs the 4-seat run) without a self-dies or leaves-battlefield handler still cause the engine to push an empty trigger onto the stack whenever they leave the battlefield. Every empty trigger becomes a `[RESPOND TO ...]` prompt the LLM has to pass through.

**Raw counts from the 8-seat log:**

```
grep -c "'s dies trigger" verify-draft-8seat.log      # 57
grep -c "'s LTB trigger"  verify-draft-8seat.log      # 78
grep -c "RESPOND TO.*trigger" verify-draft-8seat.log  # 149
```

**Sample stack with 4 empty triggers + 1 real trigger** (line 56076):

```
Stack: Falkenrath Noble's triggered ability (target player loses 1 life, you gain 1 life) (your),
       Diregraf Ghoul's dies trigger (your),
       Civilized Scholar's LTB trigger (opp's),
       Civilized Scholar's dies trigger (opp's),
       Diregraf Ghoul's LTB trigger (opp's)
```

Only the Falkenrath Noble line is a real trigger. The rest are empty.

**Sample empty aura LTB trigger** (line 94077), after Naturalize destroyed Bonds of Faith:

```
Stack: Bonds of Faith's LTB trigger (opp's)
[RESPOND TO p255's Bonds of Faith's LTB trigger]
```

### Oracle verification of offenders

All of the following cards are confirmed by `scripts/oracle_lookup.py lookup` to have **no** "when this creature dies" or "when this [creature/aura] leaves the battlefield" clause, yet each fires empty `dies` and/or `LTB` triggers in the log:

| Card | Oracle text (verbatim) | Has real self-dies? | Has real LTB? |
|---|---|---|---|
| **Typhoid Rats** | `Deathtouch` | no | no |
| **Ambush Viper** | `Flash\nDeathtouch` | no | no |
| **Diregraf Ghoul** | `This creature enters tapped.` | no | no |
| **Walking Corpse** | *(empty oracle text — vanilla)* | no | no |
| **Fortress Crab** | *(empty oracle text — vanilla)* | no | no |
| **Boneyard Wurm** | `Boneyard Wurm's power and toughness are each equal to the number of creature cards in your graveyard.` | no | no |
| **Markov Patrician** | `Lifelink (Damage dealt by this creature also causes you to gain that much life.)` | no | no |
| **Orchard Spirit** | `This creature can't be blocked except by creatures with flying or reach.` | no | no |
| **Mindshrieker** | `Flying\n{2}: Target player mills a card. This creature gets +X/+X until end of turn, where X is the milled card's mana value.` | no | no |
| **Crossway Vampire** | `When this creature enters, target creature can't block this turn.` | no (ETB only) | no |
| **Slayer of the Wicked** | `When this creature enters, you may destroy target Vampire, Werewolf, or Zombie.` | no (ETB only) | no |
| **Armored Skaab** | `When this creature enters, mill four cards.` | no (ETB only) | no |
| **Ghoulraiser** | `When this creature enters, return a Zombie card at random from your graveyard to your hand.` | no (ETB only) | no |
| **Civilized Scholar** | `{T}: Draw a card, then discard a card. If a creature card is discarded this way, untap this creature, then transform it.` | no (activated ability only) | no |
| **Delver of Secrets** | `At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform this creature.` | no (upkeep trigger only) | no |
| **Skirsdag Cultist** | `{R}, {T}, Sacrifice a creature: This creature deals 2 damage to any target.` | no (activated ability only) | no |
| **Deranged Assistant** | `{T}, Mill a card: Add {C}.` | no (mana ability only) | no |
| **Evil Twin** | `You may have this creature enter as a copy of any creature on the battlefield, except it has "{U}{B}, {T}: Destroy target creature with the same name as this creature."` | no (ETB only) | no |
| **Tormented Pariah** | `At the beginning of each upkeep, if no spells were cast last turn, transform this creature.` (werewolf DFC, no dies/LTB) | no | no |
| **Fiend Hunter** | `When this creature enters, you may exile another target creature.\nWhen this creature leaves the battlefield, return the exiled card to the battlefield under its owner's control.` | no | **YES (real LTB)** |
| **Bonds of Faith** *(aura)* | `Enchant creature\nEnchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.` | n/a | no |
| **Dead Weight** *(aura)* | `Enchant creature\nEnchanted creature gets -2/-2.` | n/a | no |
| **Sensory Deprivation** *(aura)* | `Enchant creature\nEnchanted creature gets -3/-0.` | n/a | no |
| **Spectral Flight** *(aura)* | `Enchant creature\nEnchanted creature gets +2/+2 and has flying.` | n/a | no |

**Important nuance — cards with "Whenever this creature or another creature dies":**

Several cards in the log produce stack lines that *look* like "Card's dies trigger" but actually have real triggered abilities on self-death per oracle. These are listed below. Per CR 603.10a, "Whenever this creature or another creature dies" is a single trigger condition that **does** trigger when the creature itself dies. Depending on how the engine implements these (as `SelfDies` or as `AnyCreatureDies`/`DeathWatch`), they may or may not be incorrectly firing an empty `SelfDies` trigger in addition to their real ability:

| Card | Oracle text (verbatim) | Fires on self-death? |
|---|---|---|
| **Falkenrath Noble** | `Flying\nWhenever this creature or another creature dies, target player loses 1 life and you gain 1 life.` | YES (per oracle, "this creature or another") |
| **Selhoff Occultist** | `Whenever this creature or another creature dies, target player mills a card.` | YES |
| **Abattoir Ghoul** | `First strike\nWhenever a creature dealt damage by this creature this turn dies, you gain life equal to that creature's toughness.` | NO — "a creature **dealt damage by this creature**", does not include the self-death case unless Abattoir Ghoul damaged itself |
| **Murder of Crows** | `Flying\nWhenever another creature dies, you may draw a card. If you do, discard a card.` | NO — "another creature" excludes self |
| **Unruly Mob** | `Whenever another creature you control dies, put a +1/+1 counter on this creature.` | NO — "another creature" excludes self |

For Falkenrath Noble and Selhoff Occultist, if the engine implements the trigger as `AnyCreatureDies` (DeathWatch), then a `SelfDies` variant should still fire per oracle — the engine may be erroneously eliding it, or may be redundantly firing both. **During the fix, audit each card's `triggered_abilities` vec against oracle** and make sure the real triggers are preserved.

Abattoir Ghoul, Murder of Crows, and Unruly Mob's abilities correctly should NOT fire on their own death per oracle (oracle says "another creature" or "a creature dealt damage by this creature"). If the log shows `Abattoir Ghoul's dies trigger` firing when Abattoir Ghoul itself dies with no other simultaneous death, that's a pure bug A instance (empty trigger).

### Root cause

Two sites in `mtg-engine/src/triggers.rs`:

**Site 1 — `SelfDies` at triggers.rs:409-422:**

```rust
GameEvent::CreatureDied { object, card_id, controller, damaged_by, last_known_toughness } => {
    ...
    // 1. Self-dies trigger.
    if registry.get(dead_card_id).is_some() {
        let desc = trigger_description(registry, dead_card_id, &crate::cards::TriggerKind::SelfDies, false);
        let trigger = PendingTrigger::SelfDies { ... };
        if dead_controller == active_player { ap_triggers.push(trigger); }
        else { nap_triggers.push(trigger); }
    }
```

Gates only on "registry knows this card", not "card has a dies handler". Every creature death creates a SelfDies trigger.

**Site 2 — `LeftBattlefield` at triggers.rs:466-480:**

```rust
GameEvent::LeftBattlefield { object, .. } => {
    let (card_id,) = match state.get_object(*object) { ... };
    ...
    if registry.get(card_id).is_some() {
        let desc = trigger_description(...);
        let trigger = PendingTrigger::LeftBattlefield { ... };
        ap_triggers.push(trigger);
    }
}
```

Same pattern, fires for **any** permanent leaving the battlefield (creature or not), which is why auras like Bonds of Faith produce empty LTB triggers.

The analogous ETB fix already exists at `triggers.rs:356`:

```rust
// EnteredBattlefield handling at triggers.rs:344-370 (reference)
if let Some(behavior) = registry.get(card_id) {
    if behavior.has_etb_handler() {
        let trigger = PendingTrigger::EnteredBattlefield { ... };
        ...
    }
}
```

### Fix

**Step 1.** Add two methods to the `CardBehavior` trait in `mtg-engine/src/cards/mod.rs` (near `has_etb_handler` at line 321):

```rust
/// True if this card has a "when this creature dies" triggered ability (CR 603.6c).
/// Used to gate empty trigger creation at `triggers.rs` SelfDies site.
///
/// Note: this is for *self*-dies only. Cards with "Whenever this creature OR
/// another creature dies" (Falkenrath Noble, Selhoff Occultist) should also
/// return `true` since their ability triggers on their own death per oracle.
/// Cards with "Whenever ANOTHER creature dies" (Murder of Crows, Unruly Mob)
/// should return `false` — their death-watch is handled by the separate
/// `AnyCreatureDies` trigger path at triggers.rs:434.
fn has_dies_handler(&self) -> bool { false }

/// True if this card has a "when this [permanent] leaves the battlefield"
/// triggered ability (CR 603.6b). Used to gate empty trigger creation at
/// `triggers.rs` LeftBattlefield site. Applies to creatures AND auras /
/// other permanent types.
fn has_ltb_handler(&self) -> bool { false }
```

**Step 2.** Gate both trigger-creation sites:

```rust
// triggers.rs SelfDies (around line 409):
if let Some(behavior) = registry.get(dead_card_id) {
    if behavior.has_dies_handler() {
        let desc = trigger_description(registry, dead_card_id, &crate::cards::TriggerKind::SelfDies, false);
        let trigger = PendingTrigger::SelfDies { ... };
        if dead_controller == active_player {
            ap_triggers.push(trigger);
        } else {
            nap_triggers.push(trigger);
        }
    }
}

// triggers.rs LeftBattlefield (around line 471):
if let Some(behavior) = registry.get(card_id) {
    if behavior.has_ltb_handler() {
        let desc = trigger_description(...);
        let trigger = PendingTrigger::LeftBattlefield { ... };
        // See Bug C for the correct AP/NAP bucket placement once the controller is fixed.
        ap_triggers.push(trigger);
    }
}
```

**Step 3.** Override the defaults to `true` on cards that genuinely have the handlers. Based on the oracle sweep above, the ISD cards I identified that have real handlers:

- **`has_dies_handler = true`** on:
  - `doomed_traveler.rs` — oracle: `When this creature dies, create a 1/1 white Spirit creature token with flying.`
  - `mausoleum_guard.rs` — oracle: `When this creature dies, create two 1/1 white Spirit creature tokens with flying.`
  - `elder_cathar.rs` — oracle: `When this creature dies, put a +1/+1 counter on target creature you control. If that creature is a Human, put two +1/+1 counters on it instead.`
  - **Falkenrath Noble** if its engine impl uses `SelfDies` for the "this creature" part of "this creature or another creature dies" — check `cards/isd/falkenrath_noble.rs` and compare against oracle. If it's implemented purely as `AnyCreatureDies`/DeathWatch, then leave `has_dies_handler = false` but add an explicit test that the ability fires on self-death (CR 603.10a).
  - **Selhoff Occultist** — same treatment as Falkenrath Noble.

  Not in the default list but worth checking against the `cards/isd/` directory for any card with a `TriggerKind::SelfDies` entry in its `triggered_abilities` — grep:

  ```bash
  grep -rl "TriggerKind::SelfDies" mtg-engine/src/cards/isd/
  ```

  Every hit needs `has_dies_handler = true`.

- **`has_ltb_handler = true`** on:
  - `fiend_hunter.rs` — oracle: `When this creature enters, you may exile another target creature.\nWhen this creature leaves the battlefield, return the exiled card to the battlefield under its owner's control.` This is the only confirmed real LTB in the ISD pool from the 8-seat run.

  Grep to check if any other card files have `TriggerKind::LeavesBattlefield`:

  ```bash
  grep -rl "TriggerKind::LeavesBattlefield" mtg-engine/src/cards/isd/
  ```

**Step 4 (optional consistency check).** Add a runtime assertion or unit test that iterates the registry and verifies `has_dies_handler()` is true iff the card's `triggered_abilities` vec contains a `SelfDies` entry (same for LTB). This catches drift when new cards are added.

### Verification

**Unit tests** (new file `mtg-engine/tests/empty_triggers.rs`):

```rust
//! Regression tests for the bug where cards without self-dies or LTB handlers
//! produce empty triggers on the stack.
//! See VERIFICATION_REPORT_8SEAT.md §4.1 and BUG_REPORT_8SEAT.md Bug A.

use mtg_engine::*;
mod common;
use common::*;

#[test]
fn typhoid_rats_dying_does_not_create_empty_dies_trigger() {
    // Oracle: Typhoid Rats has only "Deathtouch". No dies or LTB handler.
    let (mut state, registry, p0, _) = setup_game(...);
    let rats = cast_and_resolve(&mut state, p0, "Typhoid Rats", &registry);
    assert!(state.stack.is_empty(), "no triggers before death");

    destroy_creature(&mut state, rats, &registry);
    engine::resolve_state_based_actions(&mut state, &registry);

    assert_eq!(state.stack.len(), 0,
        "Typhoid Rats has no dies/LTB handler per oracle; stack should be empty");
}

#[test]
fn bonds_of_faith_destroyed_does_not_create_empty_ltb_trigger() {
    // Oracle: Bonds of Faith has no LTB text.
    let (mut state, registry, p0, p1) = setup_game(...);
    let creature = cast_and_resolve(&mut state, p0, "Grizzly Bears", &registry);
    cast_targeting(&mut state, p0, "Bonds of Faith", creature, &registry);
    let _naturalize = cast_targeting(&mut state, p1, "Naturalize", /* bonds */, &registry);
    engine::resolve_state_based_actions(&mut state, &registry);

    assert_eq!(state.stack.len(), 0,
        "Bonds of Faith is an aura with no LTB trigger per oracle; stack should be empty after Naturalize resolves");
}

#[test]
fn fortress_crab_dying_no_triggers() {
    // Oracle: Fortress Crab has empty oracle text (vanilla 1/6).
    let (mut state, registry, p0, _) = setup_game(...);
    let crab = cast_and_resolve(&mut state, p0, "Fortress Crab", &registry);
    destroy_creature(&mut state, crab, &registry);
    engine::resolve_state_based_actions(&mut state, &registry);
    assert!(state.stack.is_empty(), "Fortress Crab is vanilla — no triggers");
}

#[test]
fn fiend_hunter_ltb_trigger_still_fires() {
    // Regression the other way: Fiend Hunter's LTB trigger MUST still fire.
    // Oracle: "When this creature leaves the battlefield, return the exiled card..."
    let (mut state, registry, p0, p1) = setup_game(...);
    let enemy = cast_and_resolve(&mut state, p1, "Grizzly Bears", &registry);
    let hunter = cast_targeting(&mut state, p0, "Fiend Hunter", enemy, &registry);
    engine::resolve_stack(&mut state, &registry);
    assert_eq!(state.zone_count(Zone::Exile), 1);

    destroy_creature(&mut state, hunter, &registry);
    engine::resolve_state_based_actions(&mut state, &registry);

    // Hunter's LTB trigger should be on the stack.
    assert!(state.pending_triggers.iter().any(|t|
        matches!(t, PendingTrigger::LeftBattlefield { .. })
    ), "Fiend Hunter's LTB trigger should fire");

    engine::resolve_stack(&mut state, &registry);
    // The exiled creature should be back on the battlefield.
    assert!(state.get_object(enemy).unwrap().zone == Zone::Battlefield);
}

#[test]
fn doomed_traveler_dies_trigger_still_creates_spirit() {
    // Regression the other way: Doomed Traveler's dies trigger MUST still fire.
    // Oracle: "When this creature dies, create a 1/1 white Spirit creature token with flying."
    let (mut state, registry, p0, _) = setup_game(...);
    let traveler = cast_and_resolve(&mut state, p0, "Doomed Traveler", &registry);
    destroy_creature(&mut state, traveler, &registry);
    engine::resolve_stack(&mut state, &registry);

    let spirits: Vec<_> = state.all_objects_in_zone(Zone::Battlefield)
        .iter()
        .filter(|o| o.name == "Spirit")
        .collect();
    assert_eq!(spirits.len(), 1, "Doomed Traveler's dies trigger should create 1 Spirit");
    // Verify the token has flying.
    let keyword_set = state.has_keyword(spirits[0].id, Keyword::Flying, &registry);
    assert!(keyword_set);
}

#[test]
fn elder_cathar_dies_gives_counter_to_human() {
    // Oracle: "When this creature dies, put a +1/+1 counter on target creature you control.
    //          If that creature is a Human, put two +1/+1 counters on it instead."
    let (mut state, registry, p0, _) = setup_game(...);
    let human_target = cast_and_resolve(&mut state, p0, "Grizzly Bears", &registry); // non-human as control
    let cathar = cast_and_resolve(&mut state, p0, "Elder Cathar", &registry);
    destroy_creature(&mut state, cathar, &registry);
    engine::resolve_stack_targeting(&mut state, human_target, &registry);

    let counters = state.get_object(human_target).unwrap().counters
        .get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0);
    assert_eq!(counters, 1, "non-Human target gets 1 counter");
}
```

**Log regression check after the next 8-seat run:**

```bash
# Expect drastically reduced counts. The only remaining trigger lines should
# be from cards with real handlers: Doomed Traveler, Mausoleum Guard, Elder Cathar,
# Fiend Hunter (LTB), and any 'Whenever this or another creature dies' cards.
grep -c "'s dies trigger" verify-draft-8seat-v2.log   # expect ~10-15 (only real handlers)
grep -c "'s LTB trigger"  verify-draft-8seat-v2.log   # expect ~5 (Fiend Hunter only)
grep "'s dies trigger" verify-draft-8seat-v2.log | sort -u  # should only show real handlers
```

**Before the fix:** 57 dies + 78 LTB raw mentions (with ~29 unique offending cards).
**After the fix:** expect a small handful, all verifiable against oracle as legitimate.

---

## Bug B — Transformed creature display shows front-face name

**Severity: MEDIUM** — harness presentation. ~636 buggy display lines across the 8-seat log. The LLM mostly works around it but the prompt actively lies.

### What was observed

The compact board display for a transformed DFC creature shows the **front-face name** together with the **back-face stats** (and, because effective_power/toughness and keywords come from a different code path, the back-face keywords too).

The event log is correct — it shows the back-face name. For example, at line 15252:

```
p0 declared attackers: Terror of Kruin Pass (#12)   ← correct back-face name
```

Right next to that at line 15258, the board display shows:

```
Opp board: 2x Mountain (tapped), 1x Swamp (tapped), Kruin Outlaw 3/3 double strike, menace [T]
```

— where `Kruin Outlaw` is the front-face name and `3/3 double strike, menace` is the back-face stats and keywords.

**Raw counts from the 8-seat log** (grepping for `<front-face-name> <back-face-P/T>` combinations that can't exist naturally):

| Grep pattern | Count | What it is |
|---|---|---|
| `Delver of Secrets 3/2 flying` | 219 | Insectile Aberration 3/2 flying with front name |
| `Reckless Waif 3/2` | 147 | Merciless Predator 3/2 with front name |
| `Gatstaf Shepherd 3/3` | 84 | Gatstaf Howler 3/3 intimidate with front name |
| `Kruin Outlaw 3/3` | 69 | Terror of Kruin Pass 3/3 double strike, menace with front name |
| `Villagers of Estwald 4/6` | 57 | Howlpack of Estwald 4/6 with front name |
| `Thraben Sentry 5/4` | 31 | Thraben Militia 5/4 trample with front name |
| `Civilized Scholar 5/1` | 29 | Homicidal Brute 5/1 with front name |

**Total: 636 confirmed buggy display lines.** (Excludes cases like `Delver of Secrets 3/3 flying` which can legitimately be front face 1/1 + Spectral Flight +2/+2 = 3/3 flying — those are not bug B.)

### Oracle verification of front/back face stats

| Card | Front P/T (oracle) | Back face | Back P/T (oracle) | Back keywords/abilities from oracle |
|---|---|---|---|---|
| Delver of Secrets | `1/1` Human Wizard | Insectile Aberration | `3/2` Human Insect | `Flying` |
| Reckless Waif | `1/1` Human Rogue Werewolf | Merciless Predator | `3/2` Werewolf | (no keywords; back-to-front transform trigger only) |
| Gatstaf Shepherd | `2/2` Human Werewolf | Gatstaf Howler | `3/3` Werewolf | `Intimidate` |
| Kruin Outlaw | `2/2` Human Rogue Werewolf (has `First strike`) | Terror of Kruin Pass | `3/3` Werewolf | `Double strike` + "Werewolves you control have menace" |
| Villagers of Estwald | `2/3` Human Werewolf | Howlpack of Estwald | `4/6` Werewolf | (none) |
| Thraben Sentry | `2/2` Human Soldier (has `Vigilance`) | Thraben Militia | `5/4` Human Soldier | `Trample` |
| Civilized Scholar | `0/1` Human Advisor | Homicidal Brute | `5/1` Human Mutant | (plus end-step transform back trigger — see Bug E) |
| Tormented Pariah | `3/2` Human Warrior Werewolf | Rampaging Werewolf | `6/4` Werewolf | (none) |

### Why the display is wrong

The compact display builder at `mtg-player/src/llm.rs::format_creature_inline` reads `PermanentView::name`, which is built in `mtg-engine/src/view.rs:137`:

```rust
name: registry.card_data(obj.card_id)
    .map(|d| d.name)
    .unwrap_or_else(|| obj.name.clone()),
```

`registry.card_data(obj.card_id)` always returns the **front face** `CardData`. Because known cards always return `Some`, the `unwrap_or_else` fallback (which would use the correctly-updated `obj.name`) is never hit.

Meanwhile the P/T is pulled via `effective_power`/`effective_toughness`, which for werewolves goes through each card's `dynamic_pt()` override (e.g. `villagers_of_estwald.rs:70-76` returns `(4, 6)` when transformed). This means P/T is correct but the name and (see Bug D) subtypes are stale.

### Root cause

`mtg-engine/src/view.rs:137` and the analogous stack-item builder at `view.rs:179`.

### Fix

Rewrite `view.rs:134-155` to branch on `obj.is_transformed`:

```rust
let name = {
    let behavior = registry.get(obj.card_id);
    if obj.is_transformed {
        behavior
            .and_then(|b| b.back_face_data())
            .map(|d| d.name)
            .unwrap_or_else(|| obj.name.clone())
    } else {
        behavior
            .map(|b| b.card_data().name)
            .unwrap_or_else(|| obj.name.clone())
    }
};

let card_types = if obj.is_transformed {
    registry.get(obj.card_id)
        .and_then(|b| b.back_face_data())
        .map(|d| d.card_types)
        .unwrap_or_else(|| obj.card_types.clone())
} else {
    registry.card_data(obj.card_id)
        .map(|d| d.card_types)
        .unwrap_or_else(|| obj.card_types.clone())
};

PermanentView {
    object_id: obj.id,
    card_id: obj.card_id,
    name,
    card_types,
    ...
};
```

Apply the same treatment to `StackItemView::name` at `view.rs:179`.

**Do not change** the `obj.name` update path in card-specific `on_upkeep` code — those already update `obj.name` correctly, and event log lines like `p1 declared attackers: Terror of Kruin Pass (#12)` rely on it. Only the `view.rs` lookup is wrong.

### Verification

**Unit tests** (`mtg-engine/tests/transformed_display.rs`):

```rust
use mtg_engine::*;
mod common;
use common::*;

const DFC_FRONT_BACK: &[(&str, &str, i32, i32)] = &[
    ("Villagers of Estwald", "Howlpack of Estwald", 4, 6),
    ("Delver of Secrets",    "Insectile Aberration", 3, 2),
    ("Civilized Scholar",    "Homicidal Brute",      5, 1),
    ("Thraben Sentry",       "Thraben Militia",      5, 4),
    ("Kruin Outlaw",         "Terror of Kruin Pass", 3, 3),
    ("Gatstaf Shepherd",     "Gatstaf Howler",       3, 3),
    ("Reckless Waif",        "Merciless Predator",   3, 2),
    ("Tormented Pariah",     "Rampaging Werewolf",   6, 4),
];

#[test]
fn view_shows_back_face_name_and_stats_when_transformed() {
    for (front, back, back_pow, back_tough) in DFC_FRONT_BACK {
        let (mut state, registry, p0, _) = setup_game(...);
        let obj = cast_and_resolve(&mut state, p0, front, &registry);

        // Force transform by flipping is_transformed and applying the back-face data
        // (or, once Bug D's `transform_dfc` helper lands, use that instead).
        flip_to_back_face(&mut state, obj, &registry);

        let view = GameView::for_player(&state, p0, &registry);
        let perm = view.battlefield.iter().find(|p| p.object_id == obj).unwrap();
        assert_eq!(perm.name, *back, "{} transformed should display as {}", front, back);
        assert_eq!(perm.effective_power, Some(*back_pow), "{} back face power", back);
        assert_eq!(perm.effective_toughness, Some(*back_tough), "{} back face toughness", back);
    }
}

#[test]
fn view_shows_front_face_name_when_not_transformed() {
    for (front, _, _, _) in DFC_FRONT_BACK {
        let (mut state, registry, p0, _) = setup_game(...);
        let obj = cast_and_resolve(&mut state, p0, front, &registry);
        let view = GameView::for_player(&state, p0, &registry);
        let perm = view.battlefield.iter().find(|p| p.object_id == obj).unwrap();
        assert_eq!(perm.name, *front, "Untransformed should display as front face");
    }
}
```

**Log regression check:**

```bash
# Each of these should be 0 after the fix.
grep -c "Kruin Outlaw 3/3"              verify-draft-8seat-v2.log
grep -c "Delver of Secrets 3/2 flying"  verify-draft-8seat-v2.log
grep -c "Villagers of Estwald 4/6"      verify-draft-8seat-v2.log
grep -c "Civilized Scholar 5/1"         verify-draft-8seat-v2.log
grep -c "Thraben Sentry 5/4"            verify-draft-8seat-v2.log
grep -c "Gatstaf Shepherd 3/3"          verify-draft-8seat-v2.log
grep -c "Reckless Waif 3/2"             verify-draft-8seat-v2.log

# And these should now appear (back-face names):
grep -c "Terror of Kruin Pass 3/3"                 verify-draft-8seat-v2.log  # >0
grep -c "Insectile Aberration 3/2 flying"          verify-draft-8seat-v2.log  # >0
grep -c "Howlpack of Estwald 4/6"                  verify-draft-8seat-v2.log  # >0
grep -c "Homicidal Brute 5/1"                      verify-draft-8seat-v2.log  # >0
grep -c "Thraben Militia 5/4 trample"              verify-draft-8seat-v2.log  # >0
grep -c "Gatstaf Howler 3/3 intimidate"            verify-draft-8seat-v2.log  # >0
grep -c "Merciless Predator 3/2"                   verify-draft-8seat-v2.log  # >0
```

---

## Bug C — `p255` controller in LTB-trigger display

**Severity: LOW-MEDIUM** — cosmetic until Bug A is fixed, then important for Fiend Hunter's real LTB to track last controller per CR 603.10c.

### What was observed

Every LTB trigger in the log displays its controller as `p255`:

```
[RESPOND TO p255's Fortress Crab's LTB trigger]
[RESPOND TO p255's Civilized Scholar's LTB trigger]
[RESPOND TO p255's Bonds of Faith's LTB trigger]
... (19 total `p255` mentions in the log, every one an LTB trigger)
```

Meanwhile `dies` triggers display the correct controller (`your`/`opp's`/`p0`/`p1`):

```
Stack: Abattoir Ghoul's dies trigger (your), Abattoir Ghoul's LTB trigger (opp's)
```

The `dies trigger (your)` has the real controller; the `LTB trigger (opp's)` is actually `p255` (rendered as "opp's" when viewed by p0 because 255 ≠ 0).

### Why it's wrong

Per **CR 603.10c**: leaves-the-battlefield triggered abilities are controlled by the player who controlled the permanent immediately before it left the battlefield. Storing a sentinel value instead of the actual last controller:

1. Makes logs misleading (19 `p255` lines).
2. Breaks APNAP bucket placement — the current code at `triggers.rs:478` always pushes LTB triggers to `ap_triggers` regardless of whose creature died.
3. Once Bug A is fixed and only real LTB triggers remain (Fiend Hunter), those triggers need the correct controller to properly resolve the "return the exiled card to the battlefield under its owner's control" effect. If Fiend Hunter's controller is read as `p255`, the effect targeting/ownership may behave unexpectedly.

### Root cause

Three spots in `mtg-engine/src/triggers.rs`:

**Line 107-111 — `PendingTrigger::LeftBattlefield` variant has no `controller` field:**

```rust
PendingTrigger::LeftBattlefield {
    object_id: ObjectId,
    card_id: CardId,
    description: String,
},
```

**Line 179 — `controller()` method hardcodes `PlayerId(255)` for this variant:**

```rust
pub fn controller(&self) -> PlayerId {
    match self {
        ...
        PendingTrigger::LeftBattlefield { .. } => PlayerId(255),
        ...
    }
}
```

**Line 466-480 — the collector doesn't capture the controller from the event:**

```rust
GameEvent::LeftBattlefield { object, .. } => {
    let (card_id,) = match state.get_object(*object) {
        Some(o) => (o.card_id,),
        None => continue,
    };
    ...
    let trigger = PendingTrigger::LeftBattlefield {
        object_id: *object,
        card_id,
        description: desc,
    };
    // LTB triggers go on AP side (they're usually self-referential).
    ap_triggers.push(trigger);
}
```

Note: by the time the collector runs, the object may already have been moved to the graveyard / exile / hand, and its `controller` field may have been cleared. Capture the controller at event emission time, not here.

### Fix

**Step 1.** Add a `controller: PlayerId` field to `PendingTrigger::LeftBattlefield`:

```rust
// triggers.rs:107-111
PendingTrigger::LeftBattlefield {
    object_id: ObjectId,
    card_id: CardId,
    controller: PlayerId,  // NEW
    description: String,
},
```

**Step 2.** Update the `controller()` impl at line 179:

```rust
PendingTrigger::LeftBattlefield { controller, .. } => *controller,
```

**Step 3.** Update the collector at triggers.rs:466-480 to capture the last controller. This requires the `GameEvent::LeftBattlefield` event to carry the last controller at emission time. Grep for where the event is emitted:

```bash
grep -rn "GameEvent::LeftBattlefield" mtg-engine/src/
```

Likely hits: `mtg-engine/src/state.rs::move_object`, and possibly `destruction.rs`. At each emission site, capture `obj.controller` **before** the zone change and include it in the event payload:

```rust
// events.rs or similar:
GameEvent::LeftBattlefield {
    object: ObjectId,
    card_id: CardId,
    last_controller: PlayerId,  // NEW
}
```

Then update the collector:

```rust
GameEvent::LeftBattlefield { object, card_id, last_controller, .. } => {
    if let Some(behavior) = registry.get(*card_id) {
        if behavior.has_ltb_handler() {  // Bug A gate
            let desc = trigger_description(...);
            let trigger = PendingTrigger::LeftBattlefield {
                object_id: *object,
                card_id: *card_id,
                controller: *last_controller,
                description: desc,
            };
            if *last_controller == active_player {
                ap_triggers.push(trigger);
            } else {
                nap_triggers.push(trigger);
            }
        }
    }
}
```

**Step 4.** Check the comment at `triggers.rs:478` (`// LTB triggers go on AP side (they're usually self-referential).`) — delete it; the fix puts them in the correct AP/NAP bucket based on last controller.

### Verification

**Unit test** (fuse with Bug A's tests, since they interact):

```rust
#[test]
fn fiend_hunter_ltb_trigger_has_correct_controller() {
    // Oracle: "When this creature leaves the battlefield, return the exiled card to
    //          the battlefield under its owner's control."
    let (mut state, registry, p0, p1) = setup_game(...);
    let enemy = cast_and_resolve(&mut state, p1, "Grizzly Bears", &registry);
    let hunter = cast_targeting(&mut state, p0, "Fiend Hunter", enemy, &registry);
    engine::resolve_stack(&mut state, &registry); // Hunter's ETB exiles enemy.

    // p0 passes; p1 destroys Hunter with Lightning Bolt (hypothetical — adjust to
    // whatever destruction method the test harness has).
    destroy_creature(&mut state, hunter, &registry);
    engine::resolve_state_based_actions(&mut state, &registry);

    // Hunter's LTB trigger is now on the stack. Controller should be p0 (last controller),
    // NOT p255 or p1.
    let ltb_trigger = state.pending_triggers.iter().find(|t|
        matches!(t, PendingTrigger::LeftBattlefield { card_id, .. }
                 if registry.card_data(*card_id).map(|d| d.name) == Some("Fiend Hunter".into()))
    ).expect("Hunter's LTB trigger should exist");
    assert_eq!(ltb_trigger.controller(), p0,
        "LTB trigger controller must be the last controller of Hunter (p0)");
}
```

**Log regression check:**

```bash
grep -c "p255" verify-draft-8seat-v2.log   # expect 0 after the fix
```

---

## Bug D (NEW, HIGH) — Werewolf `on_upkeep` transforms don't update subtypes → Bonds of Faith (and any Human-subtype-dependent effect) misfires on transformed werewolves

**Severity: HIGH** — correctness, observed to directly affect game outcomes. A player took lethal damage from an attacker that should have been unable to attack.

### What was observed (live in the 8-seat log)

Sequence from lines 33700-33736 (Seat 6 vs Seat 7, Match 1 Game 1):

```
Turn 12: Villagers of Estwald transforms into Howlpack of Estwald
Turn 13: p0 cast Bonds of Faith (#13) targeting Howlpack of Estwald (#43)
         Bonds of Faith (#13) resolved
Turn 14: p1 cast Unruly Mob (#59)
         Unruly Mob (#59) resolved
         p1 declared attackers: Howlpack of Estwald (#43)                    ← SHOULD BE ILLEGAL
         p0 declared no blockers
         p0 took 6 combat damage (0) from Howlpack of Estwald (#43)          ← 6, not 4 — kills p0
```

**Two things are wrong with this sequence:**

1. Bonds of Faith should prevent Howlpack of Estwald from attacking (it's not a Human — see oracle below).
2. Howlpack of Estwald is base 4/6 per oracle, yet it dealt **6 damage**. The +2 power is precisely what Bonds of Faith's "+2/+2 if Human" clause would provide. So Bonds is simultaneously **buffing** Howlpack (treating it as a Human) AND **failing to restrict it** (treating it as a Human). Both mis-applications are consistent with a single root cause: the engine thinks the creature still has the Human subtype after transforming.

### Oracle text (quoted verbatim)

**Bonds of Faith:**
```
Enchant creature
Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block.
```

**Villagers of Estwald (front face, 2/3 Human Werewolf):**
```
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```

**Howlpack of Estwald (back face, 4/6 Werewolf — note: no "Human" subtype):**
```
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```

Since Howlpack of Estwald is a Werewolf (no Human subtype per oracle), Bonds of Faith's "+2/+2 as long as it's a Human" should NOT fire and the "Otherwise, it can't attack or block" clause SHOULD fire. The ruling attached to Bonds of Faith explicitly confirms this dynamic: *"Once the enchanted creature has been declared as an attacking or blocking creature, causing it to stop being a Human won't remove it from combat. It will lose the +2/+2 bonus, however."* — i.e., the subtype is checked at attack time, and the +2/+2 is subtype-conditional.

### Why the engine is wrong

`BondsOfFaith` itself is correctly implemented at `mtg-engine/src/cards/isd/bonds_of_faith.rs:28-46`:

```rust
continuous_effects: vec![
    ContinuousEffect::ConditionalModifyPT {
        power: 2, toughness: 2,
        condition: EffectCondition::AttachedHasSubtype("Human".into()),
        scope: EffectScope::Attached,
    },
    ContinuousEffect::ConditionalPreventAttack {
        condition: EffectCondition::AttachedLacksSubtype("Human".into()),
        scope: EffectScope::Attached,
    },
    ContinuousEffect::ConditionalPreventBlock {
        condition: EffectCondition::AttachedLacksSubtype("Human".into()),
        scope: EffectScope::Attached,
    },
],
```

The bug is upstream: the werewolf's `on_upkeep` transform flips `is_transformed` and updates `obj.name`, but does NOT update `obj.subtypes`.

### Root cause

`mtg-engine/src/cards/isd/villagers_of_estwald.rs:78-95`:

```rust
fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) {
    if state.get_object(self_id).map(|o| o.zone != Zone::Battlefield).unwrap_or(true) {
        return;
    }
    if self.should_transform(state, self_id, registry) {
        if let Some(obj) = state.get_object_mut(self_id) {
            obj.is_transformed = !obj.is_transformed;                         // ← only this
            let (old_name, new_name) = if obj.is_transformed {
                ("Villagers of Estwald", "Howlpack of Estwald")
            } else {
                ("Howlpack of Estwald", "Villagers of Estwald")
            };
            obj.name = new_name.into();                                        // ← and this
            state.log(crate::state::LogLevel::Event,
                format!("{} transforms into {}", old_name, new_name));
        }
    }
}
```

Only `obj.is_transformed` and `obj.name` are touched. **These are NOT updated:**

- `obj.subtypes` → still `["Human", "Werewolf"]`. `EffectCondition::AttachedHasSubtype("Human")` returns `true` → Bonds of Faith buffs +2/+2. `AttachedLacksSubtype("Human")` returns `false` → attack/block restriction does not fire.
- `obj.power` / `obj.toughness` → still `(2, 3)`. This is masked by the `dynamic_pt()` override at lines 70-76 which returns `(4, 6)` when transformed.
- `obj.keywords` → empty list for Villagers front, also empty for Howlpack. Happens to be a no-op here, but for Kruin Outlaw (front=first strike, back=double strike) and Thraben Sentry (front=vigilance, back=trample) and Gatstaf Shepherd (front=[], back=intimidate) this matters — the on_upkeep code doesn't update keywords there either. See the full keyword audit table below.
- No triggered abilities are re-registered for the current face.

### Oracle-verified keyword differences across DFC faces (demonstrating the fix matters beyond Villagers)

| Card | Front face keywords (oracle) | Back face keywords (oracle) |
|---|---|---|
| Villagers of Estwald | none | none |
| Kruin Outlaw | `First strike` | `Double strike` + "Werewolves you control have menace" |
| Thraben Sentry | `Vigilance` | `Trample` |
| Gatstaf Shepherd | none | `Intimidate` |
| Reckless Waif | none | none |
| Tormented Pariah | none | none |
| Delver of Secrets | none | `Flying` (Insectile Aberration) |
| Civilized Scholar | none | (no keyword; has back-face trigger — see Bug E) |

So:
- A transformed Kruin Outlaw should **lose** First strike and **gain** Double strike + menace. With the current bug, it likely keeps the old keyword list from front face in `obj.keywords`.
- A transformed Thraben Sentry should **lose** Vigilance and **gain** Trample.
- A transformed Gatstaf Shepherd should **gain** Intimidate.
- A transformed Delver of Secrets should **gain** Flying.

Whether these actually manifest as correctness bugs depends on where the engine reads keywords from. Check `state.has_keyword()` — if it reads from `obj.keywords`, all of the above are bugs. If it goes through the registry's front face, it's also bugs but with a different root cause.

### Likely secondary effects

Once subtypes aren't updated, every card that checks the Human subtype on the transformed creature misbehaves. From oracle (verbatim):

- **Hamlet Captain**: `Whenever this creature attacks or blocks, other Humans you control get +1/+1 until end of turn.` → will buff a transformed werewolf as if it were still a Human.
- **Elder Cathar**: `When this creature dies, put a +1/+1 counter on target creature you control. If that creature is a Human, put two +1/+1 counters on it instead.` → gives 2 counters to a transformed werewolf instead of 1.
- **Sharpened Pitchfork**: `Equipped creature has first strike. As long as equipped creature is a Human, it gets +1/+1.` → buffs transformed werewolves.
- **Butcher's Cleaver**: `Equipped creature gets +3/+0. As long as equipped creature is a Human, it has lifelink.` → grants lifelink to transformed werewolves.
- **Bonds of Faith**: confirmed live in the log (above).

Not observed in the 8-seat run, but predicted by code inspection.

### Reference: correct implementation exists in `moonmist.rs`

`mtg-engine/src/cards/isd/moonmist.rs:66-99` handles Moonmist's "transform all Humans" effect correctly:

```rust
for (hid, was_transformed) in &humans_to_transform {
    if let Some(obj) = state.get_object_mut(*hid) {
        obj.is_transformed = !obj.is_transformed;
    }
    if let Some(behavior) = state.get_object(*hid).and_then(|o| registry.get(o.card_id)) {
        if *was_transformed {
            // Was on back face → transform to front face. Restore front face data.
            let front = behavior.card_data();
            if let Some(obj) = state.get_object_mut(*hid) {
                obj.name = front.name.clone();
                obj.power = front.power;
                obj.toughness = front.toughness;
                obj.keywords = front.keywords.clone();
                obj.subtypes = front.subtypes.clone();
            }
        } else {
            // Was on front face → transform to back face. Apply back face data.
            if let Some(back) = behavior.back_face_data() {
                if let Some(obj) = state.get_object_mut(*hid) {
                    obj.name = back.name.clone();
                    if let Some(p) = back.power { obj.power = Some(p); }
                    if let Some(t) = back.toughness { obj.toughness = Some(t); }
                    obj.keywords = back.keywords.clone();
                    obj.subtypes = back.subtypes.clone();
                }
            }
        }
    }
}
```

This is the template.

### Affected files

Every ISD DFC with a self-transforming `on_upkeep`:

```
mtg-engine/src/cards/isd/daybreak_ranger.rs
mtg-engine/src/cards/isd/gatstaf_shepherd.rs
mtg-engine/src/cards/isd/grizzled_outcasts.rs
mtg-engine/src/cards/isd/hanweir_watchkeep.rs
mtg-engine/src/cards/isd/kruin_outlaw.rs
mtg-engine/src/cards/isd/mayor_of_avabruck.rs
mtg-engine/src/cards/isd/reckless_waif.rs
mtg-engine/src/cards/isd/tormented_pariah.rs
mtg-engine/src/cards/isd/ulvenwald_mystics.rs
mtg-engine/src/cards/isd/village_ironsmith.rs
mtg-engine/src/cards/isd/villagers_of_estwald.rs
```

And every non-on_upkeep transformer should be audited too:
- `cloistered_youth.rs` — has a self-sacrificing trigger that transforms, not an on_upkeep flip
- `civilized_scholar.rs` — transforms via activated ability (front → back) and end-step trigger (back → front — also see Bug E)
- `thraben_sentry.rs` — transforms via a death-watch trigger
- `delver_of_secrets.rs` — transforms via upkeep reveal trigger
- `ludevics_test_subject.rs`, `ravenous_demon.rs` — various
- `moonmist.rs` — transforms all humans (already correct, use as template)

### Fix

**Step 1.** Create a shared helper. Add to `mtg-engine/src/cards/helpers.rs` (create the file if needed):

```rust
use crate::cards::CardRegistry;
use crate::ids::ObjectId;
use crate::state::{GameState, LogLevel};
use crate::types::Zone;

/// Flip a DFC permanent between its front and back face.
/// Updates `is_transformed`, `name`, `power`, `toughness`, `keywords`, and `subtypes`
/// from the new face's `CardData`. Also emits the canonical
/// "<old_name> transforms into <new_name>" log line at Event level.
///
/// Use this from any card's transform code (on_upkeep, on_resolve, activated ability,
/// triggered ability) instead of hand-rolling `obj.is_transformed = !obj.is_transformed`.
/// Hand-rolled transforms caused Bug D — see BUG_REPORT_8SEAT.md §D.
///
/// Note: `dynamic_pt()` overrides on individual werewolf cards become redundant after
/// this helper runs; they can be removed in a cleanup pass.
pub fn transform_dfc(
    state: &mut GameState,
    object_id: ObjectId,
    registry: &CardRegistry,
) {
    // Read current state.
    let (was_transformed, card_id) = match state.get_object(object_id) {
        Some(o) if o.zone == Zone::Battlefield => (o.is_transformed, o.card_id),
        _ => return,
    };
    let behavior = match registry.get(card_id) {
        Some(b) => b,
        None => return,
    };

    // Capture old name for the log line.
    let old_name = state.get_object(object_id).map(|o| o.name.clone()).unwrap_or_default();

    // Determine the new face's data.
    let new_face = if was_transformed {
        // back → front
        Some(behavior.card_data())
    } else {
        // front → back
        behavior.back_face_data()
    };

    // Flip the flag and apply the new face.
    if let Some(obj) = state.get_object_mut(object_id) {
        obj.is_transformed = !obj.is_transformed;
    }

    let Some(face) = new_face else {
        return;
    };

    if let Some(obj) = state.get_object_mut(object_id) {
        obj.name = face.name.clone();
        obj.power = face.power;           // Option<i32>
        obj.toughness = face.toughness;   // Option<i32>
        obj.keywords = face.keywords.clone();
        obj.subtypes = face.subtypes.clone();
    }

    state.log(LogLevel::Event,
        format!("{} transforms into {}", old_name, face.name));
}
```

**Step 2.** Replace the hand-rolled flip in all 11 werewolf files. For example, `villagers_of_estwald.rs:78-95` becomes:

```rust
fn on_upkeep(&self, state: &mut GameState, self_id: ObjectId, registry: &CardRegistry) {
    if state.get_object(self_id).map(|o| o.zone != Zone::Battlefield).unwrap_or(true) {
        return;
    }
    if self.should_transform(state, self_id, registry) {
        crate::cards::helpers::transform_dfc(state, self_id, registry);
    }
}
```

Repeat for all 11 werewolf files.

**Step 3.** Remove the now-redundant `dynamic_pt()` overrides in those card files. Since `obj.power`/`obj.toughness` are now updated correctly, `dynamic_pt` isn't needed for P/T and can be deleted. Do this as a cleanup pass after the primary fix lands and tests pass.

**Step 4.** Audit the other DFC transformers (civilized_scholar.rs, cloistered_youth.rs, thraben_sentry.rs, delver_of_secrets.rs, ludevics_test_subject.rs, ravenous_demon.rs) and replace their hand-rolled transforms with `transform_dfc` too. This may take more care because some of them have complex extra side effects.

**Step 5.** Consider routing `moonmist.rs` through `transform_dfc` as well, so every transform path uses the same helper and the code is in one place.

**Step 6 (consistency sweep).** Grep for remaining hand-rolled `is_transformed = !` flips:

```bash
grep -rn "is_transformed = !" mtg-engine/src/cards/
```

Every hit that doesn't go through `transform_dfc` is a potential source of the same bug class.

### Verification

**Core regression test — reproduce the exact log sequence:**

```rust
// mtg-engine/tests/werewolf_subtype_after_transform.rs
use mtg_engine::*;
mod common;
use common::*;

#[test]
fn bonds_of_faith_prevents_attack_on_transformed_werewolf() {
    // Regression for BUG_REPORT_8SEAT.md §D.
    //
    // Oracle (Bonds of Faith):
    //   Enchant creature
    //   Enchanted creature gets +2/+2 as long as it's a Human.
    //   Otherwise, it can't attack or block.
    //
    // Oracle (Howlpack of Estwald, back face of Villagers of Estwald):
    //   At the beginning of each upkeep, if a player cast two or more spells
    //   last turn, transform this creature.
    //   (Subtype: Werewolf — NOT Human.)
    let (mut state, registry, p0, p1) = setup_game(...);

    // p1 casts Villagers of Estwald.
    let villagers = cast_and_resolve(&mut state, p1, "Villagers of Estwald", &registry);

    // Transform it. In practice this happens via on_upkeep after no-spell turns;
    // here we force it directly via the new helper.
    crate::cards::helpers::transform_dfc(&mut state, villagers, &registry);
    assert!(state.get_object(villagers).unwrap().is_transformed);
    assert_eq!(state.get_object(villagers).unwrap().name, "Howlpack of Estwald");
    // CRITICAL: subtypes must no longer contain "Human".
    let subtypes = &state.get_object(villagers).unwrap().subtypes;
    assert!(subtypes.contains(&"Werewolf".into()));
    assert!(!subtypes.contains(&"Human".into()),
        "Transformed Howlpack of Estwald is NOT a Human per oracle");

    // p0 casts Bonds of Faith targeting Howlpack.
    let bonds = cast_targeting(&mut state, p0, "Bonds of Faith", villagers, &registry);
    engine::resolve_stack(&mut state, &registry);

    // Effective P/T should be 4/6 (base — no Bonds buff).
    assert_eq!(state.effective_power(villagers, &registry), Some(4),
        "Bonds of Faith should NOT grant +2/+2 to a non-Human");
    assert_eq!(state.effective_toughness(villagers, &registry), Some(6));

    // Verify restriction: Howlpack can't attack or block.
    assert!(!state.can_attack(villagers, &registry),
        "Bonds of Faith should prevent Howlpack (non-Human) from attacking");
    assert!(!state.can_block(villagers, &registry),
        "Bonds of Faith should prevent Howlpack (non-Human) from blocking");
}

#[test]
fn elder_cathar_gives_one_counter_to_transformed_werewolf() {
    // Oracle (Elder Cathar):
    //   When this creature dies, put a +1/+1 counter on target creature you control.
    //   If that creature is a Human, put two +1/+1 counters on it instead.
    let (mut state, registry, p0, _) = setup_game(...);
    let villagers = cast_and_resolve(&mut state, p0, "Villagers of Estwald", &registry);
    crate::cards::helpers::transform_dfc(&mut state, villagers, &registry);
    assert!(!state.get_object(villagers).unwrap().subtypes.contains(&"Human".into()));

    let cathar = cast_and_resolve(&mut state, p0, "Elder Cathar", &registry);
    destroy_creature(&mut state, cathar, &registry);
    engine::resolve_stack_targeting(&mut state, villagers, &registry);

    let counters = state.get_object(villagers).unwrap().counters
        .get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0);
    assert_eq!(counters, 1,
        "Transformed Howlpack is not a Human; Elder Cathar gives 1 counter, not 2");
}

#[test]
fn hamlet_captain_pump_does_not_buff_transformed_werewolf() {
    // Oracle (Hamlet Captain):
    //   Whenever this creature attacks or blocks, other Humans you control get
    //   +1/+1 until end of turn.
    let (mut state, registry, p0, _) = setup_game(...);
    let villagers = cast_and_resolve(&mut state, p0, "Villagers of Estwald", &registry);
    crate::cards::helpers::transform_dfc(&mut state, villagers, &registry);
    let captain = cast_and_resolve(&mut state, p0, "Hamlet Captain", &registry);

    // Captain attacks (alone — werewolf holds back).
    declare_attackers(&mut state, p0, &[captain], &registry);
    engine::resolve_stack(&mut state, &registry);

    // Howlpack should NOT receive the +1/+1 pump.
    assert_eq!(state.effective_power(villagers, &registry), Some(4),
        "Transformed Howlpack is not a Human; Hamlet Captain pump should not apply");
    assert_eq!(state.effective_toughness(villagers, &registry), Some(6));
}

#[test]
fn butchers_cleaver_does_not_grant_lifelink_to_transformed_werewolf() {
    // Oracle (Butcher's Cleaver):
    //   Equipped creature gets +3/+0.
    //   As long as equipped creature is a Human, it has lifelink.
    //   Equip {3}
    let (mut state, registry, p0, _) = setup_game(...);
    let villagers = cast_and_resolve(&mut state, p0, "Villagers of Estwald", &registry);
    crate::cards::helpers::transform_dfc(&mut state, villagers, &registry);
    let cleaver = cast_and_resolve(&mut state, p0, "Butcher's Cleaver", &registry);
    equip(&mut state, cleaver, villagers, &registry);

    // +3/+0 should apply (unconditional).
    assert_eq!(state.effective_power(villagers, &registry), Some(7)); // 4 + 3
    assert_eq!(state.effective_toughness(villagers, &registry), Some(6));
    // Lifelink should NOT apply (non-Human).
    assert!(!state.has_keyword(villagers, Keyword::Lifelink, &registry),
        "Transformed werewolf should not get Butcher's Cleaver's Human-conditional lifelink");
}

#[test]
fn kruin_outlaw_transform_updates_keywords() {
    // Oracle (Kruin Outlaw, front face): First strike.
    // Oracle (Terror of Kruin Pass, back face): Double strike.
    let (mut state, registry, p0, _) = setup_game(...);
    let kruin = cast_and_resolve(&mut state, p0, "Kruin Outlaw", &registry);
    assert!(state.has_keyword(kruin, Keyword::FirstStrike, &registry));
    assert!(!state.has_keyword(kruin, Keyword::DoubleStrike, &registry));

    crate::cards::helpers::transform_dfc(&mut state, kruin, &registry);
    assert!(!state.has_keyword(kruin, Keyword::FirstStrike, &registry),
        "Transformed Terror of Kruin Pass should no longer have First strike");
    assert!(state.has_keyword(kruin, Keyword::DoubleStrike, &registry),
        "Transformed Terror of Kruin Pass should have Double strike");
}
```

**Log regression check on the next 8-seat run:**

```bash
# Howlpack of Estwald base power is 4, so no single hit from it should exceed 4 damage
# to a player (unless it's been buffed by a legit non-Human-conditional effect).
grep -E "took [5-9] combat damage.*Howlpack of Estwald" verify-draft-8seat-v2.log   # expect 0

# Same for Merciless Predator (3) and Gatstaf Howler (3):
grep -E "took [4-9] combat damage.*Gatstaf Howler"     verify-draft-8seat-v2.log   # expect 0
grep -E "took [4-9] combat damage.*Merciless Predator" verify-draft-8seat-v2.log   # expect 0

# Note: Terror of Kruin Pass has double strike (base 3 power × 2 hits = 6 max without buff),
# so this grep would need a different approach for Kruin.
```

---

## Bug E (NEW, MEDIUM) — Civilized Scholar front face has back-face triggered abilities; fires empty end-step trigger every turn

**Severity: MEDIUM** — bug-A-class stack pollution, but specifically because the triggered abilities are defined on the wrong face.

### What was observed

Line 91822 of the log:

```
Stack: Civilized Scholar's end step trigger (transform back if didn't attack) (your)
```

The creature in question is on the **front** face (never transformed), sitting untapped after activating its normal `{T}: draw + discard` ability earlier in the turn. The end-step transform-back trigger fires anyway, gets displayed on the stack, and the LLM has to pass priority to let it resolve as a no-op.

### Oracle text (quoted verbatim)

**Civilized Scholar** (front face):
```
{T}: Draw a card, then discard a card. If a creature card is discarded this way, untap this creature, then transform it.
```

No triggered abilities. Only the `{T}` activated ability, which includes a transform as part of its own resolution (not as a separate triggered ability).

**Homicidal Brute** (back face):
```
At the beginning of your end step, if this creature didn't attack this turn, tap this creature, then transform it.
```

One triggered ability: an end-step trigger that transforms the creature back to Civilized Scholar if it didn't attack.

Relevant Scryfall ruling on Homicidal Brute (from the oracle lookup output):

> [2011-09-22] If Civilized Scholar attacks, and later in the turn (but before the beginning of your end step), it transforms, Homicidal Brute's last ability won't trigger. This is because the creature attacked that turn, even if had its other face up at the time.

So the "attacked this turn" state is tracked across face flips (the same underlying object), but the end-step trigger only exists on the back face per oracle.

### Why the engine is wrong

`mtg-engine/src/cards/isd/civilized_scholar.rs:38-47` puts **two** triggered abilities on the **front-face** `card_data`:

```rust
triggered_abilities: vec![
    TriggeredAbilityDef {
        kind: TriggerKind::Attacks,
        description: "mark as attacked this turn".into(),
    },
    TriggeredAbilityDef {
        kind: TriggerKind::EndStep,
        description: "transform back if didn't attack".into(),
    },
],
```

And the back-face data at `back_face_data()` has `triggered_abilities: vec![]` (line 65).

Per oracle:
- The front face has NO triggered abilities.
- The back face has ONE triggered ability (the end-step transform-back).

The "mark as attacked this turn" Attacks trigger is an internal implementation detail — not present in oracle — probably used as state tracking. That's fine conceptually but the engine needs to apply it to whichever face actually cares (either face, since per the Scryfall ruling the tracked attacked state carries over).

The trigger collector in `mtg-engine/src/triggers.rs` reads `behavior.card_data().triggered_abilities` (which is the front face's list) without checking `obj.is_transformed`. Consequently:

1. The EndStep trigger fires every end step on a **front-face** Civilized Scholar, even though oracle says it should only exist on the back face (Homicidal Brute).
2. It resolves as a no-op because the resolver probably checks `is_transformed` and bails if not on the back face, but it still eats an LLM prompt cycle.

### Root cause

- `civilized_scholar.rs:38-47` — triggered abilities defined on the wrong face.
- `triggers.rs` — collector is not face-aware; always reads `card_data().triggered_abilities`.

### Fix (two-part)

**Part 1: Move the triggers to the correct face in `civilized_scholar.rs`.**

```rust
// civilized_scholar.rs
fn card_data(&self) -> CardData {
    CardData {
        name: "Civilized Scholar".into(),
        ...
        triggered_abilities: vec![],   // FRONT face has no triggered abilities
    }
}

fn back_face_data(&self) -> Option<CardData> {
    Some(CardData {
        name: "Homicidal Brute".into(),
        ...
        triggered_abilities: vec![
            // Track attacks on the underlying object — needed so Homicidal Brute's
            // end-step check can see "did this creature attack this turn?".
            // Per Scryfall ruling [2011-09-22], attacks count regardless of face.
            TriggeredAbilityDef {
                kind: TriggerKind::Attacks,
                description: "mark as attacked this turn".into(),
            },
            // The real end-step transform-back trigger from oracle.
            TriggeredAbilityDef {
                kind: TriggerKind::EndStep,
                description: "transform back if didn't attack".into(),
            },
        ],
    })
}
```

**Part 2: Make the trigger collector face-aware.**

Add a helper to `CardBehavior` or similar:

```rust
// mtg-engine/src/cards/mod.rs
pub trait CardBehavior {
    ...
    /// Returns the triggered abilities active on this card's currently visible face.
    /// Prefers back face when `is_transformed`, otherwise front.
    fn triggered_abilities_for_face(&self, is_transformed: bool) -> Vec<TriggeredAbilityDef> {
        if is_transformed {
            self.back_face_data()
                .map(|d| d.triggered_abilities)
                .unwrap_or_default()
        } else {
            self.card_data().triggered_abilities
        }
    }
}
```

Update `mtg-engine/src/triggers.rs` to use this helper everywhere it currently reads `behavior.card_data().triggered_abilities`. Grep for usages:

```bash
grep -n "card_data().triggered_abilities\|card_data().triggered_abilities" mtg-engine/src/triggers.rs
```

Each site should be converted to `behavior.triggered_abilities_for_face(obj.is_transformed)` (reading `obj.is_transformed` from the relevant object at collection time).

**Subtlety with the "mark as attacked this turn" trick.** There's a catch: if the Attacks trigger is only active on the back face, then Civilized Scholar attacking (front face) won't set the attacked-this-turn flag. Per the Scryfall ruling, the attack should still count for Homicidal Brute's end-step check. Two options:

- **Option A (cleanest):** Put the Attacks trigger on BOTH faces (duplicate the `TriggeredAbilityDef`). Front-face and back-face both watch for attacks and update the object state.
- **Option B:** Drop the card-level Attacks trigger entirely and implement "attacked this turn" as a state flag set by the engine's attack declaration code (engine.rs or combat.rs) on every attacking creature. Then Homicidal Brute's end-step trigger reads the flag.

Option B is cleaner architecturally but a bigger change. Option A is a one-line fix on top of Part 1.

### Verification

**Unit tests:**

```rust
// mtg-engine/tests/civilized_scholar_triggers.rs
use mtg_engine::*;
mod common;
use common::*;

#[test]
fn civilized_scholar_front_face_has_no_end_step_trigger() {
    // Oracle: Civilized Scholar (front) has only {T}: draw + discard ability.
    // No triggered abilities.
    let (mut state, registry, p0, _) = setup_game(...);
    let scholar = cast_and_resolve(&mut state, p0, "Civilized Scholar", &registry);
    assert!(!state.get_object(scholar).unwrap().is_transformed);

    // Advance to end step without attacking.
    advance_to_end_step(&mut state, p0, &registry);

    // Stack should be empty. No transform-back trigger should fire.
    assert!(state.stack.is_empty(),
        "Front-face Civilized Scholar should not emit an end-step trigger");
    assert_eq!(state.pending_triggers.len(), 0);
}

#[test]
fn homicidal_brute_end_step_trigger_fires_when_it_didnt_attack() {
    // Oracle (Homicidal Brute): "At the beginning of your end step, if this creature
    //   didn't attack this turn, tap this creature, then transform it."
    let (mut state, registry, p0, _) = setup_game(...);
    let scholar = cast_and_resolve(&mut state, p0, "Civilized Scholar", &registry);
    // Force-transform to Homicidal Brute.
    crate::cards::helpers::transform_dfc(&mut state, scholar, &registry);
    assert!(state.get_object(scholar).unwrap().is_transformed);
    assert_eq!(state.get_object(scholar).unwrap().name, "Homicidal Brute");

    // Don't attack. Advance to end step.
    advance_to_end_step_no_attack(&mut state, p0, &registry);

    // End step trigger should fire.
    assert!(state.pending_triggers.iter().any(|t| /* end step transform back */));
    engine::resolve_stack(&mut state, &registry);

    // Creature should now be back to front face.
    assert!(!state.get_object(scholar).unwrap().is_transformed);
    assert_eq!(state.get_object(scholar).unwrap().name, "Civilized Scholar");
}

#[test]
fn homicidal_brute_end_step_trigger_does_not_transform_back_if_it_attacked_as_scholar() {
    // Per Scryfall ruling [2011-09-22]: "If Civilized Scholar attacks, and later in the
    // turn ... it transforms, Homicidal Brute's last ability won't trigger. This is
    // because the creature attacked that turn, even if had its other face up at the time."
    let (mut state, registry, p0, _) = setup_game(...);
    let scholar = cast_and_resolve(&mut state, p0, "Civilized Scholar", &registry);
    // Buff it so it can attack usefully (or just attack with 0 power).
    declare_attackers(&mut state, p0, &[scholar], &registry);
    engine::resolve_combat_damage(&mut state, &registry);
    // Now transform mid-turn (post-combat, pre-end-step).
    crate::cards::helpers::transform_dfc(&mut state, scholar, &registry);
    // Advance to end step.
    advance_to_end_step(&mut state, p0, &registry);
    engine::resolve_stack(&mut state, &registry);

    // The scholar should STILL be transformed (the end-step trigger should not have
    // transformed it back, because it attacked earlier this turn).
    assert!(state.get_object(scholar).unwrap().is_transformed,
        "Per ruling, Homicidal Brute should NOT transform back if it (or its front face) attacked this turn");
}
```

**Log regression check:**

```bash
# Count end-step triggers firing on Civilized Scholar in untransformed state.
# Before fix: fires every turn on every front-face Scholar.
# After fix: only fires when it's actually Homicidal Brute on the stack.
grep -A1 "Civilized Scholar's end step trigger" verify-draft-8seat-v2.log
# Cross-check that the creature was actually on the back face when the trigger fired.
```

### Other DFC cards to audit for the same bug pattern

Grep for `back_face_data` and inspect the `triggered_abilities` on both faces. Likely to need similar migration:

```bash
grep -l "back_face_data" mtg-engine/src/cards/isd/
```

For each hit, pull oracle text and compare which face has triggered abilities vs what the impl does. Flagged candidates:

- **Delver of Secrets** — front face: `At the beginning of your upkeep, look at the top card of your library. You may reveal that card. If an instant or sorcery card is revealed this way, transform this creature.` — upkeep trigger belongs to FRONT face. Back face (Insectile Aberration) per oracle: only `Flying`, no triggers. Verify the impl puts the upkeep trigger on the front face only.

- **Cloistered Youth** — needs oracle lookup for exact text; its transform is via a self-sacrificing mechanism.

- **Ludevic's Test Subject**, **Ravenous Demon** — complex transform mechanics. Oracle-check each.

- **Mayor of Avabruck** — is a werewolf DFC; similar to the Bug D cluster.

---

## Bug F (FIXED during the audit) — `mulligans to 7` log wording and missing LLM prompt state

**Severity: cosmetic (log) + minor harness (LLM prompt). ALREADY FIXED.**

### What was observed (before fix)

Log lines hardcoded "to 7":

```
p1 mulligans to 7 (mulligan #2 — will bottom 2 on keep)
```

Standard Magic shorthand uses the final hand size ("mull to 6" = "play with 6"), not the physical redraw count. The hardcoded `to 7` confused a user mid-audit.

Additionally, the LLM keep/mull decision prompt in `mtg-player/src/llm.rs::choose_mulligan` showed only the seven-card hand and a generic sentence. It did not tell the LLM how many mulligans had been taken, so the LLM couldn't compute the resulting hand size of a keep vs. another mull.

### Fix (already applied in this session)

**`mtg-engine/src/engine.rs:2425-2427`:**

```rust
format!("p{} mulligans to {} (mulligan #{} — will bottom {} on keep)",
    player.0, 7 - mull_count as i32, mull_count, mull_count)
```

**`mtg-engine/src/view.rs`:** added `pub your_mulligan_count: u32` to `GameView`, populated from `player_state.mulligan_count`.

**`mtg-player/src/llm.rs::choose_mulligan`:** prompt rewritten to say:

```
London mulligan. You have already taken 2 mulligans. If you KEEP now, you will bottom
2 cards and play with 5 cards in hand. If you MULLIGAN, you will redraw a fresh seven
and — if you then keep — bottom 3 cards to play with 4.
```

`cargo test --test mulligan` passes (9/9). No further action needed unless regression.

---

## Bug G (MINOR) — `[BOTTOM 2 CARDs AFTER MULLIGAN]` uses lowercase s

**Severity: cosmetic.** 12 occurrences in the 8-seat log.

### What was observed

```
[BOTTOM 1 CARD AFTER MULLIGAN]    (n=1, correct)
[BOTTOM 2 CARDs AFTER MULLIGAN]   (n>1, lowercase s — inconsistent)
[BOTTOM 3 CARDs AFTER MULLIGAN]
```

Every other bracketed label is all-caps: `[MAIN PHASE 1]`, `[DRAW]`, `[MULLIGAN DECISION]`, `[END STEP]`, `[COMBAT DAMAGE]`, etc.

### Root cause

`mtg-player/src/llm.rs:1753-1754`:

```rust
action_prompt.push_str(&format!("[BOTTOM {} CARD{} AFTER MULLIGAN]\n", n,
    if n == 1 { "" } else { "s" }));
```

### Fix

```rust
action_prompt.push_str(&format!("[BOTTOM {} CARD{} AFTER MULLIGAN]\n", n,
    if n == 1 { "" } else { "S" }));
```

### Verification

```bash
grep -c "CARDs AFTER MULLIGAN" verify-draft-8seat-v2.log   # expect 0
grep -c "CARDS AFTER MULLIGAN" verify-draft-8seat-v2.log   # expect >0 for n>1 games
```

---

## Bug H (LOW) — Harvest Pyre action label doesn't indicate the X value

**Severity: low — harness presentation, but causes concrete LLM misplays.**

### What was observed

Multiple times in the log, the LLM cast Harvest Pyre while the graveyard was empty and was surprised by the result.

Example at line 53903: The LLM's thought was *"Targeting the Makeshift Mauler (4/5) with Harvest Pyre is the most efficient use of the spell, removing their largest threat from the board."*

The engine log that followed (lines 53912-53914):

```
Exiled 0 cards from graveyard as additional cost
p1 cast Harvest Pyre (#42) targeting Makeshift Mauler (#2)
Harvest Pyre (#42) resolved
```

X=0 → 0 damage → Makeshift Mauler untouched. The LLM wasted the card because the action label `Cast Harvest Pyre (tap 2x Mountain)` gave no hint about the X value or the resulting damage.

### Oracle text (verbatim)

**Harvest Pyre:**
```
As an additional cost to cast this spell, exile X cards from your graveyard.
Harvest Pyre deals X damage to target creature.
```

Both the additional cost and the damage depend on graveyard contents at cast time. The player needs to know X when deciding whether to cast.

### Fix candidates

**Option 1 (minimal).** Append the X that will be used to the action label:

```
0: Cast Harvest Pyre (X=0, tap 2x Mountain)   ← warns the LLM about X
0: Cast Harvest Pyre (X=3, deal 3 damage, tap 2x Mountain)
```

Implementation in the Harvest Pyre card code: the action-generation path should compute max-X from the current graveyard and format it into the action description.

**Option 2 (suppress at zero).** Suppress the Harvest Pyre action when X=0 (i.e., graveyard is empty of exilable cards) and no other reason to cast it exists. Risk: hides legitimate "cast to sink extra mana" edge cases, but those are rare and probably acceptable to sacrifice.

**Option 3 (explicit prompt).** Add a follow-up prompt after the cast decision: `[HOW MANY CARDS TO EXILE FOR HARVEST PYRE? 0..N]` giving the LLM an explicit choice. This is more general: applies to any variable-X spell the engine might add later.

**Recommendation:** Option 1 is the smallest useful fix. Option 3 is the right long-term direction but out of scope for this bug.

### Verification

```bash
# Before fix: LLMs waste Harvest Pyre on empty graveyards.
grep -B1 "Exiled 0 cards from graveyard" verify-draft-8seat.log | grep -c "cast Harvest Pyre"
# Expect ~2 hits before the fix.

# After Option 1: action labels should show X=N.
grep "Cast Harvest Pyre" verify-draft-8seat-v2.log | grep -oE "X=[0-9]+"
# Expect every Harvest Pyre action to include X=N.
```

---

## Bug I (LOW) — Blocker option list not pre-filtered for intimidate / menace legality

**Severity: low — harness presentation.** Engine appears to still validate on submit (0 `BLOCKER_VALIDATION` entries in the log), but the prompt is misleading.

### What was observed

When Spectral Rider (white Spirit Knight with Intimidate) attacks, the blocker list offered to the defender includes non-white, non-artifact creatures. Sample from the log:

```
Attackers: 0:Spectral Rider 2/2 intimidate
Your blockers: 0:Markov Patrician 3/1 lifelink
```

Markov Patrician is a BLACK Vampire (oracle: cost `{2}{B}`, type line `Creature — Vampire`). Cannot legally block a white-intimidate attacker per oracle:

**Spectral Rider** (verbatim oracle):
```
Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)
```

Spectral Rider is white, Markov Patrician is black and not an artifact → cannot block. But the engine still offers it as an option.

In every observed instance in the 8-seat log, the LLM self-enforced and responded with `{"0": -1}` (no block). Sample LLM reasoning:

> [line 12704 context, LLM thought]: *"Markov Patrician is black, and Spectral Rider has intimidate, which means it can only be blocked by artifact creatures or white creatures. Since Markov Patrician is neither, it cannot block the Spectral Rider."*

Zero `BLOCKER_VALIDATION` entries in the log means either the LLM never actually submitted an illegal block OR the engine is silently validating correctly on submit. Haven't verified which.

### Why it matters

1. LLM wastes tokens reasoning about intimidate/menace/etc. on every prompt instead of picking from a pre-filtered legal set.
2. Prompt is misleading — an inferior LLM might eventually pick an illegal block and waste an API call.
3. If the engine's validator has a gap (e.g. for menace specifically), illegal blocks could slip through and cause correctness bugs.

### Fix

In the block prompt generator in `mtg-player/src/llm.rs` (search for `Your blockers:`), pre-filter the candidate creature list using a per-attacker legality check before offering them.

Rough shape:

```rust
let legal_blockers: Vec<&PermanentView> = your_creatures.iter()
    .filter(|c| !c.tapped && !c.summoning_sick)
    .filter(|c| attackers.iter().any(|a| state.can_block(c.object_id, a.object_id, registry)))
    .collect();
```

`can_block(blocker, attacker, registry)` should encapsulate:
- Tapped / summoning sick → no.
- Flying / reach requirement.
- Intimidate: blocker is artifact OR shares a color with attacker.
- Menace: attacker requires 2+ blockers (hard to enforce in a single-blocker check — this one may need to stay advisory).
- Protection from X where attacker has a protected-from trait.
- `can't be blocked` effects (e.g. Invisible Stalker is unblockable).
- Landwalk (not in ISD, but for completeness).

Alternatively, keep the full untapped-creature list but append a legality tag to each: `0:Markov Patrician 3/1 lifelink (cannot block: intimidate, wrong color)`.

### Verification

Re-run the 8-seat draft and grep for the specific combos that should now be filtered:

```bash
awk '/Attackers: 0:Spectral Rider/,/For each blocker/' verify-draft-8seat-v2.log | \
    grep -E "Your blockers:" | \
    grep -E "Markov Patrician|Crossway Vampire|Abattoir Ghoul"
# Expect 0 hits after fix.
```

Also add an engine-level regression test that tries to submit an illegal block and verifies the engine rejects it (populating `BLOCKER_VALIDATION`).

---

## Bug J (LOW, UX) — Aura / equipment ability text not shown inline on board display

**Severity: low — harness presentation.** Observed LLM misreadings from the 8-seat log.

### What was observed

LLMs repeatedly misread auras because the board display shows only the aura name, not its effect:

**Example 1, line 25222:** LLM stated *"Doomed Traveler is a 3/3 due to Ghostly Possession"*.

Oracle for Ghostly Possession (verbatim):
```
Enchant creature
Enchanted creature has flying.
Prevent all combat damage that would be dealt to and dealt by enchanted creature.
```

Ghostly Possession grants flying and prevents combat damage — it does NOT give +2/+2. The display correctly showed `Doomed Traveler 1/1 flying (Ghostly Possession)` (base 1/1, flying granted by the aura), but the LLM misremembered the aura's text. Multiple instances across the log.

**Example 2:** The display shows `Armored Skaab 1/4 (Bonds of Faith)` without any hint that Bonds of Faith on a non-Human prevents the Skaab from attacking or blocking.

**Example 3:** Equipment like `Blazing Torch`, `Inquisitor's Flail`, `Butcher's Cleaver`, `Sharpened Pitchfork` appears as bare names with no equip cost or ability hint. The LLM has to remember the text from the system-prompt decklist.

### Fix suggestion

Append a 1-line effective summary after each aura/equipment name in the board display. For example:

```
Doomed Traveler 1/1 flying (Ghostly Possession: prevents all combat damage)
Armored Skaab 1/4 (Bonds of Faith: Zombie, can't attack or block)
Slayer of the Wicked 3/2 [Butcher's Cleaver: +3/+0]
Equipment on board: Inquisitor's Flail (equip 2, deals/takes double damage)
```

This requires storing a short effective-description on each ISD aura/equipment behavior — not a trivial mechanical fix. Consider pulling it from the card's `card_data.oracle_text` first line or adding a new `fn short_description()` method to `CardBehavior`.

### Verification

Manual spot-check after the next run — grep for `(Ghostly Possession)`, `(Bonds of Faith)`, `[Butcher's Cleaver]` etc. and confirm each has an inline hint.

---

## Bug K (LOW, UX) — +1/+1 counter state not shown inline on board display

**Severity: low — harness presentation.**

### What was observed

When a creature has +1/+1 counters, the display shows effective P/T but not counter count. Example from the log: `Villagers of Estwald 4/5` is the *untransformed* Villagers (base 2/3 per oracle) with 2 +1/+1 counters (granted by Elder Cathar's death trigger, which gives 2 counters to a Human target).

The display shows `2/3 + 2+ counters = 4/5` effective, but there's no indication to the reader that this is 2/3 + counters vs. transformed vs. pumped by some aura. This is ambiguous and combines badly with Bug B (transformed display name).

### Fix suggestion

Append counter state as a flag in the existing `[T]`/`[S]`/`[Ndmg]` convention:

```
Villagers of Estwald 4/5 [+1+1x2]
Lilliana's Wrath victim 1/1 [-1-1x1]
Civilized Scholar 5/1 [S, +1+1x2]      (sick with 2 counters)
```

Or use a dedicated counter-only format like `{+1/+1 x2}`. Pick whichever is shortest.

### Verification

Add a regression test that builds a board with a creature that has counters and asserts the display includes the counter suffix.

---

## Fix order recommendation

1. **Bug A** — mechanical, low risk, follows the existing ETB-gating pattern. Land this FIRST because it unclutters the stack and makes later bugs easier to see in logs. Independent of everything else.
2. **Bug C** — depends on Bug A (otherwise empty LTB triggers swamp the real ones). Small change once A is landed.
3. **Bug D** — correctness-critical. Independent of A/C. Shared helper + 11 file edits. Most impactful fix; do this before the next full 8-seat run.
4. **Bug E** — medium complexity. Depends on adding face-aware trigger collection (can be reused or extended from Bug A's infrastructure).
5. **Bug B** — low risk, independent but cleaner to fix *after* D lands (because D will make `obj.subtypes`/`obj.power` consistent with the back face, so the view fix becomes purely about the name).
6. **Bug G** — one-character fix, bundle with anything.
7. **Bug H** — optional. Do after the core fixes land.
8. **Bug I** — optional. Do after the core fixes land.
9. **Bugs J, K** — UX polish, lowest priority.

Bug F is already fixed.

---

## Post-fix verification run

After Bugs A–E land, re-run the 8-seat verification draft:

```bash
cargo build --release -p mtg-draft-runner
./target/release/mtg-draft-runner \
    --set isd --players 8 --best-of 3 \
    --log verify-draft-8seat-v2.log \
    --model gemini:gemini-3.1-flash-lite-preview:medium:medium
```

Then run the regression greps listed under each bug's "Verification" section. Quick one-liner summary:

```bash
# Bug A — expect very low counts (only real handlers)
grep -c "'s dies trigger" verify-draft-8seat-v2.log
grep -c "'s LTB trigger"  verify-draft-8seat-v2.log

# Bug B — each should be 0
for pat in "Kruin Outlaw 3/3" "Delver of Secrets 3/2 flying" "Villagers of Estwald 4/6" \
           "Civilized Scholar 5/1" "Thraben Sentry 5/4" "Gatstaf Shepherd 3/3" "Reckless Waif 3/2"; do
    echo "$pat: $(grep -c "$pat" verify-draft-8seat-v2.log)"
done

# Bug C — expect 0
grep -c "p255" verify-draft-8seat-v2.log

# Bug D — expect 0 suspicious high-damage hits from Howlpack of Estwald
grep -E "took [5-9] combat damage.*Howlpack of Estwald" verify-draft-8seat-v2.log

# Sanity (should still pass):
grep -c "MALFORMED\|API_FATAL" verify-draft-8seat-v2.log        # 0
grep -c "BLOCKER_VALIDATION"    verify-draft-8seat-v2.log        # 0
grep -c "transforms into"       verify-draft-8seat-v2.log        # >0
```

Expected wall clock: ~45 min. Expected cost: ~$0.50–$1.20.

---

## Appendix: file index of proposed changes

| File | Bug | Change |
|---|---|---|
| `mtg-engine/src/cards/mod.rs` | A | Add `has_dies_handler` / `has_ltb_handler` default methods |
| `mtg-engine/src/cards/mod.rs` | E | Add `triggered_abilities_for_face(is_transformed)` helper |
| `mtg-engine/src/cards/helpers.rs` | D | New shared `transform_dfc` helper |
| `mtg-engine/src/triggers.rs` | A | Gate SelfDies and LeftBattlefield creation on the new methods |
| `mtg-engine/src/triggers.rs` | C | Add `controller: PlayerId` to `PendingTrigger::LeftBattlefield`; route LTB triggers to correct AP/NAP bucket |
| `mtg-engine/src/triggers.rs` | E | Use `triggered_abilities_for_face` instead of `card_data().triggered_abilities` |
| `mtg-engine/src/events.rs` | C | Add `last_controller` to `GameEvent::LeftBattlefield` |
| `mtg-engine/src/state.rs` (or wherever the event is emitted) | C | Populate `last_controller` before zone change |
| `mtg-engine/src/view.rs` | B | Transform-aware name/card_types lookup at lines 137 and 179 |
| `mtg-engine/src/cards/isd/{reckless_waif,villagers_of_estwald,gatstaf_shepherd,grizzled_outcasts,hanweir_watchkeep,kruin_outlaw,mayor_of_avabruck,tormented_pariah,ulvenwald_mystics,village_ironsmith,daybreak_ranger}.rs` | D | Replace hand-rolled transforms with `helpers::transform_dfc` |
| `mtg-engine/src/cards/isd/civilized_scholar.rs` | E | Move triggered abilities from `card_data()` to `back_face_data()`; duplicate Attacks trigger on both faces OR implement attacks-this-turn as engine state |
| `mtg-engine/src/cards/isd/*.rs` (several) | A | Override `has_dies_handler`/`has_ltb_handler` on cards with real handlers (at minimum: Doomed Traveler, Mausoleum Guard, Elder Cathar, Fiend Hunter; audit Falkenrath Noble, Selhoff Occultist, Murder of Crows, Unruly Mob, Abattoir Ghoul against oracle) |
| `mtg-player/src/llm.rs:1754` | G | Change `"s"` → `"S"` |
| `mtg-player/src/llm.rs` | H | Include X value in Harvest Pyre action label |
| `mtg-player/src/llm.rs` | I | Pre-filter blocker options for intimidate / menace / hexproof legality |
| `mtg-player/src/llm.rs` | J | Append inline aura/equipment effect summaries |
| `mtg-player/src/llm.rs` | K | Append +1/+1 counter count to creature display |

**New test files:**

- `mtg-engine/tests/empty_triggers.rs` (bug A)
- `mtg-engine/tests/transformed_display.rs` (bug B)
- `mtg-engine/tests/ltb_controller.rs` (bug C, after A lands)
- `mtg-engine/tests/werewolf_subtype_after_transform.rs` (bug D)
- `mtg-engine/tests/civilized_scholar_triggers.rs` (bug E)
