# Iterating on the CLI Priority Passing Experience

## Your Task

Play the CLI game via tmux and iterate on the auto-pass mechanics and UI.
The goal is to make the priority-passing experience feel smooth and natural.
Any time something feels clunky or takes too long, make a note, figure out
how to improve it, implement the change, and then play again to see if it's
better. Keep iterating.

Key constraint: **don't mess up people's games.** If a player asks to pass
but then the opponent casts an instant, the pass must not steamroll through
the chance to respond. The system needs to be smart about when to stop.

Some directions to explore (but don't limit yourself to these):
- Different options for how long to pass (pass until my next turn, pass until
  my next draw/main phase, pass through this phase only, etc.)
- A configuration or preference system that lets the player control how often
  they get priority back
- Smart auto-passing when there's genuinely nothing to do
- Visual feedback about what pass mode is active

Explicitly seek out weird situations — complex priority, stack interactions,
combat tricks, triggered abilities — to make sure the CLI handles them
smoothly. Create save games with specific board states, use scripted players,
whatever you need to stress-test edge cases.

---

## How to Drive the CLI via tmux

### Setup
```bash
# tmux should already be installed (brew install tmux if not)

# Start a game in a detached tmux session (120 cols x 45 rows)
tmux new-session -d -s mtg -x 120 -y 45 \
  "cargo run --release --bin mtg-runner -- --p1 cli --p2 random \
   --deck1 red-green --deck2 white-black; sleep 60"

# For games against the AI:
tmux new-session -d -s mtg -x 120 -y 45 \
  "cargo run --release --bin mtg-runner -- --p1 cli --p2 claude \
   --deck1 decks/wb-death-triggers.txt --deck2 decks/rg-aggro-triggers.txt; sleep 120"

# The `; sleep` keeps the pane alive after the game ends
# so you can read the final result. Use 120s for AI games (API calls are slow).
```

### Interaction Commands
```bash
# View the current screen (returns full text, no ANSI codes)
tmux capture-pane -t mtg -p

# Send a keystroke (e.g., select option 1)
# IMPORTANT: Use C-m (Ctrl-M) instead of Enter to avoid garbled input!
tmux send-keys -t mtg '1' C-m

# Send a shortcut key
tmux send-keys -t mtg 'f' C-m

# Send just Enter/pass priority
tmux send-keys -t mtg C-m

# Kill the session when done
tmux kill-session -t mtg
```

### CRITICAL: Input Timing and the 'j' Bug

The CLI uses crossterm raw mode for input (`read_line_with_search()`). When
tmux sends `Enter`, it sometimes gets interpreted as the character 'j' instead
of a carriage return. This causes garbled input like "1j1j1j" in the prompt.

**Workarounds:**
1. **Use `C-m` instead of `Enter`** in all `send-keys` commands. `C-m` sends
   a literal carriage return that crossterm handles reliably.
2. **Sleep at least 0.6–0.8s between commands.** Shorter delays (0.2–0.3s)
   cause crossterm to miss or misinterpret keystrokes. The CLI needs time to
   process the action, re-render, and re-enter raw mode before the next key.
3. **Send the character and C-m separately** for maximum reliability:
   ```bash
   tmux send-keys -t mtg '3'    # character arrives
   sleep 0.3
   tmux send-keys -t mtg C-m    # submit
   sleep 0.8                    # wait for processing
   ```
4. **If input gets garbled**, send `C-u` to clear the line buffer, then retry:
   ```bash
   tmux send-keys -t mtg C-u
   sleep 0.3
   tmux send-keys -t mtg '3' C-m
   ```

### How Auto-Pass Works Now

The engine and CLI have smart auto-passing that dramatically reduces keypresses:

**Engine-level auto-pass** (transparent to the player):
- When the only legal actions are Pass + Concede + mana abilities, AND there
  are no spells the player could cast even with full potential mana, the engine
  skips the priority prompt entirely.
- Instants are only counted as "castable" when something interesting is
  happening: items on the stack (to respond to) or key combat steps
  (DeclareAttackers/DeclareBlockers for combat tricks).
- Empty combat (no eligible attackers) auto-declares zero attackers.

**CLI-level auto-pass** (`f` key):
- Press `f` to engage **[AUTO-PASS]** mode (shown in the turn/step bar).
- Passes until YOUR next Main Phase 1 on a later turn.
- **Breaks automatically when:**
  - Something is on the stack AND you have a meaningful response
  - Opponent declares attackers AND they have creatures
  - It's your combat and you have eligible attackers (lets you choose)
  - It's your blockers step and you have eligible blockers

**Result:** An empty turn (play land, nothing else to do) goes from **20
keypresses down to 1–2 keypresses.** Full turns with creatures and combat
typically need 3–5 keypresses.

### Key UI Behaviors to Know

- **Enter at Declare Attackers = attack with ALL** (not zero). Type `none` or
  `n` to skip attacking. This prevents accidental attack skips.
- **Enter at Declare Blockers = no blocks.** Blocking requires `blocker:attacker`
  format (e.g., `0:0`).
- **Casting spells requires manual mana tapping.** You must tap lands first
  (select the "Tap Mountain for mana" actions), then the "Cast Spell" option
  appears once you have enough mana in your pool. The engine stops you at
  phases where you COULD cast something with potential mana, even if you
  haven't tapped yet.
- **AI games are slow.** Each AI decision takes 3–10s (API call). Wait at
  least 10–15s after passing priority before checking the screen. A spinner
  (⠹) shows next to "Opp:" when the AI is thinking.
- **Target selection is a separate prompt** — when casting a targeted spell,
  the screen shows a numbered target list. Select a number + C-m.
- **Object IDs are not shown.** Creatures display as "Goblin Piker 2/1 [T]"
  (no "#32" suffix). Flags: [T]=tapped, [S]=summoning sick, (Nd)=N damage.
- **Log and stack panels wrap text** instead of truncating, so long card names
  like "Swords to Plowshares" are fully readable.

### Useful grep patterns for reading the screen
```bash
# Just the turn/step header
tmux capture-pane -t mtg -p | grep "Turn [0-9]"

# Board state summary
tmux capture-pane -t mtg -p | grep -E "Opp:|You:|Lands:"

# Action list
tmux capture-pane -t mtg -p | grep -E "^[[:space:]]*[0-9]+:"

# Log entries
tmux capture-pane -t mtg -p | grep -E "cast|died|took|resolved|played|declared"

# Full screen (3-4KB)
tmux capture-pane -t mtg -p
```

### Context Cost
Each `capture-pane` returns ~3-4KB (the full 120x45 grid). Budget 1-2 full
games per context window. For efficiency, use the grep patterns above to grab
just what you need. The full screen is only needed when debugging display issues.

### Creating Specific Test Scenarios
You can construct specific board states programmatically and resume from them.
Write a Rust test/binary that builds a GameState and serializes it as a save
file:

```rust
// In a test or helper binary:
let registry = CardRegistry::with_all_cards();
let mut state = GameState::new(2);
state.step = Step::PrecombatMain;
state.active_player = PlayerId(0);
state.priority_player = Some(PlayerId(0));
state.turn_number = 5;
state.is_first_turn = false;
state.players[0].life = 20;
state.players[1].life = 20;

// Place creatures, lands, spells in hand, etc. using:
//   state.create_object(card_id, owner, zone, power, toughness)
//   registry.get_id_by_name("Card Name")

let save = SaveData { state, player_names: vec!["P1".into(), "P2".into()] };
let json = serde_json::to_string_pretty(&save).unwrap();
std::fs::write("/tmp/test_scenario.json", json).unwrap();
```

Then: `cargo run --release -- --resume /tmp/test_scenario.json --p1 cli --p2 random`

See `mtg-runner/tests/scripted_scenarios.rs` for detailed examples of state
construction. The `mtg-engine/tests/common/mod.rs` helpers (`game_at_step`,
`ready_creature`, `castable_spell`, etc.) are useful patterns too.

You can also use `--p2 scripted` (if you wire it up) to control exactly what
the opponent does, which is useful for testing "opponent casts spell while
you're in pass mode" scenarios.

---

## How the Current System Works

### Engine level (`mtg-engine/src/engine.rs`)
- The game loop calls `advance_step()` which sets `priority_player` at each step
- Every step except Untap grants priority to someone
- **Smart auto-pass**: The engine auto-passes when the player has no meaningful
  actions. "Meaningful" means anything beyond Pass/Concede/ActivateManaAbility,
  OR having castable spells with potential mana from untapped sources. Instants
  are only counted as castable when the stack has items or it's a key combat
  step (DeclareAttackers/DeclareBlockers).
- **Auto-declare zero attackers**: When no creatures are eligible to attack,
  the engine auto-declares zero without prompting.
- `consecutive_passes` tracks sequential passes; when all players pass, either
  resolve the stack or advance the step
- `has_castable_with_potential_mana()` computes whether tapping all available
  mana sources would let the player cast any spell (checking timing, mana cost,
  AND valid targets). This prevents skipping turns where the player has spells
  to cast but hasn't tapped mana yet.

### CLI level (`mtg-player/src/cli.rs`)
- **Enter**: Pass priority once (at action prompt) OR attack with all (at
  declare attackers prompt)
- **`f`**: Engages `PassMode::UntilNextTurn`, which auto-passes until your
  next Main Phase 1. Smart break conditions:
  1. It's your Main Phase 1 on a later turn than when `f` was pressed
  2. The stack has a spell AND you have a meaningful response (not just
     pass/concede/mana abilities)
  3. Opponent is at DeclareAttackers AND they have creatures on the battlefield
- Combat: in pass mode, auto-declares zero attackers only if you have NO
  eligible creatures. If you have creatures, pass mode breaks so you can choose.
- Auto-declares zero blockers when you have no eligible blockers.
- **[AUTO-PASS]** indicator shows in the turn/step bar when pass mode is active.

### LLM player (`mtg-player/src/llm.rs`)
- Auto-passes when only Pass/Concede are available (no API call needed)
- See `should_auto_pass()` around line 283
- Card knowledge for 100+ cards in the system prompt (lines 48-138)

### Phases that grant priority (per turn)

| Step | Priority holder | Usually meaningful? |
|------|----------------|---------------------|
| Untap | None | No priority at all |
| Upkeep | Active player | Rarely (only with upkeep triggers) |
| Draw | Active player | Rarely |
| Main Phase 1 | Active player | **Yes** — main decision point |
| Begin Combat | Active player | Rarely |
| Declare Attackers | Active player | **Yes** — choose attackers |
| Declare Blockers | Defending player | **Yes** — choose blockers |
| Combat Damage | Active player | Sometimes (post-damage instants) |
| End Combat | Active player | Rarely |
| Main Phase 2 | Active player | Sometimes |
| End Step | Active player | Rarely |
| Cleanup | None (usually) | Only if discard needed |

Both players get priority at each step (active first, then non-active). A full
turn with nothing to do = ~10 priority passes per player per turn.

---

## What We Observed Playing the Game

### Resolved issues (from initial playtesting)
These were all fixed in the implementation:

- **20-keypress problem** → FIXED. Smart auto-pass at the engine level now
  skips all phases where the player has no meaningful actions. A turn with
  just a land play and nothing else takes 1-2 keypresses.
- **`f` stops at opponent's DeclareAttackers with no creatures** → FIXED.
  Now checks if opponent actually has creatures before breaking.
- **`f` skips your own turn** → FIXED. Now passes to YOUR next Main Phase 1
  (not just any new turn), and breaks to let you declare attackers if you
  have eligible creatures.
- **`f` auto-declares zero attackers** → FIXED. If you have creatures, pass
  mode breaks so you can choose. Only auto-skips if you have zero eligible.
- **No visual indicator** → FIXED. [AUTO-PASS] shows in the turn/step bar.
- **Object IDs (#32, #63) shown in creature display** → FIXED. Removed.
- **Log/stack text truncation** → FIXED. Long card names now wrap.
- **Token names showing as "?"** → FIXED. Combat log now shows "Spirit".
- **Accidental attack skip** → FIXED. Enter = attack all, type "none" to skip.

### Remaining issues (not yet addressed)
- **Manual mana tapping is clunky**: Player must tap lands one at a time
  before seeing "Cast" options. Auto-tap (compute mana payment automatically)
  would be a major engine improvement but is a larger project.
- **Rapid input can skip meaningful prompts**: When the opponent casts a spell
  and the player has a response, the response prompt appears briefly. If the
  player is pressing Enter rapidly, they can accidentally pass it. Could add
  a visual/audio cue when the stack changes.
- **Main Phase 2 always stops**: Even if the player just passed MP1, MP2
  stops again when castable sorcery-speed spells exist. Technically correct
  (post-combat casting is valid strategy) but slightly annoying.
- **tmux input garbling**: The crossterm raw mode + tmux `send-keys Enter`
  interaction sometimes produces 'j' characters. Use `C-m` instead (see
  "How to Drive the CLI via tmux" above).

---

## Scenarios to Seek Out

When playing and iterating, make sure to specifically test these situations:

1. **Empty board, no spells** — both players have only lands. How many
   keypresses to get through a turn?

2. **Opponent casts instant during your combat** — you attack, opponent casts
   removal. Does your pass mode break so you can respond?

3. **Opponent casts instant during their turn** — you're in pass mode and
   have mana + an instant in hand. Does the stack check break pass mode?

4. **Declare blockers with creatures** — opponent attacks, you have creatures.
   Must always prompt.

5. **Declare blockers with no creatures** — opponent attacks, you have no
   untapped creatures. Should this auto-pass? Currently it stops.

6. **Multiple spells on stack** — both players casting instants back and forth.
   Does priority feel natural?

7. **Triggered abilities** — a creature dies and triggers something. Does
   the player get prompted when they should?

8. **`f` pressed during your main phase** — what happens to your combat step?

9. **`f` pressed during your upkeep** — does it skip your main phase?

10. **Discard to hand size while in pass mode** — what should happen?

---

## Starting Points for Improvement

These are ideas that came out of playing. Use them as starting points but
come up with your own ideas too as you play and iterate.

- **Smart auto-pass**: When the only legal action is Pass/Concede (no castable
  spells, no mana abilities, no combat decisions), just auto-pass without
  prompting. The LLM player already does this. This alone might eliminate
  most of the 20-keypress problem.

- **Rethink what `f` means**: Maybe `f` should mean different things depending
  on context. During your turn, maybe it means "pass to combat." During
  opponent's turn, maybe it means "pass until my next main phase." Or maybe
  there should be multiple keys for different pass durations.

- **Smarter break conditions for `f`**: Check whether the opponent actually
  has creatures before stopping at their Declare Attackers. Check whether
  you have any possible responses before breaking on stack changes.

- **Pass mode indicator**: Show something on screen when auto-pass is active
  so the player knows what's happening.

- **MTGO-style stops**: Let the player configure which phases they want to
  stop at. Default to just Main Phase 1 + Declare Attackers/Blockers.

- **Phase grouping**: Maybe instead of individual phase stops, group phases:
  "pre-combat" (upkeep through begin combat), "combat" (attackers through
  end combat), "post-combat" (main 2 through end step).

---

## Key Files

| File | What's there |
|------|-------------|
| `mtg-player/src/cli.rs` | CliPlayer — all pass logic, rendering, input handling |
| `mtg-engine/src/engine.rs` | Game loop, priority system, auto-pass, step advancement |
| `mtg-player/src/lib.rs` | Player trait definition |
| `mtg-player/src/llm.rs` | LLM player auto-pass logic (good reference) |
| `mtg-runner/src/main.rs` | Game loop callback, save/resume |
| `mtg-engine/src/view.rs` | GameView — what the player can see |
| `mtg-engine/src/state.rs` | GameState — full game state structure |
| `mtg-runner/tests/scripted_scenarios.rs` | Examples of constructing specific game states |
| `mtg-engine/tests/common/mod.rs` | Test helpers for state setup |
