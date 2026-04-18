"""Per-agent sandbox profiles for `@anthropic-ai/sandbox-runtime` (`srt`).

Each agent role has a JSON template in this directory describing its
filesystem and network rules in srt's native schema. `render(profile,
**vars)` loads the named template, substitutes `${project_root}`,
`${home}`, and (test_writer profiles only) `${test_file}` placeholders,
and returns the rendered JSON string. The caller writes it to a file
and passes it as `srt --settings <path>`.

The sandbox is enforced at the OS level (Seatbelt on macOS, bubblewrap
on Linux) and applies to every syscall the wrapped process and its
children make — including Claude Code's own Edit/Write tools, which
ultimately become open()/write() syscalls the kernel sees and rejects.

The profiles use srt's allow-only model: `allowWrite` lists every
path the agent legitimately needs to touch. Anything not in that list
is denied. Some paths (engine src for the locked test-writer; the test
file for the fixer) are deliberately absent.

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
