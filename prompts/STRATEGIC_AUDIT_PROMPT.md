# Task: Audit substantive LLM decisions in `verify-draft-8seat-high-v5.log`

You're working in `/Users/dlaw/mtg`, a Rust workspace implementing a Magic: The Gathering engine plus LLM AI players. Your job is to read through a long structured log from a recent 8-seat best-of-3 Innistrad draft tournament, audit how the LLM player handled **substantive game decisions**, and report your findings.

This is a research / analysis task. **Do not modify any code, do not commit anything, do not relaunch any drafts.**

The log is at:

```
/Users/dlaw/mtg/verify-draft-8seat-high-v5.log
```

It is ~85,000 lines. You will not read it sequentially. You will jump around, sample broadly, and follow hypotheses.

The model under audit is `gemini-3.1-flash-lite-preview` running with `high:high` thinking. 8 players, best-of-3 matches, single-elimination, full London mulligans.

---

## Why this audit exists

We are trying to answer two questions:

1. **Is the harness giving the model the information it needs to make good decisions?** Are the prompts complete, accurate, and unambiguous? Do the action labels truthfully describe what the engine will do?
2. **Is the model making reasonable decisions with that information?** When it goes wrong, is it because the prompt was misleading (fixable in the harness), because the system prompt didn't teach the right concept (fixable in the rules text), or because the model is simply not capable enough (informational only)?

A previous audit covered ~25 decisions and produced these findings — **do not re-flag them, but DO flag adjacent / similar instances**:

### Already fixed
- **Engine bug** (commit `44dd43a`): `Full Moon's Rise` activated-ability label said *"Regenerate all Wolf and Werewolf creatures you control"*, but the actual handler only regenerates Werewolves. The card text and the ability label were corrected to match the oracle text. The model was misled into sacrificing the enchantment to save a Wolf token, which then died anyway.
- **System prompt updates** (in `mtg-player/src/llm.rs`): added an Equipment section (the rules never mentioned equip), added a worked first-strike combat-math example (`A 2/2 blocking a 3/2 first strike takes 3 and dies before dealing its damage`), added an anti-hallucination instruction in both response-format intros (`Ground every claim in your thoughts in the actual prompt text...`), and added a "When you're behind" strategy note.

### Patterns to look for (these are the *categories* the previous pass surfaced)
- **Hallucinated board state**: model's THOUGHT references creatures, cards, or zones that aren't actually in the prompt.
- **Combat math errors**: especially around first strike, deathtouch, lifelink, trample, +1/+1 counters, and damage-prevention auras (Bonds of Faith, Ghostly Possession, etc.).
- **Equipment ignored**: artifact equipment sitting on the battlefield for many turns without ever being equipped, even when it would meaningfully change a race.
- **Stuck in defensive loops at low life**: passing turn after turn waiting for a topdeck while losing to chip damage, instead of trying to *change* the situation.
- **Rules misunderstandings**: the model misstates how a keyword/ability works in its THOUGHT (even when the conclusion happens to be right).
- **Engine bugs surfacing**: action labels describing an effect different from what the engine actually executes.

You should look for *more* of these and also for *new* categories.

---

## How the log is structured

Each line is tab-delimited:

```
<wall_clock_ts>\t<LEVEL>\t<thread>\t<file>:<line>\t<TAG>\t<content>
```

Levels are `DEBUG`, `INFO`, `ERROR`. Multi-line entries (notably `PROMPT`) have the first line in tab-delimited format and continuation lines flush-left, ending at the next tab-delimited line.

The tags you care about most:

| Tag | Meaning |
|---|---|
| `PROMPT [SeatN]` | Multi-line block — the full game state shown to the LLM player at seat N. |
| `THOUGHT [SeatN]` | Single-line — the model's internal reasoning for the decision it's about to make. |
| `RESPONSE [SeatN]` | Single-line JSON — the raw response from the API. |
| `CHOSE [SeatN]` | Single-line — engine confirmation of the chosen action. |
| `AUTO-PASS [SeatN]` | The engine auto-passed priority on the seat's behalf (no decision was needed). Skip these. |
| `COLLAPSED [SeatN]` | The action menu was collapsed from N raw actions to fewer options. Informational. |

The 8 player threads interleave. To audit a decision, find a `THOUGHT [SeatX]` you want to investigate, then walk *backwards* through the file to the most recent preceding `PROMPT [SeatX]` (same seat — ignore prompts for other seats). The PROMPT block continues until the next tab-prefixed line.

A typical PROMPT body looks like:

```
Turn 16 - Declare Attackers (your turn)

You: 10hp, 1cards, 26lib, 2gy, 0exile
Opp: 17hp, 0cards, 26lib, 5gy, 0exile
Your board: 2x Forest (1 tapped), 3x Mountain (1 tapped), Avacyn's Pilgrim 1/1, ...
Opp board: 2x Swamp (tapped), 4x Plains (2 tapped), Walking Corpse 2/2, ...
Hand: Moldgraf Monstrosity {4}{G}{G}{G} 8/8
Opp graveyard: Spectral Rider 2/2, ...
Your graveyard: Prey Upon, ...

Choose attackers: 0:Howlpack of Estwald 5/6 trample 1:Avacyn's Pilgrim 1/1 ...
Respond with {"thoughts": "...", "attacker_indices": [..]} ...
```

Status flags after creatures appear in single brackets, comma-separated when multiple: `[T,1dmg,+1+1x2]` = tapped, 1 damage marked, two +1/+1 counters. Common flags:
- `T` = tapped
- `S` = summoning sick
- `Ndmg` = N damage marked
- `+1+1xN`, `-1-1xN` = counter counts

Auras and equipment appear inline as plaintext appended to the creature, e.g. `Howlpack of Estwald 5/7 [+1+1x1] (Bonds of Faith: ... )`.

---

## Reference files in the repo

You may want to read these to understand the harness:

- `mtg-player/src/llm.rs` — system prompts, prompt building, log rewriting. Search for `GAME_RULES`, `GEMINI_RESPONSE_FORMAT`, `ANTHROPIC_RESPONSE_FORMAT`, `format_state_body`, `build_prompt_with_header`. This is how the model sees the game.
- `mtg-engine/src/cards/isd/` — card implementations. If you suspect an engine bug where the action label disagrees with the actual effect, the card's source file is usually here. Look for `activated_abilities()` and `on_activate_ability()`.
- `mtg-engine/src/view.rs` — defines `OpponentView` and other state-projection structs. Useful if you suspect a field is missing from what the model sees.

To verify the **true** oracle text of any card, use:

```bash
curl -sS "https://api.scryfall.com/cards/named?exact=$(python3 -c 'import urllib.parse,sys; print(urllib.parse.quote(sys.argv[1]))' 'Card Name')" \
  -H 'User-Agent: mtg-engine-research/1.0' \
  -H 'Accept: application/json' \
  | python3 -c 'import sys,json; d=json.load(sys.stdin); print(d.get("name")); print("---"); print(d.get("oracle_text"))'
```

This is how the previous audit confirmed the Full Moon's Rise bug. **Always verify card text against Scryfall before flagging an engine bug** — the model could be wrong about the text *and* the engine could be right.

---

## What counts as a "substantive" decision

You are auditing substantive decisions only. SKIP the noise:

**Skip:**
- Passing priority during opponent's untap/upkeep/draw/end steps when nothing is on the stack and the model holds no instants. These dominate the log but are uninteresting.
- Passing in your own untap/upkeep with no triggers and no instants worth casting.
- Tap-land actions (these are engine-mechanical, not strategic).
- "Cleanup" pass-priority decisions.
- Trivial single-option declares (e.g. "Choose attackers: 0:Wolf 2/2" with no opponent blockers).

**Audit:**
- Declare-attackers decisions where there's a real choice (>0 attackers possible AND opponent has blockers OR life totals are interesting).
- Declare-blockers decisions with multiple plausible blocking assignments.
- Main-phase casts where the model picked one of several non-trivial plays, OR chose to hold mana when it could cast something.
- Combat tricks / removal / counterspells cast in response to a stack event.
- Target selection for spells with multiple legal targets.
- Mulligan keep/mull decisions and the subsequent "bottom N cards" decisions.
- Activated-ability decisions (especially equipment, sacrifice abilities, transform triggers).
- Any decision the model spent unusually many words explaining — long THOUGHT entries are a tell that the model thought it was hard.

---

## How to find substantive decisions

Don't read sequentially. Use grep to surface candidates, then walk back to the matching PROMPT.

### Find long thoughts (model perceived complexity)

```bash
grep -n "THOUGHT \[Seat" /Users/dlaw/mtg/verify-draft-8seat-high-v5.log \
  | awk -F'\t' '{ print $1, length($0), $0 }' \
  | awk '$2 > 400' \
  | head -100
```

### Find all blocking decisions (these are usually substantive)

```bash
grep -n "Declare Blockers" /Users/dlaw/mtg/verify-draft-8seat-high-v5.log | head -50
```

### Find all attack decisions with multiple attacker options

```bash
grep -n "Choose attackers:" /Users/dlaw/mtg/verify-draft-8seat-high-v5.log \
  | grep -E "[2-9]:" \
  | head -50
```

### Find mulligan decisions

```bash
grep -n "London mulligan decision" /Users/dlaw/mtg/verify-draft-8seat-high-v5.log
grep -n "Bottom .* cards after mulligan" /Users/dlaw/mtg/verify-draft-8seat-high-v5.log
```

### Find combat tricks (instants cast during combat)

```bash
grep -nE "Cast .* \(tap.*\)" /Users/dlaw/mtg/verify-draft-8seat-high-v5.log \
  | grep -B2 "Declare" | head -50
```

### Find equipment-related actions

```bash
grep -n "Equip" /Users/dlaw/mtg/verify-draft-8seat-high-v5.log | head -50
```

### Find low-life decisions (likely high-stakes)

```bash
grep -nE "You: [1-5]hp" /Users/dlaw/mtg/verify-draft-8seat-high-v5.log | head -50
```

### Sample randomly (if you suspect you're over-clustering)

```bash
wc -l /Users/dlaw/mtg/verify-draft-8seat-high-v5.log
# pick random offsets and read 200-line windows from each
```

---

## How to evaluate each audited decision

For every substantive decision you select, walk through these three checks in order:

### Check 1 — Did the harness give the model the info it needed?

- Was the relevant card text shown for auras/equipment/triggered abilities currently on the battlefield?
- Were summoning sickness, tap state, damage marked, and counters all visible on the right creatures?
- Was opponent's hand size, library size, graveyard contents, and exile count shown?
- Were opponent's flashback options shown (if relevant)?
- Were the action labels accurate — does the cost shown match what would actually be spent? does the effect described match what the engine actually does?
- **Engine bug pattern (highest priority finding)**: does any action description claim an effect different from what the engine executes? The Full Moon's Rise bug is the template — verify with Scryfall before flagging, then look at the source file under `mtg-engine/src/cards/isd/`.

### Check 2 — Did the model reason correctly about that info?

- **Hallucination**: did the THOUGHT reference creatures, cards, zones, or stats not in the prompt? Quote the offending phrase and the prompt line that contradicts it.
- **Combat math**: did the model mis-evaluate a trade? Walk the math yourself. Especially watch first strike (damage timing), deathtouch (any damage destroys), lifelink (race math), trample (excess damage), +1/+1 counters from auras/Travel Preparations/etc., damage prevention from Ghostly Possession.
- **Rules misunderstandings**: did the model misstate how a keyword or ability works, even in passing? Note these even when the chosen action was still right — repeated misstatements suggest the system prompt should clarify.
- **Missed lines**: was there an obviously better play the model didn't even consider? Equipping a sitting equipment, casting a buff for a finishing attack, using a removal spell at a critical moment, attacking when opp had no blockers, etc.
- **Targeting errors**: did the model pick a worse target when a strictly better one was legal?

### Check 3 — Was the chosen action defensible?

- Even if the reasoning was muddled, was the chosen action *itself* reasonable given the actual game state?
- Or was the choice clearly wrong (attacking into lethal blocks for no value, declining an attack when lethal was available, casting a spell at the wrong target, mulliganing a fine hand, keeping an unkeepable hand)?

A decision can have a bad THOUGHT but a good ACTION (model reasoned wrong but happened to pick correctly). Note both separately. Decisions with both a bad THOUGHT and a bad ACTION are the most damning.

---

## Sample broadly

Aim for **30–50 substantive decisions audited**, distributed across:

- All 8 seats (Seat0 through Seat7) — don't let one seat dominate.
- Early game (turns 1–5), mid game (turns 6–12), late game (turns 13+).
- All match games — at least one game from each round of the bracket.
- A mix of decision types: at least 5 mulligan decisions, at least 10 combat decisions, at least 5 main-phase cast decisions, at least 3 target-selection decisions.
- Both winning and losing positions for the seat being audited.

Quality over quantity. A thorough analysis of 30 decisions is more useful than a shallow scan of 100. If you can't tell whether a play was right without simulating the entire subgame, *say so* — don't manufacture findings to hit a quota.

---

## Output: a single audit report

Produce a markdown report on stdout with these sections:

### 1. Engine / harness bugs (highest priority)

For each: the log file:line where the bug surfaced, what the action label or prompt said, what actually happened (verified by reading the engine source AND Scryfall), the source file:line of the bug, and a one-sentence proposed fix. These are real bugs to fix in the codebase.

### 2. Prompt-fixable issues

Patterns where the model would do better with a system-prompt change. Group by category. For each category, give 2–3 example log line numbers and a one-sentence proposed fix to `GAME_RULES` or one of the response-format constants in `mtg-player/src/llm.rs`. Don't repeat fixes already made (equipment section, first-strike example, hallucination guard, "when you're behind" note).

### 3. Model capability issues (informational only)

Patterns that look like raw model weakness — things a system prompt won't fix (e.g. consistent failure to do 4-creature combat math even with all info present). Report them but mark them as informational; the user is not going to retrain the model.

### 4. Strong play examples

3–5 instances where the model made a notably good decision: recognized a subtle interaction, found a non-obvious line, made a correct sacrifice, etc. This calibrates whether high-thinking is actually buying us anything.

### 5. Sample list

A flat numbered list of every decision you audited, one per line:

```
1. PROMPT line 16857 → THOUGHT line 16917 (Seat3, turn 12, declare attackers) — ATTACKED with 3 creatures into 2 blockers — verdict: ok
2. PROMPT line 22376 → THOUGHT line 22492 (Seat0, turn 14, main 1) — CAST Rally the Peasants — verdict: good
...
```

This lets the user spot-check your work without rereading the whole log.

---

## Constraints and ground rules

- **Read-only.** Do not edit, write, or commit any files.
- **Do not run cargo build, cargo test, or relaunch any drafts.** The smoke test was deliberately killed; don't restart it.
- **Verify before flagging engine bugs.** Every claimed engine bug must be backed by (a) a log line showing the misleading action label, (b) the actual engine source file:line, AND (c) the true Scryfall oracle text. Three sources of evidence.
- **Be honest about uncertainty.** Missed reads, ambiguous board states, and "I'd need to simulate this to know" are all valid notes. Don't fabricate confidence.
- **Don't repeat the prior audit.** The four findings already addressed (Full Moon's Rise bug + 3 system prompt categories) are off-limits as findings — but you can and should look for adjacent instances.
- **Don't audit pass-priority noise.** If you find yourself analyzing dozens of "I will pass to advance to my main phase" decisions, you're in the wrong part of the log.
- **Output is plain markdown to stdout.** Don't write the report to a file unless explicitly asked.

The previous audit took shortcuts. You should be more thorough. The user is using this audit to decide whether the system is ready for an overnight 100-draft tournament run.
