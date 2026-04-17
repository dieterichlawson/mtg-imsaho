"""pipeline/models.py — domain dataclasses, parsing, disk I/O.

Every type the pipeline reads or writes lives here, together with the
code that serializes it. Callers elsewhere never parse JSON, tokenize
frontmatter, or format tickets by hand — they go through the methods
on these classes.

Two model families share the file:

- The *ticket* on disk (markdown + frontmatter): `Ticket`, `Frontmatter`,
  `TestEntry`. `Ticket.load` / `.try_load` / `.list_all` / `.create` for
  disk I/O; `.save` for writes; `parse_tests_section` for the `## Tests`
  block inside a body.
- The *staging* JSON each agent produces: `AuditReport`, `TestReport`,
  `FixReport`, `ConsolidationProposal`, plus their nested types. Each
  has a `.load(path)` classmethod.
"""

from __future__ import annotations

import json
import re
from collections.abc import Iterator
from dataclasses import dataclass, field, fields
from pathlib import Path
from typing import Any

from pipeline import utils
from pipeline.state import Status
from pipeline.utils import parse_slug

# ─── Errors ─────────────────────────────────────────────────────────


class TicketError(ValueError):
    """Raised when a ticket file is missing, malformed, or misused."""


class StagingError(ValueError):
    """Raised when an agent's staging JSON is malformed or missing fields."""


# ─── Low-level JSON parse helpers (staging) ────────────────────────


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


# ─── Frontmatter ────────────────────────────────────────────────────

# Numeric frontmatter fields — parsed from string to int on load,
# serialized back to string on dump (and omitted when 0).
_INT_FIELDS = frozenset({
    "audit_tokens", "audit_duration",
    "test_tokens", "test_duration", "tests_confirmed", "tests_total",
    "fix_tokens", "fix_duration",
})


@dataclass
class Frontmatter:
    """Parsed YAML-ish frontmatter for a ticket.

    Every field the pipeline writes has an explicit slot. Anything else
    (legacy keys, manually-authored fields) round-trips via `extras`.
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
    audit_tokens: int = 0
    audit_duration: int = 0

    # Test phase
    test_run_id: str = ""
    test_model: str = ""
    test_tokens: int = 0
    test_duration: int = 0
    test_file: str = ""
    tests_confirmed: int = 0
    tests_total: int = 0
    tested_at: str = ""
    tested_sha: str = ""
    worktree: str = ""
    allow_engine_edits: bool = False
    engine_block_at: str = ""

    # Fix phase
    fix_run_id: str = ""
    fix_model: str = ""
    fix_tokens: int = 0
    fix_duration: int = 0
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

    # Anything else the pipeline didn't model; preserved verbatim.
    extras: dict[str, str] = field(default_factory=dict)

    @classmethod
    def parse(
        cls, raw: dict[str, str], *, source: Path | str = ""
    ) -> Frontmatter:
        """Build a Frontmatter from raw string → string lines.

        Required keys (`id`, `status`) raise TicketError if missing.
        Numeric fields are parsed to int; unknown keys go into `extras`.
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
            elif k in _INT_FIELDS:
                init_kwargs[k] = int(v) if v else 0
            elif k in known:
                init_kwargs[k] = v
            else:
                extras[k] = v
        init_kwargs["extras"] = extras
        return cls(**init_kwargs)

    def dump(self) -> dict[str, str]:
        """Serialize to the flat string-keyed dict written to disk.

        Empty strings, `0`, and `False` are omitted.
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
            elif isinstance(value, int):
                if value:
                    out[f.name] = str(value)
            elif value:
                out[f.name] = value
        out.update(self.extras)
        return out

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
        known = {f.name: f for f in fields(type(self))}
        for name in names:
            if name == "status":
                continue
            if name in known:
                f = known[name]
                if f.type == "bool":
                    setattr(self, name, False)
                elif f.type == "int":
                    setattr(self, name, 0)
                else:
                    setattr(self, name, "")
            else:
                self.extras.pop(name, None)


# ─── TestEntry + body parsing ──────────────────────────────────────


@dataclass
class TestEntry:
    """One `### slug` entry from a ticket's `## Tests` section."""

    slug: str
    source_ticket: str | None = None
    implementation: str = ""
    scenario: str = ""


def parse_tests_section(body: str) -> list[TestEntry]:
    """Return the parsed `## Tests` entries from a body string."""
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


# ─── Ticket ────────────────────────────────────────────────────────


@dataclass(kw_only=True)
class Ticket:
    """A single bug ticket. Mirrors one markdown file on disk."""

    id: str
    status: Status
    body: str
    frontmatter: Frontmatter
    tests: list[TestEntry] = field(default_factory=list)
    path: Path | None = None  # None until saved

    # ── Reads ────────────────────────────────────────────────────

    @classmethod
    def load(cls, ticket_id: str) -> Ticket:
        """Read a ticket from active or archive. Raises if missing."""
        path = cls._path_for(ticket_id)
        if path is None:
            raise TicketError(f"ticket not found: {ticket_id}")
        return cls._from_file(path)

    @classmethod
    def try_load(cls, ticket_id: str) -> Ticket | None:
        """Load if present, else return None."""
        path = cls._path_for(ticket_id)
        return cls._from_file(path) if path else None

    @classmethod
    def exists(cls, ticket_id: str) -> bool:
        """True iff the ticket file is present (active or archive)."""
        return cls._path_for(ticket_id) is not None

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
                continue  # skip malformed files rather than aborting
            if status is not None and t.status != status:
                continue
            if card and card.lower() not in t.card.lower():
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
        """Mint + persist a new ticket. Raises if the id already exists."""
        if cls.exists(ticket_id):
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
            tests=parse_tests_section(body),
            path=utils.TICKETS_DIR / f"{ticket_id}.md",
        )
        t.save()
        return t

    def save(self) -> None:
        """Write back to disk at the current path.

        Moves the file between active and archive directories if the
        status's terminality has changed since the last save.
        """
        if self.path is None:
            raise TicketError(
                f"ticket {self.id!r} has no path; call Ticket.create(...)"
            )
        self.frontmatter.id = self.id
        self.frontmatter.status = self.status
        self._write()
        in_archive = self.path.parent == utils.ARCHIVE_DIR
        if self.status.is_terminal and not in_archive:
            self._move_to(utils.ARCHIVE_DIR)
        elif not self.status.is_terminal and in_archive:
            self._move_to(utils.TICKETS_DIR)

    def append_section(self, section: str) -> None:
        """Append a `## Something` section to the body and save."""
        self.body = self.body.rstrip() + "\n\n" + section.rstrip() + "\n"
        self.save()

    def set_test_implementations(self, impls: dict[str, str]) -> None:
        """Rewrite each matching slug's `Implementation:` line in-place."""
        current: str | None = None
        out: list[str] = []
        for line in self.body.split("\n"):
            if line.startswith("### "):
                current = line[4:].strip()
                out.append(line)
            elif (
                current and current in impls
                and line.startswith("Implementation:")
            ):
                out.append(f"Implementation: {impls[current]}")
            else:
                out.append(line)
        self.body = "\n".join(out).rstrip() + "\n"
        self.tests = parse_tests_section(self.body)
        self.save()

    def reset_test_implementations(self) -> None:
        """Set every `Implementation:` line back to `(not yet written)`."""
        self.body = re.sub(
            r"(?m)^Implementation:.*$",
            "Implementation: (not yet written)",
            self.body,
        )
        self.tests = parse_tests_section(self.body)
        self.save()

    # ── Shortcuts for the frontmatter fields most commonly read ──

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
            id=fm.id,
            status=fm.status,
            body=body,
            frontmatter=fm,
            tests=parse_tests_section(body),
            path=path,
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
        if utils.ARCHIVE_DIR.exists()
        else []
    )
    return iter(sorted(active + archive, key=lambda p: p.stem))


# ─── Staging: per-test statuses ────────────────────────────────────


TEST_CONFIRMED = "confirmed"
TEST_REJECTED = "rejected"
TEST_BLOCKED = "blocked"
VALID_TEST_STATUSES = frozenset({TEST_CONFIRMED, TEST_REJECTED, TEST_BLOCKED})


# ─── Auditor staging ───────────────────────────────────────────────


@dataclass
class FindingTest:
    """A single test slug + scenario the auditor wants written."""

    slug: str
    scenario: str


@dataclass
class Finding:
    """One bug identified by the auditor."""

    oracle_quote: str
    code_quote: str
    description: str
    engine_path: list[str] = field(default_factory=list)
    check: str = ""
    affected_cards: list[str] = field(default_factory=list)
    tests: list[FindingTest] = field(default_factory=list)


@dataclass
class Insight:
    """A generalizable pattern the auditor discovered."""

    title: str
    description: str


@dataclass
class AuditReport:
    """The full staging output from one auditor run."""

    card: str
    card_data_status: str
    checks: dict[str, str]
    findings: list[Finding]
    insights: list[Insight]
    untested_rulings: list[str]

    @property
    def is_pass(self) -> bool:
        """True when the auditor found no bugs."""
        return not self.findings

    @classmethod
    def load(cls, path: Path) -> AuditReport:
        """Parse and validate an auditor staging file."""
        d = _load_json(path)
        findings = [_parse_finding(f) for f in _require_objects(d, "findings")]
        insights = [
            Insight(
                title=_require(i, "title", str),
                description=_require(i, "description", str),
            )
            for i in (d.get("insights") or [])
        ]
        return cls(
            card=str(d.get("card") or ""),
            card_data_status=str(d.get("card_data_status") or ""),
            checks=dict(d.get("checks_performed") or {}),
            findings=findings,
            insights=insights,
            untested_rulings=list(d.get("untested_rulings") or []),
        )


def _parse_finding(f: dict) -> Finding:
    tests = [
        FindingTest(
            slug=_require(t, "slug", str),
            scenario=_require(t, "scenario", str),
        )
        for t in (f.get("tests") or [])
    ]
    return Finding(
        oracle_quote=_require(f, "oracle_quote", str),
        code_quote=_require(f, "code_quote", str),
        description=_require(f, "description", str),
        engine_path=list(f.get("engine_path") or []),
        check=str(f.get("check") or ""),
        affected_cards=list(f.get("affected_cards") or []),
        tests=tests,
    )


# ─── Test-writer staging ───────────────────────────────────────────


@dataclass
class TestResult:
    """What the test-writer reported for one slug in `## Tests`."""

    slug: str
    status: str  # one of VALID_TEST_STATUSES
    test_name: str = ""
    assertion_message: str = ""
    explanation: str = ""
    blocked_by: str | None = None


@dataclass
class TestReport:
    """Staging output from one test-writer run."""

    test_file: str
    tests: list[TestResult]

    @classmethod
    def load(cls, path: Path) -> TestReport:
        """Parse and validate a test-writer staging file."""
        d = _load_json(path)
        tests: list[TestResult] = []
        for i, t in enumerate(_require_objects(d, "tests")):
            slug = _require(t, "slug", str)
            status = _require(t, "status", str).lower()
            if status not in VALID_TEST_STATUSES:
                raise StagingError(
                    f"tests[{i}] {slug!r}: status must be one of "
                    f"{sorted(VALID_TEST_STATUSES)}, got {status!r}"
                )
            tests.append(
                TestResult(
                    slug=slug,
                    status=status,
                    test_name=str(t.get("test_name") or slug),
                    assertion_message=str(t.get("assertion_message") or ""),
                    explanation=str(t.get("explanation") or ""),
                    blocked_by=t.get("blocked_by") or None,
                )
            )
        return cls(test_file=str(d.get("test_file") or ""), tests=tests)


# ─── Fixer staging ─────────────────────────────────────────────────


@dataclass
class FixReport:
    """Staging output from one fixer run."""

    status: str  # "fixed" | "failed"
    files_changed: list[str]
    description: str

    @classmethod
    def load(cls, path: Path) -> FixReport:
        """Parse and validate a fixer staging file."""
        d = _load_json(path)
        status = _require(d, "status", str).lower()
        if status not in ("fixed", "failed"):
            raise StagingError(
                f"status must be 'fixed' or 'failed', got {status!r}"
            )
        return cls(
            status=status,
            files_changed=list(d.get("files_changed") or []),
            description=str(d.get("description") or ""),
        )


# ─── Dedup consolidation proposal ─────────────────────────────────


@dataclass
class ProposedTest:
    """A test the dedup agent proposes for a consolidated parent.

    `source_ticket` is required — every test must come from exactly
    one of the tickets being merged. Agents may not invent new tests.
    """

    slug: str
    source_ticket: str
    scenario: str


@dataclass
class ConsolidationProposal:
    """The dedup agent's proposal to merge N tickets into one parent."""

    slug: str
    title: str
    description: str
    engine_path: list[str]
    tests: list[ProposedTest]
    also_closes: list[str]

    @classmethod
    def load(cls, path: Path) -> ConsolidationProposal:
        """Parse and validate a consolidation staging file."""
        d = _load_json(path)
        slug = parse_slug(_require(d, "slug", str), kind="kebab")
        tests = [
            _parse_proposed_test(i, t)
            for i, t in enumerate(_require_objects(d, "tests"))
        ]
        if not tests:
            raise StagingError("tests array must contain at least one entry")
        return cls(
            slug=slug,
            title=_require(d, "title", str),
            description=_require(d, "description", str),
            engine_path=list(d.get("engine_path") or []),
            tests=tests,
            also_closes=list(d.get("also_closes") or []),
        )


def _parse_proposed_test(i: int, t: dict) -> ProposedTest:
    slug = parse_slug(_require(t, "slug", str), kind="snake")
    src = t.get("source_ticket")
    if src in (None, "", "(new)"):
        raise StagingError(
            f"tests[{i}] {slug!r}: dedup proposals must list a "
            f"`source_ticket` for every test — agents cannot invent "
            f"new tests, only combine existing ones from the merged set"
        )
    return ProposedTest(
        slug=slug,
        source_ticket=str(src),
        scenario=_require(t, "scenario", str),
    )
