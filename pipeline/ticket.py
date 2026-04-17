"""Ticket — a bug ticket on disk, stored as a markdown file with YAML-ish
frontmatter. This module owns every read/write/move of those files.

Callers elsewhere should go through the `Ticket` class and the
module-level `list_all`, `find`, `new` functions; they should not touch
the TICKETS_DIR / ARCHIVE_DIR paths directly.
"""
from __future__ import annotations

import re
from collections.abc import Iterator
from dataclasses import dataclass, field
from pathlib import Path

from pipeline import paths
from pipeline.state import Status


class TicketError(ValueError):
    """Raised when a ticket file is missing, malformed, or misused."""


@dataclass
class Ticket:
    """A single bug ticket. Mirrors one markdown file on disk."""

    id: str
    status: Status
    body: str
    frontmatter: dict[str, str] = field(default_factory=dict)
    path: Path | None = None  # None for freshly-minted tickets not yet saved

    # ── Construction ────────────────────────────────────────────────

    @classmethod
    def load(cls, ticket_id: str) -> Ticket:
        """Read a ticket from active or archive. Raises if missing or malformed."""
        path = _resolve(ticket_id)
        if path is None:
            raise TicketError(f"ticket not found: {ticket_id}")
        return cls._from_file(path)

    @classmethod
    def _from_file(cls, path: Path) -> Ticket:
        text = path.read_text()
        if not text.startswith("---"):
            raise TicketError(f"{path}: missing frontmatter header")
        try:
            end = text.index("---", 3)
        except ValueError:
            raise TicketError(f"{path}: unterminated frontmatter") from None
        fm: dict[str, str] = {}
        for line in text[3:end].strip().split("\n"):
            if not line.strip():
                continue
            if ":" not in line:
                raise TicketError(f"{path}: malformed frontmatter line: {line!r}")
            k, v = line.split(":", 1)
            fm[k.strip()] = v.strip()
        if "id" not in fm:
            raise TicketError(f"{path}: frontmatter missing 'id'")
        if "status" not in fm:
            raise TicketError(f"{path}: frontmatter missing 'status'")
        try:
            status = Status(fm["status"])
        except ValueError:
            raise TicketError(
                f"{path}: unknown status {fm['status']!r}") from None
        return cls(id=fm["id"], status=status,
                   body=text[end+3:].strip(),
                   frontmatter=fm, path=path)

    # ── Attribute access for common frontmatter fields ──────────────
    #
    # Frontmatter values are always strings on disk. These properties
    # give a typed view + a single name to grep for when a field moves.

    # Dict-style read access for callers/tests that still speak the old
    # `parse_ticket` → {"frontmatter": ..., "body": ...} shape.
    def __getitem__(self, key: str):
        if key == "frontmatter":
            return self.frontmatter
        if key == "body":
            return self.body
        if key == "path":
            return self.path
        raise KeyError(key)

    @property
    def card(self) -> str:
        """Card name (`multiple` for merged-* tickets)."""
        return self.frontmatter.get("card", "")

    @property
    def test_file(self) -> str:
        """Path to the Rust test file written for this ticket, if any."""
        return self.frontmatter.get("test_file", "")

    @property
    def tested_sha(self) -> str:
        """Branch head sha recorded at the end of the test phase."""
        return self.frontmatter.get("tested_sha", "")

    @property
    def allow_engine_edits(self) -> bool:
        """True if the test-writer is permitted to modify engine source."""
        return self.frontmatter.get("allow_engine_edits") == "true"

    # ── Writes ──────────────────────────────────────────────────────

    def save(self) -> None:
        """Write back to disk at the current path. Moves the file between
        active and archive directories if `self.status`'s terminality has
        changed since load.
        """
        if self.path is None:
            raise TicketError(
                f"ticket {self.id!r} was never saved; use Ticket.new(...) "
                f"first")
        # Reflect current status in frontmatter so the written file matches.
        self.frontmatter["status"] = self.status.value
        self.frontmatter["id"] = self.id
        self._write_to(self.path)

        # Move across active/archive if status terminality disagrees with
        # current location.
        should_archive = self.status.is_terminal
        in_archive = self.path.parent == paths.ARCHIVE_DIR
        if should_archive and not in_archive:
            self._move_to(paths.ARCHIVE_DIR)
        elif not should_archive and in_archive:
            self._move_to(paths.TICKETS_DIR)

    def _write_to(self, path: Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        head = ["---", *(f"{k}: {v}" for k, v in self.frontmatter.items()), "---"]
        path.write_text("\n".join(head) + "\n\n" + self.body + "\n")

    def _move_to(self, new_dir: Path) -> None:
        new_dir.mkdir(parents=True, exist_ok=True)
        new_path = new_dir / f"{self.id}.md"
        self.path.rename(new_path)
        self.path = new_path

    def append_section(self, section: str) -> None:
        """Append a `## Something` section to the body and save."""
        self.body = self.body.rstrip() + "\n\n" + section.rstrip() + "\n"
        self.save()


# ── Module-level helpers ────────────────────────────────────────────

def load(ticket_id: str) -> Ticket:
    """Read a ticket from disk. Raises TicketError if missing or malformed."""
    return Ticket.load(ticket_id)


def new(ticket_id: str, *, status: Status, card: str, body: str,
        extra: dict[str, str] | None = None) -> Ticket:
    """Create and persist a new ticket. Errors if the id already exists."""
    if _resolve(ticket_id) is not None:
        raise TicketError(f"ticket already exists: {ticket_id}")
    fm = {"id": ticket_id, "status": status.value, "card": card}
    if extra:
        fm.update(extra)
    ticket = Ticket(id=ticket_id, status=status, body=body.rstrip() + "\n",
                    frontmatter=fm, path=paths.TICKETS_DIR / f"{ticket_id}.md")
    ticket.save()
    return ticket


def exists(ticket_id: str) -> bool:
    """Return True iff the ticket is on disk (active or archive)."""
    return _resolve(ticket_id) is not None


def find(ticket_id: str) -> Ticket | None:
    """Return the ticket, or None if it doesn't exist (contrast with `load`)."""
    path = _resolve(ticket_id)
    return Ticket._from_file(path) if path else None


def list_all(status: Status | None = None, card: str | None = None) -> list[Ticket]:
    """Every ticket on disk (active + archive), optionally filtered."""
    out: list[Ticket] = []
    for p in _all_paths():
        try:
            t = Ticket._from_file(p)
        except TicketError:
            # Skip files that don't parse rather than failing the whole listing.
            continue
        if status is not None and t.status != status:
            continue
        if card and card.lower() not in t.card.lower():
            continue
        out.append(t)
    return out


def allocate_id(stem: str) -> str:
    """Return the next unused `{stem}-NN` ticket id, scanning active + archive."""
    nums = []
    for p in _all_paths():
        m = re.match(rf"{re.escape(stem)}-(\d+)$", p.stem)
        if m:
            nums.append(int(m.group(1)))
    return f"{stem}-{max(nums, default=0) + 1:02d}"


# ── Tests section (ticket body) ─────────────────────────────────────

def parse_tests_section(body: str) -> list[dict]:
    """Pull `### slug` entries out of a ticket body's `## Tests` block."""
    m = re.search(r"##\s+Tests\n(.*?)(?=\n##\s|\Z)", body, re.DOTALL)
    if not m:
        return []
    entries = []
    for block in re.split(r"\n(?=###\s+)", m.group(1)):
        block = block.strip()
        if not block.startswith("###"):
            continue
        head, _, rest = block.partition("\n")
        src = re.search(r"Source ticket:\s*(.+)", rest)
        impl = re.search(r"Implementation:\s*(.+)", rest)
        entries.append({
            "slug": head[3:].strip(),
            "source_ticket": src.group(1).strip() if src else None,
            "implementation": impl.group(1).strip() if impl else ""})
    return entries


def set_test_implementations(ticket: Ticket, impls: dict[str, str]) -> None:
    """Rewrite the `Implementation:` line for each matching slug in-place."""
    current: str | None = None
    out: list[str] = []
    for line in ticket.body.split("\n"):
        if line.startswith("### "):
            current = line[4:].strip()
            out.append(line)
        elif current and current in impls and line.startswith("Implementation:"):
            out.append(f"Implementation: {impls[current]}")
        else:
            out.append(line)
    ticket.body = "\n".join(out).rstrip() + "\n"
    ticket.save()


def reset_test_implementations(ticket: Ticket) -> None:
    """Set every `Implementation:` line back to `(not yet written)`."""
    ticket.body = re.sub(r"(?m)^Implementation:.*$",
                         "Implementation: (not yet written)", ticket.body)
    ticket.save()


# ── Internals ───────────────────────────────────────────────────────

def _resolve(ticket_id: str) -> Path | None:
    active = paths.TICKETS_DIR / f"{ticket_id}.md"
    if active.exists():
        return active
    archived = paths.ARCHIVE_DIR / f"{ticket_id}.md"
    if archived.exists():
        return archived
    return None


def _all_paths() -> Iterator[Path]:
    active = list(paths.TICKETS_DIR.glob("*.md"))
    archive = (list(paths.ARCHIVE_DIR.glob("*.md"))
               if paths.ARCHIVE_DIR.exists() else [])
    return iter(sorted(active + archive, key=lambda p: p.stem))
