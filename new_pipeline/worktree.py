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
    _write_cargo_env_override(wt)
    return wt


def _write_cargo_env_override(wt: Path) -> None:
    """Write `<wt>/.cargo/config.toml` with a dummy ANTHROPIC_API_KEY.

    mtg-player tests (e.g. `llm_conversation`) `.expect()` the var
    just to construct an `AnthropicBackend` for prompt-formatting
    tests — they never hit the API. Our pipeline strips the real
    key from every subprocess (so claude uses subscription auth), so
    cargo runs inherit an unset var and those tests panic.

    cargo walks up from cwd looking for `.cargo/config.toml` and
    applies the `[env]` table to every cargo subprocess (test
    binaries, build scripts). `force = false` means a real operator
    key still wins when running cargo outside the pipeline.

    The file is per-worktree and never committed — touching it only
    affects pipeline runs in that worktree.
    """
    cfg_dir = wt / ".cargo"
    cfg_dir.mkdir(exist_ok=True)
    (cfg_dir / "config.toml").write_text(
        "[env]\n"
        'ANTHROPIC_API_KEY = { value = "pipeline-dummy", force = false }\n'
    )


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


def current_branch() -> str:
    """Name of the branch currently checked out at the project root."""
    r = subprocess.run(
        ["git", "rev-parse", "--abbrev-ref", "HEAD"],
        capture_output=True,
        text=True,
        cwd=str(utils.PROJECT_ROOT),
    )
    return r.stdout.strip() if r.returncode == 0 else ""


def is_clean() -> bool:
    """True if the project root has no uncommitted or untracked changes."""
    r = subprocess.run(
        ["git", "status", "--porcelain"],
        capture_output=True,
        text=True,
        cwd=str(utils.PROJECT_ROOT),
    )
    return r.returncode == 0 and not r.stdout.strip()


class MergeConflictError(RuntimeError):
    """`git merge` left the repo in a conflict state — caller must intervene."""


def merge_to_master(ticket_id: str) -> str:
    """Merge `fix/<ticket_id>` into the current branch with `--no-ff`.

    Returns the new merge-commit sha on success. Raises
    `MergeConflictError` if the merge produced conflicts; the merge
    is aborted before raising so the working tree is restored to its
    pre-merge state. Any other git failure raises
    `subprocess.CalledProcessError`.
    """
    branch = branch_for(ticket_id)
    r = subprocess.run(
        [
            "git", "merge", "--no-ff", branch,
            "-m", f"Merge {branch}",
        ],
        capture_output=True,
        text=True,
        cwd=str(utils.PROJECT_ROOT),
    )
    if r.returncode != 0:
        # Conflict or other merge failure — abort to restore the tree.
        subprocess.run(
            ["git", "merge", "--abort"],
            capture_output=True,
            cwd=str(utils.PROJECT_ROOT),
        )
        raise MergeConflictError(
            f"merging {branch}: {r.stdout.strip() or r.stderr.strip()}"
        )
    return branch_head("HEAD")


def create_from_sha(ticket_id: str, sha: str) -> Path:
    """Create a fresh worktree for `ticket_id` checked out at `sha`.

    Used by retry to mint a new worktree at the parent ticket's
    `tested_sha` — the new branch starts from the post-test state,
    leaving any failed-fix commits behind. The old worktree is left
    untouched so an operator can still inspect what the previous
    attempt tried.
    """
    wt = dir_for(ticket_id)
    utils.WORKTREES_DIR.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [
            "git", "worktree", "add",
            "-b", branch_for(ticket_id),
            str(wt), sha,
        ],
        check=True,
        capture_output=True,
        cwd=str(utils.PROJECT_ROOT),
    )
    _write_cargo_env_override(wt)
    return wt
