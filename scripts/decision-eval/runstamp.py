"""Which run a replay's results belong to.

`replay.py` and `draft-replay.py` carry on from the results already in
`<out>/<name>.jsonl`, keyed by position and sample only: a stub dry run,
then the real run under the same --out and --name, had the stub's answers
taken as the real seat's (#899's review, as night.sh did with its save).
The run is stamped beside its results, and one stamped for another run is
refused rather than carried on.
"""
import json
import os
import sys


def check(out, name, run, script):
    """Stamp `<out>/<name>.run.json` with `run`, or exit if its results
    are another run's. Results from before the stamp are carried on."""
    path = os.path.join(out, f"{name}.run.json")
    results = os.path.join(out, f"{name}.jsonl")
    if os.path.exists(path) and os.path.exists(results):
        with open(path) as f:
            was = json.load(f)
        if was != run:
            changed = ", ".join(sorted(k for k in set(was) | set(run) if was.get(k) != run.get(k)))
            sys.exit(f"{script}: {results} is another run's ({changed} differ); "
                     f"use another --name, or move it away to start afresh")
    with open(path, "w") as f:
        json.dump(run, f, indent=1, sort_keys=True)
