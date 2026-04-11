# Task: Schema constraints, missing logging, and mulligan follow-ups

You're working in `/Users/dlaw/mtg`, a Rust workspace implementing Magic: The Gathering. This task fixes a set of issues found while auditing logs from a recent draft tournament. Some are correctness bugs, some are missing logging that breaks log-based analysis, some are schema constraints that allow LLM piloters to emit out-of-range answers.

The full experiment plan that motivates these fixes is in `DRAFT_ELO_EXPERIMENT_PLAN.md`. Read its goal section for context, but everything you need to implement is in this prompt.

**Important context: oracle text discipline.** Two issues that initially looked like card-implementation bugs (Fiend Hunter and Lost in the Mist) turned out to be wrong after verifying current oracle text via `scripts/oracle_lookup.py`. Before flagging *any* card as having wrong behavior, verify against the cached oracle text using:
```
python3 scripts/oracle_lookup.py lookup "Card Name"
```
If the cache says the engine matches the current oracle, the card is not buggy regardless of what older printings or your training data say. Wizards has errata'd many Innistrad cards.

---

## 1. LLM action/target schemas don't constrain indices (HIGH PRIORITY)

This is the biggest issue and the one that lets the LLM make legal-looking but out-of-bounds choices that fall back to a default.

### Background: how decisions reach the model today

Look at `mtg-player/src/llm.rs`. There are several decision-point pathways and **none of them properly constrain integer ranges at generation time.** They all rely on post-hoc validation and a fallback to `0` / `options[0]` if the model returns garbage.

| Path | Where | Schema today |
|---|---|---|
| Main action picker (Gemini) | `call_interactions`, line ~768 | `{"action": {"type": "integer", "minimum": 0}}` — no maximum |
| Multi-target spells (`UpToTargets`) | `choose_cast_targets`, line ~1277 | `target_indices: array of integer minimum 0` — no maximum, no length bound |
| Single-target spells | `prompt_target_selection` → `choose_with_retry`, line ~1347 | **No schema at all** — plain text parsing |
| Mulligan bottom | `choose_mulligan_bottom`, line ~1807 | `bottom_indices: array of integer` — no min/max, no length, no enum |
| Mulligan keep/mull | `choose_mulligan`, line ~1740 | `{"mull": boolean}` — **already correct** (booleans are naturally bounded) |
| Blocker assignment | `choose_combat_blockers`, line ~2034 | `{"type": "integer", "enum": legal_indices}` — **already correct, this is the model to copy** |

### The Anthropic backend actively strips numeric range constraints

`AnthropicBackend::sanitize_schema` at `mtg-player/src/llm.rs:497-531` strips `minimum`, `maximum`, and `multipleOf` from any schema before sending. The comment claims they're "unsupported", which may or may not be current. **Do not rely on adding `maximum` to schemas — it will be silently stripped on the Anthropic path.** Use `enum` constraints instead, which are not stripped and are supported by both providers.

### What to fix

For every decision point that takes a numeric index, build an `enum` of the legal values and put it in the schema, the same way the blocker code already does. Concretely:

**(a) Main action picker.** In `GeminiBackend::call_interactions` (or wherever you choose to inject the constraint), the schema for the `action` field needs to be `{"type": "integer", "enum": [0, 1, ..., N-1]}` where `N` is the number of legal actions for the current decision. This is the most invasive change because the schema currently lives at the backend level, not the per-call site. You may need to plumb the action count down into the backend, or build the schema at the call site (in `choose_action`) and pass it through.

**(b) Multi-target spells.** In `choose_cast_targets` for `CastTargetSpec::UpToTargets`, change:
```rust
"target_indices": {
    "type": "array",
    "items": {"type": "integer", "minimum": 0},
    ...
}
```
to use `enum` constraints on items, plus length bounds:
```rust
"target_indices": {
    "type": "array",
    "items": {"type": "integer", "enum": [0, 1, ..., options.len()-1]},
    "minItems": 0,
    "maxItems": *max,
    ...
}
```
Verify on Gemini that `minItems`/`maxItems` are honored. If not, fall back to length validation in code but keep the enum constraint on items.

**(c) Single-target spells.** This is the path that bit us in the logs (Spectral Flight defaulting to opponent's creature). Currently goes through `prompt_target_selection` → `choose_with_retry` → `call_api` (plain text). Convert this to structured output: build a schema with `{"target_index": {"type": "integer", "enum": [0, 1, ..., options.len()-1]}}` and use `send_message_structured`. Same model as the blocker code.

**(d) Mulligan bottom.** In `choose_mulligan_bottom`, change:
```rust
"bottom_indices": {
    "type": "array",
    "items": {"type": "integer"},
    ...
}
```
to:
```rust
"bottom_indices": {
    "type": "array",
    "items": {"type": "integer", "enum": [0, 1, ..., hand_size-1]},
    "minItems": n,
    "maxItems": n,
    ...
}
```
Note that distinctness still has to be validated in code — JSON Schema's `uniqueItems` may or may not be enforced by the providers. Keep the existing `HashSet`-based distinctness check.

**(e) Don't touch the blocker code** at line ~2034. It's already correct and is the reference pattern.

### Validation

After the fix, the existing fallback paths (defaulting to `0`, etc.) should still exist as belt-and-suspenders, but they should rarely if ever fire. Add a unit or integration test that confirms the schema sent to the backend includes the enum constraint.

### Bonus: re-examine the Anthropic sanitizer

While you're in there, **verify whether Anthropic actually still strips `minimum`/`maximum`** in current API versions. If they do support it, remove the strip lines and add a test. If they don't, leave the strip but update the comment to cite the current docs. Either way, this experiment uses Gemini exclusively so it's not a blocker — just a cleanup if you have time.

---

## 2. Token creation events not logged (LOGGING BUG)

The engine creates tokens correctly — the tokens appear on the battlefield in subsequent prompts. But the structured engine state log only emits a `"X created N tokens"` line for **Spider Spawning**. All other token creators are silent in the log.

### Verified missing log lines (oracle text confirmed via `oracle_lookup.py`)

| Card | Effect | Logged? |
|---|---|---|
| Spider Spawning | Create X 1/2 green Spider tokens | ✅ |
| Mausoleum Guard | "When this creature dies, create two 1/1 white Spirit creature tokens with flying." | ❌ |
| Doomed Traveler | "When this creature dies, create a 1/1 white Spirit creature token with flying." | ❌ |
| Midnight Haunting | "Create two 1/1 white Spirit creature tokens with flying." | ❌ |
| Moan of the Unhallowed | "Create two 2/2 black Zombie creature tokens." | ❌ |
| Endless Ranks of the Dead | "At the beginning of your upkeep, create X 2/2 black Zombie creature tokens, where X is half the number of Zombies you control, rounded down." | ❌ |

### What to fix

Find the engine code that creates token objects (`state.create_object` calls in card behavior files, or the helper function they share) and ensure that **every** token creation emits an `Event`-level `state.log` line of the form:
```
{source_name} created {n} {token_type} tokens
```
(matching the existing Spider Spawning format). The cleanest fix is probably a single helper in `cards/helpers.rs` (or similar) that all token creators call, which emits the log line as a side effect. Then audit the affected card behaviors to make sure they go through the helper.

### Validation

Grep the test logs after a draft for `created \d+ \w+ token` and confirm at least one log line per token-creating card type. Alternatively, add a unit test that resolves Mausoleum Guard's death trigger and asserts the event log contains the token creation line.

---

## 3. Reckless Waif transform-back log line uses wrong source name (LOGGING BUG)

The engine logs transformations like this:

```
Reckless Waif transforms into Merciless Predator   ← correct
Reckless Waif transforms into Reckless Waif        ← wrong: source side should say "Merciless Predator"
```

Verified oracle text via `oracle_lookup.py`:
- Front face: `Reckless Waif` — "At the beginning of each upkeep, if no spells were cast last turn, transform this creature."
- Back face: `Merciless Predator` — "At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature."

When the back face transforms back to the front face, the log line should be `"Merciless Predator transforms into Reckless Waif"` — currently it says `"Reckless Waif transforms into Reckless Waif"`.

### What to fix

Find the transform handler in the engine. Looking at line 36067 of `draft-8seat-31flite.log` it appears to be using the *base/front* card name on the source side regardless of the current face. The fix is to use the *current face's* name when constructing the log message. Compare the handling in Civilized Scholar's transform (which DOES use the correct source name — `"Homicidal Brute transforms back into Civilized Scholar (didn't attack)"`). Look at how that one builds the log line and apply the same pattern to the upkeep-trigger transform path.

Confirmed 12 occurrences of `"Reckless Waif transforms into Reckless Waif"` in the test log; should drop to 0.

---

## 4. Discard action labels lose card identity (UX BUG)

When a player ends their turn at hand size > 7, the engine prompts for a discard. The action labels look like this (verified at `draft-8seat-31flite.log:12063`):

```
[DISCARD 1 CARD]

0:Discard 1 cards 1:Discard 1 cards 2:Discard 1 cards 3:Discard 1 cards 4:Discard 1 cards 5:Discard 1 cards 6:Discard 1 cards 7:Discard 1 cards
```

All 8 options are identical strings. The LLM has no way to know which index corresponds to which card. It has to guess blind.

Two issues here:
1. **The label lacks the card name.** Each option should say `"Discard <Card Name>"`, like `"Discard Ulvenwald Mystics"` instead of `"Discard 1 cards"`.
2. **`"Discard 1 cards"` should be `"Discard 1 card"`** (singular). Minor.

### What to fix

Find the discard action label generation. Likely in `mtg-engine/src/actions.rs` or the engine's action display code. Each `Discard` action wraps a specific card object — use that object's name in the label.

### Validation

Add a test that constructs a `Discard` action and asserts the display string contains the card name.

---

## 5. Mulligan implementation review (FOLLOW-UP TO PRIOR AGENT)

A different agent recently implemented London mulligans (capped at mull-to-4). The bulk of the work looks correct, but there are two issues to address.

### 5a. `bottom_indices` schema is unconstrained (covered by issue #1d above)

Already specified above. Listed here so it's not missed in the mulligan context.

### 5b. Mulligan rounds are collapsed per player, not alternated (DESIGN ISSUE)

The prior agent flagged this themselves:

> Mulligan rounds are collapsed per player, not alternated. Strict London-mulligan rules say each round, every player in turn order decides; those who mulled then shuffle and redraw; then another round. My implementation drains one player's decisions fully before moving to the next. This is functionally equivalent (each player's mulligan decisions are independent...)

**The "functionally equivalent" claim is incorrect, but the information leak is narrower than it might first appear.** Here's exactly how the info flow differs.

Under real London mulligan rules, within a round R:
1. Active player (on the play) decides keep/mull first. If they mull, they reshuffle and draw 7 — observable to the opponent.
2. Non-active player decides keep/mull, with full knowledge of what the active player did *this round*.
3. End of round R. If anyone mulled, advance to round R+1.

So within a round, the non-active player **does** see the active player's current-round decision before deciding — that's allowed and standard. What no player ever sees is **future-round decisions** that haven't happened yet.

Compare what the non-active player ("B") sees when making B's round-1 decision:

| Mode | What B sees |
|---|---|
| Real (alternating) | Whether A kept or mulled in round 1 only |
| Collapsed (current code) | A's *complete* decision history — round 1, round 2, ..., final keep |

The leak is specifically that the collapsed mode lets B see A's *future-round* decisions before B has made their own round-1 decision. B is allowed to see A's round-1 decision in real Magic, but not A's round 2/3/4. For LLM piloters this leak is probably small in practice, but the experimental design relies on symmetric treatment of both seats and we should fix it before running Phase 0.

#### What to fix

Restructure the mulligan phase so that the loop variable is "round," not "player." Within a round, decisions go in turn order (active player first), and the non-active player sees the active player's current-round decision before deciding. The structure should look roughly like this:

```rust
loop {
    // Round R starts. All decisions and reshuffles happen in turn order.

    // 1. Active player decides keep/mull.
    let active_decision = ask(active_player);
    if active_decision == Mull {
        reshuffle_and_redraw(active_player);
        increment_mulligan_count(active_player);
    }

    // 2. Non-active player decides — sees active's current-round decision.
    let nonactive_decision = ask(nonactive_player);
    if nonactive_decision == Mull {
        reshuffle_and_redraw(nonactive_player);
        increment_mulligan_count(nonactive_player);
    }

    // 3. If both kept, exit the loop.
    if active_decision == Keep && nonactive_decision == Keep {
        break;
    }

    // Otherwise, advance to round R+1. Both players carry their current 7 forward.
}

// After both players have kept, run the bottoming phase in turn order.
```

Compared to the current collapsed implementation, the diff is small but important: instead of asking player A to make all of their mulligan decisions in sequence and then asking player B to make all of theirs, the loop alternates `A_decide → B_decide → check → A_decide → B_decide → check`. The order in which prompts are issued matches the order in real Magic. Each prompt to player B sees only the active player's decisions through the current round, not future rounds.

Note that "player on the play decides first" within a round is the correct rule for current London mulligans (not pre-London Vancouver order), so the prior agent's choice on that point is right.

The mull cap (mull-to-4 = 3 mulligans max) still applies per player. A player who has hit the cap should automatically be in "must keep" mode for any further round; the engine should not offer them `MulliganMull` as a legal action, and the LLM player's existing handling for `mull_allowed = false` already covers this.

#### Validation

Add a test with two scripted players that records the order of decision callbacks and the visible hand sizes at each callback. Confirm:
- Within round R, the active player is asked first.
- The non-active player's prompt happens after the active player's reshuffle (if any), so it can observe the active player's round-R decision.
- The non-active player is **not** asked about round R+1 before the active player has been asked about round R+1.
- A player who has hit the mull cap is auto-skipped (or asked but with no `MulliganMull` option).

### 5c. (Not a bug, just verify) Mull cap actually triggers force-keep at mull #4

Per the prompt, after 3 mulligans (cap = 3), the player must keep their next 7 (mull-to-4 final hand size). The current code looks like it does this in `actions.rs:56-57` and the engine panics if `MulliganMull` is attempted past the cap. Verify that:
- The legal actions for the 4th decision do NOT include `MulliganMull`
- The LLM player's `choose_mulligan` correctly handles `mull_allowed = false` (it currently does — line 1716-1737)
- The engine panics with a clear message if someone tries to send `MulliganMull` past the cap

This is just a sanity check, not a fix unless something's broken.

---

## What I checked and is NOT a bug (RETRACTIONS)

The audit initially flagged two cards as having wrong behavior. Verifying against current oracle text via `scripts/oracle_lookup.py` showed these were both wrong claims:

### Fiend Hunter — IMPLEMENTATION IS CORRECT
- Cached oracle text: *"When this creature enters, you may exile another target creature. When this creature leaves the battlefield, return the exiled card to the battlefield under its owner's control."*
- The card has been errata'd. The current oracle text does **not** restrict to "an opponent controls" — any creature except Fiend Hunter itself is a legal target. The engine implementation in `mtg-engine/src/cards/isd/fiend_hunter.rs` is correct, and the comment in the source code (`// Can target own creatures (Oracle doesn't restrict to opponents).`) is accurate.
- **Do not change Fiend Hunter.**

### Lost in the Mist — IMPLEMENTATION IS CORRECT
- Cached oracle text: *"Counter target spell. Return target permanent to its owner's hand."*
- This really is a two-target spell (counter half + bounce half). Lands are permanents, so offering Swamps/Forests/Islands as valid targets for the bounce half is correct. The engine implementation matches the current oracle.
- **Do not change Lost in the Mist.**

I'm noting these explicitly so that nobody else "fixes" them based on remembering older oracle text.

---

## Things to be careful about

These are CLAUDE.md rules and project memory, follow them strictly:

- **Verify oracle text from `scripts/oracle_lookup.py` before flagging any card as buggy.** Wizards has errata'd many Innistrad cards. Memory and training data are unreliable for older cards.
- **Never take silent shortcuts.** If a fix turns out to require a wider refactor, surface that explicitly rather than half-implementing.
- **Engine limits are real issues.** If the architecture makes one of these fixes structurally hard, flag it as ISSUE.
- **Correctness over convenience.** Code must work for the right reasons.
- **Small incremental commits.** Each numbered fix should be its own commit (or small series). Each commit must pass `cargo check` with **zero warnings**.
- **Don't add unrelated changes.** Don't refactor or "improve" code outside the scope of these fixes.
- **Don't touch the blocker assignment code at `llm.rs:~2034`.** It's the reference pattern for how schemas should be built — copy the approach, don't modify the original.

---

## Definition of done

- `cargo check` passes with zero warnings on all crates.
- All existing tests still pass.
- New tests cover: enum-constrained schemas for actions, single-target spells, multi-target spells, and mulligan bottoming; token creation logging for at least one ETB-trigger and one death-trigger creator; transform-back log line correctness; discard action label includes card name; mulligan rounds alternate properly between players.
- A test draft with `mtg-draft-runner` and a Gemini player produces a log file where:
  - No `MALFORMED` log entries appear from action/target picks
  - Token creation lines appear for Mausoleum Guard, Doomed Traveler, Midnight Haunting, and any other token creator that fires
  - No `"X transforms into X"` log lines appear when transforming back
  - Discard action labels include card names
- Short summary of what changed, what files were touched, any issues you flagged but did not fix, and a note on whether the Anthropic `sanitize_schema` strip-list was kept or removed.

## Where to start

1. Read `CLAUDE.md`, the memory at `~/.claude/projects/-Users-dlaw-mtg/memory/MEMORY.md`, and `DRAFT_ELO_EXPERIMENT_PLAN.md` for context.
2. Read `mtg-player/src/llm.rs` carefully — particularly `prompt_target_selection`, `choose_cast_targets`, `choose_mulligan_bottom`, the blocker assignment code, and the two backend `send_with_schema` paths.
3. Read `AnthropicBackend::sanitize_schema` to understand what gets stripped.
4. Look at recent commits (`git log --oneline -20`) and the uncommitted mulligan work (`git diff` on `actions.rs`, `engine.rs`, `state.rs`, and `llm.rs`) to understand what's already there.
5. Sketch the schema-construction approach (probably a small helper that builds an enum-constrained integer schema given a count) before writing code.

Ask if anything in the spec is ambiguous before committing to a design.
