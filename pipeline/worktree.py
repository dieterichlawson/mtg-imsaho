"""Git worktree helpers — one worktree per ticket, stored under .worktrees/.

Each ticket that's progressed past the audit phase owns a worktree on a
branch named `fix/<ticket_id>`. The worktree is created by the
test-writer phase, carried forward through fix, and removed on merge
or abandon.
"""
from __future__ import annotations

import subprocess
from pathlib import Path

from pipeline import paths


def dir_for(ticket_id: str) -> Path:
    """Path to the worktree directory for a ticket."""
    return paths.WORKTREES_DIR / f"fix-{ticket_id}"


def branch_for(ticket_id: str) -> str:
    """Name of the git branch for a ticket's worktree."""
    return f"fix/{ticket_id}"


def ensure(ticket_id: str) -> Path:
    """Create (or reuse) the ticket's worktree. Returns its path."""
    wt = dir_for(ticket_id)
    if wt.exists():
        return wt
    paths.WORKTREES_DIR.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        ["git", "worktree", "add", "-b", branch_for(ticket_id), str(wt), "HEAD"],
        capture_output=True, check=True, cwd=str(paths.PROJECT_ROOT))
    return wt


def remove(ticket_id: str) -> None:
    """Tear down a ticket's worktree directory and branch."""
    wt = dir_for(ticket_id)
    if wt.exists():
        subprocess.run(["git", "worktree", "remove", "--force", str(wt)],
                       capture_output=True, cwd=str(paths.PROJECT_ROOT))
    subprocess.run(["git", "branch", "-D", branch_for(ticket_id)],
                   capture_output=True, cwd=str(paths.PROJECT_ROOT))


def branch_head(branch: str) -> str:
    """Return the sha at the tip of a branch, or '' if absent."""
    r = subprocess.run(["git", "rev-parse", branch],
                       capture_output=True, text=True,
                       cwd=str(paths.PROJECT_ROOT))
    return r.stdout.strip() if r.returncode == 0 else ""


def reset_to(ticket_id: str, sha: str) -> None:
    """Hard-reset the ticket's branch to a sha (or 'master'). Creates the
    worktree if it's missing.
    """
    wt = dir_for(ticket_id)
    branch = branch_for(ticket_id)
    if not wt.exists():
        paths.WORKTREES_DIR.mkdir(parents=True, exist_ok=True)
        has_branch = subprocess.run(
            ["git", "rev-parse", "--verify", branch],
            capture_output=True, cwd=str(paths.PROJECT_ROOT)).returncode == 0
        cmd = (["git", "worktree", "add", str(wt), branch] if has_branch
               else ["git", "worktree", "add", "-b", branch, str(wt), sha])
        subprocess.run(cmd, check=True, cwd=str(paths.PROJECT_ROOT))
    subprocess.run(["git", "reset", "--hard", sha], check=True, cwd=str(wt))


def rename(old_id: str, new_id: str) -> str:
    """Move the worktree directory and rename the branch to `new_id`.

    Used by consolidate when a tested source is absorbed into a new parent.
    Returns the head sha after the rename.
    """
    old_wt = dir_for(old_id)
    new_wt = dir_for(new_id)
    subprocess.run(["git", "worktree", "move", str(old_wt), str(new_wt)],
                   check=True, cwd=str(paths.PROJECT_ROOT),
                   capture_output=True, text=True)
    subprocess.run(["git", "branch", "-m",
                    branch_for(old_id), branch_for(new_id)],
                   check=True, cwd=str(new_wt),
                   capture_output=True, text=True)
    return subprocess.run(["git", "rev-parse", "HEAD"], check=True,
                          cwd=str(new_wt), capture_output=True,
                          text=True).stdout.strip()


def is_clean(wt: Path) -> str:
    """Return the `git status --porcelain` output ('' means clean)."""
    return subprocess.run(["git", "status", "--porcelain"],
                          capture_output=True, text=True,
                          cwd=str(wt)).stdout.strip()


def merge_into_head(ticket_id: str) -> tuple[bool, str]:
    """Merge the ticket's branch into PROJECT_ROOT HEAD. Returns
    (success, merge_sha_or_error_tail).
    """
    r = subprocess.run(
        ["git", "merge", branch_for(ticket_id), "--no-edit"],
        capture_output=True, text=True, cwd=str(paths.PROJECT_ROOT))
    if r.returncode != 0:
        return False, r.stderr[:200]
    sha = subprocess.run(
        ["git", "rev-parse", "HEAD"], capture_output=True, text=True,
        cwd=str(paths.PROJECT_ROOT)).stdout.strip()
    return True, sha


def revert_head(to: str = "HEAD~1") -> None:
    """Undo the most recent merge by resetting master. Used when a post-
    merge validation fails.
    """
    subprocess.run(["git", "reset", "--hard", to],
                   capture_output=True, cwd=str(paths.PROJECT_ROOT))
