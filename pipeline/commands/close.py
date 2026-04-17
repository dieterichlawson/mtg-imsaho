"""Close a ticket as abandoned."""

from __future__ import annotations

from pipeline import ticket, worktree
from pipeline.state import CloseReason, LifecycleEvent, next_status
from pipeline.utils import now_iso


def cmd_close(args):
    """Entry point for `./pipeline/cli.py close`."""
    t = ticket.load(args.ticket_id)
    t.status = next_status(t.status, LifecycleEvent.ABANDONED)
    t.frontmatter.closed_reason = CloseReason.ABANDONED.value
    t.frontmatter.closed_at = now_iso()
    if args.note:
        t.frontmatter.closed_note = args.note
    t.save()

    if worktree.dir_for(t.id).exists():
        worktree.remove(t.id)
        print(f"Removed worktree for {t.id}")
    print(f"Closed {t.id} (reason={CloseReason.ABANDONED.value})")
