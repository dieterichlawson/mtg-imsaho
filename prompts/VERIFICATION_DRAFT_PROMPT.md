# Task: Run a verification draft and hunt for engine bugs

You're working in `/Users/dlaw/mtg`, a Rust workspace implementing a Magic: The Gathering engine plus LLM AI players. Another agent recently landed a set of fixes (detailed below), and the prior agent also landed London mulligans and an ETB trigger filter. Your job is to run a **4-player Innistrad draft tournament**, verify those fixes work end-to-end, and hunt aggressively for engine bugs we haven't found yet.

This is a real audit, not a smoke test. Take it seriously — the output feeds a follow-up experiment (see `DRAFT_ELO_EXPERIMENT_PLAN.md`) that depends on the engine behaving correctly.

---

## Context: what changed recently

Read these commit messages before starting so you know what to verify:

```
git log --oneline -10
```

Expected recent commits (verify these exist):

1. `Constrain LLM action and target indices via enum schemas` — replaced unbounded action/target schemas with enum-constrained JSON schemas. Now the LLM cannot return an out-of-range index. All decision points (main action picker, single-target spells, multi-target spells, mulligan bottoming) go through `pick_action_index`. The blocker code was already correct and is the reference pattern.
2. `Alternate mulligan rounds between players, expose run_mulligan_phase` — rewrote the keep/mull sub-phase so both players alternate within a round rather than one player draining their decisions first. Also exposes `run_mulligan_phase` publicly for test isolation.
3. `Log token creation for previously-silent token creators` — Doomed Traveler, Mausoleum Guard, Midnight Haunting, Moan of the Unhallowed, Geist-Honored Monk now emit a state.log line when they create tokens.
4. `Use current face name as source in werewolf transform-back logs` — 12 werewolf card files now correctly use the current face's name on the source side of transform log lines (no more "Reckless Waif transforms into Reckless Waif").
5. `Show card names in discard action labels` — discard actions now say `Discard <CardName>` instead of identical `Discard 1 cards` strings.
6. `Add London mulligans and surface legal blocks to combat prompt` — prior agent's work, adds the full London mulligan phase with cap at mull-to-4, and adds a `legal_blocks` map to the `ChooseBlockers` combat prompt.
7. `Gate ETB triggers on has_etb_handler` — prior agent's work, adds a CardBehavior::has_etb_handler method and gates ETB trigger collection on it so vanilla creatures don't put empty triggers on the stack.

You will verify all of these work in a real game, and you will look for new issues.

---

## Step 1: Baseline check

```
cargo check
cargo test
```

Both must pass with zero warnings and zero test failures. If either fails, STOP and report the failure — don't proceed with a broken baseline.

If `cargo test --test mulligan` fails intermittently, run it 5-10 times. The flake we patched was fixed by `run_mulligan_phase`, so it should be stable now. If you still see flakes, report them.

Run the mulligan tests specifically to confirm the alternation test passes:

```
cargo test --test mulligan
```

You should see 9 tests pass, including `keep_mull_decisions_alternate_round_by_round`.

---

## Step 2: Run the draft

```
cargo build --release -p mtg-draft-runner
./target/release/mtg-draft-runner \
  --set isd \
  --players 4 \
  --best-of 3 \
  --log verify-draft.log \
  --model gemini:gemini-3.1-flash-lite-preview:medium:medium
```

Notes on the flags:

- `--set isd` — Innistrad, the format we have cards for.
- `--players 4` — 4-seat draft, ~15 min per round, ~45 min total.
- `--best-of 3` — bo3 matches with the full mulligan phase per game.
- `--log verify-draft.log` — the log we're going to audit.
- `--model gemini:gemini-3.1-flash-lite-preview:medium:medium` — all 4 seats use the same model and thinking levels. Using the same level across seats keeps the logs easier to read — we're verifying behavior, not comparing thinking levels.

Run the draft in the foreground and wait for it to finish. Expect ~30-50 min wall clock and roughly $0.50 in API costs. Don't interrupt it. If it hangs for more than 5 minutes with no log activity, report that as a probable bug.

**If the API is rate-limited (HTTP 429) or has transient errors (HTTP 500/503), the runner has built-in retry — those are fine as long as they recover. Flag them if they cascade into failures.**

---

## Step 3: Verification checklist

After the draft finishes, audit `verify-draft.log` against this checklist. Use `grep`, `wc -l`, `head`, etc. as needed. For each item, either confirm it's working or report the issue with specific log lines.

### 3.1 — New schema constraints (MUST be clean)

- **Zero `MALFORMED` log entries.** Run `grep -c MALFORMED verify-draft.log`. The count must be 0. If any MALFORMED entries exist, read them carefully — they indicate the LLM returned something the parser couldn't handle, which should no longer be possible with enum-bounded schemas. An exception is the mulligan response-missing-field path (unlikely but possible) — if you see it, quote the full entry.
- **Zero out-of-range target defaults.** Look for any prompt where the LLM's thought says it wants to target one thing but the CHOSE line picks a different one. In particular, look for the pattern "the opponent's creature got buffed" / "my own creature got removed" — those indicate the old off-by-one behavior. There should be none.
- **Action picks match intent.** Sample 5-10 random decisions. Compare the LLM's THOUGHT entry to the CHOSE entry. They should be consistent. If you find a case where the thought describes one action but the chosen index was obviously a different one, flag it.

### 3.2 — Token creation logs (MUST be present)

For each of these cards, if it appeared in the log (grep for the card name in the state log), there should be a corresponding `created N ... tokens` log line nearby:

- Doomed Traveler → `"Doomed Traveler: created a 1/1 white Spirit token with flying"` on death
- Mausoleum Guard → `"Mausoleum Guard: created two 1/1 white Spirit tokens with flying"` on death
- Midnight Haunting → `"Midnight Haunting: created two 1/1 white Spirit tokens with flying"` on resolution
- Moan of the Unhallowed → `"Moan of the Unhallowed: created two 2/2 black Zombie tokens"` on resolution
- Geist-Honored Monk → `"Geist-Honored Monk: created two 1/1 white Spirit tokens with flying"` on ETB

Also verify Spider Spawning still logs `"Spider Spawning created N Spider tokens"` (regression check).

Report any instance where a card created tokens (visible by subsequent spirit/zombie tokens appearing on the board) but didn't emit the corresponding log line.

### 3.3 — Werewolf transform-back log lines (MUST be correct)

Grep for `transforms into` in the log. For each two-way DFC werewolf (Reckless Waif, Villagers of Estwald, Tormented Pariah, Village Ironsmith, Hanweir Watchkeep, Gatstaf Shepherd, Grizzled Outcasts, Daybreak Ranger, Ulvenwald Mystics, Mayor of Avabruck, Kruin Outlaw, Instigator Gang):

- Transform forward: `"<front> transforms into <back>"` — correct pattern
- Transform back: `"<back> transforms into <front>"` — correct pattern

Specifically confirm you do NOT see any `"<front> transforms into <front>"` patterns like `"Reckless Waif transforms into Reckless Waif"`. If you see the old buggy pattern, report it.

### 3.4 — Discard action labels (MUST be specific)

If any player ends a turn with more than 7 cards, there will be `[DISCARD 1 CARD]` prompts. Grep for `DISCARD` and find a few instances.

- Expected: each discard action option shows a specific card name, like `"0:Discard Grizzly Bears 1:Discard Mountain 2:Discard Doomed Traveler ..."`.
- Bug: all options identical, like `"0:Discard 1 cards 1:Discard 1 cards ..."`.

If discarding doesn't happen during the draft, note that the feature wasn't exercised but don't consider it a failure.

### 3.5 — Mulligan phase works end-to-end (MUST be present)

Grep for `Mulligan phase`, `p\d keeps`, `p\d mulligans to 7`, `bottomed`. Verify:

- Every game starts with a `Mulligan phase` log line.
- Both players decide keep/mull (look for two decisions per round).
- Mull-to-4 cap: if any player mulls 3 times, their 4th decision should be forced keep and they bottom 3 cards.
- Bottoming: after all keeps, any player who mulled gets a `BOTTOM N CARD{S} AFTER MULLIGAN` prompt and then a `bottomed N card...` log line.

**Round alternation**: This is the subtle one. Pick one game where both players mulled at least once and trace the sequence of keep/mull decisions. The order should be strict alternation:

```
Round 1: p0 decides, p1 decides
Round 2: (if anyone mulled in round 1) p0 decides (if not already kept), p1 decides (if not already kept)
...
```

You should NOT see one player make 3 decisions in a row followed by the other player making 3 decisions in a row. If you do, that's the old collapsed behavior and it's a bug.

### 3.6 — ETB triggers gated correctly (MUST be efficient)

Grep for `'s ETB trigger` in the log. For each hit, the source card should be one that actually has an ETB handler. Cards that should NOT produce ETB triggers:

- Vanilla creatures with no ETB effect (Grizzly Bears, Walking Corpse, etc.)
- Basic lands
- Most equipment / enchantments without ETB effects

If you see an ETB trigger line for a vanilla creature or basic land, that's a regression — the `has_etb_handler` filter isn't working.

### 3.7 — Legal blocker enforcement (NEW, MUST be correct)

The prior agent added a `legal_blocks` map to `ChooseBlockers`. Verify:

- The LLM player should now reject illegal block assignments (e.g. a non-flying creature trying to block a flying attacker without reach).
- Grep for `BLOCKER_VALIDATION` entries in the log. Their presence is NOT a bug — it just means the validation is doing its job. If you see a validation error, check whether the retry produced a valid assignment.
- Sample a few block decisions where flying/reach/intimidate matter and verify they're legal in real Magic (e.g. Spider Spawning spiders with reach CAN block flying creatures; a Grizzly Bears CANNOT block a Voiceless Spirit).

### 3.8 — API errors and cost

At the end of the log, look for the `TOKEN USAGE` summary block. Sanity check:

- Total calls: should be in the hundreds to low thousands (typical for a 4-seat draft with ~6 matches).
- Total cost: should be $0.10-$0.50 for Gemini 3.1 flash lite preview.
- Any `API_ERROR` / `API_RETRY` / `API_FATAL` entries should be transient (HTTP 429/500/503) with a recovery. If there's an `API_FATAL` that didn't recover, report it.

---

## Step 4: Hunt for new engine bugs

This is the open-ended part. The previous audit found Lost in the Mist was misimplemented (turned out to be wrong — it really is a 2-target counter + bounce permanent), Fiend Hunter was flagged wrongly (also correct per current oracle), and several real bugs (token logging, transform-back logs, discard labels, schema bounds). Your job is to find what we missed.

**Methodology: read full game logs.** Pick 2-3 complete games from the log and read them turn-by-turn. The engine log is structured as `── Turn N (pX) ──` sections followed by events. Don't just grep — actually read them and check whether each event matches real MTG rules.

### Ideas for things to look for

These are categories, not an exhaustive list. Use your knowledge of MTG rules and your ability to cross-reference `scripts/oracle_lookup.py` to verify card behavior. **Always verify oracle text via `python3 scripts/oracle_lookup.py lookup "Card Name"` before flagging a card as buggy.** Modern oracle text has been errata'd for many old cards, and your training data may be stale.

#### Combat math

- **Trample with blockers**: when a trampler is blocked by smaller creatures, excess damage should go through to the defending player. Check that the numbers work.
- **First strike / double strike**: first-strike creatures deal damage in a separate step before non-first-strike creatures. Does the log show two damage resolutions? Does a 2/2 first-striker kill a 2/2 non-first-striker without dying?
- **Deathtouch**: any damage from a deathtouch source is lethal. Does a 1/1 deathtouch creature correctly kill a 4/4 in combat?
- **Lifelink**: damage dealt by a lifelink source gains the controller that much life. Check the life total delta after combat.
- **Menace**: can't be blocked by fewer than two creatures. Does the engine enforce this?
- **Intimidate**: can't be blocked except by artifact creatures or creatures that share a color. Look for any block that violates this.
- **Multiple blockers**: when a creature is blocked by multiple creatures, the attacker's controller assigns damage. Does the log show damage distributed in the expected way?
- **"Blocked" vs "unblocked" state**: if all blockers of an attacker die to first-strike damage, does the attacker deal damage to the player or is it still considered blocked? (It should still be blocked but deal no damage to the creatures.)
- **Damage ordering**: in a complicated combat, the order of damage application matters if creatures have different stats. The engine should enforce MTG's damage-assignment-order rule or make it deterministic.

#### Triggered abilities

- **APNAP order**: when both players have triggers on the same event, the active player's go on the stack first (so they resolve last — LIFO). Look for events that trigger both players and check the ordering.
- **Triggers checking at the wrong time**: an "at the beginning of your upkeep" trigger should check conditions at the time it triggers, not at resolution. Example: a trigger that says "if X, do Y" should check X at trigger time.
- **ETB triggers referring to self**: cards like "when ~ enters, draw a card" should trigger with self as the source. Does the log show the source correctly?
- **Triggers that die to their own trigger**: e.g. a creature with "when ~ enters, deal 1 damage to ~" should die if it's a 1-toughness creature. Does the engine handle this?
- **Multiple copies of the same trigger**: two copies of Falkenrath Noble both triggering on the same death — do you see two separate trigger resolutions?

#### Replacement effects

- **Damage prevention**: is there any card that prevents damage? Does it work?
- **Enters tapped**: check-lands (Hinterland Harbor, etc.) enter tapped unless a condition is met. Does the engine enter them correctly on turn 1?
- **"Would die, exile instead"**: cards like Rest in Peace. Not sure if any are in this set, but check.
- **Double tokens**: Parallel Lives doubles token creation. If Parallel Lives is in play and a token creator resolves, do you see 2x the tokens?

#### State-based actions

- **Creatures with 0 or less toughness**: should die immediately. Does something with -1/-1 counters die the moment its toughness hits 0?
- **Legend rule**: if two legendary creatures with the same name are on the battlefield under the same controller, the owner chooses one to keep; the other goes to the graveyard. Unlikely to come up in draft but worth a check.
- **Aura without a valid enchant**: if the creature an Aura is attached to leaves the battlefield or stops being legal, the Aura should go to the graveyard. Does the engine enforce this?
- **Creatures with lethal damage**: should die. Damage resets at end of turn (cleanup). Check that damage is actually cleared.
- **Player loses at 0 or less life**: does the game end correctly?
- **Player loses from drawing with empty library**: does the game end when a player tries to draw from an empty library?

#### Priority and phase transitions

- **Missing priority pass**: after a spell resolves, does the active player get priority before moving to the next step?
- **Priority for the non-active player**: during the opponent's turn, does the non-active player get priority to cast instants?
- **Stack order**: LIFO — last spell cast resolves first. Verify.
- **Cleanup step discards**: if a player ends their turn with > 7 cards, they discard during cleanup. Does the discard happen in the right player's turn?
- **Step skipping**: some steps are auto-skipped if no one has anything to do. Is the skipping legitimate?

#### Targeting

- **Hexproof**: creatures with hexproof can't be targeted by opponent spells/abilities. A creature with hexproof should NOT appear in the target list for an opponent's removal spell. Verify with Invisible Stalker (which has hexproof).
- **Targeting own vs opponent**: cards with explicit "opponent" or "you" restrictions should enforce them.
- **Target legality at resolution**: if a target becomes illegal before resolution (e.g. the creature leaves the battlefield), the spell should fizzle or lose that target.
- **"Nonland" / "noncreature" filters**: cards that say "target nonland permanent" should not offer lands.
- **Protection**: if any creature has protection from a color, it should be immune to that color's spells/damage/enchanting/blocking.

#### Mana costs

- **Double-pip requirements**: a spell costing `{B}{B}` should not be castable with just 1 Swamp. Check that 2-swamp + 1-other hand can cast Walking Corpse but 1-swamp + 1-other cannot.
- **Generic cost with colorless mana**: generic costs can be paid with any mana. Does the engine handle this?
- **Auto-tap**: when the LLM casts a spell, the engine auto-taps lands. Verify that it taps the right number and color.
- **Insufficient mana**: if the LLM tries to cast a spell it can't afford, the engine should reject it (or it shouldn't be in the legal actions list).

#### Zones and object identity

- **Leaves battlefield triggers firing**: creatures with "when ~ leaves the battlefield" should trigger. Does Fiend Hunter's leave trigger fire and return the exiled creature?
- **Snapcaster Mage flashback**: does Snapcaster correctly grant flashback to the chosen instant/sorcery? Does the flashback cast then exile the card after resolution?
- **Cards returning to hand vs library vs graveyard**: check that cards end up in the right zone.
- **Token "return to hand"**: if a token is supposed to go to hand (via bounce), it should cease to exist instead. Check that bouncing a token with Silent Departure / Grasp of Phantoms works correctly.
- **Counters transferring when a creature is copied / transformed**: if a creature transforms, do its counters persist? (They should.)

#### Transform mechanics

- **Day/Night (if present)**: tracks spells cast per turn. Was it updated correctly by the mulligan phase? After mulligans, did the counters reset?
- **"At the beginning of each upkeep, if no spells were cast last turn"**: is "last turn" the same player's last turn, or the previous turn regardless of whose it was? (It's the previous turn.) Verify Reckless Waif transforms on the correct upkeeps.
- **Transform stats**: after transform, does the creature's P/T / keywords / subtypes update?
- **"Until X" effects on transformed creatures**: does an Aura on the front face survive transformation? (Yes, auras persist.) Does an enchantment's grant of a keyword persist?

#### Mulligans (new feature)

- **Mull-to-4 cap**: after 3 mulligans, the next keep is forced. Verify the forced-keep was logged and the player bottomed 3 cards.
- **Alternation within rounds**: confirmed above in 3.5, but if you see any weirdness in the mulligan phase logs, flag it.
- **Bottoming validity**: the cards bottomed should come from the player's hand, not somewhere else. Library size should increase by the bottom count.
- **Bottoming order**: the first card in the bottom list should end up bottom-most in the library. Hard to verify from logs alone.
- **Cap edge cases**: what if a player bottoms all 7 cards (mulligan to 0)? The cap prevents this but there might be an off-by-one.

#### Logging completeness

- **Missing events**: look for turns where a player's actions are summarized but intermediate events are missing. For example, a land was played but no tap was logged, or a creature died but no trigger fired.
- **Event ordering**: do trigger resolutions come AFTER the triggering event in the log?
- **Life total correctness**: track life totals across a game. Do they match the damage + gain events? Off-by-one errors here are common in engine development.

---

## Step 5: Write a report

Write a final report that lists:

1. **Baseline status**: did `cargo check` and `cargo test` pass cleanly? Any flakes on repeated runs?
2. **Draft run status**: did the draft complete? Total wall clock, total cost, any API errors.
3. **Verification checklist results** (3.1 through 3.8): for each item, confirm pass or describe the issue. Be specific — quote log lines.
4. **New bugs found**: for each new bug:
   - What you observed (with specific log lines)
   - Why you think it's wrong (cross-referenced to oracle text via `oracle_lookup.py` if it's a card bug)
   - Severity (correctness, logging-only, UX, edge case)
   - A suggested fix or who/where to look in the code
5. **Things you checked and are OK**: things you verified to rule out bugs. Don't skip this — negative results are valuable.
6. **Things you didn't get to**: if you ran out of time or compute, say what you would have checked.

Save the report as `VERIFICATION_REPORT.md` in the repo root.

---

## Rules of engagement

- **Verify oracle text before flagging card bugs.** `python3 scripts/oracle_lookup.py lookup "Card Name"`. If the cache says the engine matches current oracle text, it's not a bug regardless of what you remember. The prior audit wrongly flagged Fiend Hunter and Lost in the Mist for this reason.
- **Don't "fix" anything.** Your job is to find issues, not resolve them. Document them and leave the code alone.
- **Don't skip the draft.** The whole point is to run the engine end-to-end and find issues the unit tests miss. If the API is misconfigured and you can't run it, STOP and report that rather than making something up.
- **Don't delete or commit log files.** The `verify-draft.log` stays untracked; leave it. Same for any draft_N.log, results.json, etc.
- **Be thorough but bounded.** Don't try to audit every single decision in thousands of LLM calls. Sample strategically: read a full game, then spot-check. If you find a bug, investigate how widespread it is.
- **Flag uncertain issues as "possible bug, needs verification"** rather than claiming them as definitive. The prior audit's Fiend Hunter / Lost in the Mist debacle happened because the auditor was too confident.

## Where to start

1. Read the recent commits (`git log --oneline -10`) and one or two commit diffs for the substantive fixes.
2. `cargo check && cargo test` — baseline.
3. Run the draft. While it runs, re-read this prompt and `DRAFT_ELO_EXPERIMENT_PLAN.md` for context.
4. When the draft finishes, work through the verification checklist in order.
5. Then do the open-ended bug hunt.
6. Write `VERIFICATION_REPORT.md`.

Ask if anything in the spec is ambiguous before running the draft — a failed run is ~45 min of wasted compute.
