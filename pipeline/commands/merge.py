"""Merge `fixed` ticket(s) into master — they become `shipped`.

The merge runs each ticket's own tests at HEAD as a post-merge gate.
If any fail, the merge is reverted and the ticket stays `fixed`.
"""
from __future__ import annotations

import re
import subprocess

from pipeline import paths, ticket, worktree
from pipeline.metrics import log_finding, now_iso
from pipeline.state import Status, after_merge
from pipeline.ticket import Ticket, parse_tests_section


def cmd_merge(args):
    ids = [t.strip() for t in args.ticket_id.split(",")]
    candidates = _collect(ids)
    if not candidates:
        print(f"No {Status.FIXED.value} tickets to merge.")
        return

    print(f"\n{'='*60}\nMERGE — {len(candidates)} ticket(s)\n{'='*60}")
    for t in candidates:
        print(f"  {t.id:<30} {t.card}")

    for t in candidates:
        merge_one(t)


def _collect(ids: list[str]) -> list[Ticket]:
    if ids == ["all"]:
        return ticket.list_all(status=Status.FIXED)
    out = []
    for tid in ids:
        t = ticket.find(tid)
        if t is None:
            print(f"  Skipping {tid}: not found")
            continue
        if t.status is not Status.FIXED:
            print(f"  Skipping {tid}: status is {t.status.value}, "
                  f"not {Status.FIXED.value}")
            continue
        out.append(t)
    return out


def merge_one(t: Ticket) -> None:
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

    t.status = after_merge(t.status)
    t.frontmatter["shipped_at"] = now_iso()
    if merge_sha:
        t.frontmatter["shipped_sha"] = merge_sha
    t.frontmatter.pop("worktree", None)
    t.save()

    worktree.remove(t.id)
    removed = _remove_logs(t.id)
    if removed:
        print(f"  [{t.id}] Removed {removed} agent log file(s)")
    log_finding(t.id, "shipped", card=t.card, run_id="merge")
    print(f"  [{t.id}] Shipped and cleaned up")


def _tests_pass_at_head(t: Ticket) -> bool:
    """Run the ticket's test functions at PROJECT_ROOT HEAD. Returns
    True if they all pass (or if we can't derive any names — no-op
    rather than blocking).
    """
    fns = [impl.split("::", 1)[1]
           for entry in parse_tests_section(t.body)
           if "::" in (impl := entry.get("implementation", "").strip())]
    if not fns:
        return True
    r = subprocess.run(["cargo", "test", "--", *fns],
                       capture_output=True, text=True, timeout=600,
                       cwd=str(paths.PROJECT_ROOT))
    ok = r.returncode == 0 and "FAILED" not in (r.stdout or "")
    if ok:
        print(f"  [{t.id}] {len(fns)} test fn(s) pass at HEAD")
    return ok


def _remove_logs(ticket_id: str) -> int:
    if not paths.LOGS_DIR.exists():
        return 0
    # Match `<date>-<ticket_id>{-|.}` — substring match would clobber logs
    # for differently-named tickets that contain our id.
    pat = re.compile(rf"\b{re.escape(ticket_id)}(?:[-.]|$)")
    removed = 0
    for f in paths.LOGS_DIR.iterdir():
        if pat.search(f.name):
            try:
                f.unlink()
                removed += 1
            except OSError:
                pass
    return removed
