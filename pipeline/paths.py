"""Pipeline filesystem layout. One source of truth, easy to patch in tests."""
from pathlib import Path

PROJECT_ROOT  = Path(__file__).resolve().parent.parent
PIPELINE_DIR  = PROJECT_ROOT / "pipeline"
TICKETS_DIR   = PIPELINE_DIR / "tickets"
ARCHIVE_DIR   = TICKETS_DIR / "archive"
STAGING_DIR   = PIPELINE_DIR / "staging"
PROMPTS_DIR   = PIPELINE_DIR / "prompts"
SCRIPTS_DIR   = PIPELINE_DIR / "scripts"
METRICS_DIR   = PIPELINE_DIR / "metrics"
LOGS_DIR      = PIPELINE_DIR / "logs"
WORKTREES_DIR = PROJECT_ROOT / ".worktrees"
ORACLE_SCRIPT = PROJECT_ROOT / "scripts" / "oracle_lookup.py"

# Agent subprocesses get this settings file to enforce the "no access to
# archived tickets" rule.
AGENT_SETTINGS = PIPELINE_DIR / "agent-settings.json"
