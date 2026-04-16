"""Consolidate + dedup commands — merged-* parent creation and dedup agent."""
from __future__ import annotations

import argparse
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

from pipeline._agent import MAX_ATTEMPTS
from pipeline._staging import StagingError, load_consolidation


def make_commands(*, cli_module) -> tuple:
    """Build cmd_consolidate and cmd_dedup bound to `cli_module`'s state.

    All constants, paths, and I/O helpers are looked up on cli_module at call
    time so the journey tests' patches to cli.run_agent / cli.PROJECT_ROOT /
    cli.parse_ticket etc. take effect."""

    def reuse_worktree_for_new_parent(
        old_tid: str, new_tid: str, old_test_file: str,
    ) -> tuple[str, str]:
        """Rename the tested source's worktree + branch to the new parent
        and rename its test file. Returns (new_test_rel, head_sha)."""
        old_wt = cli_module.get_worktree_dir(old_tid)
        new_wt = cli_module.get_worktree_dir(new_tid)
        subprocess.run(["git", "worktree", "move", str(old_wt), str(new_wt)],
                       check=True, cwd=str(cli_module.PROJECT_ROOT),
                       capture_output=True, text=True)
        subprocess.run(["git", "branch", "-m",
                        cli_module.get_worktree_branch(old_tid),
                        cli_module.get_worktree_branch(new_tid)],
                       check=True, cwd=str(new_wt),
                       capture_output=True, text=True)
        new_rel = old_test_file.replace(
            f"pipeline_bugs_{old_tid.replace('-', '_')}",
            f"pipeline_bugs_{new_tid.replace('-', '_')}")
        if new_rel != old_test_file and (new_wt / old_test_file).exists():
            subprocess.run(["git", "mv", old_test_file, new_rel], check=True,
                           cwd=str(new_wt), capture_output=True, text=True)
            subprocess.run(["git", "commit", "-m",
                            f"Rename test file for consolidation into {new_tid}"],
                           check=True, cwd=str(new_wt),
                           capture_output=True, text=True)
        head = subprocess.run(["git", "rev-parse", "HEAD"], check=True,
                              cwd=str(new_wt), capture_output=True,
                              text=True).stdout.strip()
        return new_rel, head

    def render_body(parsed: dict, impls: dict[str, str]) -> str:
        lines = [f"# {parsed['title']}", "", "## Description",
                 parsed["description"]]
        if parsed["engine_path"]:
            lines += ["", "## Engine path"] + [f"- {p}" for p in parsed["engine_path"]]
        lines += ["", "## Tests", ""]
        for t in parsed["tests"]:
            lines += [f"### {t['slug']}",
                      f"Source ticket: {t['source_ticket'] or '(new)'}",
                      f"Implementation: {impls.get(t['slug']) or '(not yet written)'}",
                      f"Scenario: {t['scenario']}", ""]
        if parsed["also_closes"]:
            lines += ["## Also closes", ""] + [f"- {tid}" for tid in parsed["also_closes"]] + [""]
        return "\n".join(lines).rstrip() + "\n"

    def cmd_consolidate(args):
        input_path = Path(args.input)
        if not input_path.exists():
            print(f"ERROR: input file not found: {input_path}"); sys.exit(1)
        parsed = load_consolidation(input_path)

        # also_closes are EXPLICIT absorption requests. Per-test source_ticket
        # values are coverage metadata — absorbed if open, labels only if
        # already closed (supports copy-through from merged parents).
        sources: list[str] = []
        seen = set()
        for t in parsed["tests"]:
            sid = t.get("source_ticket")
            if sid and sid not in seen:
                sources.append(sid); seen.add(sid)
        also = parsed["also_closes"]

        if set(sources) & set(also):
            print(f"ERROR: ticket(s) in both Source ticket and Also closes: "
                  f"{set(sources) & set(also)}"); sys.exit(1)
        if len(also) != len(set(also)):
            dup = {x for x in also if also.count(x) > 1}
            print(f"ERROR: ticket appears multiple times in Also closes: {dup}")
            sys.exit(1)

        missing = [tid for tid in sources + also
                   if not cli_module.ticket_path(tid).exists()]
        if missing:
            print(f"ERROR: referenced ticket(s) not found: {missing}")
            sys.exit(1)

        open_sources: list[str] = []
        closed_meta: list[str] = []
        bad_src: list[str] = []
        for tid in sources:
            s = cli_module.parse_ticket(
                cli_module.ticket_path(tid))["frontmatter"].get("status", "")
            if s in cli_module.ABSORBABLE_STATUSES:
                open_sources.append(tid)
            elif s in cli_module.OPEN_STATUSES:
                bad_src.append(f"  {tid}: status={s}")
            else:
                closed_meta.append(tid)
        bad_also = [f"  {tid}: status={s}" for tid in also
                    if (s := cli_module.parse_ticket(
                        cli_module.ticket_path(tid))["frontmatter"].get("status", ""))
                    not in cli_module.ABSORBABLE_STATUSES]
        if bad_src or bad_also:
            print("ERROR: consolidation names ticket(s) whose status isn't "
                  "absorbable. Only `new` and `tested` tickets may be absorbed; "
                  "`fixed` and `fix_failed` carry downstream state that would be "
                  "silently dropped. Use `retry --to new` (or --to tested) on "
                  "those tickets first if you really want to cluster them.")
            for line in bad_src + bad_also:
                print(line)
            sys.exit(1)

        all_closed = open_sources + list(also)
        if closed_meta:
            print(f"NOTE: {len(closed_meta)} source_ticket value(s) already "
                  f"closed — kept as metadata, not re-absorbed: {closed_meta}")

        # Coverage invariant: parent's per-Source-ticket count ≥ child's.
        parent_counts: Counter = Counter()
        for t in parsed["tests"]:
            if t["source_ticket"]:
                parent_counts[t["source_ticket"]] += 1
        gaps = []
        for tid in all_closed:
            needs: Counter = Counter()
            for ct in cli_module._parse_tests_section(
                    cli_module.parse_ticket(cli_module.ticket_path(tid))["body"]):
                sid = (ct.get("source_ticket") or "").strip()
                if not sid or sid.lower() in ("(new)", "none", "null"):
                    sid = tid
                needs[sid] += 1
            for src, n in needs.items():
                if parent_counts.get(src, 0) < n:
                    gaps.append(f"  {tid}: needs ≥ {n} test(s) with Source "
                                f"ticket '{src}', found "
                                f"{parent_counts.get(src, 0)}")
        if gaps:
            print("ERROR: new parent is missing tests that exist on closed tickets.")
            for g in gaps:
                print(g)
            sys.exit(1)

        tested_sources = [
            tid for tid in all_closed
            if cli_module.parse_ticket(
                cli_module.ticket_path(tid))["frontmatter"].get("status") == cli_module.STATUS_TESTED]
        if len(tested_sources) > 1:
            print("ERROR: more than one absorbed source is `tested`:")
            for tid in tested_sources:
                print(f"  {tid}")
            print("Consolidation can inherit from at most one tested source. "
                  "Use `retry --to new` on all but one first.")
            sys.exit(1)
        tested_src = tested_sources[0] if tested_sources else None

        inherited: dict[str, str] = {}
        if tested_src:
            for entry in cli_module._parse_tests_section(
                    cli_module.parse_ticket(cli_module.ticket_path(tested_src))["body"]):
                impl = entry.get("implementation", "").strip()
                if "::" in impl:
                    inherited[entry["slug"]] = impl

        slug = parsed["slug"]
        nums = [int(m.group(1)) for p in cli_module.all_ticket_paths()
                if (m := re.match(rf"merged-{re.escape(slug)}-(\d+)$", p.stem))]
        new_id = f"merged-{slug}-{max(nums, default=0) + 1:02d}"

        if args.dry_run:
            print(f"\nWould create: {new_id}")
            print(f"Would close {len(all_closed)} ticket(s) as closed-duplicate:")
            for tid in all_closed:
                print(f"  {tid} → {new_id}")
            if tested_src:
                print(f"Would inherit worktree + test file from {tested_src} "
                      f"({len(inherited)} test(s) already implemented)")
            return

        new_test_file: str | None = None
        tested_sha: str | None = None
        if tested_src:
            src_tf = cli_module.parse_ticket(
                cli_module.ticket_path(tested_src))["frontmatter"].get("test_file", "")
            new_test_file, tested_sha = reuse_worktree_for_new_parent(
                tested_src, new_id, src_tf)
            inherited = {s: f"{new_test_file}::{impl.split('::', 1)[1]}"
                         for s, impl in inherited.items()}

        fm = {"id": new_id, "status": cli_module.STATUS_NEW, "card": "multiple",
              "created": cli_module.now_iso(), "kind": "consolidated"}
        if all_closed:
            fm["source_tickets"] = ", ".join(all_closed)
        body = render_body(parsed, inherited)

        all_inherited = (tested_src is not None
                         and all(t["slug"] in inherited for t in parsed["tests"]))
        if all_inherited:
            fm.update({"status": cli_module.STATUS_TESTED,
                       "tested_at": cli_module.now_iso(),
                       "test_file": new_test_file or "",
                       "worktree": str(cli_module.get_worktree_dir(new_id)),
                       "inherited_from": tested_src or ""})
            if tested_sha:
                fm["tested_sha"] = tested_sha
        elif tested_src:
            fm.update({"inherited_from": tested_src,
                       "test_file": new_test_file or "",
                       "worktree": str(cli_module.get_worktree_dir(new_id))})
        cli_module.write_ticket(new_id, fm, body)

        for tid in all_closed:
            cli_module.transition(
                tid, cli_module.STATUS_CLOSED,
                set_={"closed_reason": cli_module.CLOSED_REASON_ABSORBED,
                      "absorbed_into": new_id,
                      "closed_at": cli_module.now_iso()},
                unset=("worktree",))

        print(f"Created {new_id} with {len(parsed['tests'])} test(s)")
        print(f"Marked {len(all_closed)} ticket(s) as status={cli_module.STATUS_CLOSED} "
              f"(reason={cli_module.CLOSED_REASON_ABSORBED}; "
              f"{len(open_sources)} per-test, {len(also)} via Also closes)")
        if tested_src:
            note = ("ready for fix (fully inherited)" if all_inherited else
                    f"{len(inherited)} test(s) inherited, "
                    f"{len(parsed['tests']) - len(inherited)} still need implementation")
            print(f"Inherited worktree + test file from {tested_src}: {note}")

        if not args.keep_input:
            try:
                input_path.unlink()
                print(f"Removed staging file: {input_path}")
            except OSError as e:
                print(f"WARNING: could not remove staging file {input_path}: {e}")

    def cmd_dedup(args):
        ids = [t.strip() for t in args.tickets.split(",") if t.strip()]
        if not ids:
            print("ERROR: dedup requires at least one candidate ticket id")
            sys.exit(1)
        bodies: list[tuple[str, str]] = []
        for tid in ids:
            path = cli_module.ticket_path(tid)
            if not path.exists():
                print(f"ERROR: ticket not found: {tid}"); sys.exit(1)
            t = cli_module.parse_ticket(path)
            if t["frontmatter"].get("status", "new") in {cli_module.STATUS_CLOSED, cli_module.STATUS_FIXED, cli_module.STATUS_SHIPPED}:
                print(f"ERROR: {tid} already closed "
                      f"(status={t['frontmatter'].get('status')})")
                sys.exit(1)
            bodies.append((tid, path.read_text()))

        tickets_section = "\n\n---\n\n".join(
            f"## Candidate ticket: {tid}\n\n{body}" for tid, body in bodies)
        builder = cli_module.build_prompt(
            "dedup", num_tickets=len(ids), tickets_section=tickets_section)
        if args.dry_run:
            print(f"[dry-run] Would spawn dedup agent for {len(ids)} candidate "
                  f"ticket(s) (model={args.model})"); return

        def stagings() -> list[Path]:
            return sorted(cli_module.STAGING_DIR.glob("consolidation-*.json"))
        preexisting = {f.name for f in stagings()}

        retry_note = ""
        last_error = None
        for attempt in range(1, MAX_ATTEMPTS + 1):
            print(f"\nSpawning dedup agent (attempt {attempt}/{MAX_ATTEMPTS})...")
            prompt = builder(retry_note, attempt)
            log_path = cli_module.LOGS_DIR / f"{cli_module.today()}-dedup-attempt{attempt}.log"
            result = cli_module.run_agent(prompt, args.model, args.effort,
                                          log_path=log_path,
                                          progress_prefix="  [dedup] ")
            if result.get("is_error"):
                last_error = result.get("error_message") or "unknown agent error"
                print(f"  Agent error: {last_error} ({result['duration']}s)")
                retry_note = (f"\n\n## Retry note (attempt {attempt} failed)\n"
                              f"Previous attempt errored: {last_error}\n")
                continue
            new_files = [f for f in stagings() if f.name not in preexisting]
            if not new_files:
                print(f"  Agent produced no consolidation files — nothing to merge "
                      f"({result['duration']}s, {result['tokens']} tok)")
                return
            errs = []
            for f in new_files:
                try:
                    load_consolidation(f)
                except StagingError as e:
                    errs.append((f, str(e)))
            if errs:
                last_error = "; ".join(f"{f.name}: {m}" for f, m in errs)
                print(f"  Parse errors in {len(errs)} file(s)")
                for f, m in errs:
                    print(f"    {f.name}: {m}")
                detail = "\n".join(f"- {f.name}: {m}" for f, m in errs)
                retry_note = (f"\n\n## Retry note (attempt {attempt} failed)\n"
                              f"Previous attempt produced {len(errs)} "
                              f"unparseable staging file(s):\n{detail}\n"
                              f"Edit them in place to fix.\n")
                continue
            print(f"  Agent produced {len(new_files)} valid staging file(s) "
                  f"({result['duration']}s, {result['tokens']} tok)")
            for f in new_files:
                print(f"\n── consolidating {f.name} ──")
                cmd_consolidate(argparse.Namespace(
                    input=str(f), keep_input=args.keep_input, dry_run=False))
            return
        print(f"\nERROR: dedup failed after {MAX_ATTEMPTS} attempt(s). "
              f"Last error: {last_error}")
        sys.exit(1)

    return cmd_consolidate, cmd_dedup
