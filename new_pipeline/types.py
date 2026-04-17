"""The ticket on disk and the audit-report JSON it's minted from.

One file so the ticket's shape (`Ticket`, `Frontmatter`), its state
machine (`Status`, `LifecycleEvent`, `next_status`), and the staging
data types that feed it (`AuditReport`, `Finding`) stay next to each
other. Callers never build a ticket file or parse JSON by hand — they
go through the methods on these classes.
"""

from __future__ import annotations

import json
import re
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
    TESTED = "tested"
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


class StagingError(ValueError):
    """Raised when an agent-produced JSON staging file fails validation."""


# ISO-8601 with trailing `Z`, e.g. `2026-04-17T12:34:56Z`.
_ISO_FMT = "%Y-%m-%dT%H:%M:%SZ"


def _parse_datetime(raw: str) -> datetime:
    """Parse an ISO-8601 `...Z` timestamp into a timezone-aware datetime."""
    return datetime.strptime(raw, _ISO_FMT).replace(tzinfo=timezone.utc)


def _format_datetime(dt: datetime) -> str:
    """Render a datetime as our canonical ISO-8601 `...Z` string."""
    return dt.astimezone(timezone.utc).strftime(_ISO_FMT)


# ─── Frontmatter ───────────────────────────────────────────────────

# Numeric frontmatter fields — parsed str → int on load, str on dump.
_INT_FIELDS = frozenset({
    "audit_tokens", "audit_duration",
    "test_tokens", "test_duration",
})


@dataclass
class Frontmatter:
    """Parsed YAML-ish frontmatter for a ticket."""

    # Required
    id: str
    status: Status

    # Identification
    card: str = ""
    created: datetime | None = None

    # Audit phase
    audit_run_id: str = ""
    audit_model: str = ""
    audit_tokens: int = 0
    audit_duration: int = 0

    # Test phase
    test_run_id: str = ""
    test_model: str = ""
    test_tokens: int = 0
    test_duration: int = 0
    test_file: str = ""
    tested_sha: str = ""
    tested_at: datetime | None = None

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

        datetime_fields = {"created", "closed_at", "tested_at"}
        known = {f.name for f in fields(cls)} - {"extras"}
        init_kwargs: dict[str, Any] = {}
        extras: dict[str, str] = {}
        for k, v in raw.items():
            if k == "status":
                init_kwargs["status"] = status
            elif k in datetime_fields:
                init_kwargs[k] = _parse_datetime(v) if v else None
            elif k in _INT_FIELDS:
                init_kwargs[k] = int(v) if v else 0
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
            if value is None or value == "" or value == 0:
                continue
            if isinstance(value, Status):
                out[f.name] = value.value
            elif isinstance(value, datetime):
                out[f.name] = _format_datetime(value)
            elif isinstance(value, int):
                out[f.name] = str(value)
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

    @classmethod
    def allocate_id(cls, stem: str) -> str:
        """Return the next unused `{stem}-NN` ticket id.

        Scans active + archive so a previously-shipped id isn't reused.
        """
        nums: list[int] = []
        for p in _all_ticket_paths():
            m = re.match(rf"{re.escape(stem)}-(\d+)$", p.stem)
            if m:
                nums.append(int(m.group(1)))
        return f"{stem}-{max(nums, default=0) + 1:02d}"

    # ── Writes ───────────────────────────────────────────────────

    @classmethod
    def create(
        cls,
        ticket_id: str,
        *,
        status: Status,
        card: str,
        body: str,
        extra: dict[str, str] | None = None,
    ) -> Ticket:
        """Mint + persist a new ticket. Raises TicketError if id collides."""
        if cls._path_for(ticket_id) is not None:
            raise TicketError(f"ticket already exists: {ticket_id}")
        raw = {
            "id": ticket_id,
            "status": status.value,
            "card": card,
            **(extra or {}),
        }
        fm = Frontmatter.parse(raw, source=ticket_id)
        t = cls(
            id=ticket_id,
            status=status,
            body=body.rstrip() + "\n",
            frontmatter=fm,
            path=utils.TICKETS_DIR / f"{ticket_id}.md",
        )
        t.save()
        return t

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


# ─── Audit staging ─────────────────────────────────────────────────


def _load_json(path: Path) -> dict:
    try:
        return json.loads(path.read_text())
    except json.JSONDecodeError as e:
        raise StagingError(f"{path.name}: invalid JSON — {e}") from None


def _require(d: dict, key: str, kind: type) -> Any:
    if key not in d:
        raise StagingError(f"missing required field: {key!r}")
    v = d[key]
    if not isinstance(v, kind):
        raise StagingError(
            f"field {key!r}: expected {kind.__name__}, got {type(v).__name__}"
        )
    return v


def _require_objects(d: dict, key: str) -> list[dict]:
    arr = _require(d, key, list)
    for i, x in enumerate(arr):
        if not isinstance(x, dict):
            raise StagingError(
                f"{key}[{i}]: expected object, got {type(x).__name__}"
            )
    return arr


def _optional_str(d: dict, key: str) -> str:
    """`d[key]` as a string; empty-string if absent; raise if wrong type."""
    v = d.get(key)
    if v is None:
        return ""
    if not isinstance(v, str):
        raise StagingError(
            f"field {key!r}: expected str, got {type(v).__name__}"
        )
    return v


@dataclass
class FindingTest:
    """A single test slug + scenario the auditor wants written."""

    slug: str
    scenario: str

    @classmethod
    def from_dict(cls, d: dict) -> FindingTest:
        """Parse one `tests[*]` entry from a finding's JSON."""
        return cls(
            slug=_require(d, "slug", str),
            scenario=_require(d, "scenario", str),
        )


@dataclass
class Finding:
    """One bug identified by the auditor."""

    oracle_quote: str
    code_quote: str
    description: str
    engine_path: str = ""
    check: str = ""
    affected_cards: list[str] = field(default_factory=list)
    tests: list[FindingTest] = field(default_factory=list)

    @classmethod
    def from_dict(cls, d: dict) -> Finding:
        """Parse one `findings[*]` entry from an audit-report JSON."""
        return cls(
            oracle_quote=_require(d, "oracle_quote", str),
            code_quote=_require(d, "code_quote", str),
            description=_require(d, "description", str),
            engine_path=_optional_str(d, "engine_path"),
            check=_optional_str(d, "check"),
            affected_cards=list(d.get("affected_cards") or []),
            tests=[
                FindingTest.from_dict(t) for t in (d.get("tests") or [])
            ],
        )

    def to_ticket_body(self, card_snake: str, ticket_num: int) -> str:
        """Render this finding as the body of the ticket that tracks it.

        `card_snake` + `ticket_num` name the fallback test slug used when
        the auditor didn't propose any test names of its own.
        """
        parts = [
            "## Audit Finding",
            "",
            f"**Oracle text:**\n> {self.oracle_quote}",
            "",
            f"**Code:**\n> {self.code_quote}",
            "",
            f"**Description:**\n{self.description}",
            "",
        ]
        if self.engine_path:
            parts += [f"**Engine path:** {self.engine_path}", ""]
        if self.check:
            parts += [f"**Required check:** {self.check}", ""]
        if self.affected_cards:
            parts += (
                ["**Affected cards:**"]
                + [f"- {c}" for c in self.affected_cards]
                + [""]
            )
        parts += ["## Tests", ""]
        if self.tests:
            tests = [(t.slug, t.scenario) for t in self.tests]
        else:
            default_scenario = (
                self.description.split(".")[0][:240] or "See description above."
            )
            tests = [
                (f"test_{card_snake}_{ticket_num:02d}", default_scenario),
            ]
        for slug, scenario in tests:
            parts += [
                f"### {slug}",
                f"Scenario: {scenario}",
                "",
            ]
        return "\n".join(parts).rstrip() + "\n"


@dataclass
class AuditReport:
    """One auditor run's findings for a single card."""

    card: str
    findings: list[Finding]

    @classmethod
    def load(cls, path: Path) -> AuditReport:
        """Parse and validate an auditor staging file."""
        d = _load_json(path)
        return cls(
            card=_require(d, "card", str),
            findings=[
                Finding.from_dict(f)
                for f in _require_objects(d, "findings")
            ],
        )

    def mint_tickets(
        self, *, extra: dict[str, str] | None = None,
    ) -> list[Ticket]:
        """Mint one ticket per finding. Returns the newly-created tickets.

        `extra` is applied to every minted ticket's frontmatter — useful
        for stamping audit-run metadata (run id, model, tokens, duration)
        onto every ticket produced by the same run.
        """
        snake = utils.card_to_snake(self.card)
        out: list[Ticket] = []
        for finding in self.findings:
            new_id = Ticket.allocate_id(snake)
            num = int(new_id.rsplit("-", 1)[1])
            t = Ticket.create(
                new_id,
                status=Status.NEW,
                card=self.card,
                body=finding.to_ticket_body(snake, num),
                extra=extra,
            )
            out.append(t)
        return out


# ─── Test-writer staging ───────────────────────────────────────────

class TestStatus(str, Enum):
    """Per-test verdict the test-writer reports for one scenario slug.

    - `CONFIRMED`: the agent wrote a test that compiles and fails,
      proving the bug is real.
    - `REJECTED`: the scenario passes against current code (not a bug).
    """

    CONFIRMED = "confirmed"
    REJECTED = "rejected"

    @classmethod
    def parse(cls, raw: str, *, context: str = "") -> TestStatus:
        """Parse a case-insensitive string into a TestStatus.

        Raises StagingError (prefixed with `context`) if the value isn't
        one of the enum members.
        """
        try:
            return cls(raw.lower())
        except ValueError:
            valid = [s.value for s in cls]
            prefix = f"{context}: " if context else ""
            raise StagingError(
                f"{prefix}status must be one of {valid}, got {raw!r}"
            ) from None


@dataclass
class TestResult:
    """The agent's verdict on one test scenario from a ticket.

    The `slug` here is the per-scenario slug that appears inside a
    ticket's `## Tests` section — e.g. `test_lifelink_first_strike` —
    not the ticket id. A single ticket can have multiple tests, and
    the agent returns one `TestResult` per slug so the pipeline can
    match each verdict back to its originating entry.
    """

    slug: str
    status: TestStatus
    test_name: str = ""
    assertion_message: str = ""
    explanation: str = ""

    @classmethod
    def from_dict(cls, i: int, d: dict) -> TestResult:
        """Parse one `tests[i]` entry. `i` is used for error messages."""
        slug = _require(d, "slug", str)
        status = TestStatus.parse(
            _require(d, "status", str),
            context=f"tests[{i}] {slug!r}",
        )
        return cls(
            slug=slug,
            status=status,
            test_name=_optional_str(d, "test_name") or slug,
            assertion_message=_optional_str(d, "assertion_message"),
            explanation=_optional_str(d, "explanation"),
        )


@dataclass
class TestReport:
    """The test-writer agent's JSON staging file."""

    test_file: str
    tests: list[TestResult]

    @classmethod
    def load(cls, path: Path) -> TestReport:
        """Parse and validate a test-writer staging file."""
        d = _load_json(path)
        return cls(
            test_file=_require(d, "test_file", str),
            tests=[
                TestResult.from_dict(i, t)
                for i, t in enumerate(_require_objects(d, "tests"))
            ],
        )
