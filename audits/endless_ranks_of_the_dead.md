## Audit — 2026-08-27 — CR 603.2 trigger scope

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/99/endless-ranks-of-the-dead?utm_source=api
**Type line**: `Enchantment` — {2}{B}{B}
**Oracle text**:
```
At the beginning of your upkeep, create X 2/2 black Zombie creature tokens, where X is half the number of Zombies you control, rounded down.
```

**Status**: ISSUE (fixed) — duplication, not a rules defect

### Code issue
- Oracle text says the trigger happens at **your** upkeep / **your** end step.
- Code did: declared `step_trigger_scope` → `TriggerScope::Your`, which is
  correct and sufficient, and then re-derived the same thing inside the handler
  as `state.active_player != controller`.
- The engine's gate is not taken on trust: `your_upkeep_scope.rs` sweeps the
  registry for every card with a controller-scoped step trigger and checks both
  directions — fires on the controller's step, silent on the opponent's. The
  handler check was provably dead.
- Fixed: removed, with a comment saying where the scoping actually lives.


### What else was checked
- Card data verified exact set-wide (see `ISD_AUDIT_PROGRESS.md`): cost, types,
  subtypes, supertypes, P/T, oracle text, keywords on both faces, flashback
  cost, and trigger kinds against the oracle phrasing.
- Step 9 anti-patterns: clean after this change.

### Test coverage
`your_upkeep_scope.rs::a_your_step_trigger_fires_on_its_controllers_step_and_no_one_elses`
covers this card by registry sweep, in both directions.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/99/endless-ranks-of-the-dead?utm_source=api
**Type line**: `Enchantment` — {2}{B}{B}
**Oracle text**:
```
At the beginning of your upkeep, create X 2/2 black Zombie creature tokens, where X is half the number of Zombies you control, rounded down.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "If you control **fewer than two** Zombies, you won't get any tokens."
  Integer division — `zombie_count / 2` — rounds down, so one Zombie makes none:
  PASS
- "**rounded down**": PASS
- Ruling: "The number of Zombies you control is counted **when the ability
  resolves**. If you control multiple Endless Ranks of the Dead, the tokens you
  get when the first ability resolves will count for the subsequent abilities."
  The count is taken inside the trigger handler, so a second copy resolving
  afterwards sees the first copy's tokens: PASS
- Zombie *tokens* count toward the number — the text says "Zombies you control",
  not "Zombie cards": PASS
- The tokens carry colour and the Zombie subtype, so they feed the next upkeep:
  PASS
- "At the beginning of **your** upkeep": PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The halving and the token subtype: `cards_complex_creatures.rs`, `subtype.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/99/endless-ranks-of-the-dead?utm_source=api
**Type line**: `Enchantment` — {2}{B}{B}
**Oracle text**:
```
At the beginning of your upkeep, create X 2/2 black Zombie creature tokens, where X is half the number of Zombies you control, rounded down.
```

**Rulings fetched**:
- [2011-09-22] If you control fewer than two Zombies, you won't get any tokens.
- [2011-09-22] The number of Zombies you control is counted when the ability resolves. If you control multiple Endless Ranks of the Dead, the tokens you get when the first ability resolves will count for the subsequent abilities (if the tokens are still under your control at that time).

**Status**: PASS


Two rulings:
1. "If you control fewer than two Zombies, you won't get any tokens."
2. "The number of Zombies you control is counted when the ability resolves. If
   you control multiple Endless Ranks of the Dead, the tokens you get when the
   first ability resolves will count for the subsequent abilities (if the
   tokens are still under your control at that time)."

### Code issues
No issues found. Card data matches exactly — {2}{B}{B}, Enchantment, oracle
text verbatim, an `Upkeep` trigger declared with
`step_trigger_scope => TriggerScope::Your` for "at the beginning of **your**
upkeep".

- The count happens inside `on_upkeep`, i.e. as the ability resolves, which is
  ruling 2's first half.
- `zombie_count / 2` on `usize` is floor division, so ruling 1 falls out of it:
  one Zombie gives zero tokens.
- `objects_in_zone(Battlefield, controller)` scopes to "Zombies **you**
  control".
- Tokens go through `create_token_with_subtypes` with the Zombie subtype, so
  CR 111.4 names them and they count themselves for the next trigger.
- `controller_of` for "you"; the comment notes that re-deriving the step's
  owner here would duplicate what `step_trigger_scope` already decides, which
  is right.

The `is_creature` filter alongside `has_subtype("Zombie")` is narrower than
"Zombies you control" strictly reads, since a non-creature Zombie permanent
would not count. There is no such card in the pool, so this is not a defect I
can demonstrate; recorded rather than flagged.

### What the existing test could not tell apart
`endless_ranks_creates_zombie_tokens` set up five Zombies and asserted seven
afterwards. That pins 5 → 2, which a great many wrong formulas also satisfy —
`n/2`, `(n-1)/2`, and `n.div_ceil(2) - 1` all give 2 at five. It had no
opponent on the board, so nothing pinned "you control"; it counted by reading
`o.subtypes` directly rather than through `has_subtype`; and it checked neither
what the tokens are nor either ruling.

### Tricky interactions checked
- Floor division across a range, 0/1/2/3/4/7 Zombies: pass
- Ruling 1, fewer than two Zombies makes nothing: pass
- An opponent's Zombies never count, and no token arrives under them: pass
- The tokens are 2/2, black, Zombie, and tokens: pass
- Ruling 2, a second copy counts the first copy's tokens: pass
- "Your upkeep" and not each upkeep: pass, covered by the oracle-derived sweep
  in `your_upkeep_scope.rs`, which walks every card in the registry
- The trigger resolves after the enchantment is destroyed (CR 113.7a): pass
  (`trigger_source_independence.rs:420`)

### Test coverage
- 5 Zombies make 2 tokens: `cards_upkeep_triggers_and_curses.rs:190`
- Trigger survives the source's destruction:
  `trigger_source_independence.rs:420`
- Token subtype is real for other cards' purposes: `subtype.rs:197`
- Scope is "your upkeep": `your_upkeep_scope.rs:111` (registry-wide sweep)
- **NEW** floor(your Zombies / 2) across six boards, with an opponent's
  Zombies present and ignored: `cards_upkeep_triggers_and_curses.rs:216`
- **NEW** the tokens are 2/2 black Zombies:
  `cards_upkeep_triggers_and_curses.rs:243`
- **NEW** ruling 2, a second copy counts the first's tokens:
  `cards_upkeep_triggers_and_curses.rs:277`

### On the multi-copy test's numbers
Four Zombies, deliberately. Counted at resolution the two abilities make 2 then
3, for nine Zombies; counted once up front they would make 2 each, for eight.
Two or three starting Zombies would give the same total either way and the test
would prove nothing. Confirmed live by mutation: forcing both abilities to make
the same number produces exactly the 8 the ruling distinguishes from 9.

