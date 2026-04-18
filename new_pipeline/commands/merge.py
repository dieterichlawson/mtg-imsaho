"""Merge command — land `fixed` ticket commits onto master.

For each ticket id the operator passes:
    load → guard status == FIXED → ensure master is checked out and
    clean → `git merge --no-ff fix/<id>` → mark merged with the merge
    sha → save (auto-archives because MERGED is terminal) → commit the
    archival as a follow-up so the next merge sees a clean tree → tear
    down the worktree.

The follow-up commit (separate from the merge commit) is necessary
because `Ticket.save()` moves the file from `tickets/` to
`tickets/archive/`. Without committing that move, the project root is
dirty for the next merge.

If the merge hits a conflict, `worktree.merge_to_master` aborts the
merge so the working tree is restored, raises `MergeConflictError`,
and the ticket is left in `fixed` for the operator to deal with
manually.
After a conflict we stop processing remaining tickets — there's no
guarantee a later merge would apply cleanly while the operator is
mid-resolution on the failing one.

Pre-flight guards (current branch, clean tree) run once up front.
After a successful merge those properties may have changed (the new
merge commit moved HEAD), but they remain valid for chained merges
since each merge leaves the tree clean and on the same branch.
"""

from __future__ import annotations

import subprocess

from new_pipeline import utils, worktree
from new_pipeline.types import Status, Ticket, TicketError

_MASTER_BRANCH = "master"


def cmd_merge(args) -> None:
    """Entry point for `./new_pipeline/cli.py merge`."""
    ids = [i.strip() for i in args.tickets.split(",") if i.strip()]
    if not ids:
        raise ValueError("--tickets needs at least one non-empty id")

    branch = worktree.current_branch()
    if branch != _MASTER_BRANCH:
        raise TicketError(
            f"merge must run from {_MASTER_BRANCH!r}; "
            f"project root is currently on {branch!r}"
        )
    if not worktree.is_clean():
        raise TicketError(
            "merge requires a clean working tree at the project root; "
            "commit or stash before running"
        )

    merged: list[str] = []
    for tid in ids:
        ok = _merge_one(tid)
        if not ok:
            print(
                f"\nStopped at conflict on {tid!r}. "
                f"Merged so far: {merged or 'none'}"
            )
            return
        merged.append(tid)

    print(f"\nMerge done: {len(merged)} ticket(s) landed on {_MASTER_BRANCH}.")


def _merge_one(tid: str) -> bool:
    """Merge one ticket. Returns True on success, False on conflict."""
    t = Ticket.load(tid)
    if t.status is not Status.FIXED:
        print(
            f"[{tid}] Skip — status is {t.status.value}, not "
            f"{Status.FIXED.value}"
        )
        return True  # not a conflict; just nothing to do here

    branch = worktree.branch_for(tid)
    if not worktree.branch_head(branch):
        raise TicketError(
            f"[{tid}] no branch {branch!r}; expected from the fix phase"
        )

    print(f"\n[{tid}] Merging {branch} into {_MASTER_BRANCH}...")
    try:
        merge_sha = worktree.merge_to_master(tid)
    except worktree.MergeConflictError as e:
        print(f"[{tid}] CONFLICT: {e}")
        return False

    t.mark_merged(merged_sha=merge_sha)
    t.save()
    _commit_archival(tid, merge_sha)
    worktree.remove(tid)
    print(f"[{tid}] {Status.MERGED.value} at {merge_sha[:8]}; worktree removed")
    return True


def _commit_archival(tid: str, merge_sha: str) -> None:
    """Commit the ticket file move (active → archive) on master."""
    subprocess.run(
        ["git", "add", "new_pipeline/tickets/"],
        check=True, capture_output=True,
        cwd=str(utils.PROJECT_ROOT),
    )
    subprocess.run(
        [
            "git", "commit", "-q",
            "-m", f"Archive ticket {tid} (merged at {merge_sha[:8]})",
        ],
        check=True, capture_output=True,
        cwd=str(utils.PROJECT_ROOT),
    )
