# Play the CLI Game via tmux

Play the MTG CLI game via tmux, driving all input programmatically. Use this to test the UI, verify card interactions, or play a full game.

## Arguments
- `$ARGUMENTS` — Optional: opponent type and deck specs (e.g., "random red-green vs white-black", "claude decks/wb-death-triggers.txt vs decks/rg-aggro-triggers.txt"). Defaults to `--p2 random --deck1 red-green --deck2 white-black`.

## Setup

```bash
# Kill any existing session
tmux kill-session -t mtg 2>/dev/null; sleep 0.3

# Start the game (adjust --p2, --deck1, --deck2 based on $ARGUMENTS)
tmux new-session -d -s mtg -x 120 -y 45 \
  "cargo run --release --bin mtg-runner -- --p1 cli --p2 <OPP> \
   --deck1 <DECK1> --deck2 <DECK2>; sleep 120"

# Wait for compilation + startup
sleep 5
```

Available opponents: `random`, `claude`, `claude:claude-haiku-4-5-20251001`, `gemini`

Built-in decks: `red-green`, `white-black`, `blue-white`, `black-aggro`

Custom deck files in `decks/`: `decks/wb-death-triggers.txt`, `decks/rg-aggro-triggers.txt`, `decks/stress-test-wb.txt`, `decks/stress-test-rg.txt`

## CRITICAL: How to Send Input

The CLI uses crossterm raw mode. **You MUST use `C-m` instead of `Enter`** in all tmux send-keys commands. Using `Enter` causes garbled input ('j' characters appear).

**Correct pattern for every input:**
```bash
tmux send-keys -t mtg '3'     # send the character
sleep 0.3
tmux send-keys -t mtg C-m     # submit (NOT Enter!)
sleep 0.8                     # wait for processing + re-render
```

**Shorthand (works most of the time):**
```bash
tmux send-keys -t mtg '3' C-m; sleep 0.8
```

**Pass priority / confirm default:**
```bash
tmux send-keys -t mtg C-m; sleep 0.8
```

**If input gets garbled** (you see random characters in the `>` prompt):
```bash
tmux send-keys -t mtg C-u; sleep 0.3    # clear line buffer
tmux send-keys -t mtg '3' C-m; sleep 0.8  # retry
```

### Timing Rules
- **0.8s minimum** between sequential commands (the CLI needs time to process, re-render, and re-enter raw mode)
- **10–15s** after passing priority to AI opponent (API calls are slow)
- **5s** after starting the tmux session (compilation time)
- A spinner `⠹` shows next to `Opp:` when the AI is thinking — wait until it disappears

## Reading the Screen

```bash
# Full screen (3-4KB — use sparingly)
tmux capture-pane -t mtg -p

# Turn/step header only
tmux capture-pane -t mtg -p | grep "Turn [0-9]"

# Board state
tmux capture-pane -t mtg -p | grep -E "Opp:|You:|Lands:"

# Action list
tmux capture-pane -t mtg -p | grep -E "[0-9]+:"

# Game log
tmux capture-pane -t mtg -p | grep -E "cast|died|took|resolved|played|declared"
```

## How the Game Works

### Action Selection (Main Phase, Priority)
The screen shows numbered actions:
```
0: Pass priority
1: Tap Mountain for mana
2: Play land Forest
3: Cast Goblin Piker
4: Concede
```
Send the number + C-m to choose. **C-m alone = Pass priority** (option 0).

### Casting Spells (Manual Mana Tapping)
Spells require mana in your pool BEFORE the "Cast" option appears:
1. Tap lands one at a time (e.g., send `'1' C-m` for "Tap Mountain for mana")
2. Repeat until you have enough mana (shown as `Mana: Red:2 Green:1`)
3. The "Cast [Spell]" option appears in the action list
4. Send the cast option number + C-m
5. If the spell needs a target, a **target selection prompt** appears — choose a number + C-m

### Declare Attackers
```
Eligible attackers:
  0: Goblin Piker 2/1
  1: Kalonian Tusker 3/3
  Attack (numbers/all/none)>
```
- **C-m or `all`** = attack with ALL eligible creatures
- **`none` or `n`** = don't attack
- **`0` or `0 1`** = attack with specific creatures by number

### Declare Blockers
```
Attackers:
  0: Grizzly Bears 2/2
Your blockers:
  0: Goblin Piker 2/1
  1: Kalonian Tusker 3/3
  Block (blocker:attacker / enter=none)>
```
- **C-m** = no blocks
- **`0:0`** = blocker 0 blocks attacker 0
- **`0:0 1:0`** = both blockers block attacker 0 (double block)

### Shortcuts
- **`f` C-m** = engage auto-pass mode (passes until your next Main Phase 1). Shows `[AUTO-PASS]` in the header. Breaks automatically for: stack responses, opponent attacks with creatures, your own combat with creatures, blocker decisions.
- **`g` C-m** = view graveyards
- **`e` C-m** = view exile zone
- **`d` C-m** = browse deck
- **`l` C-m** = view full log
- **`i` C-m** = inspect battlefield (detailed creature stats)
- **`/`** = card search (type immediately, no C-m needed first)

### Auto-Pass Behavior
The engine auto-passes most boring phases. You are only prompted when:
- You can play a land (Main Phase, your turn)
- You can cast a sorcery-speed spell with available mana (Main Phase, your turn)
- You have eligible attackers (Declare Attackers)
- Opponent attacks and you have eligible blockers (Declare Blockers)
- The stack has items AND you have a meaningful response (instant + mana)
- You're in a key combat step with instants available (DeclareAttackers/Blockers)
- You need to discard to hand size (Cleanup)

An empty turn (play land, nothing else) takes **1–2 keypresses**. A full combat turn takes **3–5 keypresses**.

## Screen Layout
```
┌─── STACK ───────┬─ Turn N - Phase | Whose turn ─────────────────┬─── CARDS ───────┐
│ (stack items)   │ BATTLEFIELD                                   │ (card reference) │
│                 │  Opp: 20hp  33lib  0gy  0ex  7hand            │ Card Name {cost} │
│                 │  Lands: 2x Mountain, 1x Forest                │ Type line        │
│                 │  Goblin Piker 2/1 [T]                         │ Oracle text...   │
│                 │                · · ·                          │                  │
│                 │  Your creatures...                            │                  │
│                 │  Lands: ...                                   │                  │
├─── LOG ─────────│▸ You: 20hp  33lib  0gy  0ex  7hand            │                  │
│ (game events)   ├─── HAND ──────────────────────────────────────│                  │
│                 │  Mountain                                     │                  │
│                 │  Goblin Piker {1}{R} 2/1                      │                  │
│                 ├────────────────────────────────────────────────│                  │
│                 │  0: Pass priority                              │                  │
│                 │  1: Tap Mountain for mana                     │                  │
│                 │  [enter=pass] [f=auto-pass] [/=search]...     │                  │
│                 │  >                                             │                  │
└─────────────────┴───────────────────────────────────────────────┴──────────────────┘
```

**Creature flags:** `[T]`=tapped, `[S]`=summoning sick, `(2d)`=2 damage marked, `[Pacifism]`=attached aura

**`▸`** = active player indicator. **`⠹`** = AI thinking spinner.

## Example: Play a Land and Cast a Creature

```bash
# At Turn 1 Main Phase 1, play a land (option 1)
tmux send-keys -t mtg '1' C-m; sleep 0.8

# Auto-passes to Turn 3 Main Phase 1 (skips empty Turn 2)
# Play another land
tmux send-keys -t mtg '2' C-m; sleep 0.8

# Tap both lands for mana
tmux send-keys -t mtg '1' C-m; sleep 0.8   # Tap first land
tmux send-keys -t mtg '1' C-m; sleep 0.8   # Tap second land

# Now "Cast Grizzly Bears" should appear — check:
tmux capture-pane -t mtg -p | grep "Cast"

# Cast (whatever option number it is, e.g., 2)
tmux send-keys -t mtg '2' C-m; sleep 0.8

# Pass remaining priority
tmux send-keys -t mtg C-m; sleep 0.8
```

## Cleanup
```bash
tmux kill-session -t mtg 2>/dev/null
```
