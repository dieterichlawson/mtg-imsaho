#!/usr/bin/env python3
"""A `claude` that keeps the position every LLM-seat call was asked about.

Point `CLAUDE_CODE_BIN` at this file and run `mtg-runner --p1 cc:<model>
--save <file>`: the runner rewrites the save before every decision, so at
the moment the seat calls the model the save holds that decision's
position — the game, and the seat's memory (notes, how far into the log it
has been shown, a pass-until in progress). This copies it into
`$EVAL_POSITIONS` and then hands the call to `$EVAL_REAL_BIN` (the real
`claude`, or `scripts/llm-prompt-stub.py`) unchanged.

A decision that takes two calls — a cast, then its target — is one
position: copies are named by a hash of the save, so the second call finds
its position already kept. `scripts/decision-eval/replay.py` replays each
kept position as one decision under any seat configuration.

Environment (absolute paths: the seat runs the CLI from a scratch
directory of its own):
  EVAL_SAVE       the --save path the runner is writing
  EVAL_POSITIONS  directory to keep positions in
  EVAL_REAL_BIN   the claude to delegate every call to
"""
import hashlib
import os
import sys

real = os.environ["EVAL_REAL_BIN"]
args = sys.argv[1:]
if args and args[0] != "--version":
    save = os.environ["EVAL_SAVE"]
    out = os.environ["EVAL_POSITIONS"]
    try:
        with open(save, "rb") as f:
            data = f.read()
        digest = hashlib.sha256(data).hexdigest()[:16]
        os.makedirs(out, exist_ok=True)
        if not any(name.endswith(f"-{digest}.save") for name in os.listdir(out)):
            n = len([x for x in os.listdir(out) if x.endswith(".save")])
            path = os.path.join(out, f"{n:04d}-{digest}.save")
            tmp = path + ".tmp"
            with open(tmp, "wb") as f:
                f.write(data)
            os.rename(tmp, path)
    except OSError as e:
        print(f"capture-claude: could not keep the position: {e}", file=sys.stderr)
os.execv(real, [real] + args)
