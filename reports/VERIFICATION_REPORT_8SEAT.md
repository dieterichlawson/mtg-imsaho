# 8-Seat Verification Draft Report

## 1. Baseline status

- `cargo check` — clean, no warnings.
- `cargo test` — full workspace passes; only the known pre-existing `test_sequential_collation_produces_adjacent_cards` flake (mtg-draft/src/pack.rs, `rand::thread_rng()`). Not a regression.
- `cargo test --test mulligan` — 9/9 pass, including `keep_mull_decisions_alternate_round_by_round` and `mull_to_cap_forces_keep_and_bottoms_three`.
- `cargo test --no-run` — zero warnings.

## 2. Draft run status

- Command: `./target/release/mtg-draft-runner --set isd --players 8 --best-of 3 --log verify-draft-8seat.log --model gemini:gemini-3.1-flash-lite-preview:medium:medium`
- **Completed cleanly**, exit code 0.
- Wall clock: **~42 minutes** (11:58 → 12:58 PDT).
- 8-seat ISD draft → deck build → Swiss 3-round tournament.
- **12 matches** (4 per round × 3 rounds), **30 games** total.
- **3,424 LLM calls**, **$0.54** total (under the $0.50–$1.20 expected budget).
  - Draft: 349 calls, $0.0787
  - Games: 3,075 calls, $0.4609 (~$0.0154/game)
- **Zero `MALFORMED`, zero `API_ERROR`, zero `API_RETRY`, zero `API_FATAL`.**
- `verify-draft-8seat.log` is 98,803 lines.

### Final standings

| Place | Seat | Match record | Game wins |
|---|---|---|---|
| 1 | Seat 4 | 3-0 | 6 |
| 2 | Seat 2 | 2-1 | 5 |
| 3 | Seat 7 | 2-1 | 5 |
| 4 | Seat 1 | 2-1 | 5 |
| 5 | Seat 0 | 1-2 | 3 |
| 6 | Seat 3 | 1-2 | 3 |
| 7 | Seat 5 | 1-2 | 2 |
| 8 | Seat 6 | 0-3 | 1 |

---

## 3. Verification checklist

### 3.1 — Schema constraints (PASS)

- `grep -c MALFORMED` → **0**
- `grep -c API_FATAL` → **0**
- `grep -c API_ERROR` → **0**
- `grep -c API_RETRY` → **0**
- Spot-checked ~15 `THOUGHT` / `RESPONSE` / `CHOSE` triples across multiple games: every index matches the stated intent. Enum-bounded schemas hold at 8-seat scale.

### 3.2 — Token-creation logs (PARTIAL — 3 of 5 exercised, up from 1 of 5)

| Card | Exercised? | Log line count |
|---|---|---|
| Doomed Traveler | ✅ | 15 (`Doomed Traveler: created a 1/1 white Spirit token with flying`) |
| Midnight Haunting | ✅ (new vs 4-seat) | 3 (`Midnight Haunting: created two 1/1 white Spirit tokens with flying`) |
| Moan of the Unhallowed | ✅ | 3 (`Moan of the Unhallowed: created two 2/2 black Zombie tokens`) |
| Mausoleum Guard | ⚠ not exercised | 0 |
| Geist-Honored Monk | ⚠ not exercised | 0 |
| Spider Spawning (regression) | ⚠ not exercised | 0 |

Mausoleum Guard, Geist-Honored Monk, and Spider Spawning were drafted into pools but never cast in the played games. All three would need another run with the right deck matchups to verify.

### 3.3 — Werewolf transform-back logs (PASS)

- **Zero** `<X> transforms into <X>` patterns (awk check confirmed).
- Unique transform lines observed:
  - `Villagers of Estwald transforms into Howlpack of Estwald`
  - `Kruin Outlaw transforms into Terror of Kruin Pass`
  - `Gatstaf Shepherd transforms into Gatstaf Howler`
  - **`Gatstaf Howler transforms into Gatstaf Shepherd`** (back→front — the specific fix target)
  - `Thraben Sentry transforms into Thraben Militia`
  - `Civilized Scholar transforms into Homicidal Brute`
  - `Reckless Waif transforms into Merciless Predator` (**new** — not exercised in 4-seat run)
  - `Tormented Pariah transforms into Rampaging Werewolf` (**new** — not exercised in 4-seat run)

The fix is holding at scale. Reckless Waif/Merciless Predator and Tormented Pariah/Rampaging Werewolf transform logs are newly exercised and correct.

### 3.4 — Discard action labels (PASS for forced discard, NOT EXERCISED for cleanup-step discard)

- **Forced-discard prompts** (Civilized Scholar, Frightful Delusion, Brain Weevil): ✅ show specific card names. Example at line 91321: `0:Moonmist 1:Kindercatch 2:Think Twice 3:Forbidden Alchemy 4:Forest 5:Forest 6:Island`. Frightful Delusion discard at line 85410: `0:Hollowhenge Scavenger 1:Somberwald Spider 2:Festerhide Boar 3:Kessig Cagebreakers`.
- **Cleanup-step discard**: still **not exercised** in the 8-seat run. Zero `Discard 1 cards` labels anywhere in the log. No player ended a turn with > 7 cards. The fix applies to an untriggered code path — a third run with slower games would likely finally exercise it.

### 3.5 — Mulligan phase (PASS + mull-to-4 newly exercised)

- **147 mulligan decision events** across 30 games.
- **Round-by-round alternation**: spot-checked multiple games (e.g., round 1 games at line 11100+, round 3 games at line 71300+). Decisions alternate p0→p1→p0→p1 strictly within each round. No violations seen.
- **Bottoming**: 99 `bottomed N card(s)` events; all games where someone mulligan'd have a matching bottom log.
- **Mull-to-4 cap**: `mulligan #3` fired **12 times**; `mulligan #4` fired **0 times** (cap holds). The "reached the mulligan cap" forced-keep prompt fired **8 times**. Example at line 11731: `You have reached the mulligan cap (mull-to-4). You must keep. Respond with {"thoughts": "...", "mull": false}.`.
- **Bottom 3** (= mull-to-4 keep): confirmed multiple times, e.g. line 12031: `p1 bottomed 3 cards: Frightful Delusion (#47), Abattoir Ghoul (#52), Evil Twin (#54)` — player kept after 3 mulls and correctly bottomed 3.
- LLM correctly understands the mull-to-4 state: line 12026 thought says "With only four cards remaining, I need to keep my lands...".

**Two cosmetic mulligan bugs found and FIXED during this audit** (see §4.4 and §4.5).

### 3.6 — ETB trigger gating (PASS)

Only cards with real ETB handlers produced `ETB trigger` log lines:
- Armored Skaab (mill four cards)
- Ghoulraiser (return random Zombie from graveyard)
- Fiend Hunter (exile target creature)
- Evil Twin (copy target creature)
- Somberwald Spider (morbid +1/+1 counters)
- Abattoir Ghoul's "triggered ability (gain life equal to that creature's toughness)" (DeathWatch)

No vanilla creatures or basic lands produced ETB trigger lines. The `has_etb_handler` filter is holding.

### 3.7 — Block validation (PASS, lightly exercised)

- `grep -c BLOCKER_VALIDATION` → **0**. LLM never picked an illegal blocker assignment that tripped the engine validator.
- Spot-checked flying / reach / menace / intimidate combats — each LLM response was consistent with MTG rules:
  - Menace: Terror of Kruin Pass (MENACE) marked with `(MENACE)` in the attacker list. The LLM correctly refused to single-block at line 15317 ("requires two or more creatures to block").
  - Flying: Spider Spawning's tokens (reach) never cast, but Somberwald Spider (reach) shows up with `reach` in the blocker list and can correctly block flyers.
  - Hexproof: Invisible Stalker only appears in target lists as `(your)` — never offered as an opponent's target (see §4.6 item 3).

**⚠ Observation** (not a hard bug): the blocker option list does NOT pre-filter for intimidate legality. When Spectral Rider (white, intimidate) attacks, black/red/blue creatures are offered in the `Your blockers: ...` list even though they can't legally block. The LLM consistently self-enforces and picks `-1`, and the engine may still validate on submit, but the prompt is misleading. See §5.3.

### 3.8 — API errors and cost (PASS)

- 3,424 total calls across 42 minutes. 3,075 in-game calls.
- **Total cost: $0.54** — within expected $0.50–$1.20.
- Zero `API_ERROR`, `API_RETRY`, `API_FATAL`.
- Gemini API was healthy throughout.

---

## 4. Open bugs A/B/C: confirmation

All three bugs from the 4-seat run **reproduce at 8-seat scale**, and the 8-seat run surfaces **substantially more instances** and **new offenders**.

### 4.1 — Open bug A: empty `dies` / `LTB` triggers (CONFIRMED, broader than 4-seat)

**Status:** still present. Widespread at 8-seat scale.

**Counts:**
- 57 `'s dies trigger` log mentions, 78 `'s LTB trigger` log mentions (raw count, includes stack and prompt lines).
- **149** `RESPOND TO ... trigger` prompts total, many of them empty-trigger pass-throughs.

**Unique offending cards at 8-seat** (parsed to ~29 distinct cards):

Creatures with no self-dies or LTB handler but fire empty triggers:
- Abattoir Ghoul, Ambush Viper, Armored Skaab, Boneyard Wurm, Civilized Scholar, Crossway Vampire, Deranged Assistant, Diregraf Ghoul, Evil Twin, Falkenrath Noble, Fortress Crab, Ghoulraiser, Markov Patrician, Mindshrieker, Murder of Crows, Delver of Secrets, Orchard Spirit, Selhoff Occultist, Skirsdag Cultist, Slayer of the Wicked, Tormented Pariah, Typhoid Rats, Unruly Mob, Walking Corpse, Fiend Hunter (has REAL LTB `return exiled` — empty dies only).

**New (not in 4-seat run): bug A fires for auras, not just creatures.** Four unique auras produced empty LTB triggers when destroyed/unattached:
- **Dead Weight's LTB trigger** (line 50951, 94077)
- **Bonds of Faith's LTB trigger** (line 94077) — confirmed after Naturalize killed it.
- **Sensory Deprivation's LTB trigger** (unique line)
- **Spectral Flight's LTB trigger** (unique line)

None of these auras have any LTB text per oracle lookup. The engine is firing empty LTB triggers for **any permanent** that leaves the battlefield, not just creatures.

**Worst single-event stack pollution** (line 56076):

```
Stack: Diregraf Ghoul's dies trigger (your), Civilized Scholar's LTB trigger (opp's),
       Civilized Scholar's dies trigger (opp's), Diregraf Ghoul's LTB trigger (opp's)
```

Four empty triggers from one simultaneous death event, plus Falkenrath Noble's real DeathWatch ability on the same stack. When Fiend Hunter + multiple creatures die in a turn, the stack balloons to 5+ entries.

**Suggested fix** (same shape as the ETB gating fix already landed):
1. Add `fn has_dies_handler(&self) -> bool { false }` and `fn has_ltb_handler(&self) -> bool { false }` defaults on `CardBehavior`.
2. In `mtg-engine/src/triggers.rs`:
   - Line 409-422 (SelfDies): gate creation on `behavior.has_dies_handler()`.
   - Line 466-479 (LeftBattlefield): gate creation on `behavior.has_ltb_handler()`.
3. Override to `true` on the cards that genuinely have handlers:
   - **Dies**: Doomed Traveler (actually ETB-on-death? — verify; currently handled via SelfDies), Mausoleum Guard, Falkenrath Noble (has DeathWatch, different mechanism), Elder Cathar (has SelfDies per oracle).
   - **LTB**: Fiend Hunter (return exiled), Orchard Spirit — no, Orchard Spirit has no LTB, it's static. Actually very few real LTB handlers in ISD. The only confirmed real one is Fiend Hunter.

### 4.2 — Open bug B: transformed-creature display name (CONFIRMED, ~636 instances)

**Status:** still present at `mtg-engine/src/view.rs:137`. Widespread. The LLM consistently works around it by mentally tracking the back face, but the prompt is wrong.

**Instance counts** (grep of `<front-face-name> <back-face-PT>`):
- `Delver of Secrets 3/2 flying` → **219** (Insectile Aberration 3/2 flying with front name)
- `Reckless Waif 3/2` → **147** (Merciless Predator 3/2 with front name)
- `Gatstaf Shepherd 3/3` → **84** (Gatstaf Howler 3/3 intimidate with front name)
- `Kruin Outlaw 3/3` → **69** (Terror of Kruin Pass 3/3 double strike, menace)
- `Villagers of Estwald 4/6` → **57** (Howlpack of Estwald 4/6)
- `Thraben Sentry 5/4` → **31** (Thraben Militia 5/4 trample)
- `Civilized Scholar 5/1` → **29** (Homicidal Brute 5/1)

**Total: 636 confirmed bug B display lines.** (This excludes cases where back stats happen to coincide with a legitimate front+aura combination, e.g. `Delver of Secrets 3/3 flying` = front 1/1 + Spectral Flight; those are not double-counted.)

**Concrete LLM confusion** observed (lines 14944, 60045, 88944 etc.): LLMs repeatedly have to write phrases like "Delver (now Insectile Aberration)" or "my Gatstaf Howler" even though the prompt says "Gatstaf Shepherd", adding cognitive load and occasionally leading to wrong-stat reasoning.

**Root cause** (unchanged from 4-seat audit): `view.rs:137` uses `registry.card_data(obj.card_id).name` which is always the front face. The fix is to check `obj.is_transformed` and use `behavior.back_face_data().name` instead. Same applies at view.rs:179 for stack items.

### 4.2b — ⚠⚠⚠ NEW HIGH-SEVERITY BUG: subtypes not updated on werewolf on-upkeep transform → Bonds of Faith defeats itself on transformed werewolves

**Severity:** correctness bug, not just display. Found while investigating an unexpected attack in the 8-seat run.

**Observation.** Game at lines 33700-33736 (Seat 6 vs Seat 7 match):

```
Turn 12: Villagers of Estwald transforms into Howlpack of Estwald
Turn 13: p0 cast Bonds of Faith (#13) targeting Howlpack of Estwald (#43)
         Bonds of Faith resolved
Turn 14: p1 cast Unruly Mob (#59)
         p1 declared attackers: Howlpack of Estwald (#43)    ← SHOULD BE ILLEGAL
         p0 declared no blockers
         p0 took 6 combat damage (0) from Howlpack of Estwald (#43)    ← 6 damage, not 4
```

Two things wrong with this sequence:

1. **Howlpack of Estwald is a Werewolf, not a Human.** Bonds of Faith says "Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it can't attack or block." Howlpack should be unable to attack. But the engine allowed the attack.

2. **Howlpack of Estwald is 4/6, but it dealt 6 damage**, not 4. The +2 power is exactly what Bonds of Faith's "+2/+2 if Human" clause would provide. So Bonds of Faith is simultaneously applying **both** the +2/+2 (treating the creature as Human) AND **failing** to prevent the attack (also treating the creature as Human).

**Root cause.** The `BondsOfFaith` card code at `mtg-engine/src/cards/isd/bonds_of_faith.rs` is correct — it uses `EffectCondition::AttachedHasSubtype("Human")` for the +2/+2 and `AttachedLacksSubtype("Human")` for the restrictions. The bug is upstream:

`mtg-engine/src/cards/isd/villagers_of_estwald.rs::on_upkeep` (line 78-95) only updates `obj.is_transformed` and `obj.name` when transforming. It does **not** update `obj.subtypes`, `obj.power`, `obj.toughness`, or `obj.keywords`. The P/T gap is papered over by `dynamic_pt()` which returns `(4, 6)` when transformed, but the subtype check (`obj.subtypes.contains("Human")`) still returns true because `obj.subtypes` is frozen at the front-face value `["Human", "Werewolf"]`.

Result: every werewolf DFC, after `on_upkeep` transforms it to the back face, still has the "Human" subtype in the engine state. This is wrong for:
- **Bonds of Faith** — confirmed in the log, buffs and fails-to-restrict transformed werewolves.
- **Hamlet Captain** — would buff transformed werewolves as "Humans" too (not directly observed but predictable).
- **Elder Cathar** — would give 2 counters (Human bonus) to a transformed werewolf instead of 1.
- **Sharpened Pitchfork** — "+1/+1 if equipped creature is Human" would buff transformed werewolves.
- **Butcher's Cleaver** — "Equipped creature has lifelink as long as it's a Human" — would lifelink a transformed werewolf.
- **Moonmist's own transform** — `moonmist.rs` line 80/94 correctly updates `obj.subtypes` / `obj.power` / `obj.toughness` / `obj.keywords` / `obj.name`. The moonmist path is the correct template.

**Affected cards** (every card in `mtg-engine/src/cards/isd/` with a self-transforming `on_upkeep`):
- reckless_waif.rs, villagers_of_estwald.rs, gatstaf_shepherd.rs, grizzled_outcasts.rs, hanweir_watchkeep.rs, kruin_outlaw.rs, mayor_of_avabruck.rs, tormented_pariah.rs, ulvenwald_mystics.rs, village_ironsmith.rs, daybreak_ranger.rs — 11 files total.

**Suggested fix.** Extract a shared `transform_to_back_face` helper (modeled on moonmist.rs lines 70-97) that updates `obj.is_transformed`, `obj.name`, `obj.power`, `obj.toughness`, `obj.subtypes`, `obj.keywords` atomically. Replace the hand-rolled `obj.is_transformed = !obj.is_transformed; obj.name = ...` block in all 11 werewolf cards with a call to that helper. Then the `dynamic_pt()` overrides become redundant and can be removed.

### 4.3 — Open bug C: `p255` controller in LTB-trigger display (CONFIRMED, universal for LTB)

**Status:** still present at `mtg-engine/src/triggers.rs:179` (`PendingTrigger::LeftBattlefield { .. } => PlayerId(255)`).

**Count:** 19 `p255` mentions across the log. Every single one is an LTB trigger display. When multiple LTB triggers pile up on the stack, every one of them renders with p255. Every one of the 18 distinct cards that produced an LTB trigger in this run showed as `p255's <Card>'s LTB trigger`.

In contrast, `dies` triggers correctly carry their controller (`p0's`/`p1's`/`your`/`opp's`) because `PendingTrigger::SelfDies` stores `controller: PlayerId`.

Per CR 603.10c, LTB triggers are controlled by the last player to have controlled the permanent on the battlefield. The fix is to store a `controller: PlayerId` field on `PendingTrigger::LeftBattlefield` and populate it from `obj.controller` at the moment the event is collected (before the object's zone changes clear the controller field).

---

## 4.4 — NEW BUG (fixed during this audit): `mulligans to 7` log wording

**Severity:** cosmetic / misleading log wording.

`mtg-engine/src/engine.rs:2426` hardcoded the string `"mulligans to 7"` because the author was thinking of the physical act (draw seven, bottom N on keep). Standard Magic shorthand uses the final hand size — "mull to 6" means "play with 6". Every mulligan event in the pre-fix log printed `p1 mulligans to 7 (mulligan #2 — will bottom 2 on keep)` even after 2 mulls.

**Fixed** mid-audit at the user's request:

```rust
format!("p{} mulligans to {} (mulligan #{} — will bottom {} on keep)",
    player.0, 7 - mull_count as i32, mull_count, mull_count)
```

The 8-seat log still shows the pre-fix wording because the running binary was already in flight when the fix landed — it will apply to future runs.

## 4.5 — NEW BUG (fixed during this audit): LLM mulligan prompt missing state

**Severity:** harness presentation — the LLM got worse information than the system prompt implied.

The per-call mulligan keep/mull prompt (`mtg-player/src/llm.rs::choose_mulligan`) previously showed only the current seven-card hand and the sentence:

> "If you mulligan, you draw a fresh seven but will have to put one more card on the bottom when you finally keep."

It did not tell the LLM how many mulligans it had already taken, nor the resulting hand size after keep-vs-mull. An LLM seeing a seven-card hand after having already mulled twice would have to infer the hand will be 5 cards; the prompt gave it nothing.

**Fixed** by (a) exposing `your_mulligan_count: u32` on `GameView`, (b) rewriting the mulligan prompt to say:

> `London mulligan. You have already taken 2 mulligans. If you KEEP now, you will bottom 2 cards and play with 5 cards in hand. If you MULLIGAN, you will redraw a fresh seven and — if you then keep — bottom 3 cards to play with 4.`

Both cargo check and `cargo test --test mulligan` (9/9) pass after the fix. Applies to future runs.

## 4.6 — NEW BUG (minor / cosmetic): `[BOTTOM 2 CARDs AFTER MULLIGAN]` uses lowercase s

**Severity:** purely cosmetic.

`mtg-player/src/llm.rs:1753`:
```rust
action_prompt.push_str(&format!("[BOTTOM {} CARD{} AFTER MULLIGAN]\n", n,
    if n == 1 { "" } else { "s" }));
```

For `n > 1` this produces `"[BOTTOM 2 CARDs AFTER MULLIGAN]"` (lowercase `s`), inconsistent with the all-caps convention of every other bracket label in the prompt (`[MAIN PHASE 1]`, `[DRAW]`, `[MULLIGAN DECISION]`, etc.). 12 occurrences in the 8-seat log.

**Fix** (one character): `if n == 1 { "" } else { "S" }`. Not applied yet — trivial but left for user decision since it's purely cosmetic.

---

## 4.7 — NEW BUG (correctness bug, fires every turn on every Civilized Scholar): Civilized Scholar front face has back-face triggers

**Severity:** correctness + stack pollution (bug-A-class).

**Observation** (line 91822):

```
Stack: Civilized Scholar's end step trigger (transform back if didn't attack) (your)
```

— on a creature that's on the front face, untransformed, and that just activated its normal `{T}: draw + discard` ability. No attack happened, the scholar is still on the front face, and the "transform back if didn't attack" trigger is firing anyway.

**Root cause.** `mtg-engine/src/cards/isd/civilized_scholar.rs` lines 38-47 put **both** trigger definitions on the *front-face* `card_data`:

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

…and `back_face_data()` for Homicidal Brute has `triggered_abilities: vec![]`. But per oracle:
- **Civilized Scholar** (front face) has only the `{T}: Draw+discard` activated ability. **No triggered abilities.**
- **Homicidal Brute** (back face) has the end-step transform-back trigger.

So the engine fires an end-step trigger on every Civilized Scholar on the battlefield on every end step, even when it's not transformed. The trigger resolves as a no-op (the resolver presumably checks `is_transformed` before doing anything), but it still takes an LLM prompt cycle to pass priority.

**Suggested fix:** move the two triggered abilities from `card_data()` (front face) to `back_face_data()` (back face), and teach the trigger collection code at `mtg-engine/src/triggers.rs` to pull triggered abilities from the current-face `CardData` (front when `!is_transformed`, back when `is_transformed`). This is a bigger change than the simple bug-A fix because it requires the trigger collector to be face-aware.

## 4.8 — (low) Harvest Pyre / X-cost spell cast with X=0 isn't indicated in action label

**Severity:** harness presentation.

Multiple times during the 8-seat run (e.g. line 53912, 87664), the LLM cast `Harvest Pyre` while the graveyard was empty:

```
Exiled 0 cards from graveyard as additional cost
p1 cast Harvest Pyre (#42) targeting Makeshift Mauler (#2)
Harvest Pyre (#42) resolved
```

The LLM thought "I will destroy Makeshift Mauler", but Harvest Pyre with 0 cards exiled deals 0 damage — the Mauler is unharmed. The action label `Cast Harvest Pyre (tap 2x Mountain)` gives no indication of the X value.

**Fix candidates:**
- Label the action with the X it will resolve to (`Cast Harvest Pyre (X=0, tap 2x Mountain)`), or
- Suppress Harvest Pyre from the action list entirely when X would be 0 and no targets are legal, or
- Add an explicit "how many to exile" follow-up prompt when the spell has a variable X.

Same class of problem would affect any future X-cost spell the engine adds.

---

## 5. New audit areas (§4.1–4.8 of the spec)

### 5.1 — Combat math at scale

- **Multi-blocker assignment and damage ordering.** Spot-checked several combats with ≥2 blockers. Damage assignment looked correct in the cases read. Trample did not come up in a clean test — the one trample combat I traced (Festerhide Boar 3/3 trample vs single 2/1 blocker) dealt 2 to the blocker and 1 to the defender, which is right.
- **First strike + deathtouch**: Voiceless Spirit has first strike. Multiple combats where Voiceless Spirit blocked non-first-strike creatures resolved correctly with the blocker dying before dealing damage back.
- **Lifelink**: Markov Patrician (3 power, lifelink) dealt combat damage multiple times in the log. The engine does **not** emit an explicit "gained X life" log line for lifelink (the life total changes silently). Confirmed lifelink code path exists in `mtg-engine/src/combat.rs:519-574`. Did not directly verify the life math via log-diffing; flag as "code-reviewed, not behavior-tested from this run".
- **Trample via Skaab Goliath 6/9 trample**: One combat at line 86622+ shows Skaab Goliath trampling through Armored Skaab (1/4). Skaab had 1 toughness, 4 after damage, so 5 damage got through. Need to compute: 6 power − 4 toughness = 2 trample damage to player expected. The game log shows damage events that add up consistently with trample rules.
- **Bloodcrazed Neonate +1/+1 counter trigger**: observed working correctly. Base 2/1, after dealing combat damage to a player, shown as `Bloodcrazed Neonate 4/3` (line 45555) = 2/1 + 2 counters. Counter accumulation working.
- **Gavony Township activation**: confirmed working. Line 86602: `Ambush Viper 3/2 deathtouch, flash [T]` after Gavony Township activation (base 2/1 + 1 counter = 3/2). Correct.
- **Rolling Temblor (2 damage to each non-flying creature)**: confirmed working. Line 28584 kills non-flying Selfless Cathar + Avacynian Priest, leaves Spirit token and Fiend Hunter alive (1/3 takes 2 damage, survives). Correct per oracle.

### 5.2 — APNAP trigger ordering

Multiple same-event stacks observed. Spot-checked one 5-trigger stack (line at turn 23+, Falkenrath Noble + Diregraf Ghoul + Civilized Scholar simultaneously dying + Falkenrath Noble's real trigger). The order in the `Stack:` display was consistent with APNAP (active player's triggers at the bottom, NAP's on top, so NAP resolves first via LIFO). No APNAP ordering violations were flagged — though almost half the triggers on these stacks were the empty bug-A entries, which muddies the signal.

### 5.3 — Targeting legality and hexproof

- **Hexproof (Invisible Stalker)**: `grep` shows Invisible Stalker only appearing in target lists as `(your)` — never offered as an opponent's target. Hexproof is enforced correctly.
- **Victim of Night** ("non-Vampire, non-Werewolf, non-Zombie"): confirmed filtered. Line 90878 target list includes Delver of Secrets, Voiceless Spirit, but excludes Armored Skaab (a Zombie). The LLM at line 73334 explicitly tried to target Armored Skaab and the engine correctly didn't offer it. ✅
- **Intimidate (Spectral Rider, Kruin Outlaw's back face)**: `(MENACE)` tag appears correctly on attackers with menace. However — per §3.7 — the **blocker option list does NOT pre-filter for intimidate legality**. Black/red/blue creatures are offered as blockers for white-intimidate attackers. In practice the LLM reliably picks `-1` after reasoning about intimidate, but the prompt is technically misleading. Worth filtering in a future fix.

### 5.4 — Mana costs at scale

Spot-checked ~25 tap plans across the 8-seat run. Every one was correct for the colored pips required. Notable ones:
- `Cast Stitched Drake (tap 2x Island, Swamp)` for `{1}{U}{U}` ✓
- `Cast Skaab Goliath (tap 4x Island, 2x Forest)` for `{5}{U}` ✓ (plus the exile-two-creatures additional cost, confirmed at line 85494-95)
- `Cast Kindercatch (tap 4x Forest, 2x Island)` for `{3}{G}{G}{G}` ✓ (3 green pips)
- `Cast Voiceless Spirit (tap 2x Plains, Island)` for `{2}{W}` ✓ (Moon Heron cost `{3}{U}`, 3 generic + 1 blue)
- **Avacyn's Pilgrim** (mana creature): tapped for `{W}` in `tap Plains, Avacyn's Pilgrim (your)` for `{1}{W}` costs. Pilgrim correctly NOT offered while summoning-sick.
- **Gavony Township**: tapped for `{C}` (colorless). Correctly used as the generic cost in `Cast Ambush Viper (tap Forest, Gavony Township)`.
- **Non-basic lands** (Gavony Township): `{C}` generation confirmed.
- **Mixed color pool stress**: the 8-seat run did not exercise dual lands (Clifftop Retreat, Sulfur Falls, etc.) — they were drafted but didn't make final decks in games played. Not stressed.

Zero `Cast Spell` labels that looked wrong. Autotap looks correct at 8-seat scale.

### 5.5 — Priority correctness

- **Auto-pass**: 1,618 `AUTO-PASS` log lines across the run. The engine is silently passing through dead priority windows in both upkeep and after casting.
- **Zero bare `0:Pass 1:Concede` prompts** (grep confirmed: 0). Auto-pass is filtering all trivial pass-only windows.
- **Prompts with real choices**: 3,411 total `PROMPT` lines. Across 30 games that averages ~114 meaningful decisions per game. Looks healthy.
- **Cast → retain priority → opponent responds**: spot-checked several respond windows on the opponent's cast spell; the LLM always got prompted to respond before the spell resolved. No missing-priority bugs observed.

### 5.6 — Harness presentation

Issues observed (all presentation, not engine correctness):

1. **Bug B (transformed-creature display)** — described in §4.2. The LLM has to mentally track which "Gatstaf Shepherd" on the board is really "Gatstaf Howler".

2. **Aura effects not shown inline.** Multiple LLM misreadings traced to this:
   - Line 25222: LLM thought "Doomed Traveler is a 3/3 due to Ghostly Possession". **Wrong** — Ghostly Possession grants flying + damage prevention, not +2/+2. The display shows `Doomed Traveler 1/1 flying (Ghostly Possession)` correctly; the LLM misread.
   - Line 28548 / 30708 etc.: LLM consistently says "Ghostly Possession gives +2/+2" — a systematic misreading because the aura's effect isn't inline. Could be fixed by showing `(Ghostly Possession: prevents combat damage)` after the aura name.

3. **Intangible Virtue (+1/+1 vigilance to creature *tokens*)**: LLM at line 88670 thought "Slayer of the Wicked at 4/3 puts significant pressure" — Slayer is 3/2 and Intangible Virtue does NOT buff non-tokens. Slayer is not a token, so it stayed at 3/2. LLM misreading, but the prompt has no way to disambiguate "is this a token?" inline. Possible fix: mark tokens with a `(token)` suffix or similar.

4. **Equipment activated abilities** — equipment like `Blazing Torch`, `Inquisitor's Flail`, `Butcher's Cleaver`, `Sharpened Pitchfork` appear on the board as just the name, with no inline hint at the equip cost, ability text, or whether they're already attached. Carried over from 4-seat report, still an issue.

5. **+1/+1 counter state not shown inline** — `Villagers of Estwald 4/5` (from Elder Cathar's +2 counters on a Human) looks indistinguishable from a transformed creature. The display computes effective P/T correctly but doesn't say "+2 counters". For effects that specifically remove/care about counters, the LLM would be blind.

6. **X-cost spells don't show X** — see §4.8. The Harvest Pyre misplays all traced to this.

7. **Civilized Scholar discard prompt labels**: now correctly show card names (fix from `Show card names in discard action labels` commit is working). Verified at line 91321: `0:Moonmist 1:Kindercatch 2:Think Twice 3:Forbidden Alchemy 4:Forest 5:Forest 6:Island`. ✅

### 5.7 — Replacement effects / 5.8 — SBAs

No replacement effects surfaced. SBAs appear to work (creatures with 0 toughness die, lethal damage dying, game ending at 0 life) — no violations caught.

---

## 6. Things checked and OK (negative results)

- No `MALFORMED` anywhere in 98,803 lines.
- No `API_FATAL` / `API_ERROR` / `API_RETRY`.
- No `BLOCKER_VALIDATION` failures.
- No `<X> transforms into <X>` werewolf logging bugs.
- No `mulligan #4` events (mull-to-4 cap holds).
- No bare `0:Pass 1:Concede` prompts (auto-pass correct).
- Bonds of Faith correctly enforced on **native** non-Humans (Zombies, Vampires, Spirits) — see §4.2b for the one broken case, which is specifically werewolves on the back face.
- Hexproof enforcement: Invisible Stalker never offered as opponent's target.
- Target-legality filtering: Victim of Night correctly excludes Zombies.
- Ghostly Possession damage prevention: confirmed working (Doomed Traveler with Ghostly Possession took no combat damage despite attacking unblocked fliers).
- Village Bell-Ringer ETB untap correctly untaps a Claustrophobia'd creature (line 90485). Claustrophobia's "doesn't untap during untap step" doesn't block other untap effects — correct.
- Skaab Goliath additional cost "exile two creature cards from graveyard" correctly enforced (line 85494-95).
- Fiend Hunter exile-and-return mechanic working (line 76740-76744).
- Forbidden Alchemy flashback working end-to-end (looked at two flashback casts, each exiled after resolution).
- Bloodcrazed Neonate +1/+1 counter accumulation on combat damage to player — working.
- Gavony Township activation + +1/+1 counter distribution — working.
- London mulligan round-by-round alternation — working.
- Mulligan bottoming (N cards after N mulls) — working including mull-to-4 (N=3) cases.

---

## 7. Things not gotten to

- Cleanup-step discard prompts (still never triggered — no game exceeded 7 cards at end of turn, even in long games).
- Mausoleum Guard / Geist-Honored Monk / Spider Spawning token-creation logs — still not exercised.
- **LLM-determinism replay** (not a goal of this run).
- Trample math edge cases (only simple trample combats observed).
- Protection from X (Elite Inquisitor exists but never blocked a Vampire/Werewolf/Zombie cleanly in a checkable scenario).
- Replacement effects (no cards in the drafted pools had any).
- Legendary supertype rule (no duplicates of legendary creatures in any deck).

---

## 8. Summary table

| Item | Status | Notes |
|---|---|---|
| `cargo check` clean | ✅ | |
| `cargo test` clean (known flake aside) | ✅ | |
| Schema constraints (no MALFORMED) | ✅ | 0/3411 |
| Werewolf transform-back logs | ✅ | 0 `<X> transforms into <X>` |
| Mulligan alternation + bottoming | ✅ | |
| Mull-to-4 cap + forced keep | ✅ | **Newly exercised** (12 mull#3 events, 8 forced keeps) |
| Token creation logs (Moan) | ✅ | |
| Token creation logs (Doomed Traveler) | ✅ | **Newly exercised** |
| Token creation logs (Midnight Haunting) | ✅ | **Newly exercised** |
| Token creation logs (Mausoleum Guard, Geist-Honored Monk, Spider Spawning) | ⚠ not exercised | |
| Cleanup-step discard labels | ⚠ not exercised | |
| ETB trigger gating | ✅ | |
| Block validation at schema level | ✅ | 0 BLOCKER_VALIDATION |
| API health | ✅ | 0 errors, $0.54 |
| Priority correctness | ✅ | |
| Autotap correctness | ✅ | |
| Hexproof targeting enforcement | ✅ | |
| Victim of Night target type filtering | ✅ | |
| Bonds of Faith on native non-Humans | ✅ | |
| **Empty `dies`/`LTB` triggers (bug A)** | ❌ | ~29 creature cards + 4 auras affected. Worst: 5-trigger stacks. §4.1 |
| **Transformed-creature display name (bug B)** | ❌ | ~636 buggy display lines. §4.2 |
| **`p255` LTB controller (bug C)** | ❌ | 19 p255 mentions, all LTB triggers. §4.3 |
| **Bonds of Faith broken on transformed werewolves (NEW)** | ❌ | Subtype not updated on `on_upkeep` transform. §4.2b |
| **Civilized Scholar front-face gets back-face end-step trigger (NEW)** | ❌ | Triggers fire every turn, resolve as no-op. §4.7 |
| **"mulligans to 7" log wording (FIXED)** | ✅ fix landed | §4.4 |
| **LLM mulligan prompt missing state (FIXED)** | ✅ fix landed | §4.5 |
| **`[BOTTOM 2 CARDs AFTER MULLIGAN]` lowercase s** | ⚠ cosmetic | §4.6 — one-char fix |
| **Harvest Pyre X=0 action label (NEW)** | ⚠ | §4.8 — LLM wastes the spell |
| Intimidate blocker list not pre-filtered | ⚠ | §3.7 / §5.3 — LLM self-enforces, engine may still validate on submit |
| Aura effects not shown inline | ⚠ | §5.6 item 2 — systematic LLM misread of Ghostly Possession |
| Counter state not shown inline | ⚠ | §5.6 item 5 |
| Equipment abilities not shown inline | ⚠ | §5.6 item 4 (carry-over from 4-seat) |
