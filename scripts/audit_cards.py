#!/usr/bin/env python3
"""Audit MTG cards using Claude Code CLI subagents.

Usage:
    python3 scripts/audit_cards.py --set isd --batch-size 10
    python3 scripts/audit_cards.py --cards "Fiend Hunter,Delver of Secrets" --batch-size 2
    python3 scripts/audit_cards.py --set isd --start 141 --batch-size 10  # resume from card 141
    python3 scripts/audit_cards.py --set isd --filter-status PASS  # re-audit only PASSing cards
"""

import argparse
import asyncio
import json
import os
import re
import subprocess
import sys
from datetime import datetime
from pathlib import Path


# ─── Card discovery ───────────────────────────────────────────────

def get_cards_for_set(set_code: str) -> list[str]:
    """Get all card filenames (without .rs) for a set, sorted alphabetically."""
    cards_dir = Path(f"mtg-engine/src/cards/{set_code}")
    if not cards_dir.exists():
        print(f"Error: directory {cards_dir} not found", file=sys.stderr)
        sys.exit(1)
    return sorted([f.stem for f in cards_dir.glob("*.rs") if f.stem != "mod"])


def filename_to_card_name(filename: str) -> str:
    """Convert snake_case filename to Title Case card name.

    This is a rough heuristic. The oracle lookup will use fuzzy matching.
    """
    # Special cases
    special = {
        "avacyns_pilgrim": "Avacyn's Pilgrim",
        "ghoulcallers_bell": "Ghoulcaller's Bell",
        "ghoulcallers_chant": "Ghoulcaller's Chant",
        "grimgrin_corpse_born": "Grimgrin, Corpse-Born",
        "ludevics_test_subject": "Ludevic's Test Subject",
        "runechanters_pike": "Runechanter's Pike",
        "butchers_cleaver": "Butcher's Cleaver",
        "geistcatchers_rig": "Geistcatcher's Rig",
        "heretics_punishment": "Heretic's Punishment",
        "inquisitors_flail": "Inquisitor's Flail",
        "travelers_amulet": "Traveler's Amulet",
        "nightbirds_clutches": "Nightbird's Clutches",
        "memorys_journey": "Memory's Journey",
        "devils_play": "Devil's Play",
        "stitchers_apprentice": "Stitcher's Apprentice",
        "altars_reap": "Altar's Reap",
    }
    if filename in special:
        return special[filename]

    # General: split on underscore, title case each word
    return " ".join(word.capitalize() for word in filename.split("_"))


def get_oracle_text(card_name: str) -> str | None:
    """Fetch oracle text from local cache or Scryfall API."""
    # Try lookup first
    result = subprocess.run(
        ["python3", "scripts/oracle_lookup.py", "lookup", card_name],
        capture_output=True, text=True, cwd=os.getcwd()
    )
    if result.returncode == 0 and result.stdout.strip():
        return result.stdout.strip()

    # Try fetch
    result = subprocess.run(
        ["python3", "scripts/oracle_lookup.py", "fetch", card_name],
        capture_output=True, text=True, cwd=os.getcwd()
    )
    if result.returncode == 0 and result.stdout.strip():
        return result.stdout.strip()

    return None


def get_latest_audit_status(audit_file: Path) -> str | None:
    """Parse the most recent audit status from an audit file."""
    if not audit_file.exists():
        return None
    text = audit_file.read_text()
    # Find all Status lines
    statuses = re.findall(r'\*\*Status\*\*:\s*(PASS|ISSUE|SKIPPED)', text)
    return statuses[-1] if statuses else None


# ─── Prompt building ──────────────────────────────────────────────

def build_prompt(card_name: str, card_file: str, audit_file: str,
                 oracle_text: str, now: str) -> str:
    """Build the full audit prompt with oracle text pre-included."""
    return f"""You are auditing the MTG card **{card_name}** and the engine mechanics it relies on. Your job is to verify that the card behaves correctly — if it doesn't, for ANY reason (card bug or engine bug), that is an ISSUE.

The current date and time is {now}.

The card implementation is at `mtg-engine/src/cards/{card_file}`.

## Oracle text (pre-fetched from Scryfall — this is your source of truth)

{oracle_text}

## CRITICAL RULES

Previous audits produced false positives that wasted time and eroded trust in the audit process. Every rule below exists to prevent a specific failure mode we've actually hit.

Auditors have "remembered" old oracle text from training data (e.g., pre-2018 planeswalker damage redirect wording) and flagged working code as wrong, when in fact the card had been errata'd and the auditor's memory was stale. To prevent this, you must **NEVER use your training data as a source for oracle text, rulings, types, subtypes, costs, or any other card data.** The oracle text above was fetched from Scryfall and is your single source of truth. Do not compare code against what you think the card does; compare only against the oracle text provided above.

Auditors have also claimed "Scryfall says X but code says Y" without actually quoting both sides, and X turned out to be hallucinated. When forced to produce exact quotes, these phantom issues evaporated. To prevent this, **when claiming any mismatch you must quote both sides exactly** — the oracle text and the code. If you cannot produce both exact quotes, the mismatch is not verified and must not be flagged.

**If a card's behavior doesn't match the oracle text for ANY reason — whether the bug is in the card code, the engine, or both — that is an ISSUE.** Do not distinguish between "card bugs" and "engine bugs." The question is simple: does the card behave correctly? If no, mark ISSUE. Examples of engine bugs that must be marked ISSUE:
- Trigger system doesn't scan graveyard -> card's graveyard ability never fires -> ISSUE
- ETB trigger skipped when source leaves battlefield -> wrong per MTG rules -> ISSUE
- `abilities_activated_this_turn` never clears -> once-per-turn permanently locked -> ISSUE
- Simultaneous death triggers only fire once -> wrong trigger count -> ISSUE

Do not read any previous audit logs before conducting your audit. Your audit must be independent.

## Procedure — follow ALL steps

### Step 1. Record oracle text

The oracle text has been provided above. Write it down in your report verbatim. This is your single source of truth for the rest of the audit.

Pay special attention to rulings about timing, targeting, "you may" vs mandatory, "another" vs "a", and "each opponent" vs "target player".

### Step 2. Research community knowledge (for complex cards)

Skip for vanilla creatures and basic spells. For anything with triggered/activated abilities, unusual timing, replacement effects, or multi-step resolution:

- WebSearch: `{card_name} MTG rulings interactions`
- WebSearch: `{card_name} MTG rules corner cases`

### Step 3-4. Check the code against oracle text

Read the implementation. Verify all card data and behavior against the oracle text above. Use the checklists in `.claude/commands/check-card-procedure.md` for reference if needed.

Key things to verify: mana cost, card types, supertypes, subtypes, P/T, keywords, oracle text field, flashback cost, continuous effects, triggered_abilities TriggerKinds, targeting, "you may" optionality, "target" player choice, "each" vs "target", damage types (NonCombatDamageDealt not CombatDamageDealt), spell cleanup (move_spell_after_resolve), dynamic_pt, token subtypes.

### Step 5. Think about tricky interactions

Consider stack interactions, semantic precision ("destroy" vs "sacrifice", "target" vs "choose", "you may" vs mandatory, "another" vs "a", "each" vs "target", "combat damage" vs "damage"), and interactions with other cards (indestructible, tokens, lifelink).

### Step 6-8. Check tests, UI, shortcuts

Search for tests in `mtg-engine/tests/`. Check UI presentation (choices presented? LLM card knowledge?). Check for known anti-patterns: `move_object(Zone::Graveyard)` instead of `move_spell_after_resolve`, `CombatDamageDealt` for non-combat damage, missing triggered_abilities declarations, auto-selecting choices, `try_destroy` when oracle says "sacrifice".

### Step 9. Reconcile findings

Before writing your final report:
- Re-read the oracle text provided above.
- For each flagged issue, confirm the discrepancy still holds by quoting the oracle text AND the code side by side.
- Drop any issue where the quotes actually match or where you cannot produce an exact quote from the oracle text supporting the issue.
- Check for **outdated rules** — if your issue depends on a rule that may have changed (e.g., planeswalker damage redirect removed in 2018, "Hound" -> "Dog" errata, "dies" templating), verify the current rule applies.
- If you corrected yourself during the audit (e.g., "actually, this is fine"), make sure the correction is reflected in your final status. Do NOT leave a stale ISSUE status from before the correction.

### Step 10. Write report

Append to `audits/{audit_file}` (create if needed). Never overwrite previous entries.

You MUST use this EXACT format — no prose summaries, no shortcuts:

## Audit — {now}

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: {{exact text from above}}
**Type line**: {{exact type line from above}}
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
{{List EVERY ruling and interaction with its test status.}}"""


# ─── Subagent execution ──────────────────────────────────────────

async def audit_card(card_name: str, filename: str, prompt: str,
                     semaphore: asyncio.Semaphore) -> dict:
    """Run a single card audit via claude CLI."""
    async with semaphore:
        print(f"  Starting: {card_name}")
        proc = await asyncio.create_subprocess_exec(
            "claude", "-p", prompt,
            "--allowedTools", "Read,Grep,Glob,Write,WebSearch,Bash",
            "--max-turns", "50",
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        stdout, stderr = await proc.communicate()

        # Parse status from the audit file
        audit_path = Path(f"audits/{filename}.md")
        status = get_latest_audit_status(audit_path)

        result = {
            "card": card_name,
            "filename": filename,
            "status": status or "UNKNOWN",
            "returncode": proc.returncode,
            "output_length": len(stdout),
        }

        icon = {"PASS": "✓", "ISSUE": "✗", "SKIPPED": "?", "UNKNOWN": "!"}.get(result["status"], "?")
        print(f"  {icon} {card_name}: {result['status']}")
        return result


async def run_audit(cards: list[tuple[str, str]], batch_size: int, set_code: str):
    """Run the full audit pipeline."""
    now = datetime.now().strftime("%Y-%m-%d %H:%M")

    # Pre-fetch all oracle text
    print(f"\nFetching oracle text for {len(cards)} cards...")
    oracle_texts = {}
    for card_name, filename in cards:
        oracle = get_oracle_text(card_name)
        if oracle:
            oracle_texts[filename] = oracle
        else:
            print(f"  WARNING: Could not fetch oracle text for {card_name}")
    print(f"  Fetched {len(oracle_texts)}/{len(cards)} oracle texts\n")

    # Build prompts
    prompts = {}
    for card_name, filename in cards:
        if filename not in oracle_texts:
            continue
        card_file = f"{set_code}/{filename}.rs"
        audit_file = f"{filename}.md"
        prompts[filename] = build_prompt(
            card_name, card_file, audit_file,
            oracle_texts[filename], now
        )

    # Run in batches
    semaphore = asyncio.Semaphore(batch_size)
    all_results = []

    tasks = []
    for card_name, filename in cards:
        if filename not in prompts:
            all_results.append({
                "card": card_name, "filename": filename,
                "status": "SKIPPED", "returncode": -1, "output_length": 0
            })
            continue
        tasks.append(audit_card(card_name, filename, prompts[filename], semaphore))

    print(f"Running {len(tasks)} audits with batch size {batch_size}...\n")
    results = await asyncio.gather(*tasks)
    all_results.extend(results)

    # Commit results
    print("\nCommitting audit results...")
    subprocess.run(["git", "add", "audits/"], check=True)

    card_names = ", ".join(r["card"] for r in all_results[:5])
    if len(all_results) > 5:
        card_names += f", ... (+{len(all_results) - 5} more)"

    pass_count = sum(1 for r in all_results if r["status"] == "PASS")
    issue_count = sum(1 for r in all_results if r["status"] == "ISSUE")
    skip_count = sum(1 for r in all_results if r["status"] == "SKIPPED")

    commit_msg = (
        f"Audit {len(all_results)} cards: {card_names}\n\n"
        f"{pass_count} PASS, {issue_count} ISSUE, {skip_count} SKIPPED"
    )
    subprocess.run(["git", "commit", "-m", commit_msg], check=True)
    subprocess.run(["git", "push"], check=True)

    # Print summary
    print(f"\n{'='*60}")
    print(f"AUDIT SUMMARY — {now}")
    print(f"{'='*60}")
    print(f"Total: {len(all_results)} cards")
    print(f"  PASS:    {pass_count}")
    print(f"  ISSUE:   {issue_count}")
    print(f"  SKIPPED: {skip_count}")

    if issue_count > 0:
        print(f"\nISSUE cards:")
        for r in all_results:
            if r["status"] == "ISSUE":
                print(f"  - {r['card']}")

    return all_results


# ─── CLI ──────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Audit MTG cards using Claude Code CLI")
    parser.add_argument("--set", help="Set code (e.g., 'isd')")
    parser.add_argument("--cards", help="Comma-separated card names")
    parser.add_argument("--batch-size", type=int, default=10, help="Max concurrent audits")
    parser.add_argument("--start", type=int, default=1, help="Start from card N (1-indexed)")
    parser.add_argument("--limit", type=int, help="Max cards to audit")
    parser.add_argument("--filter-status", choices=["PASS", "ISSUE", "SKIPPED", "UNAUDITED"],
                        help="Only audit cards with this current status")
    parser.add_argument("--dry-run", action="store_true", help="Show what would be audited")

    args = parser.parse_args()

    if not args.set and not args.cards:
        parser.error("Must specify --set or --cards")

    # Build card list
    if args.cards:
        # Manual card list
        card_names = [c.strip() for c in args.cards.split(",")]
        cards = []
        for name in card_names:
            filename = name.lower().replace(" ", "_").replace("'", "").replace(",", "").replace("-", "_")
            cards.append((name, filename))
    else:
        # Set-based card list
        filenames = get_cards_for_set(args.set)
        cards = [(filename_to_card_name(f), f) for f in filenames]

    # Apply start offset
    cards = cards[args.start - 1:]

    # Apply limit
    if args.limit:
        cards = cards[:args.limit]

    # Apply status filter
    if args.filter_status:
        filtered = []
        for card_name, filename in cards:
            audit_path = Path(f"audits/{filename}.md")
            current_status = get_latest_audit_status(audit_path)

            if args.filter_status == "UNAUDITED" and current_status is None:
                filtered.append((card_name, filename))
            elif current_status == args.filter_status:
                filtered.append((card_name, filename))
        cards = filtered

    if not cards:
        print("No cards to audit.")
        return

    print(f"Cards to audit: {len(cards)}")
    print(f"Batch size: {args.batch_size}")

    if args.dry_run:
        for card_name, filename in cards:
            audit_path = Path(f"audits/{filename}.md")
            status = get_latest_audit_status(audit_path) or "UNAUDITED"
            print(f"  {filename}: {card_name} (current: {status})")
        return

    asyncio.run(run_audit(cards, args.batch_size, args.set or "isd"))


if __name__ == "__main__":
    main()
