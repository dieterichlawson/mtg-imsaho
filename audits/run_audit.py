#!/usr/bin/env python3
"""Run card audits using the batch agent runner.

Usage:
    python3 audits/run_audit.py --set isd                    # Audit all ISD cards
    python3 audits/run_audit.py --set isd --start 141        # Resume from card 141
    python3 audits/run_audit.py --set isd --limit 10         # Audit first 10 cards
    python3 audits/run_audit.py --cards "Fiend Hunter,Doom Blade"  # Specific cards
    python3 audits/run_audit.py --set isd --filter pass      # Re-audit PASSing cards
    python3 audits/run_audit.py --set isd --filter unaudited # Audit only unaudited cards
    python3 audits/run_audit.py --set isd --dry-run          # Show what would be audited
"""

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from datetime import datetime
from pathlib import Path

# ─── Paths ────────────────────────────────────────────────────────

PROJECT_ROOT = Path(__file__).resolve().parent.parent
AUDITS_DIR = PROJECT_ROOT / "audits"
REPORTS_DIR = AUDITS_DIR / "reports"
CARDS_DIR = PROJECT_ROOT / "mtg-engine" / "src" / "cards"
ORACLE_SCRIPT = PROJECT_ROOT / "scripts" / "oracle_lookup.py"
BATCH_RUNNER = Path.home() / "agent" / "claude-code" / "src" / "batch.ts"
SHARED_PROMPT_FILE = AUDITS_DIR / "shared_prompt.md"


# ─── Card discovery ──────────────────────────────────────────────

# Cards with apostrophes or special names that can't be derived from filename
SPECIAL_NAMES = {
    "altars_reap": "Altar's Reap",
    "avacyns_pilgrim": "Avacyn's Pilgrim",
    "butchers_cleaver": "Butcher's Cleaver",
    "devils_play": "Devil's Play",
    "geistcatchers_rig": "Geistcatcher's Rig",
    "ghoulcallers_bell": "Ghoulcaller's Bell",
    "ghoulcallers_chant": "Ghoulcaller's Chant",
    "grimgrin_corpse_born": "Grimgrin, Corpse-Born",
    "heretics_punishment": "Heretic's Punishment",
    "inquisitors_flail": "Inquisitor's Flail",
    "ludevics_test_subject": "Ludevic's Test Subject",
    "memorys_journey": "Memory's Journey",
    "nightbirds_clutches": "Nightbird's Clutches",
    "runechanters_pike": "Runechanter's Pike",
    "stitchers_apprentice": "Stitcher's Apprentice",
    "travelers_amulet": "Traveler's Amulet",
}


def filename_to_card_name(filename: str) -> str:
    """Convert snake_case filename to card name."""
    if filename in SPECIAL_NAMES:
        return SPECIAL_NAMES[filename]
    return " ".join(word.capitalize() for word in filename.split("_"))


def get_cards_for_set(set_code: str) -> list[tuple[str, str]]:
    """Return list of (card_name, filename) for all cards in a set."""
    cards_dir = CARDS_DIR / set_code
    if not cards_dir.exists():
        print(f"Error: {cards_dir} not found", file=sys.stderr)
        sys.exit(1)
    filenames = sorted(f.stem for f in cards_dir.glob("*.rs") if f.stem != "mod")
    return [(filename_to_card_name(f), f) for f in filenames]


def get_oracle_text(card_name: str) -> str | None:
    """Fetch oracle text from cache or Scryfall."""
    for cmd in ["lookup", "fetch"]:
        result = subprocess.run(
            ["python3", str(ORACLE_SCRIPT), cmd, card_name],
            capture_output=True, text=True, cwd=str(PROJECT_ROOT),
        )
        if result.returncode == 0 and result.stdout.strip():
            return result.stdout.strip()
    return None


def get_latest_audit_status(filename: str) -> str | None:
    """Parse the most recent audit status from a report file."""
    report = REPORTS_DIR / f"{filename}.md"
    if not report.exists():
        return None
    text = report.read_text()
    statuses = re.findall(r"\*\*Status\*\*:\s*(PASS|ISSUE|SKIPPED)", text)
    return statuses[-1] if statuses else None


# ─── Prompt building ─────────────────────────────────────────────

def build_shared_prompt(now: str) -> str:
    """Build the shared audit prompt (cached across all agents)."""
    return f"""You are auditing an MTG card implementation and the engine mechanics it relies on. Your job is to verify that the card behaves correctly — if it doesn't, for ANY reason (card bug or engine bug), that is an ISSUE.

The current date and time is {now}.

## CRITICAL RULES

Previous audits produced false positives that wasted time and eroded trust in the audit process. Every rule below exists to prevent a specific failure mode we've actually hit.

Auditors have "remembered" old oracle text from training data (e.g., pre-2018 planeswalker damage redirect wording) and flagged working code as wrong, when in fact the card had been errata'd and the auditor's memory was stale. To prevent this, you must **NEVER use your training data as a source for oracle text, rulings, types, subtypes, costs, or any other card data.** The oracle text has been pre-fetched from Scryfall and provided in the per-agent prompt below. That is your single source of truth. Do not compare code against what you think the card does; compare only against the oracle text provided.

Auditors have also claimed "Scryfall says X but code says Y" without actually quoting both sides, and X turned out to be hallucinated. When forced to produce exact quotes, these phantom issues evaporated. To prevent this, **when claiming any mismatch you must quote both sides exactly** — the oracle text and the code. If you cannot produce both exact quotes, the mismatch is not verified and must not be flagged.

**If a card's behavior doesn't match the oracle text for ANY reason — whether the bug is in the card code, the engine, or both — that is an ISSUE.** Do not distinguish between "card bugs" and "engine bugs." The question is simple: does the card behave correctly? If no, mark ISSUE. Examples of engine bugs that must be marked ISSUE:
- Trigger system doesn't scan graveyard -> card's graveyard ability never fires -> ISSUE
- ETB trigger skipped when source leaves battlefield -> wrong per MTG rules -> ISSUE
- `abilities_activated_this_turn` never clears -> once-per-turn permanently locked -> ISSUE
- Simultaneous death triggers only fire once -> wrong trigger count -> ISSUE

Do not read any previous audit logs before conducting your audit. Your audit must be independent.

## Procedure — follow ALL steps

### Step 1. Record oracle text

The oracle text has been provided in the per-agent prompt. Write it down in your report verbatim. This is your single source of truth for the rest of the audit.

Pay special attention to rulings about timing, targeting, "you may" vs mandatory, "another" vs "a", and "each opponent" vs "target player".

### Step 2. Research community knowledge (for complex cards)

Skip for vanilla creatures and basic spells. For anything with triggered/activated abilities, unusual timing, replacement effects, or multi-step resolution, use WebSearch to look up rulings and corner cases.

### Step 3-4. Check the code against oracle text

Read the card's implementation file. Verify all card data and behavior against the oracle text.

Key things to verify: mana cost, card types, supertypes, subtypes, P/T, keywords, oracle text field (including "Enchant creature" prefix for auras), flashback cost, continuous effects, triggered_abilities TriggerKinds, targeting, "you may" optionality, "target" player choice, "each" vs "target", damage types (NonCombatDamageDealt not CombatDamageDealt), spell cleanup (move_spell_after_resolve), dynamic_pt, token subtypes.

**Subtype/type checking**: When the card checks a creature's subtype (e.g., "is it a Human?"), verify the check covers BOTH registry data (`registry.card_data()`) AND runtime object subtypes (`obj.subtypes`). Tokens store subtypes on the object, not in the registry. A check that only reads `registry.card_data()` will miss tokens. Compare with `check_condition` in `state.rs` which correctly checks both.

**"As long as" vs snapshot**: If the oracle text says "as long as" (e.g., "gets +2/+2 as long as it's a Human"), the effect must continuously re-evaluate. If the code sets the effect once at ETB and never rechecks, that's an ISSUE — the condition could change (e.g., creature transforms, gains/loses a type) and the effect wouldn't update.

### Step 5. Trace execution paths through the engine

Don't just read the card file — trace the full execution path into the engine to verify the card actually works end-to-end:

- **For triggered abilities**: Find where the trigger is dispatched in `mtg-engine/src/triggers.rs`. Read the actual dispatch code — don't assume it works. Verify:
  - Does the dispatch filter exclude cases the oracle text covers? (e.g., a SpellCast dispatch that only fires for instant/sorcery when the oracle says "a spell" with no type restriction)
  - Does the dispatch reach the card's hook at all for every case the oracle covers?
  - Are there guard conditions in the dispatch that would prevent the trigger from firing?
- **For activated abilities**: Trace through `engine.rs` to verify the ability is generated, costs are checked, and the effect is applied correctly.
- **For continuous effects**: Verify the effect scope and filter in `state.rs` match the oracle text.
- **For log messages**: Check that log messages accurately describe what's happening. A log that says "deals damage to opponent" when the target hasn't been chosen yet, or says "flashback" when the oracle says "from your graveyard", is an issue.

### Step 6. Think about tricky interactions

Consider stack interactions, semantic precision ("destroy" vs "sacrifice", "target" vs "choose", "you may" vs mandatory, "another" vs "a", "each" vs "target", "combat damage" vs "damage"), and interactions with other cards (indestructible, tokens, lifelink).

### Step 7-9. Check tests, UI, shortcuts

Search for tests in `mtg-engine/tests/`. Check UI presentation (choices presented? LLM card knowledge?). Check for known anti-patterns: `move_object(Zone::Graveyard)` instead of `move_spell_after_resolve`, `CombatDamageDealt` for non-combat damage, missing triggered_abilities declarations, auto-selecting choices, `try_destroy` when oracle says "sacrifice".

### Step 9. Reconcile findings

Before writing your final report:
- Re-read the oracle text provided.
- For each flagged issue, confirm the discrepancy still holds by quoting the oracle text AND the code side by side.
- Drop any issue where the quotes actually match or where you cannot produce an exact quote from the oracle text supporting the issue.
- Check for **outdated rules** — if your issue depends on a rule that may have changed (e.g., planeswalker damage redirect removed in 2018, "Hound" -> "Dog" errata, "dies" templating), verify the current rule applies.
- If you corrected yourself during the audit (e.g., "actually, this is fine"), make sure the correction is reflected in your final status. Do NOT leave a stale ISSUE status from before the correction.

### Step 10. Write report

Write the audit report to the file specified in the per-agent prompt. Create the file if it doesn't exist. Append if it does — never overwrite previous entries.

You MUST use this EXACT format — no prose summaries, no shortcuts:

## Audit — {now}

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {{exact text from per-agent prompt}}
**Type line**: {{exact type line from per-agent prompt}}
**Status**: PASS / ISSUE / SKIPPED

### Code issues
{{If PASS, write "No issues found."}}
{{If ISSUE, for each issue:}}
- {{Description with file path and line number}}
  - Oracle text says: `{{exact quote}}`
  - Code does: `{{exact quote or description of code behavior}}`

### Tricky interactions checked
- {{interaction 1}}: {{pass/fail}}
- {{interaction 2}}: {{pass/fail}}
{{List EVERY interaction you checked, even for simple cards. Minimum 3 items.}}

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- {{ruling or interaction}}: `test_file.rs:line_number` / NOT TESTED
{{List EVERY ruling and interaction with its test status.}}
"""


def build_per_agent_prompt(card_name: str, card_file: str, audit_file: str,
                           oracle_text: str) -> str:
    """Build the per-agent prompt (NOT cached — small and unique)."""
    return f"""## Card to audit: {card_name}

### Implementation file
`mtg-engine/src/cards/{card_file}`

### Oracle text (pre-fetched from Scryfall)

{oracle_text}

### Audit report file
Write your report to: `audits/reports/{audit_file}`
"""


# ─── Main ─────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Run card audits via batch agent runner")
    parser.add_argument("--set", help="Set code (e.g., 'isd')")
    parser.add_argument("--cards", help="Comma-separated card names")
    parser.add_argument("--batch-size", "-n", type=int, default=10,
                        help="Max concurrent agents (default: 10)")
    parser.add_argument("--start", type=int, default=1,
                        help="Start from card N (1-indexed)")
    parser.add_argument("--limit", type=int, help="Max cards to audit")
    parser.add_argument("--filter", choices=["pass", "issue", "skipped", "unaudited"],
                        help="Only audit cards with this status")
    parser.add_argument("--model", default="claude-sonnet-4-20250514",
                        help="Model to use")
    parser.add_argument("--max-turns", type=int, default=50,
                        help="Max turns per agent")
    parser.add_argument("--dry-run", action="store_true",
                        help="Show what would be audited without running")
    parser.add_argument("--no-sandbox", action="store_true",
                        help="Disable OS-level sandboxing")
    args = parser.parse_args()

    if not args.set and not args.cards:
        parser.error("Must specify --set or --cards")

    # Build card list
    if args.cards:
        names = [c.strip() for c in args.cards.split(",")]
        cards = []
        for name in names:
            fn = name.lower().replace(" ", "_").replace("'", "").replace(",", "").replace("-", "_")
            cards.append((name, fn))
    else:
        cards = get_cards_for_set(args.set)

    # Apply start/limit
    cards = cards[args.start - 1:]
    if args.limit:
        cards = cards[:args.limit]

    # Apply status filter
    if args.filter:
        filtered = []
        for name, fn in cards:
            status = get_latest_audit_status(fn)
            if args.filter == "unaudited" and status is None:
                filtered.append((name, fn))
            elif args.filter == "pass" and status == "PASS":
                filtered.append((name, fn))
            elif args.filter == "issue" and status == "ISSUE":
                filtered.append((name, fn))
            elif args.filter == "skipped" and status == "SKIPPED":
                filtered.append((name, fn))
        cards = filtered

    if not cards:
        print("No cards to audit.")
        return

    set_code = args.set or "isd"
    now = datetime.now().strftime("%Y-%m-%d %H:%M")

    print(f"Cards to audit: {len(cards)}")
    print(f"Concurrency: {args.batch_size}")
    print(f"Model: {args.model}")

    # Dry run
    if args.dry_run:
        for name, fn in cards:
            status = get_latest_audit_status(fn) or "UNAUDITED"
            print(f"  {fn}: {name} (current: {status})")
        return

    # Ensure reports directory exists
    REPORTS_DIR.mkdir(parents=True, exist_ok=True)

    # Pre-fetch oracle text
    print(f"\nFetching oracle text for {len(cards)} cards...")
    oracle_texts = {}
    skipped = []
    for name, fn in cards:
        oracle = get_oracle_text(name)
        if oracle:
            oracle_texts[fn] = oracle
        else:
            print(f"  WARNING: No oracle text for {name}, will skip")
            skipped.append(name)
    print(f"  Got {len(oracle_texts)}/{len(cards)} oracle texts")

    # Write shared prompt
    shared_prompt = build_shared_prompt(now)
    SHARED_PROMPT_FILE.write_text(shared_prompt)
    print(f"  Shared prompt: {len(shared_prompt)} chars -> {SHARED_PROMPT_FILE}")

    # Generate agents.jsonl
    agents_file = AUDITS_DIR / "agents.jsonl"
    with open(agents_file, "w") as f:
        for name, fn in cards:
            if fn not in oracle_texts:
                continue
            card_file = f"{set_code}/{fn}.rs"
            audit_file = f"{fn}.md"
            prompt = build_per_agent_prompt(name, card_file, audit_file,
                                           oracle_texts[fn])
            agent = {
                "id": fn,
                "prompt": prompt,
                "workDir": str(PROJECT_ROOT),
            }
            f.write(json.dumps(agent) + "\n")
    agent_count = len(oracle_texts)
    print(f"  Wrote {agent_count} agents -> {agents_file}")

    # Build batch runner command
    cmd = [
        "bun", "run", str(BATCH_RUNNER),
        "--shared-prompt-file", str(SHARED_PROMPT_FILE),
        "--agents-file", str(agents_file),
        "--concurrency", str(args.batch_size),
        "--model", args.model,
        "--max-turns", str(args.max_turns),
        "--output-dir", str(AUDITS_DIR / "results"),
    ]
    if args.no_sandbox:
        cmd.append("--no-sandbox")

    print(f"\nLaunching batch runner...")
    print(f"  {' '.join(cmd)}\n")

    # Set up environment
    env = os.environ.copy()
    env["PATH"] = f"{Path.home()}/.bun/bin:/opt/homebrew/bin:{env.get('PATH', '')}"
    env["FORCE_1H_CACHE_TTL"] = "1"  # 1-hour cache for large batches

    # Run
    proc = subprocess.run(cmd, env=env, cwd=str(PROJECT_ROOT))

    if proc.returncode != 0:
        print(f"\nBatch runner exited with code {proc.returncode}", file=sys.stderr)
        sys.exit(proc.returncode)

    # Parse results
    results_dir = AUDITS_DIR / "results"
    pass_count = 0
    issue_count = 0
    skip_count = 0
    issues = []

    if results_dir.exists():
        for result_file in sorted(results_dir.glob("*.json")):
            try:
                data = json.loads(result_file.read_text())
                card_id = data.get("id", result_file.stem)
                status = get_latest_audit_status(card_id)
                if status == "PASS":
                    pass_count += 1
                elif status == "ISSUE":
                    issue_count += 1
                    issues.append(card_id)
                else:
                    skip_count += 1
            except (json.JSONDecodeError, KeyError):
                skip_count += 1

    skip_count += len(skipped)

    # Summary
    print(f"\n{'='*60}")
    print(f"AUDIT SUMMARY — {now}")
    print(f"{'='*60}")
    print(f"Total: {agent_count + len(skipped)} cards")
    print(f"  PASS:    {pass_count}")
    print(f"  ISSUE:   {issue_count}")
    print(f"  SKIPPED: {skip_count}")
    if issues:
        print(f"\nISSUE cards:")
        for card_id in issues:
            print(f"  - {filename_to_card_name(card_id)}")


if __name__ == "__main__":
    main()
