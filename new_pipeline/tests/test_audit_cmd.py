"""End-to-end tests for `cmd_audit`.

A fake `claude` executable is dropped into a temp bin dir and prepended
to `PATH`, so the command spawns a real subprocess. The fake reads its
instructions from env vars (`FAKE_AGENT_PAYLOAD_FILE`, `FAKE_AGENT_FAIL`,
etc.), writes the requested JSON to the staging path it finds in the
prompt, and emits a canned stream-json `result` event. This exercises
the real prompt build, real `run_agent` subprocess call, real
stream-json parse, and real ingest — only the agent's reasoning is
replaced.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path
from unittest.mock import patch

_THIS = Path(__file__).resolve()
sys.path.insert(0, str(_THIS.parents[2]))

from new_pipeline import oracle, utils  # noqa: E402
from new_pipeline.commands import audit as audit_mod  # noqa: E402
from new_pipeline.types import Status, Ticket  # noqa: E402

_FAKE_CLAUDE = textwrap.dedent(
    '''\
    #!/usr/bin/env python3
    """Scripted stand-in for the `claude` CLI used by tests.

    Parses `-p <prompt>`, finds the staging path in the prompt, copies
    the env-supplied payload there, and emits a stream-json result event.
    """
    import json
    import os
    import re
    import sys

    argv = sys.argv[1:]
    prompt = argv[argv.index("-p") + 1]

    # Optional: emit an agent-reported error, skipping any write.
    if os.environ.get("FAKE_AGENT_FAIL"):
        sys.stdout.write(json.dumps({
            "type": "result",
            "usage": {"input_tokens": 1, "output_tokens": 1},
            "num_turns": 1,
            "is_error": True,
            "result": os.environ.get("FAKE_AGENT_FAIL"),
        }) + "\\n")
        sys.exit(0)

    # Optional: emit a bad-JSON payload for the first N attempts, then
    # the good one. Counter lives in a file so it survives across
    # subprocess invocations within one test.
    counter_path = os.environ.get("FAKE_AGENT_ATTEMPT_COUNTER")
    attempt = 1
    if counter_path:
        try:
            attempt = int(open(counter_path).read().strip()) + 1
        except (OSError, ValueError):
            attempt = 1
        with open(counter_path, "w") as f:
            f.write(str(attempt))

    fail_attempts = int(os.environ.get("FAKE_AGENT_FAIL_FIRST_N", "0"))

    payload_file = os.environ.get("FAKE_AGENT_PAYLOAD_FILE")
    m = re.search(r"(/\\S+?\\.json)\\b", prompt)
    if payload_file and m:
        staging_path = m.group(1)
        if attempt <= fail_attempts:
            # Corrupt JSON so the loader rejects it.
            open(staging_path, "w").write("{ not json")
        else:
            open(staging_path, "w").write(open(payload_file).read())

    sys.stdout.write(json.dumps({
        "type": "result",
        "usage": {"input_tokens": 100, "output_tokens": 50},
        "num_turns": 2,
    }) + "\\n")
    '''
)


def _args(cards: str, model: str = "opus", effort: str = "max"):
    return argparse.Namespace(cards=cards, model=model, effort=effort)


class AuditCommandTest(unittest.TestCase):
    """cmd_audit exercised against a real subprocess + scripted claude."""

    def setUp(self) -> None:
        """Point utils at a temp workspace, install the fake claude on PATH."""
        self.tmp = Path(tempfile.mkdtemp(prefix="new-pipeline-audit-"))
        tickets = self.tmp / "tickets"
        staging = self.tmp / "staging"
        bindir = self.tmp / "bin"
        tickets.mkdir()
        bindir.mkdir()
        fake_claude = bindir / "claude"
        fake_claude.write_text(_FAKE_CLAUDE)
        fake_claude.chmod(0o755)

        new_path = f"{bindir}:{os.environ.get('PATH', '')}"
        self._patches = [
            patch.object(utils, "TICKETS_DIR", tickets),
            patch.object(utils, "ARCHIVE_DIR", tickets / "archive"),
            patch.object(utils, "STAGING_DIR", staging),
            patch.object(
                oracle,
                "get_oracle_text",
                lambda card: f"[oracle for {card}]",
            ),
            patch.dict(os.environ, {"PATH": new_path}, clear=False),
        ]
        for p in self._patches:
            p.start()
        self.addCleanup(self._teardown)

    def _teardown(self) -> None:
        for p in reversed(self._patches):
            p.stop()
        # Clear env vars the individual tests set for the fake agent.
        for k in (
            "FAKE_AGENT_PAYLOAD_FILE",
            "FAKE_AGENT_FAIL",
            "FAKE_AGENT_ATTEMPT_COUNTER",
            "FAKE_AGENT_FAIL_FIRST_N",
        ):
            os.environ.pop(k, None)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def _set_payload(self, payload: dict) -> None:
        """Write `payload` to disk and point the fake agent at it."""
        p = self.tmp / "payload.json"
        p.write_text(json.dumps(payload))
        os.environ["FAKE_AGENT_PAYLOAD_FILE"] = str(p)

    # ── happy path ─────────────────────────────────────────────────

    def test_successful_audit_mints_ticket_with_metadata(self) -> None:
        """One card, one finding → ticket minted with audit_* frontmatter."""
        self._set_payload({
            "card": "Olivia Voldaren",
            "findings": [
                {
                    "oracle_quote": "whenever olivia deals damage",
                    "code_quote": "if source.has_ability(...)",
                    "description": "Lifelink missing on first-strike.",
                },
            ],
        })
        audit_mod.cmd_audit(_args("Olivia Voldaren"))

        t = Ticket.load("olivia_voldaren-01")
        self.assertIs(t.status, Status.NEW)
        self.assertEqual(t.frontmatter.card, "Olivia Voldaren")
        fm = t.frontmatter
        self.assertTrue(fm.audit_run_id.endswith("-olivia_voldaren-audit"))
        self.assertEqual(fm.audit_model, "opus")
        self.assertEqual(fm.audit_tokens, 150)
        self.assertGreaterEqual(fm.audit_duration, 0)

    def test_empty_findings_mints_nothing(self) -> None:
        """`findings: []` means the auditor found no bugs."""
        self._set_payload({"card": "Clean Card", "findings": []})
        audit_mod.cmd_audit(_args("Clean Card"))
        self.assertEqual(list(utils.TICKETS_DIR.glob("*.md")), [])

    def test_multiple_cards_mint_independently(self) -> None:
        """Two cards, two audits, two tickets — one per card."""
        for card in ("Olivia Voldaren", "Bloodline Keeper"):
            self._set_payload({
                "card": card,
                "findings": [
                    {
                        "oracle_quote": "o", "code_quote": "c",
                        "description": f"bug in {card}",
                    },
                ],
            })
            audit_mod.cmd_audit(_args(card))

        self.assertTrue(
            (utils.TICKETS_DIR / "olivia_voldaren-01.md").exists()
        )
        self.assertTrue(
            (utils.TICKETS_DIR / "bloodline_keeper-01.md").exists()
        )

    # ── failure paths ─────────────────────────────────────────────

    def test_agent_error_mints_nothing(self) -> None:
        """A persistent agent-reported error → no tickets, no crash."""
        os.environ["FAKE_AGENT_FAIL"] = "broke on purpose"
        audit_mod.cmd_audit(_args("Broken"))
        self.assertEqual(list(utils.TICKETS_DIR.glob("*.md")), [])

    def test_empty_cards_raises(self) -> None:
        """`--cards ' , '` → ValueError."""
        with self.assertRaises(ValueError):
            audit_mod.cmd_audit(_args(" , "))

    # ── retry recovery ────────────────────────────────────────────

    def test_retry_recovers_from_bad_json(self) -> None:
        """First attempt writes bad JSON, loader rejects, second attempt OK."""
        self._set_payload({
            "card": "Retry Card",
            "findings": [
                {"oracle_quote": "o", "code_quote": "c", "description": "d"},
            ],
        })
        counter = self.tmp / "attempt-counter"
        os.environ["FAKE_AGENT_ATTEMPT_COUNTER"] = str(counter)
        os.environ["FAKE_AGENT_FAIL_FIRST_N"] = "1"

        audit_mod.cmd_audit(_args("Retry Card"))

        self.assertTrue((utils.TICKETS_DIR / "retry_card-01.md").exists())
        # Fake agent bumped the counter on each call → second attempt ran.
        self.assertEqual(counter.read_text().strip(), "2")


if __name__ == "__main__":
    unittest.main(verbosity=2)
