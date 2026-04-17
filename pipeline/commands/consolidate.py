"""Consolidate — ingest a dedup staging file into a merged-* parent ticket.

Deterministic counterpart to `cmd_dedup`: reads one JSON proposal, runs
every invariant check, creates the new parent, and marks each absorbed
source as closed/absorbed.

## Merge semantics

Given a proposal to absorb source tickets A, B, … into a new parent C,
C takes the *minimum* of its sources' statuses (bounded at `tested` —
fix work is never carried forward, only test coverage):

    all sources new                    ⇒  new
    ≥ 1 tested, rest new               ⇒  tested (if every proposed
                                                   test inherited an
                                                   implementation;
                                                   else new)
    exactly 1 fixed + any tested/new   ⇒  tested (fix commits dropped)
    ≥ 2 fixed                          ⇒  rejected (combining multiple
                                                     fix commit-sets
                                                     is out of scope)
    any fix_failed                     ⇒  rejected (post-mortem commits
                                                     are the only artifact
                                                     of a failed run)

When C inherits from exactly one source, the source's worktree is
renamed into C's. For ≥ 2 sources carrying a worktree, C gets a fresh
worktree off master and each source's test file is concatenated into
C's single test file in one commit. Fixed sources are first reset to
their `tested_sha` so fix commits never enter C's history.
"""

from __future__ import annotations

import subprocess
from collections import Counter
from dataclasses import dataclass
from pathlib import Path

from pipeline import ticket, worktree
from pipeline.staging import ConsolidationProposal
from pipeline.state import CloseReason, LifecycleEvent, Status, next_status
from pipeline.ticket import Ticket, parse_tests_section
from pipeline.utils import now_iso


class ConsolidateError(ValueError):
    """Raised when a consolidation proposal can't be ingested."""


@dataclass
class ReferenceBuckets:
    """Three classes a per-test `source_ticket` can fall into."""

    open_sources: list[str]  # absorbable; transition to closed/absorbed
    closed_metadata: list[str]  # already-closed; left as coverage label
    non_absorbable: list[str]  # open but not new/tested; hard error


def cmd_consolidate(args):
    """Entry point for `./pipeline/cli.py consolidate`."""
    proposal_path = Path(args.input)
    if not proposal_path.exists():
        raise ConsolidateError(f"input file not found: {proposal_path}")
    proposal = ConsolidationProposal.load(proposal_path)

    sources, also_closes = _split_references(proposal)
    _require_no_overlap(sources, also_closes)
    _require_no_missing(sources, also_closes)

    buckets = _bucket_sources(sources)
    bad_also = _bad_also_closes(also_closes)
    if buckets.non_absorbable or bad_also:
        raise ConsolidateError(
            _not_absorbable_message(buckets.non_absorbable, bad_also)
        )

    all_closed = buckets.open_sources + list(also_closes)
    if buckets.closed_metadata:
        print(
            f"NOTE: {len(buckets.closed_metadata)} source_ticket value(s) "
            f"already closed — kept as metadata, not re-absorbed: "
            f"{buckets.closed_metadata}"
        )

    _require_coverage(proposal, all_closed)
    tested, fixed = _classify_sources(all_closed)
    _require_at_most_one_fixed(fixed)

    new_id = ticket.allocate_id(f"merged-{proposal.slug}")
    if args.dry_run:
        _print_dry_run_plan(new_id, all_closed, tested, fixed)
        return

    new_test_file, tested_sha, inherited = _inherit_from(tested, fixed, new_id)
    _create_parent(
        new_id,
        proposal,
        tested,
        fixed,
        tested_sha,
        new_test_file,
        inherited,
    )
    _absorb_all(all_closed, new_id)
    _print_summary(
        new_id,
        proposal,
        all_closed,
        buckets.open_sources,
        also_closes,
        tested + fixed,
        inherited,
    )

    if not args.keep_input:
        _remove_proposal(proposal_path)


# ── Reference classification ────────────────────────────────────────


def _split_references(
    proposal: ConsolidationProposal,
) -> tuple[list[str], list[str]]:
    """Distinct `source_ticket` values (per-test) + `also_closes` list."""
    sources: list[str] = []
    seen: set[str] = set()
    for t in proposal.tests:
        if t.source_ticket and t.source_ticket not in seen:
            sources.append(t.source_ticket)
            seen.add(t.source_ticket)
    return sources, list(proposal.also_closes)


def _require_no_overlap(sources: list[str], also_closes: list[str]) -> None:
    overlap = set(sources) & set(also_closes)
    if overlap:
        raise ConsolidateError(
            f"ticket(s) in both Source ticket and Also closes: {overlap}"
        )
    if len(also_closes) != len(set(also_closes)):
        dupes = {x for x in also_closes if also_closes.count(x) > 1}
        raise ConsolidateError(
            f"ticket appears multiple times in Also closes: {dupes}"
        )


def _require_no_missing(sources: list[str], also_closes: list[str]) -> None:
    missing = [
        tid for tid in sources + also_closes if not ticket.exists_on_disk(tid)
    ]
    if missing:
        raise ConsolidateError(f"referenced ticket(s) not found: {missing}")


def _bucket_sources(sources: list[str]) -> ReferenceBuckets:
    open_sources: list[str] = []
    closed_metadata: list[str] = []
    non_absorbable: list[str] = []
    for tid in sources:
        t = ticket.load(tid)
        if t.status.is_absorbable:
            open_sources.append(tid)
        elif t.status.is_open:
            non_absorbable.append(f"  {tid}: status={t.status.value}")
        else:
            closed_metadata.append(tid)
    return ReferenceBuckets(open_sources, closed_metadata, non_absorbable)


def _bad_also_closes(also_closes: list[str]) -> list[str]:
    """`also_closes` entries MUST be absorbable — they're explicit requests."""
    out = []
    for tid in also_closes:
        t = ticket.load(tid)
        if not t.status.is_absorbable:
            out.append(f"  {tid}: status={t.status.value}")
    return out


def _not_absorbable_message(bad_src: list[str], bad_also: list[str]) -> str:
    lines = [
        "consolidation names ticket(s) whose status isn't absorbable.",
        "`new`, `tested`, and `fixed` tickets may be absorbed; `fix_failed`",
        "tickets may not — their post-mortem commits are the only artifact",
        "of a failed run and silently losing them on absorption would hide",
        "real information. `retry --to tested` first if you really want to",
        "cluster one.",
    ]
    return "\n".join(lines + bad_src + bad_also)


# ── Coverage invariant ──────────────────────────────────────────────


def _require_coverage(
    proposal: ConsolidationProposal, all_closed: list[str]
) -> None:
    """For every closed ticket, the parent must have ≥ as many tests.

    attributable to each of its own Source tickets as the closed ticket did.

    Test slugs may be renamed freely; what matters is the per-Source count.
    """
    parent_counts: Counter = Counter()
    for t in proposal.tests:
        if t.source_ticket:
            parent_counts[t.source_ticket] += 1

    gaps: list[str] = []
    for tid in all_closed:
        child = ticket.load(tid)
        needs = _required_source_counts(child)
        for source, n in needs.items():
            if parent_counts.get(source, 0) < n:
                gaps.append(
                    f"  {tid}: needs ≥ {n} test(s) with Source ticket "
                    f"'{source}', found {parent_counts.get(source, 0)}"
                )
    if gaps:
        raise ConsolidateError(
            "new parent is missing tests that exist on closed tickets:\n"
            + "\n".join(gaps)
        )


def _required_source_counts(child: Ticket) -> Counter:
    """Per-Source-ticket counts that a child's tests contribute. Entries.

    without a source (or pseudo-sources like `(new)`) are attributed to
    the child itself.
    """
    counts: Counter = Counter()
    for entry in parse_tests_section(child.body):
        sid = (entry.source_ticket or "").strip()
        if not sid or sid.lower() in ("(new)", "none", "null"):
            sid = child.id
        counts[sid] += 1
    return counts


# ── Source classification + merge semantics ─────────────────────────


def _classify_sources(all_closed: list[str]) -> tuple[list[str], list[str]]:
    """Partition absorbable sources into (tested, fixed) lists by status.

    `new`-status sources carry no worktree so they contribute nothing
    to build; they're absorbed for bookkeeping only and omitted here.
    """
    tested: list[str] = []
    fixed: list[str] = []
    for tid in all_closed:
        st = ticket.load(tid).status
        if st is Status.TESTED:
            tested.append(tid)
        elif st is Status.FIXED:
            fixed.append(tid)
    return tested, fixed


def _require_at_most_one_fixed(fixed: list[str]) -> None:
    if len(fixed) >= 2:
        raise ConsolidateError(
            "more than one absorbed source is `fixed`:\n"
            + "\n".join(f"  {tid}" for tid in fixed)
            + "\nCombining multiple fix commit-sets is out of scope. "
            "`retry --to tested` on all but one first (drops its fix)."
        )


# ── Inheritance ─────────────────────────────────────────────────────


def _inherit_from(
    tested: list[str], fixed: list[str], new_id: str
) -> tuple[str | None, str | None, dict[str, str]]:
    """Build the new parent's worktree + test file from the sources.

    Returns (new_test_file, sha, {slug: "test_file::test_name"}). Three
    dispatch cases:

    - no tested/fixed sources → no worktree (parent starts `new`).
    - one source (tested or fixed) → rename its worktree into the parent.
      Fixed sources are first reset to `tested_sha` so fix commits don't
      travel forward.
    - multiple sources → build a fresh parent worktree, concatenate each
      source's test file into one, and tear down the source worktrees.
    """
    all_srcs = tested + fixed
    if not all_srcs:
        return None, None, {}

    if len(all_srcs) == 1:
        return _inherit_by_rename(all_srcs[0], new_id)
    return _inherit_by_merge(tested, fixed, new_id)


def _inherit_by_rename(
    src_id: str, new_id: str
) -> tuple[str, str, dict[str, str]]:
    """Rename a single source worktree into the new parent's id + branch.

    If the source is `fixed`, its branch is first reset to `tested_sha`
    so fix commits are dropped.
    """
    src = ticket.load(src_id)
    if src.status is Status.FIXED:
        if not src.tested_sha:
            raise ConsolidateError(
                f"{src_id}: fixed source has no tested_sha — cannot drop "
                f"fix commits"
            )
        worktree.reset_to(src_id, src.tested_sha)
    new_test_file = _rename_test_file_on_disk(src_id, new_id, src.test_file)
    sha = worktree.rename(src_id, new_id)
    inherited = {
        e.slug: f"{new_test_file}::{e.implementation.split('::', 1)[1]}"
        for e in parse_tests_section(src.body)
        if "::" in e.implementation
    }
    return new_test_file, sha, inherited


def _inherit_by_merge(
    tested: list[str], fixed: list[str], new_id: str
) -> tuple[str, str, dict[str, str]]:
    """Concatenate every source's test file into a fresh parent worktree.

    Fixed sources are reset to `tested_sha` so fix-era edits to the test
    file aren't read. Source worktrees are removed once C's branch
    carries the merged content.
    """
    for tid in fixed:
        src = ticket.load(tid)
        if not src.tested_sha:
            raise ConsolidateError(
                f"{tid}: fixed source has no tested_sha — cannot drop "
                f"fix commits"
            )
        worktree.reset_to(tid, src.tested_sha)

    wt = worktree.ensure(new_id)
    new_test_file = (
        f"mtg-engine/tests/pipeline_bugs_{new_id.replace('-', '_')}.rs"
    )
    merged = _concat_test_files(tested + fixed)
    target = wt / new_test_file
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(merged)
    _git_commit(wt, f"Inherit tests from: {', '.join(tested + fixed)}")
    sha = worktree.branch_head(worktree.branch_for(new_id))

    for tid in tested + fixed:
        worktree.remove(tid)

    inherited = _collect_inherited_impls(tested + fixed, new_test_file)
    return new_test_file, sha, inherited


def _concat_test_files(src_ids: list[str]) -> str:
    """Merge per-source test files: union of preamble lines + all bodies.

    Preamble = leading block of `use` / `mod` / `extern crate` / `#![...]`
    / comment / blank lines. Lines after the first non-preamble line are
    the body. Each body is emitted under a divider comment for traceability.
    """
    preamble: list[str] = []
    seen: set[str] = set()
    bodies: list[str] = []
    for tid in src_ids:
        src = ticket.load(tid)
        if not src.test_file:
            continue
        path = worktree.dir_for(tid) / src.test_file
        if not path.exists():
            continue
        pre_lines, body = _split_rust_preamble(path.read_text())
        for line in pre_lines:
            if line not in seen:
                preamble.append(line)
                seen.add(line)
        bodies.append(f"// ─── inherited from {tid} ───\n{body}".rstrip())

    parts = ["\n".join(preamble).rstrip()] if preamble else []
    parts.extend(bodies)
    return "\n\n".join(p for p in parts if p).rstrip() + "\n"


_PREAMBLE_PREFIXES = (
    "use ", "mod ", "pub use ", "pub mod ", "extern ", "#![",
)


def _split_rust_preamble(content: str) -> tuple[list[str], str]:
    """Split (preamble_lines, rest) on the first non-preamble line."""
    lines = content.splitlines()
    i = 0
    while i < len(lines):
        s = lines[i].strip()
        if not s or s.startswith("//") or s.startswith(_PREAMBLE_PREFIXES):
            i += 1
            continue
        break
    return lines[:i], "\n".join(lines[i:])


def _collect_inherited_impls(
    src_ids: list[str], new_test_file: str
) -> dict[str, str]:
    """Map proposal slugs → "<new_test_file>::<test_name>" from every source."""
    impls: dict[str, str] = {}
    for tid in src_ids:
        src = ticket.load(tid)
        for e in parse_tests_section(src.body):
            if "::" in e.implementation:
                test_name = e.implementation.split("::", 1)[1]
                impls[e.slug] = f"{new_test_file}::{test_name}"
    return impls


def _git_commit(wt: Path, msg: str) -> None:
    subprocess.run(
        ["git", "add", "-A"],
        check=True,
        cwd=str(wt),
        capture_output=True,
        text=True,
    )
    subprocess.run(
        ["git", "commit", "-m", msg],
        check=True,
        cwd=str(wt),
        capture_output=True,
        text=True,
    )


def _rename_test_file_on_disk(
    old_id: str, new_id: str, old_test_file: str
) -> str:
    """Rename the source's test file inside its worktree + commit.

    Called *before* `worktree.rename()` so we can still reach the file
    via the old worktree path. Returns the new relative path regardless
    of whether a rename was actually necessary.
    """
    old_wt = worktree.dir_for(old_id)
    new_rel = old_test_file.replace(
        f"pipeline_bugs_{old_id.replace('-', '_')}",
        f"pipeline_bugs_{new_id.replace('-', '_')}",
    )
    if new_rel == old_test_file:
        return new_rel
    if not (old_wt / old_test_file).exists():
        return new_rel
    subprocess.run(
        ["git", "mv", old_test_file, new_rel],
        check=True,
        cwd=str(old_wt),
        capture_output=True,
        text=True,
    )
    subprocess.run(
        [
            "git",
            "commit",
            "-m",
            f"Rename test file for consolidation into {new_id}",
        ],
        check=True,
        cwd=str(old_wt),
        capture_output=True,
        text=True,
    )
    return new_rel


# ── Parent creation + source absorption ─────────────────────────────


def _create_parent(
    new_id: str,
    proposal: ConsolidationProposal,
    tested: list[str],
    fixed: list[str],
    tested_sha: str | None,
    new_test_file: str | None,
    inherited: dict[str, str],
) -> None:
    extra = {"created": now_iso(), "kind": "consolidated"}
    inherits = tested + fixed
    has_worktree = bool(inherits)
    all_tests_have_impls = has_worktree and all(
        t.slug in inherited for t in proposal.tests
    )

    if all_tests_have_impls:
        status = Status.TESTED
        extra.update(
            {
                "tested_at": now_iso(),
                "test_file": new_test_file or "",
                "worktree": str(worktree.dir_for(new_id)),
                "inherited_from": ", ".join(inherits),
            }
        )
        if tested_sha:
            extra["tested_sha"] = tested_sha
    else:
        status = Status.NEW
        if has_worktree:
            extra.update(
                {
                    "inherited_from": ", ".join(inherits),
                    "test_file": new_test_file or "",
                    "worktree": str(worktree.dir_for(new_id)),
                }
            )

    body = _render_body(proposal, inherited)
    ticket.new(new_id, status=status, card="multiple", body=body, extra=extra)


def _absorb_all(all_closed: list[str], parent_id: str) -> None:
    for tid in all_closed:
        t = ticket.load(tid)
        t.status = next_status(t.status, LifecycleEvent.ABSORBED)
        t.frontmatter.closed_reason = CloseReason.ABSORBED.value
        t.frontmatter.absorbed_into = parent_id
        t.frontmatter.closed_at = now_iso()
        # The tested source's worktree was renamed; drop its stale pointer.
        t.frontmatter.worktree = ""
        t.save()


def _render_body(proposal: ConsolidationProposal, impls: dict[str, str]) -> str:
    lines = [f"# {proposal.title}", "", "## Description", proposal.description]
    if proposal.engine_path:
        lines += ["", "## Engine path"] + [
            f"- {p}" for p in proposal.engine_path
        ]
    lines += ["", "## Tests", ""]
    for t in proposal.tests:
        lines += [
            f"### {t.slug}",
            f"Source ticket: {t.source_ticket}",
            f"Implementation: {impls.get(t.slug) or '(not yet written)'}",
            f"Scenario: {t.scenario}",
            "",
        ]
    if proposal.also_closes:
        lines += (
            ["## Also closes", ""]
            + [f"- {tid}" for tid in proposal.also_closes]
            + [""]
        )
    return "\n".join(lines).rstrip() + "\n"


# ── Reporting ───────────────────────────────────────────────────────


def _print_dry_run_plan(
    new_id: str,
    all_closed: list[str],
    tested: list[str],
    fixed: list[str],
) -> None:
    print(f"\nWould create: {new_id}")
    print(f"Would close {len(all_closed)} ticket(s) as closed-duplicate:")
    for tid in all_closed:
        print(f"  {tid} → {new_id}")
    inherits = tested + fixed
    if inherits:
        suffix = " (fix commits dropped)" if fixed else ""
        print(f"Would inherit worktree + test file from: "
              f"{', '.join(inherits)}{suffix}")


def _print_summary(
    new_id: str,
    proposal: ConsolidationProposal,
    all_closed: list[str],
    open_sources: list[str],
    also_closes: list[str],
    inherits: list[str],
    inherited: dict[str, str],
) -> None:
    print(f"Created {new_id} with {len(proposal.tests)} test(s)")
    print(
        f"Marked {len(all_closed)} ticket(s) as status={Status.CLOSED.value} "
        f"(reason={CloseReason.ABSORBED.value}; "
        f"{len(open_sources)} per-test, {len(also_closes)} via Also closes)"
    )
    if inherits:
        all_inherited = len(inherited) == len(proposal.tests)
        note = (
            "ready for fix (fully inherited)"
            if all_inherited
            else f"{len(inherited)} test(s) inherited, "
            f"{len(proposal.tests) - len(inherited)} still need "
            f"implementation"
        )
        print(f"Inherited worktree + test file from "
              f"{', '.join(inherits)}: {note}")


def _remove_proposal(path: Path) -> None:
    try:
        path.unlink()
        print(f"Removed staging file: {path}")
    except OSError as e:
        print(f"WARNING: could not remove staging file {path}: {e}")
