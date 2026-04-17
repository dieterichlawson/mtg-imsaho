"""Git worktree management — one isolated worktree per ticket.

Each ticket that progresses past audit gets its own git worktree on
a branch named `fix/<ticket_id>`, stored at
`<project-root>/.worktrees/fix-<ticket_id>/`. Later PRs wire this up
to the test-writer and fixer agents so they edit code on their own
branch without touching master.
"""

from __future__ import annotations

import subprocess
from pathlib import Path

from new_pipeline import utils


def dir_for(ticket_id: str) -> Path:
    """Path to the worktree directory for a ticket."""
    return utils.WORKTREES_DIR / f"fix-{ticket_id}"


def branch_for(ticket_id: str) -> str:
    """Name of the git branch for a ticket's worktree."""
    return f"fix/{ticket_id}"


def ensure(ticket_id: str) -> Path:
    """Create (or reuse) the ticket's worktree. Returns its path.

    If the worktree directory already exists, returns it unchanged —
    safe to call repeatedly within a run.
    """
    wt = dir_for(ticket_id)
    if wt.exists():
        return wt
    utils.WORKTREES_DIR.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [
            "git", "worktree", "add",
            "-b", branch_for(ticket_id),
            str(wt), "HEAD",
        ],
        check=True,
        capture_output=True,
        cwd=str(utils.PROJECT_ROOT),
    )
    return wt


def remove(ticket_id: str) -> None:
    """Tear down a ticket's worktree directory and branch. Idempotent.

    Missing worktree or branch is not an error — safe to call during
    cleanup paths without first checking whether the ticket ever had
    an associated worktree.
    """
    wt = dir_for(ticket_id)
    if wt.exists():
        subprocess.run(
            ["git", "worktree", "remove", "--force", str(wt)],
            capture_output=True,
            cwd=str(utils.PROJECT_ROOT),
        )
    subprocess.run(
        ["git", "branch", "-D", branch_for(ticket_id)],
        capture_output=True,
        cwd=str(utils.PROJECT_ROOT),
    )


def branch_head(branch: str) -> str:
    """Return the sha at the tip of `branch`, or '' if it doesn't exist."""
    r = subprocess.run(
        ["git", "rev-parse", branch],
        capture_output=True,
        text=True,
        cwd=str(utils.PROJECT_ROOT),
    )
    return r.stdout.strip() if r.returncode == 0 else ""
