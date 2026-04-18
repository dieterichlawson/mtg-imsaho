"""Per-agent sandbox profiles passed to `claude --settings <json>`.

Each agent role has a JSON template in this directory describing its
filesystem and permission rules. `render(profile, **vars)` loads the
named template, substitutes `${var}` placeholders, and returns the
result as a JSON string ready to feed to `claude --settings`.

The sandbox config is enforced at the OS level (Seatbelt on macOS,
bubblewrap on Linux) for any Bash subprocess the agent spawns; the
permissions block covers Claude Code's built-in Edit/Write/Read tools.
Both layers are needed because they cover different escape vectors.

Profiles:
    auditor               — auditor agent (cwd = project root)
    test_writer           — test-writer agent, default lockdown
    test_writer_engine    — test-writer on a needs_engine_work retry
    fixer                 — fixer agent
"""

from __future__ import annotations

import json
from pathlib import Path
from string import Template

_HERE = Path(__file__).resolve().parent

_PROFILES = {
    "auditor",
    "test_writer",
    "test_writer_engine",
    "fixer",
}


def render(profile: str, **vars: str) -> str:
    """Load `<profile>.json`, substitute `${var}` placeholders, return JSON.

    Raises ValueError on unknown profile; KeyError on missing var.
    The result is validated as JSON before return so a typo in the
    template is caught here rather than at `claude` invocation.
    """
    if profile not in _PROFILES:
        raise ValueError(
            f"unknown sandbox profile {profile!r}; "
            f"expected one of {sorted(_PROFILES)}"
        )
    raw = (_HERE / f"{profile}.json").read_text()
    rendered = Template(raw).substitute(vars)
    json.loads(rendered)
    return rendered
