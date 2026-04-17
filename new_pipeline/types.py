"""The ticket on disk — its lifecycle, its frontmatter, and the markdown body.

One file so the ticket's shape (`Ticket`, `Frontmatter`) and its state
machine (`Status`, `LifecycleEvent`, `next_status`) stay next to each
other. Callers never construct a ticket file by hand — they go through
`Ticket.load` to read and `Ticket.save` / `Ticket.close` to write.
"""

from __future__ import annotations

from collections.abc import Iterator
from dataclasses import dataclass, field, fields
from datetime import datetime, timezone
from enum import Enum
from pathlib import Path
from typing import Any

from new_pipeline import utils

# ─── Lifecycle ─────────────────────────────────────────────────────


class Status(str, Enum):
    """A ticket's lifecycle state.

    Mixes with `str` so comparisons against raw frontmatter values work.
    """

    NEW = "new"
    CLOSED = "closed"

    @property
    def is_terminal(self) -> bool:
        """True if no further transitions are allowed."""
        return self is Status.CLOSED


class CloseReason(str, Enum):
    """Why a ticket ended up in `closed`."""

    ABANDONED = "abandoned"  # human gave up


class LifecycleEvent(str, Enum):
    """Non-stage events that change a ticket's status."""

    ABANDONED = "abandoned"


def next_status(current: Status, event: LifecycleEvent) -> Status:
    """Return the status a ticket should land in after `event`.

    Raises ValueError if `current` can't accept the event (e.g. trying
    to abandon a ticket that's already terminal).
    """
    if event is LifecycleEvent.ABANDONED:
        if current.is_terminal:
            raise ValueError(
                f"cannot abandon a ticket already terminal ({current.value!r})"
            )
        return Status.CLOSED
    raise ValueError(f"unhandled lifecycle event: {event!r}")


# ─── Errors + low-level parse helpers ──────────────────────────────


class TicketError(ValueError):
    """Raised when a ticket file is missing, malformed, or misused."""


# ISO-8601 with trailing `Z`, e.g. `2026-04-17T12:34:56Z`.
_ISO_FMT = "%Y-%m-%dT%H:%M:%SZ"


def _parse_datetime(raw: str) -> datetime:
    """Parse an ISO-8601 `...Z` timestamp into a timezone-aware datetime."""
    return datetime.strptime(raw, _ISO_FMT).replace(tzinfo=timezone.utc)


def _format_datetime(dt: datetime) -> str:
    """Render a datetime as our canonical ISO-8601 `...Z` string."""
    return dt.astimezone(timezone.utc).strftime(_ISO_FMT)


# ─── Frontmatter ───────────────────────────────────────────────────


@dataclass
class Frontmatter:
    """Parsed YAML-ish frontmatter for a ticket."""

    # Required
    id: str
    status: Status

    # Identification
    card: str = ""
    created: datetime | None = None

    # Terminal: closed / abandoned
    closed_reason: str = ""
    closed_at: datetime | None = None
    closed_note: str = ""

    # Keys not modelled as typed slots above — preserved verbatim so
    # hand-authored fields aren't silently dropped on save.
    extras: dict[str, str] = field(default_factory=dict)

    @classmethod
    def parse(
        cls, raw: dict[str, str], *, source: Path | str = ""
    ) -> Frontmatter:
        """Build a Frontmatter from a raw string → string dict.

        Required keys (`id`, `status`) raise TicketError if missing.
        Unknown keys go into `extras`.
        """
        if "id" not in raw:
            raise TicketError(f"{source}: frontmatter missing 'id'")
        if "status" not in raw:
            raise TicketError(f"{source}: frontmatter missing 'status'")
        try:
            status = Status(raw["status"])
        except ValueError:
            raise TicketError(
                f"{source}: unknown status {raw['status']!r}"
            ) from None

        datetime_fields = {"created", "closed_at"}
        known = {f.name for f in fields(cls)} - {"extras"}
        init_kwargs: dict[str, Any] = {}
        extras: dict[str, str] = {}
        for k, v in raw.items():
            if k == "status":
                init_kwargs["status"] = status
            elif k in datetime_fields:
                init_kwargs[k] = _parse_datetime(v) if v else None
            elif k in known:
                init_kwargs[k] = v
            else:
                extras[k] = v
        init_kwargs["extras"] = extras
        return cls(**init_kwargs)

    def dump(self) -> dict[str, str]:
        """Serialize to the flat dict written to disk. Empty values omitted."""
        out: dict[str, str] = {}
        for f in fields(self):
            if f.name == "extras":
                continue
            value = getattr(self, f.name)
            if value is None or value == "":
                continue
            if isinstance(value, Status):
                out[f.name] = value.value
            elif isinstance(value, datetime):
                out[f.name] = _format_datetime(value)
            else:
                out[f.name] = value
        out.update(self.extras)
        return out


# ─── Ticket ────────────────────────────────────────────────────────


@dataclass(kw_only=True)
class Ticket:
    """A single bug ticket. Mirrors one markdown file on disk."""

    id: str
    status: Status
    body: str
    frontmatter: Frontmatter
    path: Path | None = None  # None until saved

    # ── Reads ────────────────────────────────────────────────────

    @classmethod
    def load(cls, ticket_id: str) -> Ticket:
        """Read a ticket from active or archive.

        Raises TicketError if the ticket file isn't found.
        """
        path = cls._path_for(ticket_id)
        if path is None:
            raise TicketError(f"ticket not found: {ticket_id}")
        return cls._from_file(path)

    @classmethod
    def list_all(
        cls, *, status: Status | None = None, card: str | None = None
    ) -> list[Ticket]:
        """Every ticket on disk (active + archive), optionally filtered."""
        out: list[Ticket] = []
        for p in _all_ticket_paths():
            try:
                t = cls._from_file(p)
            except TicketError:
                continue  # skip malformed rather than aborting the list
            if status is not None and t.status != status:
                continue
            if card and card.lower() not in t.frontmatter.card.lower():
                continue
            out.append(t)
        return out

    # ── Writes ───────────────────────────────────────────────────

    def save(self) -> None:
        """Write back to disk at the current path.

        Moves the file between active and archive if terminality changed.
        """
        assert self.path is not None, "Ticket has no path (loaded via .load?)"
        self.frontmatter.id = self.id
        self.frontmatter.status = self.status
        self._write()
        in_archive = self.path.parent == utils.ARCHIVE_DIR
        if self.status.is_terminal and not in_archive:
            self._move_to(utils.ARCHIVE_DIR)
        elif not self.status.is_terminal and in_archive:
            self._move_to(utils.TICKETS_DIR)

    def close(self, *, note: str | None = None) -> None:
        """Close the ticket as abandoned and save to disk."""
        self.status = next_status(self.status, LifecycleEvent.ABANDONED)
        self.frontmatter.closed_reason = CloseReason.ABANDONED.value
        self.frontmatter.closed_at = datetime.now(timezone.utc)
        if note:
            self.frontmatter.closed_note = note
        self.save()

    # ── Internals ────────────────────────────────────────────────

    @classmethod
    def _path_for(cls, ticket_id: str) -> Path | None:
        active = utils.TICKETS_DIR / f"{ticket_id}.md"
        if active.exists():
            return active
        archived = utils.ARCHIVE_DIR / f"{ticket_id}.md"
        return archived if archived.exists() else None

    @classmethod
    def _from_file(cls, path: Path) -> Ticket:
        text = path.read_text()
        if not text.startswith("---"):
            raise TicketError(f"{path}: missing frontmatter header")
        try:
            end = text.index("---", 3)
        except ValueError:
            raise TicketError(f"{path}: unterminated frontmatter") from None
        raw: dict[str, str] = {}
        for line in text[3:end].strip().split("\n"):
            if not line.strip():
                continue
            if ":" not in line:
                raise TicketError(
                    f"{path}: malformed frontmatter line: {line!r}"
                )
            k, v = line.split(":", 1)
            raw[k.strip()] = v.strip()
        fm = Frontmatter.parse(raw, source=path)
        body = text[end + 3 :].strip()
        return cls(
            id=fm.id, status=fm.status, body=body, frontmatter=fm, path=path,
        )

    def _write(self) -> None:
        assert self.path is not None
        self.path.parent.mkdir(parents=True, exist_ok=True)
        head = ["---"]
        head.extend(f"{k}: {v}" for k, v in self.frontmatter.dump().items())
        head.append("---")
        self.path.write_text("\n".join(head) + "\n\n" + self.body + "\n")

    def _move_to(self, new_dir: Path) -> None:
        assert self.path is not None
        new_dir.mkdir(parents=True, exist_ok=True)
        new_path = new_dir / f"{self.id}.md"
        self.path.rename(new_path)
        self.path = new_path


def _all_ticket_paths() -> Iterator[Path]:
    active = list(utils.TICKETS_DIR.glob("*.md"))
    archive = (
        list(utils.ARCHIVE_DIR.glob("*.md"))
        if utils.ARCHIVE_DIR.exists() else []
    )
    return iter(sorted(active + archive, key=lambda p: p.stem))
