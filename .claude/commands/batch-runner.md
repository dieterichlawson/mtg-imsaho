# Batch Agent Runner

Launch up to 250+ sandboxed agents in parallel with shared prompt caching. Each agent gets its own working directory and runs an autonomous tool-use loop until its task is complete.

## Location

```bash
export PATH="$HOME/.bun/bin:/opt/homebrew/bin:$PATH"
bun run /Users/dlaw/agent/claude-code/src/batch.ts [options]
```

## Prerequisites

- `ANTHROPIC_API_KEY` set in environment
- `bun` runtime (`~/.bun/bin/bun`)
- `rg` (ripgrep) for OS-level sandboxing (`brew install ripgrep`)

---

## Prompt Structure and Caching

There are three layers of prompt content. Understanding where each goes is key to getting cache hits.

```
┌──────────────────────────────────────────────┐
│ Layer 1: System prompt + tool schemas        │
│   Source: built-in (batchPrompt.ts) or       │
│           --system-prompt / --system-prompt-file
│   ~7K tokens. Defines agent behavior,        │
│   tool usage rules, sandbox constraints.     │
│   Identical for all agents.                  │
│                                              │  ← All cached together
│ Layer 2: Shared prompt                       │     as one prefix.
│   Source: --shared-prompt / --shared-prompt-file
│           or config.sharedPrompt             │     Agent #1 creates
│   Your ~10KB task description shared by all  │     the cache.
│   agents. This is the main content you       │     Agents #2-250
│   write. Goes into the system prompt blocks  │     read from cache.
│   alongside the system prompt.               │
├──────────────────────────────────────────────┤
│ Layer 3: Per-agent prompt                    │  ← NOT cached.
│   Source: "prompt" field in agents.jsonl     │     Small and unique
│           or --agent-prompt with --agents-dir│     per agent.
│   The unique suffix for each agent.          │
│   Sent as the user message.                  │
└──────────────────────────────────────────────┘
```

**Cache behavior in practice:**
- Layers 1+2 are pre-computed once before any agent launches, then passed as identical pre-built blocks to every agent's API call. This guarantees byte-for-byte identical prefixes.
- Agent #1: `cache_creation` = full prefix size (~8-15K tokens). `cache_read` = 0.
- Agents #2+: `cache_creation` ≈ 0. `cache_read` = full prefix size. They pay only for the small per-agent suffix.
- Cache TTL is 5 minutes by default. Set `FORCE_1H_CACHE_TTL=1` env var for 1-hour TTL.
- The cache is server-side at Anthropic, scoped per organization (API key). As long as agents run within the TTL window, the cache stays warm.

**Rule of thumb:** Put everything agents have in common into `--shared-prompt`. Put only what's unique per agent into the JSONL `prompt` field. The more you put in the shared prompt, the bigger the cache savings.

---

## Concurrency

Agents run as async tasks in a single Bun process, controlled by a semaphore:

```
Semaphore (max N slots)
├── Agent 1 [running]  → API call → tool exec → API call → ...
├── Agent 2 [running]  → API call → tool exec → API call → ...
├── Agent 3 [running]  → ...
├── Agent 4 [waiting for slot]
├── Agent 5 [waiting for slot]
└── ...
```

- `-n` / `--concurrency` controls how many agents run simultaneously (default: 10).
- When an agent finishes, the next waiting agent starts immediately.
- Each running agent may have 1 bash subprocess at a time, so ~N active processes total.
- Results stream to stdout as agents complete (not in order — first-finished, first-output).

**Choosing concurrency:**
- API rate limits are the main constraint. Start with `-n 10` and increase if you're not hitting 429s.
- Memory: each agent holds its conversation history. For long conversations (many turns), lower concurrency avoids memory pressure.
- Bash-heavy workloads: each agent's bash commands are sequential, so high concurrency helps throughput.

---

## Invocation Patterns

### Config file

```json
{
  "model": "claude-sonnet-4-20250514",
  "sharedPrompt": "Your shared prompt (cached across all agents)...",
  "concurrency": 10,
  "maxTurns": 100,
  "agents": [
    {"id": "agent-1", "prompt": "Per-agent task", "workDir": "/path/to/dir1"},
    {"id": "agent-2", "prompt": "Per-agent task", "workDir": "/path/to/dir2"}
  ]
}
```

```bash
bun run /Users/dlaw/agent/claude-code/src/batch.ts config.json
```

### Directory-based (each subdirectory = one agent, same prompt)

```bash
bun run /Users/dlaw/agent/claude-code/src/batch.ts \
  --shared-prompt-file prompt.txt \
  --agents-dir ./workspaces \
  --agent-prompt "Do the task described in TASK.md" \
  -n 20 -o ./results
```

### JSONL agent list (per-agent prompts)

Create `agents.jsonl` with one JSON object per line:
```
{"id": "repo-1", "prompt": "Fix the failing tests", "workDir": "/tmp/repos/repo-1"}
{"id": "repo-2", "prompt": "Add error handling", "workDir": "/tmp/repos/repo-2"}
```

```bash
bun run /Users/dlaw/agent/claude-code/src/batch.ts \
  --shared-prompt "You are a code repair agent..." \
  --agents-file agents.jsonl \
  -n 10 -o ./results
```

### Config file with CLI overrides

```bash
bun run /Users/dlaw/agent/claude-code/src/batch.ts config.json \
  --concurrency 5 --model claude-sonnet-4-20250514 --no-sandbox
```

---

## CLI Options

| Flag | Description |
|---|---|
| `config.json` (positional) | JSON config file |
| `-m, --model <model>` | Model (default: claude-sonnet-4-20250514) |
| `-n, --concurrency <num>` | Max concurrent agents (default: 10) |
| `--max-turns <num>` | Max tool-use turns per agent (default: 100) |
| `--shared-prompt <text>` | Shared prompt text (cached across agents) |
| `--shared-prompt-file <file>` | Read shared prompt from file |
| `--system-prompt <text>` | Override default system prompt |
| `--system-prompt-file <file>` | Read system prompt from file |
| `--no-sandbox` | Disable OS-level sandboxing |
| `--agents-file <file>` | JSONL file, one `{"id","prompt","workDir"}` per line |
| `--agents-dir <dir>` | Each subdirectory becomes an agent |
| `--agent-prompt <text>` | Per-agent prompt (with `--agents-dir`, same for all) |
| `--agent-prompt-file <file>` | Read per-agent prompt from file |
| `-o, --output-dir <dir>` | Write `<id>.json` per agent to this dir |

---

## Output

- **stdout**: JSON lines, one per completed agent (stream as they finish)
- **stderr**: Progress with cache stats: `[1/250] Agent foo: OK (cache_read=7615 cache_create=0)`
- **`-o` dir**: Per-agent JSON files written as `<id>.json`

Each result:
```json
{
  "id": "agent-1",
  "messages": [...],
  "usage": {
    "inputTokens": 10,
    "outputTokens": 200,
    "cacheReadTokens": 7615,
    "cacheCreationTokens": 0
  },
  "error": null
}
```

---

## Sandboxing

Each agent is sandboxed to its `workDir`:
- **OS-level (macOS seatbelt)**: Bash commands kernel-sandboxed. Cannot write outside workDir. Cannot read other agents' directories or user home.
- **Soft sandbox**: File tools (Read, Write, Edit, Glob, Grep) validate paths are within workDir before executing.
- **Web tools**: WebFetch and WebSearch pass through (no filesystem access).

---

## Tools Available to Agents

Bash, Read, Write, Edit, Glob, Grep, WebFetch, WebSearch.

---

## Workflow

1. **Prepare working directories** — one per agent, containing the files it will work on
2. **Write the shared prompt** — common instructions (put as much here as possible for caching)
3. **Define per-agent prompts** — unique task per agent (keep small)
4. **Run** — results stream as agents complete
5. **Process** — parse JSON lines from stdout or read from `--output-dir`
