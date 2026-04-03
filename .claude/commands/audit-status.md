# Audit Status

Show the audit history for cards as a table.

## Arguments
- `$ARGUMENTS` — What to show status for:
  - Card names, comma-separated: `"Fiend Hunter, Delver of Secrets"`
  - A set code: `"isd"` (all cards in the set)
  - `"all"` (every implemented card)

## Procedure

### 1. Resolve the card list

- **Set code** (e.g., `"isd"`): List all `.rs` files in `mtg-engine/src/cards/{set}/` (excluding `mod.rs`).
- **`"all"`**: List all `.rs` files in `mtg-engine/src/cards/` recursively (excluding `mod.rs`, `helpers.rs`).
- **Card names**: Convert to filenames (snake_case + `.rs`).

### 2. Parse audit history

For each card, check if `audits/{filename without .rs}.md` exists.

If it exists, parse ALL `**Status**:` lines and their preceding `## Audit —` date headers. Each audit entry has:
- Date/time from the `## Audit — {YYYY-MM-DD HH:MM}` header
- Status from the `**Status**: PASS / ISSUE / SKIPPED` line

If no audit file exists, the card is UNAUDITED.

### 3. Output the table

Output a markdown table with:
- One row per card
- Columns: Card Name | Current Status | Audit History (chronological, showing each date + status)

Example:

```
| Card | Current | History |
|------|---------|---------|
| Fiend Hunter | PASS | ISSUE (Apr 1) -> ISSUE (Apr 1) -> PASS (Apr 1) |
| Delver of Secrets | PASS | ISSUE (Apr 1) -> PASS (Apr 1) |
| Screeching Bat | PASS | ISSUE (Apr 1) -> ISSUE (Apr 1) -> PASS (Apr 1) |
| Grizzly Bears | UNAUDITED | — |
```

Then output summary counts:
```
Total: N cards — X PASS, Y ISSUE, Z UNAUDITED
```

### 4. For set-level views, group by current status

When showing a full set, group the output:
1. First show cards with ISSUE (most actionable)
2. Then UNAUDITED
3. Then PASS (collapsed — just count, don't list every passing card unless there are fewer than 20)

This keeps the output scannable rather than a wall of 250 rows.
