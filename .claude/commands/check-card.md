# Check Card Implementation

Thoroughly audit Magic: The Gathering card implementations for correctness, test coverage, and UI presentation.

## Arguments
- `$ARGUMENTS` — What to audit. Accepts any of:
  - Card names, comma-separated: `"Lightning Bolt, Fiend Hunter, Doom Blade"`
  - A set code: `"isd"` or `"innistrad"` (audits all cards in the set)
  - `"all"` (audits every implemented card)
Use `/audit-status` to view audit history and status tables instead.

## Audit mode

### Resolving the card list

1. **If a set code** (e.g., `"isd"`): List all `.rs` files in `mtg-engine/src/cards/{set}/` (excluding `mod.rs`). Convert filenames to card names by looking up each file's struct name or `card_data().name`.
2. **If `"all"`**: List all `.rs` files in `mtg-engine/src/cards/` recursively (excluding `mod.rs`, `helpers.rs`).
3. **If card names**: Use as-is.

### Dispatching audits — one agent per card, max 10 at a time

Each card MUST be audited by its own dedicated subagent with fresh context. Never give multiple cards to a single agent.

1. Take the next batch of up to 10 cards from the list.
2. Launch one Agent per card, all in parallel (up to 10 simultaneous agents). Each agent gets the prompt below.
3. Wait for all agents in the batch to complete.
4. Commit and push: `git add audits/ && git commit -m "Audit batch: Card A, Card B, ..." && git push`
5. Collect results from each agent and note PASS/ISSUE/SKIPPED.
6. If more cards remain, go to step 1 with the next batch.
7. After all batches, output a summary table.

### Agent prompt (for each card)

Launch each agent with this exact prompt, replacing `{CARD_NAME}` and `{CARD_FILE}`:

```
You are auditing the MTG card: {CARD_NAME}

Read the file `.claude/commands/check-card-procedure.md` and follow its COMPLETE procedure for this card.

Look up the oracle text: `python3 scripts/oracle_lookup.py lookup "{CARD_NAME}"`. If not cached, run `python3 scripts/oracle_lookup.py fetch "{CARD_NAME}"`.

The card implementation is at `mtg-engine/src/cards/{CARD_FILE}`.

Do NOT read any previous audit logs before conducting your audit. Write your report to `audits/{audit_file_name}.md`.
```

### After all audits complete

Output a summary table:

```
## Audit Summary — {date}

| Card | Status | Key Issues |
|------|--------|------------|
| ... | PASS / ISSUE | ... |

**Total**: N PASS, N ISSUE, N SKIPPED
```
