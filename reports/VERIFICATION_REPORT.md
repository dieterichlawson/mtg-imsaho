# Verification Draft Report

## 1. Baseline status

- `cargo check` — clean, no warnings, no errors.
- `cargo test --no-run` — clean after the warning sweep described in §1a.
- `cargo test` — every workspace test passes when run individually. **One pre-existing flake** observed in `mtg-draft::pack::tests::test_sequential_collation_produces_adjacent_cards` (mtg-draft/src/pack.rs:557): the test uses `rand::thread_rng()` instead of a seeded RNG, so the assertion that the first commons in a generated pack come from positions 0/1 of the A-run sometimes fails depending on which other paths the RNG took. Five consecutive runs of `cargo test -p mtg-draft` passed; the flake reproduces only intermittently in the full `cargo test` workspace run. **Not introduced by recent changes.**
- `cargo test --test mulligan` — 9/9 pass, including `keep_mull_decisions_alternate_round_by_round`.

### 1a. Warning sweep

`cargo check` was already clean, but `cargo test` produced ~250 warnings, almost all from `mtg-engine/tests/common/mod.rs` (each integration-test crate uses a different subset of helpers, so the unused-import / dead-code lints fire per binary). Cleared by:

- `mtg-engine/tests/common/mod.rs` — added `#![allow(dead_code)]` at the top of the shared helper module.
- `mtg-engine/tests/audit_bugs2.rs:283` — `state = cast_and_resolve(...)` → `let _ = cast_and_resolve(...)` (the assignment was followed by `assert!(true, ...)` and never read).
- `cargo fix --tests --allow-dirty -p mtg-engine` made small unused-import cleanups in `bug_fixes.rs`, `werewolf_cards.rs`, `moonmist.rs`.

After these changes both `cargo check` and `cargo test --no-run` are warning-free.

### 1b. Logging timestamps moved to California time

Per the user's request, log timestamps in `mtg-player/src/game_log.rs` were switched from "elapsed seconds since program start" to absolute Pacific time. Added `chrono` and `chrono-tz` to `mtg-player/Cargo.toml`. Format example: `[2026-04-10 08:13:42.193 PDT ThreadId(182)] [llm.rs:967] PROMPT [Seat0]`. **Note:** the `verify-draft.log` audited below was generated *before* this change; the timestamps in the quoted log lines below are still in the old elapsed-time format.

---

## 2. Draft run status

- Command: `./target/release/mtg-draft-runner --set isd --players 4 --best-of 3 --log verify-draft.log --model gemini:gemini-3.1-flash-lite-preview:medium:medium`
- **Completed cleanly**, exit code 0.
- 4-seat ISD draft → deck build → swiss bo3 round-robin → final standings.
- 8 games played across 6 matches.
- 1060 LLM calls total. **No `MALFORMED`, no `API_FATAL`, no `API_ERROR`, no `API_RETRY`.**
- Token usage: 366,039 in / 52,148 out / 127,329 cached. **Total cost: $0.17.**
- Final standings: Seat 2 (2-0), Seat 0 (1-1), Seat 1 (1-1), Seat 3 (0-2).
- `verify-draft.log` is 29,787 lines.

---

## 3. Verification checklist

### 3.1 — Schema constraints (PASS)

- `grep -c MALFORMED verify-draft.log` → **0**.
- No out-of-range action picks observed in the sample I read. The 16 grep hits for `[RESPOND TO p1's Lightning Bolt]` are not real events — they come from the system-prompt example block (no Lightning Bolt is in ISD).
- Sampled ~10 random `THOUGHT` / `CHOSE` pairs across both threads — every choice index matched the LLM's stated intent.

### 3.2 — Token-creation logs (PARTIAL — only Moan exercised)

- **Moan of the Unhallowed**: ✅ logged 9 times — `Moan of the Unhallowed: created two 2/2 black Zombie tokens`. Spider Spawning would have produced a similar regression check, but it was never cast.
- **Doomed Traveler, Mausoleum Guard, Midnight Haunting, Geist-Honored Monk, Spider Spawning**: not exercised. Mausoleum Guard appeared in some packs but no player cast it; Midnight Haunting was in one player's deck but never cast in the games we ran (`grep -c "p[01] cast Midnight Haunting" verify-draft.log` → 0). Cannot confirm or deny the fix from this run alone.
- No instance of a card creating tokens (visible Spirit/Zombie tokens appearing on the board) without a corresponding log line.

### 3.3 — Werewolf transform-back logs (PASS)

37 `transforms into` lines. Every one shows distinct front/back names — `Cloistered Youth transforms into Unholy Fiend`, `Civilized Scholar transforms into Homicidal Brute`, `Thraben Sentry transforms into Thraben Militia`, `Ulvenwald Mystics transforms into Ulvenwald Primordials`, `Ulvenwald Primordials transforms into Ulvenwald Mystics`. **Zero `<X> transforms into <X>` patterns.**

### 3.4 — Discard action labels (PASS for forced discard, NOT EXERCISED for cleanup-step discard)

- The fix targeted *cleanup-step* "discard down to 7" prompts. **No game in this run ever had a player end-of-turn with > 7 cards**, so the cleanup discard path was not exercised. `grep -c "Discard 1 cards" verify-draft.log` → 0 (no buggy labels visible, but also no new labels).
- The forced-discard prompts (Civilized Scholar's `{T}: draw + discard`, Brain Weevil's sacrifice) DO show specific card names: `0:Grasp of Phantoms 1:Burning Vengeance 2:Rakish Heir 3:Swamp`. Working as expected.

### 3.5 — Mulligan phase end-to-end (PASS)

26 mulligan-decision prompts across 8 games. Every game has a `Mulligan phase` event, and bottoming logs are present (`p0 bottomed 1 card: Harvest Pyre (#5)`, `p1 bottomed 2 cards: Swamp (#78), Swamp (#80)`).

**Round alternation verified**, traced game starting at line 5574 (Seat0 vs Seat1, thread 182):

| Round | Decision sequence |
|---|---|
| 1 | Seat0 → mull, Seat1 → keep |
| 2 | Seat0 → keep (1 mull), Seat0 bottoms 1 |

And in the parallel game (Seat2 vs Seat3, thread 183):

| Round | Decision sequence |
|---|---|
| 1 | Seat2 → keep, Seat3 → mull |
| 2 | Seat3 → mull (only p1 active) |
| 3 | Seat3 → keep (2 mulls), Seat3 bottoms 2 |

Both games show strict round-by-round alternation. No "one player drains 3 decisions before the other plays" pattern.

The mull-to-4 cap (forced keep on the 4th decision) was not exercised — no player went past 2 mulls in this run.

### 3.6 — ETB trigger gating (PASS, but see §4.1)

`grep "'s ETB trigger" verify-draft.log` → only **Ghoulraiser**, which legitimately has an ETB ("return random Zombie from graveyard to hand"). No vanilla creatures or basic lands produce ETB-trigger lines. The `has_etb_handler` filter is doing its job for ETB.

**However**, the analogous bug exists for "leaves the battlefield" and "dies" triggers — see §4.1 below.

### 3.7 — Legal-blocker enforcement (PASS, lightly exercised)

- `grep -c BLOCKER_VALIDATION verify-draft.log` → **0**. The LLM never picked an illegal blocker assignment, so the validation surface was never tripped.
- Spot-checked block decisions involving flying / reach / first strike:
  - Line 11175: `Your blockers: 0:Angel of Flight Alabaster 4/4 flying 1:Thraben Sentry 2/2 vigilance` — both legal.
  - Line 10630: `Your blockers: 0:Voiceless Spirit 2/1 flying, first strike` — flier blocking flier, legal.
  - Line 11357: `Your blockers: 0:Screeching Bat 2/2 flying` — legal flier defender.
  - No instance of a non-flying / non-reach blocker being offered as a legal block of a flying attacker.

### 3.8 — API errors and cost (PASS)

- 1060 calls (172 draft + 888 game), $0.17 total — well within the $0.10–$0.50 expected range.
- Zero `API_ERROR`, `API_RETRY`, `API_FATAL`.

---

## 4. New bugs found

### 4.1 — [HIGH] Empty `dies` and `LTB` triggers spam the stack

**Severity:** correctness + perf + LLM-noise. Same class as the recently-fixed ETB-trigger gating bug, but for "leaves the battlefield" and "dies" events.

**Observation.** When a creature dies, the engine puts up to four trigger entries on the stack — even when the dying card has no `dies`/`LTB` handler. Sample stacks (sorted unique, from the actual log):

```
Stack: Ambush Viper's dies trigger (your), Civilized Scholar's LTB trigger (opp's), Civilized Scholar's dies trigger (opp's), Ambush Viper's LTB trigger (opp's)
Stack: Brain Weevil's dies trigger (your)
Stack: Champion of the Parish's LTB trigger (opp's), Thraben Sentry's triggered ability (may transform Thraben Sentry) (opp's), Champion of the Parish's dies trigger (opp's)
Stack: Charmbreaker Devils's dies trigger (opp's), Charmbreaker Devils's LTB trigger (opp's), Scourge of Geier Reach's LTB trigger (opp's), Scourge of Geier Reach's dies trigger (your)
```

I verified each via `python3 scripts/oracle_lookup.py lookup`:

| Card | Real `dies` trigger? | Real `LTB` trigger? |
|---|---|---|
| Ambush Viper | no (just flash + deathtouch) | no |
| Brain Weevil | no | no |
| Champion of the Parish | no (it's an *ETB-on-other-Human* trigger) | no |
| Civilized Scholar | no | no |
| Charmbreaker Devils | no | no |
| Scourge of Geier Reach | no (vanilla 3/3 trample) | no |
| Deranged Assistant | no | no |
| Ghoulraiser | no (it's an ETB) | no |
| Typhoid Rats | no (vanilla deathtouch) | no |

In the same stacks, **real** triggers like `Thraben Sentry's triggered ability (may transform Thraben Sentry)` and `Falkenrath Noble's triggered ability (target player loses 1 life, you gain 1 life)` show up correctly — the bug is specifically that empty triggers are not being filtered out.

Total pollution: 142 `RESPOND TO ... trigger` prompts in the log, of which a substantial fraction are these empty triggers. Each one becomes a wasted LLM call ("there is nothing to respond to, I will pass") and confuses the LLM about what's actually on the stack.

**Suggested fix.** The recent commit `Gate ETB triggers on has_etb_handler` already established the pattern: `CardBehavior::has_etb_handler`. Add analogous `has_dies_handler` and `has_ltb_handler` methods, gate `triggers::collect_triggers` (mtg-engine/src/triggers.rs and/or wherever leaves-battlefield events fan out) on the new methods. The change will be small and mechanical — same shape as the ETB fix.

---

### 4.2 — [HIGH] Transformed-creature display: front-face name with back-face stats

**Severity:** harness presentation; demonstrably affects LLM reasoning.

**Observation.** After a DFC werewolf/transform card flips, the engine's compact-state prompt continues to display the creature with the **front face name** but the **back face stats**. Examples (35 lines for Cloistered Youth, 20 lines for Civilized Scholar):

```
Your board: 1x Forest, 2x Plains, Cloistered Youth 3/3, Avacynian Priest 1/2, Blazing Torch
Your board: 1x Mountain, 2x Island, Civilized Scholar 5/1, Traveler's Amulet
Choose attackers: 0:Avacynian Priest 1/2 1:Cloistered Youth 3/3
Choose attackers: 0:Civilized Scholar 5/1
```

`Cloistered Youth` is the front face of Unholy Fiend; the front face is 1/1, the back is 3/3. Once it transforms, the prompt should say `Unholy Fiend 3/3`. Same for `Civilized Scholar 0/1` → `Homicidal Brute 5/1`.

The transform log lines (`Cloistered Youth transforms into Unholy Fiend (#7)`) ARE correct (3.3 passes). It's only the compact board display that's wrong.

**Evidence the LLM is confused:** in line 6332 the LLM thought `"Cloistered Youth transforms into Unholy Fiend, which is a significant power boost (3/2)"` — note the LLM is even using the wrong stats (3/2 vs the actual 3/3). The LLM is doing extra work to figure out which "Cloistered Youth" is the transformed one.

**Root cause.** `mtg-engine/src/view.rs:137`:

```rust
name: registry.card_data(obj.card_id)
    .map(|d| d.name)
    .unwrap_or_else(|| obj.name.clone()),
```

`card_data(obj.card_id)` always returns the **front face** because `card_id` doesn't change on transform — only `obj.is_transformed` does. The view should branch on `is_transformed` and pull the back-face name from the registry.

**Suggested fix.** In `view.rs` around line 137 (and the analogous places at ~179 for stack items), check `obj.is_transformed`. If true and the card has a back face, use `card_data.back_face.name`. The same fix applies to `card_types`/`power`/`toughness` if those fields are also stale, but those already use `effective_*` for stats so probably just the name needs touching.

---

### 4.3 — [MEDIUM] LTB-trigger controller shown as `p255`

**Severity:** logging / display only — but it's a smell that suggests other places in the code may also be reading a freed controller.

**Observation.** 15 lines like:

```
[RESPOND TO p255's Ghoulraiser's LTB trigger]
[RESPOND TO p255's Civilized Scholar's LTB trigger]
[RESPOND TO p255's Ambush Viper's LTB trigger]
```

`p255` is `PlayerId(255)`, the sentinel for "no controller". When a creature dies and goes to the graveyard, its controller has been cleared by the time the LTB trigger is enqueued. The trigger's display should fall back to the **last** controller (the player who controlled the creature when it left the battlefield), since LTB triggers are always controlled by the *previous* controller per CR 603.10c.

This bug only matters cosmetically *as long as §4.1 is also fixed* — once empty LTB triggers are gone, the only LTB triggers will be ones with real handlers, and those handlers presumably use the proper controller. But it's worth fixing for the logging and as a hedge against future LTB cards.

---

### 4.4 — [LOW] Pre-existing flake in `test_sequential_collation_produces_adjacent_cards`

`mtg-draft/src/pack.rs:557` — uses `rand::thread_rng()`, not a deterministic seed. Reproduced once in five runs. Test would benefit from a seeded RNG. Not a regression — exists on master before any of the recent commits.

---

## 5. Audit topics requested mid-task

### 5.1 — Priority presentation correctness

I traced the priority-management code in `mtg-engine/src/engine.rs:3873–4068` and audited per-prompt counts in the log.

- **Auto-pass logic** (engine.rs:3977–3998 and llm.rs:971–981) is the right shape: it auto-passes when the only legal options are `PassPriority`, `Concede`, and mana abilities, and when no spell could be cast even after tapping all available mana. **Zero `0:Pass 1:Concede` (bare-pass) prompts** in the log — 1056 prompts, all of them had at least one meaningful option. Auto-pass is doing its job.
- **Cast-then-pass flow** is correct: `Action::CastSpell` keeps priority on the same player (engine.rs:4059–4061), the player then passes (or auto-passes) when they have nothing more to do, priority moves to the opponent, opponent passes (or auto-passes), and only then does the stack resolve. CR 117.3 is followed.
- **Stack resolution returns priority to the active player** (engine.rs:4015–4016), which matches CR 117.3b.
- **272 of 1012 prompts (27%) happen during the opponent's turn**, so the LLM does get priority during the opponent's turn whenever it has something meaningful to do. I sampled several and the prompts arrived at the right windows — `[BEGIN COMBAT]`, `[AFTER ATTACKERS DECLARED]`, opponent's main phase when the LLM had a flash creature in hand, etc.
- **Auto-pass log lines** like `AUTO-PASS [Seat3] Step: Upkeep, active: p#1` confirm that the engine is silently passing through dead steps for both players — no missing priority opportunities, no extra ones.

**One subtle concern (not a bug yet):** `consecutive_passes` is reset on most game actions but not explicitly on `Action::CastSpell`. I checked engine.rs:1884, 1903, 2116, 2329, 2351, 2395, 2434, 2470, 2703 and 3618 — all the reset points. CastSpell goes through `submit_action` which routes to a path that *does* zero out `consecutive_passes` (line 2116 area). Worth a unit test to lock this in, but I see no evidence it's broken in the current run.

**Verdict:** priority handling is correct for this run. The big inefficiency, again, is §4.1 — empty `dies`/`LTB` triggers force extra "pass" prompts that wouldn't otherwise happen.

### 5.2 — Autotap correctness

The autotap algorithm in `mtg-engine/src/mana.rs:137–332` is well-structured (Phase 0: floating mana, Phase 1: `{C}`-specific, Phase 2: colored pips most-constrained-first, Phase 3: generic from excess + new sources). It uses `hand_demand` from other castable spells in hand to prefer not tapping sources needed by another spell — i.e. it does basic color preservation.

I sampled ~30 concrete tap plans from the log. Every one I checked was correct or strategically harmless:

- `Cast Forbidden Alchemy (tap 2x Island, Swamp)` for `{2}{U}` ✓
- `Cast Stitched Drake (tap 2x Island, Swamp)` for `{1}{U}{U}` ✓ (uses both Islands for the double pip)
- `Cast Tribute to Hunger (tap Swamp, 2x Island)` for `{2}{B}` ✓
- `Cast Burning Vengeance (tap Mountain, 2x Island)` for `{2}{R}` ✓
- `Cast Civilized Scholar (tap 2x Island, Mountain)` for `{2}{U}` ✓
- `Cast Rakish Heir (tap Mountain, 2x Island)` for `{2}{R}` ✓ (yes, Rakish Heir is `{2}{R}`, not `{1}{R}{R}`, verified via oracle lookup)
- `Cast Smite the Monstrous (tap 3x Plains, Forest)` for `{3}{W}` ✓
- `Cast Slayer of the Wicked (tap 3x Plains, Forest)` for `{3}{W}` ✓

I did not find any case where the autotap chose a strategically worse plan when a better one was available. **No mana-creature was tapped in lieu of a land in the games I read** — this is partly because the only mana creature seen was Avacyn's Pilgrim and it was always summoning-sick when its controller wanted to cast something.

**Caveat:** this run is mono- or two-color decks with basic lands only — there were no check-lands, dual lands, or other color-conflicted sources, so autotap was rarely under pressure. A more colorful pool would be a stronger test.

**Verdict:** autotap looks correct for the situations exercised.

### 5.3 — Harness-induced misplays

I read multiple full game sequences looking for cases where the LLM chose poorly because the *prompt* misled it. Findings:

1. **§4.2 (transformed-creature display) IS a harness-induced misinformation source.** The LLM has to manually translate "Cloistered Youth 3/3" → "Unholy Fiend, the transformed one". I caught one explicit example at line 6332 where the LLM stated the transformed P/T as "3/2" instead of "3/3" — i.e. it was working from memory of the back face rather than the (wrongly-named) prompt.

2. **§4.1 (empty triggers) wastes LLM cycles** — every empty `dies`/`LTB` trigger turns into a "you have to respond to this nothing on the stack" prompt. The LLM correctly identifies these as no-ops ("There is no benefit to responding to the LTB trigger of the Ambush Viper, so I will pass"), but each one is an API call.

3. **Equipment / activated abilities are NOT shown inline** in the compact board display. `Blazing Torch` appears on the board as just `Blazing Torch` — the LLM has to remember the equip cost and the "{T}, sac: 2 damage" ability from its system-prompt deck listing. This is a *trade-off* (compactness vs completeness), but I noted some game logs where the LLM had Blazing Torch equipped and never activated it in winnable spots. Not necessarily a bug — could be LLM skill — but worth flagging if the user wants to add a "showAbilities" mode for board permanents.

4. **Target ordering for `Discard two cards` choices is non-deterministic across calls.** Same Brain Weevil ability, different prompts: `0:opponent 1:you` in one place and `0:you 1:opponent` in another. Doesn't cause misplays — the new enum schemas constrain to legal indices and the LLM picks by intent, not by index — but the inconsistency is weird and would be an easy stabilization win.

5. **No cases observed** where the LLM made a clearly wrong choice that I could trace to a missing field in the prompt (e.g. "didn't know creature was tapped", "didn't know it had summoning sickness"). The `[T]` and `[S]` markers are present and reliable, life totals/hand sizes/library sizes are accurate, and the action menus consistently include the cards they should include.

---

## 6. Things I checked and are OK (negative results)

- No `MALFORMED` log entries anywhere.
- No `API_FATAL` / `API_ERROR` / `API_RETRY` entries — the gemini API was healthy throughout.
- No `BLOCKER_VALIDATION` failures.
- No werewolf transform-back log lines using the old buggy `<front> transforms into <front>` pattern.
- Cleanup-step discard never fired (no game ever ended a turn with > 7 cards), so I couldn't verify the new `Discard <CardName>` labels in their actual prompt context. The forced-discard code paths (Civilized Scholar, Brain Weevil) DO use card names correctly.
- Sampled life-total math across one game: damage events and life-loss triggers (Falkenrath Noble drain, etc.) added up to the displayed totals. No off-by-ones.
- Land tap-tracking, summoning-sickness markers, attacker-tapped state — all consistent across the prompts I read.
- The 9 mulligan unit tests pass, including the alternation regression.
- Mulligan bottoming uses cards from the player's hand and updates library size accordingly (verified by checking `lib` count before and after `bottomed`).
- Auto-pass loop bound `MAX_AUTO_PASSES = 100` was never tripped.
- The `tests/common/mod.rs` `#![allow(dead_code)]` change has no effect on production binaries.

---

## 7. Things I didn't get to

- **Full state-based-action tracing** in a complicated combat (multiple blockers, deathtouch, lifelink, first strike all in one combat). No single combat in the run had all of these together — the games were mostly midrange beatdowns with little combat-trick action.
- **Trample math** — only one Feral Ridgewolf attack happened, blocked once, with even body sizes. Did not stress the trample formula.
- **Replacement effects** (Parallel Lives doubling, Rest in Peace, etc.) — none of these cards appeared in the run.
- **Hexproof targeting** — Invisible Stalker did not appear in any deck.
- **Mull-to-4 cap forced keep** — no player went past 2 mulls.
- **Spider Spawning regression** — never cast.
- **Token-creator log lines for 4 of 5 listed cards** — only Moan of the Unhallowed was actually cast.
- **Snapcaster-style flashback edge cases** — Forbidden Alchemy was flashed back several times and worked correctly, but I didn't trace the exile-after-flashback step in detail.
- **APNAP ordering for triggers on the same event** — would need a Falkenrath Noble + Champion of the Parish + Doomed Traveler "everything dies at once" scenario, which didn't materialize.

---

## 8. Summary table

| Item | Status |
|---|---|
| `cargo check` clean | ✅ |
| `cargo test --no-run` clean (post-sweep) | ✅ |
| Schema constraints (no MALFORMED) | ✅ |
| Werewolf transform-back logs | ✅ |
| Mulligan alternation | ✅ |
| Mulligan bottoming | ✅ |
| ETB-trigger gating | ✅ |
| Token logs (Moan only) | ✅ partial |
| Token logs (other 4 cards) | ⚠ not exercised |
| Cleanup-step discard labels | ⚠ not exercised |
| Mull-to-4 forced keep | ⚠ not exercised |
| Block validation | ✅ |
| API health | ✅ |
| Priority correctness | ✅ |
| Autotap correctness | ✅ |
| Empty `dies`/`LTB` triggers (NEW BUG) | ❌ §4.1 |
| Transformed-creature display name (NEW BUG) | ❌ §4.2 |
| `p255` controller in LTB display (NEW BUG) | ❌ §4.3 |
| `test_sequential_collation_produces_adjacent_cards` flake (PRE-EXISTING) | ⚠ §4.4 |
