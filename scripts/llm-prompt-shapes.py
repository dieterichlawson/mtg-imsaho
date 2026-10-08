#!/usr/bin/env python3
"""What kinds of prompt a run's LLM seat was asked, from its `--log`.

    scripts/llm-prompt-shapes.py logs/<run>/game.log [...]

For every `PROMPT` record: its byte size, and its shape — a priority offer
whose only rows are Pass / Concede / mana taps (a decision with no real
alternative to passing, which the seat auto-passes without a call), a
priority offer with something to cast or activate, an attack or block
declaration, a target or set choice, the mulligan, a concede confirmation.
Prints one row per log and the fraction of each shape.
"""
import re
import sys

HEADER = re.compile(r"^\d{4}-\d\d-\d\d \d\d:\d\d:\d\d\.\d+\t")


def prompts(path):
    """Each PROMPT record's body, as a list of lines."""
    body, inside = [], False
    for line in open(path, encoding="utf-8", errors="replace"):
        line = line.rstrip("\n")
        if HEADER.match(line):
            if inside:
                yield body
            fields = line.split("\t")
            inside = len(fields) > 4 and fields[4].startswith("PROMPT")
            body = [fields[5]] if inside and len(fields) > 5 else []
            continue
        if inside:
            body.append(line)
    if inside:
        yield body


def shape(lines):
    text = "\n".join(lines)
    if "[MULLIGAN DECISION]" in text or "AFTER MULLIGAN]" in text:
        return "mulligan"
    if "You chose to CONCEDE" in text:
        return "concede-confirm"
    if "Choose attackers:" in text:
        return "attackers"
    if "Your blockers:" in text or "Declare blocks" in text:
        return "blockers"
    if "Available actions:" in text:
        rows = [l for l in lines[lines.index("Available actions:") + 1:] if re.match(r"^\d+(-\d+)?: ", l)]
        labels = [r.split(": ", 1)[1] for r in rows]
        if all(l == "Pass" or l == "Concede" or l.startswith("Tap ") for l in labels):
            return "priority: pass/concede/tap only"
        return "priority: something to do"
    if "select a target" in text or "Options:" in text:
        return "target or set"
    return "other"


def main():
    for path in sys.argv[1:]:
        counts, sizes = {}, []
        for body in prompts(path):
            counts[shape(body)] = counts.get(shape(body), 0) + 1
            sizes.append(len("\n".join(body).encode()))
        n = len(sizes)
        print(f"{path}: {n} prompts, {sum(sizes)} bytes, median {sorted(sizes)[n // 2] if n else 0}")
        for k, v in sorted(counts.items(), key=lambda kv: -kv[1]):
            print(f"  {v:4d}  {100 * v / n:5.1f}%  {k}")


if __name__ == "__main__":
    main()
