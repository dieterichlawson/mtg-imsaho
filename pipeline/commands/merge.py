"""Merge `fixed` ticket(s) into master — they become `shipped`.

The merge runs each ticket's own tests at HEAD as a post-merge gate.
If any fail, the merge is reverted and the ticket stays `fixed`.
"""

from __future__ import annotations

import re
import subprocess

from pipeline import ticket, utils, worktree
from pipeline.metrics import log_finding
from pipeline.state import LifecycleEvent, Status, next_status
from pipeline.ticket import Ticket, parse_tests_section
from pipeline.utils import now_iso


def cmd_merge(args):
    """Entry point for `./pipeline/cli.py merge`."""
    ids = [t.strip() for t in args.ticket_id.split(",")]
    candidates = _collect(ids)
    if not candidates:
        print(f"No {Status.FIXED.value} tickets to merge.")
        return

    print(f"\n{'=' * 60}\nMERGE — {len(candidates)} ticket(s)\n{'=' * 60}")
    for t in candidates:
        print(f"  {t.id:<30} {t.card}")

    for t in candidates:
        merge_one(t)


def _collect(ids: list[str]) -> list[Ticket]:
    if ids == ["all"]:
        return ticket.list_all(status=Status.FIXED)
    out = []
    for tid in ids:
        t = ticket.get_ticket_if_exists(tid)
        if t is None:
            print(f"  Skipping {tid}: not found")
            continue
        if t.status is not Status.FIXED:
            print(
                f"  Skipping {tid}: status is {t.status.value}, "
                f"not {Status.FIXED.value}"
            )
            continue
        out.append(t)
    return out


def merge_one(t: Ticket) -> None:
    """Merge one ticket's branch into master and mark it shipped.

    Runs the ticket's own tests at HEAD after the merge; a failure
    reverts the merge and leaves the ticket in `fixed` for human triage.
    """
    print(f"\n  [{t.id}] Merging...")
    wt = worktree.dir_for(t.id)
    if not wt.exists():
        print(f"  [{t.id}] No worktree found — skipping")
        return

    ok, sha_or_err = worktree.merge_into_head(t.id)
    if not ok:
        print(f"  [{t.id}] Merge FAILED: {sha_or_err}")
        return
    merge_sha = sha_or_err

    if not _tests_pass_at_head(t):
        print(f"  [{t.id}] Tests fail at HEAD after merge — reverting")
        worktree.revert_head()
        return

    t.status = next_status(t.status, LifecycleEvent.MERGED)
    t.frontmatter.shipped_at = now_iso()
    if merge_sha:
        t.frontmatter.shipped_sha = merge_sha
    t.frontmatter.worktree = ""
    t.save()

    worktree.remove(t.id)
    removed = _remove_logs(t.id)
    if removed:
        print(f"  [{t.id}] Removed {removed} agent log file(s)")
    log_finding(t.id, "shipped", card=t.card, run_id="merge")
    print(f"  [{t.id}] Shipped and cleaned up")


def _tests_pass_at_head(t: Ticket) -> bool:
    """Run the ticket's test functions at PROJECT_ROOT HEAD.

    Returns True if they all pass (or if we can't derive any test names —
    no-op rather than blocking).
    """
    fns = [
        impl.split("::", 1)[1]
        for entry in parse_tests_section(t.body)
        if "::" in (impl := entry.implementation.strip())
    ]
    if not fns:
        return True
    r = subprocess.run(
        ["cargo", "test", "--", *fns],
        capture_output=True,
        text=True,
        timeout=600,
        cwd=str(utils.PROJECT_ROOT),
    )
    ok = r.returncode == 0 and "FAILED" not in (r.stdout or "")
    if ok:
        print(f"  [{t.id}] {len(fns)} test fn(s) pass at HEAD")
    return ok


def _remove_logs(ticket_id: str) -> int:
    if not utils.LOGS_DIR.exists():
        return 0
    # Match `<date>-<ticket_id>{-|.}` — substring match would clobber logs
    # for differently-named tickets that contain our id.
    pat = re.compile(rf"\b{re.escape(ticket_id)}(?:[-.]|$)")
    removed = 0
    for f in utils.LOGS_DIR.iterdir():
        if pat.search(f.name):
            try:
                f.unlink()
                removed += 1
            except OSError:
                pass
    return removed
