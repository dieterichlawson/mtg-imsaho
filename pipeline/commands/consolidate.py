"""Consolidate — ingest a dedup staging file into a merged-* parent ticket.

Deterministic counterpart to `cmd_dedup`: reads one JSON proposal, runs
every invariant check, creates the new parent, and marks each absorbed
source as closed/absorbed.

## Merge semantics

Given a proposal to absorb source tickets A, B, … into a new parent C:

    A status          B status          ⇒  C status
    ──────────────────────────────────────────────
    new               new               ⇒  new
    new               tested            ⇒  new  (test file inheritance
                                                  only supported from a
                                                  single tested source;
                                                  see note below)
    tested            tested            ⇒  rejected (ambiguous — which
                                                      tested source's
                                                      worktree wins?)
    new / tested      fixed / fix_failed ⇒ rejected (downstream state
                                                      would be dropped)

The ASPIRATIONAL rule is simpler: C takes the *minimum* of its
sources' statuses. Today we implement the `new`/`tested` cases and
reject the rest. Extending to `fixed` sources is deliberately out of
scope for this pass — combining multiple fix commits is a separate
engineering problem.

When exactly one source is `tested`, C inherits that worktree + test
file and starts life in whichever status matches its test coverage
(`tested` if every proposed test inherited an implementation, `new`
otherwise).
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
    tested_src = _pick_single_tested(all_closed)

    new_id = ticket.allocate_id(f"merged-{proposal.slug}")
    if args.dry_run:
        _print_dry_run_plan(new_id, all_closed, tested_src)
        return

    new_test_file, tested_sha, inherited = _inherit_from(tested_src, new_id)
    _create_parent(
        new_id,
        proposal,
        all_closed,
        tested_src,
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
        tested_src,
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
        "Only `new` and `tested` tickets may be absorbed; `fixed` and",
        "`fix_failed` carry downstream state that would be silently dropped.",
        "Use `retry --to new` (or --to tested) on those tickets first if",
        "you really want to cluster them.",
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


# ── Tested source inheritance ───────────────────────────────────────


def _pick_single_tested(all_closed: list[str]) -> str | None:
    tested = [
        tid for tid in all_closed if ticket.load(tid).status is Status.TESTED
    ]
    if len(tested) > 1:
        raise ConsolidateError(
            "more than one absorbed source is `tested`:\n"
            + "\n".join(f"  {tid}" for tid in tested)
            + "\nConsolidation can inherit from at most one tested source. "
            "Use `retry --to new` on all but one first."
        )
    return tested[0] if tested else None


def _inherit_from(
    tested_src: str | None, new_id: str
) -> tuple[str | None, str | None, dict[str, str]]:
    """If we're inheriting a tested source's worktree, rename it into the.

    new parent's branch + test file. Returns (new_test_file_rel, sha, impls).
    """
    if tested_src is None:
        return None, None, {}
    src = ticket.load(tested_src)
    old_test_file = src.test_file
    new_test_file = _rename_test_file_on_disk(tested_src, new_id, old_test_file)
    sha = worktree.rename(tested_src, new_id)
    inherited = {
        e.slug: f"{new_test_file}::{e.implementation.split('::', 1)[1]}"
        for e in parse_tests_section(src.body)
        if "::" in e.implementation
    }
    return new_test_file, sha, inherited


def _rename_test_file_on_disk(
    old_id: str, new_id: str, old_test_file: str
) -> str:
    """Move the tested source's worktree + rename its test file so the new.

    parent's branch contains `pipeline_bugs_<new_id>.rs`. Called BEFORE
    the worktree move-rename so we can inspect the file on its old path.
    """
    # The actual worktree move happens in worktree.rename(); before that,
    # the old path is still at the tested source's worktree dir. We rename
    # in _two phases_ to keep operations atomic per directory:
    #   1. Here: rename the file in the OLD worktree and commit.
    #   2. Caller: worktree.rename() moves the whole directory + branch.
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
    all_closed: list[str],
    tested_src: str | None,
    tested_sha: str | None,
    new_test_file: str | None,
    inherited: dict[str, str],
) -> None:
    extra = {"created": now_iso(), "kind": "consolidated"}
    if all_closed:
        extra["source_tickets"] = ", ".join(all_closed)

    all_tests_have_impls = tested_src is not None and all(
        t.slug in inherited for t in proposal.tests
    )

    if all_tests_have_impls:
        status = Status.TESTED
        extra.update(
            {
                "tested_at": now_iso(),
                "test_file": new_test_file or "",
                "worktree": str(worktree.dir_for(new_id)),
                "inherited_from": tested_src or "",
            }
        )
        if tested_sha:
            extra["tested_sha"] = tested_sha
    else:
        status = Status.NEW
        if tested_src:
            extra.update(
                {
                    "inherited_from": tested_src,
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
    new_id: str, all_closed: list[str], tested_src: str | None
) -> None:
    print(f"\nWould create: {new_id}")
    print(f"Would close {len(all_closed)} ticket(s) as closed-duplicate:")
    for tid in all_closed:
        print(f"  {tid} → {new_id}")
    if tested_src:
        print(f"Would inherit worktree + test file from {tested_src}")


def _print_summary(
    new_id: str,
    proposal: ConsolidationProposal,
    all_closed: list[str],
    open_sources: list[str],
    also_closes: list[str],
    tested_src: str | None,
    inherited: dict[str, str],
) -> None:
    print(f"Created {new_id} with {len(proposal.tests)} test(s)")
    print(
        f"Marked {len(all_closed)} ticket(s) as status={Status.CLOSED.value} "
        f"(reason={CloseReason.ABSORBED.value}; "
        f"{len(open_sources)} per-test, {len(also_closes)} via Also closes)"
    )
    if tested_src:
        all_inherited = len(inherited) == len(proposal.tests)
        note = (
            "ready for fix (fully inherited)"
            if all_inherited
            else f"{len(inherited)} test(s) inherited, "
            f"{len(proposal.tests) - len(inherited)} still need "
            f"implementation"
        )
        print(f"Inherited worktree + test file from {tested_src}: {note}")


def _remove_proposal(path: Path) -> None:
    try:
        path.unlink()
        print(f"Removed staging file: {path}")
    except OSError as e:
        print(f"WARNING: could not remove staging file {path}: {e}")
