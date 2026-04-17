"""Ticket — a bug ticket on disk, with frontmatter + a `## Tests` section.

This module owns every read/write/move of those files. Callers elsewhere
go through `Ticket`, `Frontmatter`, and the module-level helpers; they
should not touch TICKETS_DIR / ARCHIVE_DIR directly.
"""

from __future__ import annotations

import re
from collections.abc import Iterator
from dataclasses import dataclass, field, fields
from pathlib import Path
from typing import Any

from pipeline import utils
from pipeline.state import Status


class TicketError(ValueError):
    """Raised when a ticket file is missing, malformed, or misused."""


# ── Frontmatter ─────────────────────────────────────────────────────


@dataclass
class Frontmatter:
    """Parsed YAML-ish frontmatter for a ticket.

    Every field the pipeline itself writes has an explicit slot below so
    readers can discover it from the schema. Anything else (legacy keys,
    fields from manually-authored tickets) round-trips via `extras`.
    """

    # Required
    id: str
    status: Status

    # Identification
    card: str = ""
    card_file: str = ""
    created: str = ""

    # Audit phase
    audit_run_id: str = ""
    audit_model: str = ""
    audit_tokens: str = ""
    audit_duration: str = ""

    # Test phase
    test_run_id: str = ""
    test_model: str = ""
    test_tokens: str = ""
    test_duration: str = ""
    test_file: str = ""
    tests_confirmed: str = ""
    tests_total: str = ""
    tested_at: str = ""
    tested_sha: str = ""
    worktree: str = ""
    allow_engine_edits: bool = False
    engine_block_at: str = ""

    # Fix phase
    fix_run_id: str = ""
    fix_model: str = ""
    fix_tokens: str = ""
    fix_duration: str = ""
    fixed_at: str = ""
    fixed_sha: str = ""
    fix_failed_at: str = ""

    # Ship phase
    shipped_at: str = ""
    shipped_sha: str = ""

    # Terminal: false-positive / closed / absorbed
    false_positive_at: str = ""
    closed_reason: str = ""
    closed_at: str = ""
    closed_note: str = ""

    # Consolidation / dedup
    kind: str = ""
    source_tickets: str = ""
    absorbed_into: str = ""
    inherited_from: str = ""
    duplicate_of: str = ""
    deduped_into: str = ""

    # Agent-emitted / legacy keys that aren't worth a typed slot.
    extras: dict[str, str] = field(default_factory=dict)

    @classmethod
    def parse(
        cls, raw: dict[str, str], *, source: Path | str = ""
    ) -> Frontmatter:
        """Build a Frontmatter from a raw string → string dict.

        Unknown keys go into `extras`. Required fields (`id`, `status`)
        raise TicketError if missing.
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

        known = {f.name for f in fields(cls)} - {"extras"}
        init_kwargs: dict[str, Any] = {}
        extras: dict[str, str] = {}
        for k, v in raw.items():
            if k == "status":
                init_kwargs["status"] = status
            elif k == "allow_engine_edits":
                init_kwargs["allow_engine_edits"] = v == "true"
            elif k in known:
                init_kwargs[k] = v
            else:
                extras[k] = v
        init_kwargs["extras"] = extras
        return cls(**init_kwargs)

    def dump(self) -> dict[str, str]:
        """Serialize to the flat string-keyed dict written to disk.

        Empty strings and `allow_engine_edits=False` are omitted to keep
        frontmatter blocks compact.
        """
        out: dict[str, str] = {}
        for f in fields(self):
            if f.name == "extras":
                continue
            value = getattr(self, f.name)
            if isinstance(value, Status):
                out[f.name] = value.value
            elif isinstance(value, bool):
                if value:
                    out[f.name] = "true"
            elif value:
                out[f.name] = value
        out.update(self.extras)
        return out

    # Convenience setters — callers do `t.frontmatter.set(foo=..., bar=...)`
    # when they have several fields to assign at once.
    def set(self, **kwargs: Any) -> None:
        """Assign multiple fields at once."""
        known = {f.name for f in fields(type(self))} - {"extras"}
        for k, v in kwargs.items():
            if k in known:
                setattr(self, k, v)
            else:
                self.extras[k] = v

    def clear(self, *names: str) -> None:
        """Reset fields to their defaults (omits them from `dump`)."""
        known = {f.name for f in fields(type(self))} - {"extras"}
        defaults = {
            f.name: (False if f.type == "bool" else "")
            for f in fields(type(self))
            if f.name in known and f.name != "status"
        }
        for name in names:
            if name in known:
                setattr(self, name, defaults.get(name, ""))
            else:
                self.extras.pop(name, None)


# ── Tests section ───────────────────────────────────────────────────


@dataclass
class TestEntry:
    """One `### slug` entry from a ticket's `## Tests` section."""

    slug: str
    source_ticket: str | None = None
    implementation: str = ""
    scenario: str = ""


def _parse_tests_section(body: str) -> list[TestEntry]:
    m = re.search(r"##\s+Tests\n(.*?)(?=\n##\s|\Z)", body, re.DOTALL)
    if not m:
        return []
    entries: list[TestEntry] = []
    for block in re.split(r"\n(?=###\s+)", m.group(1)):
        block = block.strip()
        if not block.startswith("###"):
            continue
        head, _, rest = block.partition("\n")
        src = re.search(r"Source ticket:\s*(.+)", rest)
        impl = re.search(r"Implementation:\s*(.+)", rest)
        scen = re.search(r"Scenario:\s*(.+)", rest)
        entries.append(
            TestEntry(
                slug=head[3:].strip(),
                source_ticket=(src.group(1).strip() if src else None),
                implementation=(impl.group(1).strip() if impl else ""),
                scenario=(scen.group(1).strip() if scen else ""),
            )
        )
    return entries


# ── Ticket ──────────────────────────────────────────────────────────


@dataclass(kw_only=True)
class Ticket:
    """A single bug ticket. Mirrors one markdown file on disk."""

    id: str
    status: Status
    body: str
    frontmatter: Frontmatter
    tests: list[TestEntry] = field(default_factory=list)
    path: Path | None = None  # None until saved

    @classmethod
    def load(cls, ticket_id: str) -> Ticket:
        """Read a ticket from active or archive. Raises if missing/malformed."""
        path = get_ticket_path_if_exists(ticket_id)
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
            id=fm.id,
            status=fm.status,
            body=body,
            frontmatter=fm,
            tests=_parse_tests_section(body),
            path=path,
        )

    # ── Shortcuts for the frontmatter fields most commonly read ───
    @property
    def card(self) -> str:
        """Card name this ticket tracks (`multiple` for merged-* parents)."""
        return self.frontmatter.card

    @property
    def test_file(self) -> str:
        """Relative path to the Rust test file the test-writer produced."""
        return self.frontmatter.test_file

    @property
    def tested_sha(self) -> str:
        """Branch head sha when tests were confirmed — retry anchors here."""
        return self.frontmatter.tested_sha

    @property
    def allow_engine_edits(self) -> bool:
        """True while test-writer may modify `mtg-engine/src/**`."""
        return self.frontmatter.allow_engine_edits

    # ── Writes ────────────────────────────────────────────────────
    def save(self) -> None:
        """Write back to disk at the current path.

        Moves the file between active and archive directories if the
        status's terminality has changed since the last save.
        """
        if self.path is None:
            raise TicketError(
                f"ticket {self.id!r} has no path; call ticket.new(...) "
                f"to create, not Ticket() directly"
            )
        self.frontmatter.id = self.id
        self.frontmatter.status = self.status
        self._write_to(self.path)
        in_archive = self.path.parent == utils.ARCHIVE_DIR
        if self.status.is_terminal and not in_archive:
            self._move_to(utils.ARCHIVE_DIR)
        elif not self.status.is_terminal and in_archive:
            self._move_to(utils.TICKETS_DIR)

    def _write_to(self, path: Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        head = ["---"]
        head.extend(f"{k}: {v}" for k, v in self.frontmatter.dump().items())
        head.append("---")
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
    """Load a ticket from disk. Raises if missing or malformed."""
    return Ticket.load(ticket_id)


def new(
    ticket_id: str,
    *,
    status: Status,
    card: str,
    body: str,
    extra: dict[str, str] | None = None,
) -> Ticket:
    """Create and persist a new ticket. Errors if the id already exists."""
    if get_ticket_path_if_exists(ticket_id) is not None:
        raise TicketError(f"ticket already exists: {ticket_id}")
    raw = {
        "id": ticket_id,
        "status": status.value,
        "card": card,
        **(extra or {}),
    }
    fm = Frontmatter.parse(raw, source=ticket_id)
    ticket = Ticket(
        id=ticket_id,
        status=status,
        body=body.rstrip() + "\n",
        frontmatter=fm,
        tests=_parse_tests_section(body),
        path=utils.TICKETS_DIR / f"{ticket_id}.md",
    )
    ticket.save()
    return ticket


def exists_on_disk(ticket_id: str) -> bool:
    """Return True iff the ticket file is present (active or archive)."""
    return get_ticket_path_if_exists(ticket_id) is not None


def get_ticket_if_exists(ticket_id: str) -> Ticket | None:
    """Load the ticket, or return None if it doesn't exist.

    Contrast with `load`, which raises on missing tickets.
    """
    path = get_ticket_path_if_exists(ticket_id)
    return Ticket._from_file(path) if path else None


def get_ticket_path_if_exists(ticket_id: str) -> Path | None:
    """Return the ticket's on-disk path (active or archive), or None."""
    active = utils.TICKETS_DIR / f"{ticket_id}.md"
    if active.exists():
        return active
    archived = utils.ARCHIVE_DIR / f"{ticket_id}.md"
    return archived if archived.exists() else None


def list_all(
    status: Status | None = None, card: str | None = None
) -> list[Ticket]:
    """Every ticket on disk (active + archive), optionally filtered."""
    out: list[Ticket] = []
    for p in _all_paths():
        try:
            t = Ticket._from_file(p)
        except TicketError:
            continue  # skip malformed files rather than failing the listing
        if status is not None and t.status != status:
            continue
        if card and card.lower() not in t.card.lower():
            continue
        out.append(t)
    return out


def allocate_id(stem: str) -> str:
    """Return the next unused `{stem}-NN` ticket id.

    Scans active + archive so a previously-shipped id isn't reused.
    """
    nums = []
    for p in _all_paths():
        m = re.match(rf"{re.escape(stem)}-(\d+)$", p.stem)
        if m:
            nums.append(int(m.group(1)))
    return f"{stem}-{max(nums, default=0) + 1:02d}"


# ── Tests-section mutation ──────────────────────────────────────────


def set_test_implementations(ticket: Ticket, impls: dict[str, str]) -> None:
    """Rewrite the `Implementation:` line for each matching slug in-place."""
    current: str | None = None
    out: list[str] = []
    for line in ticket.body.split("\n"):
        if line.startswith("### "):
            current = line[4:].strip()
            out.append(line)
        elif (
            current and current in impls and line.startswith("Implementation:")
        ):
            out.append(f"Implementation: {impls[current]}")
        else:
            out.append(line)
    ticket.body = "\n".join(out).rstrip() + "\n"
    ticket.tests = _parse_tests_section(ticket.body)
    ticket.save()


def reset_test_implementations(ticket: Ticket) -> None:
    """Set every `Implementation:` line back to `(not yet written)`."""
    ticket.body = re.sub(
        r"(?m)^Implementation:.*$",
        "Implementation: (not yet written)",
        ticket.body,
    )
    ticket.tests = _parse_tests_section(ticket.body)
    ticket.save()


# Back-compat for callers that still reach for the function by its old name.
# Prefer the TestEntry-returning `Ticket.tests` field or the module helper.
def parse_tests_section(body: str) -> list[TestEntry]:
    """Return the parsed `## Tests` entries from a body string."""
    return _parse_tests_section(body)


# ── Internals ───────────────────────────────────────────────────────


def _all_paths() -> Iterator[Path]:

    active = list(utils.TICKETS_DIR.glob("*.md"))
    archive = (
        list(utils.ARCHIVE_DIR.glob("*.md"))
        if utils.ARCHIVE_DIR.exists()
        else []
    )
    return iter(sorted(active + archive, key=lambda p: p.stem))
