# Task: Run an 8-seat verification draft and hunt for engine bugs

You're working in `/Users/dlaw/mtg`, a Rust workspace implementing a Magic: The Gathering engine plus LLM AI players. Your job is to run an **8-player Innistrad draft tournament** and audit the resulting log for engine and harness bugs.

A previous agent ran a 4-seat verification draft with the same model and found several issues. Your job is to:

1. Confirm those issues still reproduce (nothing has been fixed yet — they're tracked in `VERIFICATION_REPORT.md`).
2. Exercise the parts of the engine the 4-seat run *didn't* cover.
3. Find anything new.

This is a real audit, not a smoke test. The 8-seat run will produce roughly 4× the games of the 4-seat run, more diverse decks (more colors share each pack), and longer tournament rounds — exactly the conditions that should surface bugs the 4-seat run missed.

---

## Context: what's in the repo right now

First read these so you know the state of the code:

```
git log --oneline -15
cat VERIFICATION_REPORT.md           # the 4-seat audit, has the open bugs
cat DRAFT_ELO_EXPERIMENT_PLAN.md     # explains why we care about a clean engine
```

The recent fixes you should verify still hold:

1. `Constrain LLM action and target indices via enum schemas` — should yield zero `MALFORMED` entries.
2. `Alternate mulligan rounds between players, expose run_mulligan_phase` — round-by-round alternation in the keep/mull phase.
3. `Log token creation for previously-silent token creators` — Doomed Traveler, Mausoleum Guard, Midnight Haunting, Moan of the Unhallowed, Geist-Honored Monk should each emit a `created N ... tokens` log line. **The 4-seat run only exercised Moan of the Unhallowed; the other four were drafted but never cast.** With 8 seats and more games, expect at least some of the others to actually be cast.
4. `Use current face name as source in werewolf transform-back logs` — `<back face> transforms into <front face>`, never `<front> transforms into <front>`.
5. `Show card names in discard action labels` — cleanup-step discard prompts should show card names. **Not exercised at all in the 4-seat run** (no game ended a turn with > 7 cards). With 8 seats, longer games are more likely; if a player ever has > 7 cards at end of turn, this triggers.
6. `Add London mulligans and surface legal blocks to combat prompt` — full London mulligan with mull-to-4 cap, plus a `legal_blocks` map exposed to the LLM.
7. `Gate ETB triggers on has_etb_handler` — vanilla creatures and basic lands should not produce `ETB trigger` lines.

The fixes for the warning sweep, the California-time log timestamps, and the Cargo.toml additions of `chrono` / `chrono-tz` are already in. **Both `cargo check` and `cargo test --no-run` should be warning-free** when you start.

---

## Known open bugs (do not fix — verify they still reproduce)

The 4-seat run found three new bugs. They have NOT been fixed. Verify each one is still present in your 8-seat run, and gather more examples — especially edge cases.

### Open bug A — Empty `dies` / `LTB` triggers spam the stack

When a creature dies, the engine puts up to four trigger entries on the stack — even when the dying card has no `dies`/`LTB` handler. Same shape as the ETB-trigger bug that was recently fixed via `has_etb_handler`, but for the leaves-battlefield path.

Reproduction grep:

```
grep "Stack:" verify-draft-8seat.log | grep -E "dies trigger|LTB trigger" | sort -u
```

Look for `Stack: <CardName>'s dies trigger` and `Stack: <CardName>'s LTB trigger` entries where the card has no real death/LTB ability. Verify each suspicious card via `python3 scripts/oracle_lookup.py lookup "Card Name"` before flagging.

Carry-over from the 4-seat run: confirmed empty triggers for Ambush Viper, Brain Weevil, Champion of the Parish, Civilized Scholar, Charmbreaker Devils, Scourge of Geier Reach, Deranged Assistant, Ghoulraiser, Typhoid Rats. With 8 seats and more cards in play, you should see *more* of these and possibly find new offenders.

### Open bug B — Transformed-creature display name

After a DFC transforms (Cloistered Youth → Unholy Fiend, Civilized Scholar → Homicidal Brute, Thraben Sentry → Thraben Militia, Ulvenwald Mystics → Ulvenwald Primordials, etc.) the compact-state prompt continues to display the **front face name** with the **back face stats**. The LLM sees `Cloistered Youth 3/3` instead of `Unholy Fiend 3/3`. Root cause is `mtg-engine/src/view.rs:137`.

Verify:

```
grep -E "Cloistered Youth 3/3|Civilized Scholar 5/1|Thraben Sentry 5/4|Ulvenwald Mystics 5/6" verify-draft-8seat.log
```

The transform *log lines* are correct (`Cloistered Youth transforms into Unholy Fiend`); only the board-state display is wrong.

In the 4-seat run I found one explicit example (line 6332) where the LLM stated the wrong P/T (3/2 instead of 3/3) for the transformed creature in its `THOUGHT`. **Look for more cases** where the LLM clearly gets the transformed creature's identity or stats wrong because of the prompt.

### Open bug C — `p255` controller in LTB-trigger display

LTB-trigger prompts show the controller as `p255` (the `PlayerId(255)` "no controller" sentinel) because by the time the trigger fires, the creature is in the graveyard. Per CR 603.10c, LTB triggers should be controlled by the *previous* controller.

```
grep "p255" verify-draft-8seat.log
```

This is mostly a logging cosmetic, but worth confirming it still happens and counting how often.

---

## Step 1: Baseline check

```
cargo check
cargo test
```

Both should pass with **zero warnings and zero errors**. If any new warnings appear, STOP and report them — the previous run cleaned them up so a regression here means a recent commit reintroduced one.

There is **one known pre-existing flake** in `mtg-draft::pack::tests::test_sequential_collation_produces_adjacent_cards` (uses `rand::thread_rng()`, not a seeded RNG). If `cargo test` fails only on that test, run `cargo test -p mtg-draft` a few times to confirm it's the same flake; do not treat it as a regression.

The mulligan tests should be fully stable:

```
cargo test --test mulligan
```

You should see 9 passing tests including `keep_mull_decisions_alternate_round_by_round`.

---

## Step 2: Run the draft

```
cargo build --release -p mtg-draft-runner
./target/release/mtg-draft-runner \
  --set isd \
  --players 8 \
  --best-of 3 \
  --log verify-draft-8seat.log \
  --model gemini:gemini-3.1-flash-lite-preview:medium:medium
```

Notes on the flags:

- `--players 8` — 8 seats. Roughly **4× the games** of the 4-seat run. Expect ~2.5–4 hours wall clock (the 4-seat run took ~25 min for 8 games; the 8-seat Swiss tournament will run ~24–32 games).
- `--best-of 3` with the full London mulligan per game. More games means more chances for the cleanup discard, mull-to-4 cap, and rare-card interactions to actually fire.
- `--model gemini:gemini-3.1-flash-lite-preview:medium:medium` — same level on every seat so the logs are easy to read. The point of this run is engine verification, not piloting comparison.

Run in the foreground and **wait for it to finish**. Expect roughly **$0.50–$1.20 in API costs** (4-seat was $0.17). Don't interrupt. If it hangs for more than 5 minutes with no log activity, report that as a probable bug.

The runner has built-in retry for HTTP 429 / 500 / 503. Those are fine as long as they recover. Flag them if they cascade to `API_FATAL`.

While the draft runs, re-read the carry-over bugs in this prompt and pull up `mtg-engine/src/view.rs:137` and `mtg-engine/src/triggers.rs` so you understand the fix shape for bugs A and B.

---

## Step 3: Verification checklist

After the draft finishes, audit `verify-draft-8seat.log` against this checklist. **Every item below was either confirmed working or marked "not exercised" in the 4-seat run** — your job is to push the "not exercised" items into the verified column, and reconfirm the working items at scale.

### 3.1 — Schema constraints (MUST be clean)

```
grep -c MALFORMED verify-draft-8seat.log         # expect 0
grep -c API_FATAL verify-draft-8seat.log         # expect 0
grep -cE "API_ERROR|API_RETRY" verify-draft-8seat.log
```

Any non-zero `MALFORMED` count is a regression — the schemas are supposed to make this impossible. Quote any matches in your report.

Sample 10 random `THOUGHT` / `CHOSE` pairs from across multiple games and confirm the chosen index matches the LLM's stated intent. The prior audit's off-by-one bugs were eliminated by the enum-bounded schemas, so this is a sanity recheck.

### 3.2 — Token creation logs (PUSH FOR EXERCISE)

For each card listed below, find casting events with:

```
grep -E "p[01] cast (Doomed Traveler|Mausoleum Guard|Midnight Haunting|Moan of the Unhallowed|Geist-Honored Monk|Spider Spawning)" verify-draft-8seat.log
```

For every cast, find the matching token-creation log line nearby:

| Card | Expected log line | Trigger |
|---|---|---|
| Doomed Traveler | `Doomed Traveler: created a 1/1 white Spirit token with flying` | on death |
| Mausoleum Guard | `Mausoleum Guard: created two 1/1 white Spirit tokens with flying` | on death |
| Midnight Haunting | `Midnight Haunting: created two 1/1 white Spirit tokens with flying` | on resolution |
| Moan of the Unhallowed | `Moan of the Unhallowed: created two 2/2 black Zombie tokens` | on resolution |
| Geist-Honored Monk | `Geist-Honored Monk: created two 1/1 white Spirit tokens with flying` | on ETB |
| Spider Spawning | `Spider Spawning created N Spider tokens` | regression check |

The 4-seat run only confirmed Moan of the Unhallowed (cast 9 times). With 8 seats and more games, expect at least 2–3 more of these cards to actually fire. **Report which ones you exercised and which you still couldn't.**

### 3.3 — Werewolf transform-back logs (MUST be correct)

```
grep "transforms into" verify-draft-8seat.log | grep -vE "GEMINI_THOUGHT|RESPONSE|THOUGHT"
```

For each werewolf DFC (Reckless Waif, Villagers of Estwald, Tormented Pariah, Village Ironsmith, Hanweir Watchkeep, Gatstaf Shepherd, Grizzled Outcasts, Daybreak Ranger, Ulvenwald Mystics, Mayor of Avabruck, Kruin Outlaw, Instigator Gang) plus the non-werewolf flippers (Cloistered Youth, Civilized Scholar, Thraben Sentry):

- Forward: `<front> transforms into <back>` — correct
- Backward: `<back> transforms into <front>` — correct
- **Bug:** any `<front> transforms into <front>` or `<back> transforms into <back>` line.

Quote any buggy lines in your report.

The 4-seat run saw 37 transform lines, all correct. With 8 seats and ~24+ games, you'll likely see 100+. **Especially try to find a Reckless Waif / Village Ironsmith / Hanweir Watchkeep transform-back** — those weren't observed in the 4-seat run and were the original target of the fix.

### 3.4 — Discard action labels (PUSH FOR EXERCISE)

The cleanup-step discard fix targeted the `[DISCARD N CARDS]` prompt that fires when a player ends their turn with > 7 cards. **The 4-seat run never triggered this** because no game lasted long enough.

```
grep -B2 -A3 "DISCARD" verify-draft-8seat.log | head -60
```

Look for cleanup discard prompts. Each option should show a specific card name like `0:Discard Grizzly Bears 1:Discard Mountain` rather than identical `0:Discard 1 cards 1:Discard 1 cards`.

If still not exercised in the 8-seat run, note it. The forced-discard prompts (Civilized Scholar's `{T}: draw + discard`, Brain Weevil's sacrifice ability) DO show card names in the 4-seat run — those are working.

### 3.5 — Mulligan phase (CONFIRMED working — recheck at scale + EXERCISE mull-to-4)

For each game, find the mulligan sequence:

```
grep -nE "Mulligan phase|p[01] keeps|p[01] mulligans|bottomed" verify-draft-8seat.log
```

Per game, verify:

- Both players make at least one keep/mull decision per round.
- Order of decisions strictly alternates within a round (p0 then p1, never p0 three times in a row before p1 even decides). The 4-seat run confirmed this in two specific games — your job is to confirm it across all 24+ games and find any game where it breaks down.
- Bottoming: any player who mulled at least once gets a `BOTTOM N CARD{S} AFTER MULLIGAN` prompt and a `bottomed N cards` log line.

**Mull-to-4 cap**: this was NOT exercised in the 4-seat run — no player ever went past 2 mulls. Specifically look for any `mulligan #4` event (which should not exist) and any `mulligans to 4` followed by a forced keep + bottom 3:

```
grep "mulligan #" verify-draft-8seat.log
```

The cap should make the 4th decision a forced keep — the player draws their 4 (effectively, after bottoming 3) and bottoms 3 cards. If you find a game where someone mulled 3 times, trace it carefully.

### 3.6 — ETB trigger gating (CONFIRMED working — re-verify)

```
grep "'s ETB trigger" verify-draft-8seat.log | sort -u
```

Each line's source card should have a real ETB ability. Cards that should NOT show ETB triggers: vanilla creatures (Grizzly Bears, Walking Corpse, Bonescythe Sliver, etc.), basic lands, equipment without ETB abilities. The 4-seat run only had Ghoulraiser's ETB firing, which is correct.

If you see an ETB trigger for a vanilla creature or basic land, **that's a regression** — the `has_etb_handler` filter broke.

### 3.7 — Block validation (LIGHTLY EXERCISED — recheck at scale)

```
grep -c BLOCKER_VALIDATION verify-draft-8seat.log
```

Validation entries are NOT a bug — they mean the engine caught an illegal block assignment. What you want to verify is that:

- Every BLOCKER_VALIDATION entry is followed by a successful retry.
- Sample a few combats with flying / reach / intimidate / menace and verify the legal blocker set is right per real MTG rules. Examples to look for:
  - Spider Spawning's spider tokens have reach → CAN block flying.
  - Grizzly Bears type creature → CANNOT block Voiceless Spirit.
  - A non-vampire/non-zombie blocker should be unable to block a creature equipped with Blazing Torch (the equipment grants "can't be blocked by Vampires or Zombies" — note this is *anti*-protection in this case; verify the flow).
  - Intimidate: a creature with intimidate can't be blocked except by artifact creatures or creatures sharing a color.
  - Menace: a creature with menace must be blocked by ≥2 creatures or not at all.

### 3.8 — API errors and cost (MUST be clean)

```
grep -nE "API_ERROR|API_RETRY|API_FATAL" verify-draft-8seat.log
```

Any `API_FATAL` that didn't recover is a bug. `API_RETRY` from transient 429/500/503 is fine as long as the retry succeeds.

Find the `TOKEN USAGE` summary at the end of the log and sanity check:

- Total calls: should be 2,500–4,500 (roughly 4× the 4-seat 1,060).
- Total cost: $0.50–$1.20 for `gemini-3.1-flash-lite-preview`.

---

## Step 4: Newly-required audit areas

These are the dimensions the 4-seat run couldn't fully cover. Read full game logs and check for each of them.

### 4.1 — Combat math at scale

**Multi-blocker combat with mixed body sizes.** When a single attacker is blocked by multiple creatures with different P/T values, the attacker's controller assigns damage in an order. Find a multi-blocker combat in the log and trace the damage assignment. Does it actually deal lethal damage in the right order? Does excess damage from a trampler reach the defending player?

**First strike vs non-first-strike.** A 2/2 first striker should kill a 2/2 vanilla in combat without dying itself. If any first-strike creatures show up in the run (e.g. Voiceless Spirit), trace their combats.

**Deathtouch + first strike.** A deathtouch first-striker kills its blocker before it can deal damage back. Look for such interactions.

**Lifelink.** Track life totals across a combat where a lifelink creature is involved (Falkenrath Noble drains 1 + gains 1, Markov Patrician has lifelink in some printings — check oracle).

**Trample.** Find a trample combat (Feral Ridgewolf, Hollowhenge Scavenger, Pitchburn Devils, Boneyard Wurm). When the blocker has less toughness than the trampler's power, excess damage should hit the defending player. Verify the math.

The 4-seat run had very little combat-trick action — the 8-seat run will give you bigger boards.

### 4.2 — Triggered ability ordering (APNAP)

When multiple triggers fire on the same event, the active player's triggers go on the stack first (so they resolve last — LIFO). Find an event that causes triggers for both players (most likely: a creature dying that triggers Falkenrath Noble for one player and Doomed Traveler / Champion of the Parish reactive triggers for another). Verify the stack order.

The 4-seat run did not surface this scenario. With 8 seats and Falkenrath Noble appearing in multiple decks, look for a multi-Noble death event.

### 4.3 — Targeting legality and hexproof

If Invisible Stalker (hexproof) shows up in any deck, verify that no opponent removal spell offers it as a legal target. Also verify the LLM is correctly told what spells can target what.

If any "target nonland permanent" or "target noncreature" spells appear, verify the offered target list excludes the disallowed types.

### 4.4 — Mana costs at scale

```
grep -E "Cast .*\(tap" verify-draft-8seat.log
```

Spot-check 20 tap plans across the run. The autotap algorithm in `mtg-engine/src/mana.rs:137` is structurally sound, but the 4-seat run didn't stress dual-color mana situations. With 8 seats, decks span more colors. Look for:

- A spell with double-colored cost like `{U}{U}` or `{B}{B}` cast with the right two basics.
- A spell where the autotap *could* tap a needed source for another spell in hand but chose otherwise (color preservation).
- A creature with a mana ability (Avacyn's Pilgrim, Birds of Paradise — only Pilgrim is in ISD) being tapped for mana when a land would have done. Mana creatures should be tapped *last*, after lands.
- Any case where Avacyn's Pilgrim is offered as `Tap` for the LLM despite being summoning sick.

### 4.5 — Priority correctness

The 4-seat run audit found priority handling correct:

- Auto-pass logic in `mtg-engine/src/engine.rs:3977` and `mtg-player/src/llm.rs:971` skips dead priority windows.
- Cast → retain priority → pass → opp gets priority follows CR 117.3.
- Stack resolution returns priority to the active player.
- Zero `0:Pass 1:Concede` bare-pass prompts in 1,056 LLM calls.

Re-verify at 8-seat scale:

```
grep -cE "^  0:Pass 1:Concede" verify-draft-8seat.log    # expect 0
grep -c "PROMPT" verify-draft-8seat.log                    # expect ~3000
```

Specifically watch for:

- An LLM prompt during the opponent's main phase where the LLM has nothing to do — should be auto-passed, not prompted.
- An LLM prompt for the LLM during its own combat damage step with no instants in hand — same thing.
- Any case where the opponent casts a spell and the LLM does NOT get a prompt to respond. That's a missing-priority bug.

### 4.6 — Harness presentation (info shown to the LLM)

The 4-seat run found bug B (transformed-creature display) and noted that equipment activated abilities aren't shown inline on the board (`Blazing Torch` appears as just the name; the LLM has to remember the equip cost from the deck listing). Look for additional cases where the LLM made a clearly wrong play that traces to missing or misleading info in the prompt:

- LLM not knowing a creature has summoning sickness when it does (or vice versa).
- LLM not knowing a creature is tapped when it is.
- LLM not seeing an aura attached to a creature.
- LLM not seeing a counter on a creature (Champion of the Parish accumulates +1/+1 counters from Human ETBs — verify the counters show up in the prompt as either elevated stats or an explicit counter list).
- LLM not knowing it has flashback available on a graveyard card (the `Flashback available:` line should be present).
- Discard / target prompts where the index ordering changes between calls in a way that confuses the LLM.

### 4.7 — Replacement effects (newly-relevant if certain cards appear)

If Parallel Lives or any token-doubler appears in a deck, verify token-creation effects produce 2× tokens. Not in ISD natively but worth a check if anything weird happens.

If any "if a creature would die, exile it instead" effect (Rest in Peace etc.) appears, verify it overrides death.

### 4.8 — State-based actions

- Creatures with 0 toughness should die immediately (verify with damage from Devil's Play, Harvest Pyre, Brimstone Volley etc.).
- Damage clears at end of turn (cleanup) — verify by tracing a creature that took damage but survived combat.
- Game ends at 0 life — verify the win condition fires the moment a player drops to ≤ 0.
- Game ends on draw from empty library — unlikely in a 24-game ISD pool, but if it happens, verify the loss condition.

---

## Step 5: Hunt for new bugs

Read 2–3 *complete* games end-to-end. The engine log is structured as `── Turn N (pX) ──` sections followed by events; pick a game and read it turn by turn. Don't just grep — check whether each event matches real MTG rules.

**Always verify oracle text via `python3 scripts/oracle_lookup.py lookup "Card Name"` before flagging a card bug.** Modern oracle has been errata'd for many old cards, and your training data may be stale. The previous audit incorrectly flagged Fiend Hunter and Lost in the Mist as buggy because of stale expectations — both turned out to be correct.

If you find something suspicious, classify it as **"possible bug — needs verification"** rather than asserting it as definitive. Categories of bugs the 4-seat run did NOT find but are possible:

- **Trigger fires at the wrong time** (e.g., a "beginning of upkeep, if X" trigger that checks X at resolution instead of trigger time).
- **Trigger sources incorrectly attributed** to the wrong controller (we already know LTB-trigger controllers are `p255`, but other triggers might also be misattributed).
- **Counter persistence across transformation** (DFC creatures should keep their +1/+1 counters when transforming).
- **Aura-on-DFC behavior** (an aura attached to the front face should persist after transforming).
- **Auras going to the graveyard when their target leaves** (state-based action).
- **Legend rule** (unlikely in draft but worth a check if any legendary lands or creatures show up in dual copies).
- **Snapcaster-style flashback** (Forbidden Alchemy was flashed back in the 4-seat run and worked, but the exile-after-flashback step was not traced in detail; do that here).
- **Bouncing a token to hand** — if Silent Departure / Grasp of Phantoms ever targets a token, the token should *cease to exist*, not return to hand.
- **Transform stats / keywords / subtypes updating** correctly after a transform.

---

## Step 6: Write the report

Save your report as `VERIFICATION_REPORT_8SEAT.md` in the repo root. **Do not overwrite the existing `VERIFICATION_REPORT.md`** — that's the 4-seat audit and we want to compare.

Structure:

1. **Baseline status** — `cargo check` clean? `cargo test` clean? Any new flakes?
2. **Draft run status** — completed, wall clock, total cost, API errors.
3. **Verification checklist results** (3.1 through 3.8) — for each, pass/fail/not-exercised with specific log lines.
4. **Newly-exercised items** — explicitly call out which "not exercised" items from the 4-seat run you DID exercise (cleanup discard, mull-to-4 cap, the four token-creator cards, etc.).
5. **Open bugs A/B/C confirmation** — for each, confirm it still reproduces with new examples + counts. Note if any of them appear to have been fixed already.
6. **New bugs found** — for each:
   - What you observed (with specific log line numbers and quoted text).
   - Why you think it's wrong (cross-reference oracle text via `oracle_lookup.py` if it's a card bug).
   - Severity (correctness / logging-only / UX / edge case).
   - A pointer to where in the code to fix it, when you can identify one.
7. **Things you checked and are OK** — negative results are valuable.
8. **Things you didn't get to** — what would need a still-larger run.

---

## Rules of engagement

- **Verify oracle text before flagging card bugs.** `python3 scripts/oracle_lookup.py lookup "Card Name"`. If the cache says the engine matches current oracle, it's not a bug regardless of what you remember.
- **Don't fix anything.** Your job is to find and document, not to resolve. The user will decide which bugs to fix based on the reports.
- **Don't skip the draft.** The whole point is to run the engine end-to-end. If the API is misconfigured and you can't run it, STOP and report rather than synthesizing results.
- **Don't delete or commit log files.** `verify-draft-8seat.log` stays untracked; leave it. Same for any intermediate files.
- **Don't commit anything** unless explicitly asked.
- **Be thorough but bounded.** Don't try to audit every one of ~3,000 LLM calls — sample strategically. Read one full game in detail, then spot-check the rest. If you find a bug, investigate how widespread it is across the log.
- **Flag uncertain issues as "possible bug, needs verification"** rather than asserting them.
- **Cargo warnings: enforce zero.** The previous audit cleaned up the test-code warnings. If you see any new warnings from `cargo check` or `cargo test --no-run`, report them — they were not present at the start.
- **Run things in parallel where you can.** While the draft is running (which will take 2–3 hours), you can read code, pull oracle text for cards you expect to see, and re-read this prompt and the previous report. Don't sit idle.

---

## Where to start

1. `git log --oneline -15` and skim `VERIFICATION_REPORT.md` so you know which bugs are open.
2. `cargo check && cargo test` — baseline, must be clean.
3. Kick off the draft (Step 2) **in the foreground** and wait. While you wait, pull up `mtg-engine/src/view.rs` and `mtg-engine/src/triggers.rs` to understand the code paths for bugs A and B.
4. When the draft finishes, work the verification checklist (Step 3) in order.
5. Run the new audit areas (Step 4), focusing on the items the 4-seat run couldn't cover.
6. Read 2–3 full game logs end-to-end (Step 5).
7. Write `VERIFICATION_REPORT_8SEAT.md`.

Ask if anything in the spec is ambiguous **before** running the draft — a failed run is 2–3 hours of wasted compute and ~$1 of API.
