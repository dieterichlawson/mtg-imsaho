"""Close a ticket as abandoned."""
from __future__ import annotations

from pipeline import ticket, worktree
from pipeline.metrics import now_iso
from pipeline.state import CloseReason, after_abandon


def cmd_close(args):
    t = ticket.load(args.ticket_id)
    t.status = after_abandon(t.status)
    t.frontmatter["closed_reason"] = CloseReason.ABANDONED.value
    t.frontmatter["closed_at"] = now_iso()
    if args.note:
        t.frontmatter["closed_note"] = args.note
    t.save()

    if worktree.dir_for(t.id).exists():
        worktree.remove(t.id)
        print(f"Removed worktree for {t.id}")
    print(f"Closed {t.id} (reason={CloseReason.ABANDONED.value})")
