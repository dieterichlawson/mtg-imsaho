# Implement Card

Implement a new Magic: The Gathering card with full correctness, test coverage, and UI support. Uses an iterative implement-then-audit loop with separate agents to ensure quality.

## Arguments
- `$ARGUMENTS` — One or more card names, comma-separated (e.g., "Lightning Bolt" or "Lightning Bolt, Doom Blade")

When multiple cards are given, implement them one at a time sequentially (do NOT parallelize — each card may require engine changes that affect subsequent cards).

## Procedure (repeat for each card)

### Step 1: Fetch oracle text

Run the oracle lookup to ensure the card's oracle text is cached:

```bash
python3 scripts/oracle_lookup.py fetch "Card Name"
```

Record the full output — you'll pass it to the implementor agent. If this is a DFC, both faces will be fetched automatically.

### Step 2: Launch the IMPLEMENTOR agent

Launch an Agent with the following prompt structure. Replace `{CARD_NAME}` with the card name, `{ORACLE_OUTPUT}` with the full oracle lookup output from step 1, and `{SET_CODE}` with the set code (e.g., "isd" for Innistrad).

```
You are implementing a new MTG card: {CARD_NAME}

## Oracle text (from Scryfall — this is your source of truth)

{ORACLE_OUTPUT}

## Your instructions

Read the file `.claude/commands/implement-card-guide.md` for full implementation instructions, patterns, and rules. Follow it exactly.

## What to do

1. Read the guide file
2. Implement the card in `mtg-engine/src/cards/{SET_CODE}/{file_name}.rs`
3. Register it in `mtg-engine/src/cards/{SET_CODE}/mod.rs` and `mtg-engine/src/cards/mod.rs` (in `with_all_cards()`)
4. Write tests in `mtg-engine/tests/`
5. Add an entry to the LLM card knowledge section in `mtg-player/src/llm.rs` if appropriate
6. Run `cargo test` and fix any compilation or test failures
```

Wait for the agent to complete. Then commit and push the implementation:

```bash
git add -A && git commit -m "Implement {CARD_NAME}: round {N} implementation" && git push
```

### Step 3: Launch the AUDITOR agent

Launch an Agent with the following prompt. Replace `{CARD_NAME}` with the card name.

```
You are auditing a newly implemented MTG card: {CARD_NAME}

Read the file `.claude/commands/check-card.md` and follow its COMPLETE procedure for this card. The oracle text is already cached — use `python3 scripts/oracle_lookup.py lookup "{CARD_NAME}"` to retrieve it.

Do NOT read any previous audit logs. Conduct a fully independent audit. Write your report to `audits/{file_name}.md`.

IMPORTANT: You must follow every step in check-card.md — do not skip steps or summarize the procedure.
```

Wait for the agent to complete. Then commit and push the audit:

```bash
git add audits/ && git commit -m "Audit {CARD_NAME}: round {N}" && git push
```

### Step 4: Check the audit result

Read the audit file at `audits/{file_name}.md`. Look at the **Status** field of the most recent entry.

- If **PASS**: proceed to step 5.
- If **ISSUE**: proceed to step 4a.
- If **SKIPPED**: something went wrong with oracle text — investigate and retry.

#### Step 4a: Fix issues (max 5 rounds total)

Extract the issue list from the audit log. Launch a new IMPLEMENTOR agent with the following prompt:

```
You are fixing audit issues for the MTG card: {CARD_NAME}

## Oracle text (from Scryfall — this is your source of truth)

{ORACLE_OUTPUT}

## Issues to fix

{ISSUE_LIST — copy the exact "Code issues" section from the audit log}

## Your instructions

Read the file `.claude/commands/implement-card-guide.md` for implementation patterns and rules. Follow it exactly.

## What to do

1. Read the guide file
2. Read the current implementation at `mtg-engine/src/cards/{SET_CODE}/{file_name}.rs`
3. Fix each issue listed above — make engine changes as needed (do NOT work around engine limitations)
4. Update or add tests to cover the fixes
5. Run `cargo test` and fix any compilation or test failures

## Disputing issues

If you believe an issue raised by the auditor is NOT a real problem (e.g., the auditor misread the oracle text, or the code is actually correct), do NOT silently ignore it. Instead, write your disagreement to a file at `audits/{file_name}_disputes.md` with:
- The issue as stated by the auditor
- Why you believe it is not a real issue
- The exact oracle text and code quotes supporting your position

Fix all issues you agree with. Disputed issues will be escalated to the user.
```

After the fix agent completes, commit and push the fixes:

```bash
git add -A && git commit -m "Fix {CARD_NAME}: round {N} fixes" && git push
```

Then check for a disputes file at `audits/{file_name}_disputes.md`. If one exists, read it and present the disputes to the user. Ask the user to rule on each disputed issue before continuing. Remove the disputes file after resolution.

Then go back to step 3 (launch a new auditor). Repeat until PASS or until you've done 5 rounds total.

If after 5 rounds the auditor still reports ISSUE, stop and report the remaining issues to the user for manual review.

### Step 5: Report

Output a summary to the user:
- Card name and status (PASS / ISSUE after N rounds)
- Files created/modified
- Any engine changes made
- Remaining issues (if any after max rounds)
