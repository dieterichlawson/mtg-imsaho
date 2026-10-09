#!/usr/bin/env python3
"""A stand-in `claude` for measuring what an LLM seat is sent, for free.

Point `CLAUDE_CODE_BIN` at this file and run `mtg-runner --p1 cc ...` (or
the draft runner): every call answers something legal, drawn from the
JSON schema it was handed, and appends one line to `$STUB_CALLS` with the
byte size of the system prompt, the stdin prompt and the schema, and
whether the call opened a session (`--session-id`) or resumed one
(`--resume`). `scripts/measure-llm-prompts.sh` runs it over several seeds
and sums the lines up.

The answer is a hash of the prompt, so a seed replays, and it leans
towards doing something: an "up to N" index array is filled rather than
left empty, which is the shape of question a stub that always answers the
minimum never exercises (CLAUDE.md, "one decision, four surfaces").

A priority menu's `Pass until something happens` row is picked with
probability `$STUB_PASS_UNTIL` (default 0.5) rather than as one index
among many: a stub that picked it one time in eight would not move the
call count the row exists to move. When it is not taken, the stub's own
pick avoids it, so the probability is exact and `STUB_PASS_UNTIL=0`
never picks it (#754: the schema fill used to land on it as well).

`cancel` (backing out of a cast at an X-funding, target-set or cost
prompt, #749) is always false: a stub that cancelled half its casts would
measure a game nobody plays.
"""
import hashlib
import json
import os
import re
import sys
import uuid

argv = sys.argv[1:]
if argv and argv[0] == "--version":
    print("1.0.0 (prompt stub)")
    sys.exit(0)


def flag(name):
    for i, a in enumerate(argv):
        if a == name and i + 1 < len(argv):
            return argv[i + 1]
    return None


message = sys.stdin.read()
schema_text = flag("--json-schema") or "{}"
schema = json.loads(schema_text)
props = schema.get("properties") or {}
system = flag("--system-prompt") or ""
opened = flag("--session-id")
resumed = flag("--resume")
sid = opened or resumed or str(uuid.uuid4())


def h(*parts):
    return int(hashlib.sha256("\x00".join(map(str, parts)).encode()).hexdigest(), 16)


def fill(name, spec):
    t = spec.get("type")
    if "enum" in spec:
        v = spec["enum"]
        return v[h(message, name) % len(v)] if v else None
    if t == "string":
        return "stub reasoning for " + name
    if t in ("integer", "number"):
        lo = spec.get("minimum", 0)
        hi = spec.get("maximum", lo + 3)
        return lo + h(message, name) % (hi - lo + 1)
    if t == "boolean":
        return False if name == "cancel" else bool(h(message, name) % 2)
    if t == "array":
        items = spec.get("items") or {}
        lo = spec.get("minItems", 0)
        hi = spec.get("maxItems", lo)
        if "enum" in items:
            pool = list(items["enum"])
            # Towards the top of the range, so set prompts do something.
            n = min(len(pool), hi if hi else lo)
            n = max(lo, n - h(message, name) % 2) if n > lo else lo
            chosen = sorted(pool, key=lambda v: h(message, name, v))[:n]
            return chosen
        return [fill(name + "[]", items) for _ in range(lo)]
    if t == "object" or "properties" in spec:
        return {k: fill(name + "." + k, v) for k, v in (spec.get("properties") or {}).items()}
    return "stub"


PASS_UNTIL = re.compile(r"^(\d+): Pass until something happens", re.M)


def pass_until_row():
    """The pass-until row's index on this menu, if it has one."""
    m = PASS_UNTIL.search(message)
    return int(m.group(1)) if m and "action" in props else None


def takes_pass_until():
    p = float(os.environ.get("STUB_PASS_UNTIL", "0.5"))
    return h(message, "pass-until") % 1000 < p * 1000


if "maindeck" in props and "lands" in props:
    md, total = {}, 0
    for n, s in (props["maindeck"].get("properties") or {}).items():
        take = min(max(s.get("enum", [0])), max(0, 23 - total))
        md[n] = take
        total += take
    out = {"thoughts": "stub deck", "maindeck": md,
           "lands": {"Plains": 4, "Island": 4, "Swamp": 3, "Mountain": 3, "Forest": 3}}
else:
    out = {k: fill(k, v) for k, v in props.items()}
    row = pass_until_row()
    if row is not None:
        if takes_pass_until():
            out["action"] = row
        elif out.get("action") == row:
            others = [v for v in props["action"].get("enum", []) if v != row]
            if others:
                out["action"] = others[h(message, "not-pass-until") % len(others)]

record = os.environ.get("STUB_CALLS")
if record:
    with open(record, "a", encoding="utf-8") as f:
        f.write(json.dumps({
            "system_bytes": len(system.encode()),
            "prompt_bytes": len(message.encode()),
            "schema_bytes": len(schema_text.encode()),
            "session": "opened" if opened else ("resumed" if resumed else "none"),
            "sid": sid,
        }) + "\n")

print(json.dumps({
    "type": "result", "subtype": "success", "is_error": False, "session_id": sid,
    "usage": {"input_tokens": 0, "output_tokens": 0,
              "cache_read_input_tokens": 0, "cache_creation_input_tokens": 0},
    "structured_output": out, "result": json.dumps(out),
}))
