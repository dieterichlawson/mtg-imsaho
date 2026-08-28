## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/249/woodland-cemetery?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
This land enters tapped unless you control a Swamp or a Forest.
{T}: Add {B} or {G}.
```

**Status**: PASS

### Code issues
No issues found.

- "enters tapped **unless** you control a [land type]" is a replacement effect
  (CR 614.1d), implemented through `replace_event` /
  `helpers::enters_tapped_unless` rather than as an ETB trigger.
- That distinction is not cosmetic, and `enters_tapped_replacement.rs` documents
  the three ways the trigger version was wrong: the land entered untapped and
  could be tapped for mana in response to its own trigger; the condition was read
  at resolution, so an opponent could destroy the enabling land in response; and
  a trigger opened a priority window even when nothing needed to happen.
- The condition reads the battlefield for a land of the right type, and the five
  lands do not satisfy each other.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`enters_tapped_replacement.rs` — all five lands, both directions, plus the already-tapped-before-priority and no-trigger-on-the-stack checks.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/249/woodland-cemetery?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
This land enters tapped unless you control a Swamp or a Forest.
{T}: Add {B} or {G}.
```

**Status**: PASS

### Code issues
No issues found.

- "This land enters tapped **unless** you control a X or a Y" is a replacement
  effect (CR 614.1d), applied as the land enters rather than tapped afterwards —
  `enters_tapped_unless` with the condition as a closure: PASS
- The condition is checked against the game state **before** the land enters
  (CR 616.1), so the land itself never satisfies its own check: PASS
- The check is for a land with that *subtype*, so a nonbasic land with the
  subtype counts and a basic with a different name does not: PASS
- Both mana abilities are declared separately, so the engine offers a choice of
  colour rather than assuming one: PASS
- The two subtypes it checks are **Swamp** and **Forest**, matching the fetched
  oracle text exactly — verified per card rather than assumed from the cycle:
  PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Entering tapped or untapped by the condition: `cards_lands_and_mana_sources.rs`, `enters_tapped.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/249/woodland-cemetery?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
This land enters tapped unless you control a Swamp or a Forest.
{T}: Add {B} or {G}.
```

**Rulings fetched**: none published for this card.

**Status**: PASS

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/249/woodland-cemetery
**Oracle text**:
```
This land enters tapped unless you control a Swamp or a Forest.
{T}: Add {B} or {G}.
```
**Type line**: `Land`
**Mana cost**: none
**Rulings**: none published for this card (no `rulings` entry in `data/oracle_cache.json`).
**Status**: PASS (test coverage extended; test-file cleanup)

### Card data
| field | oracle | `woodland_cemetery.rs` | |
|---|---|---|---|
| name | Woodland Cemetery | `"Woodland Cemetery"` | ok |
| cost | *(none)* | `cost` left at `Default` = `None` | ok |
| types | Land | `vec![CardType::Land]` | ok |
| supertypes | *(none)* | *(none)* | ok |
| subtypes | *(none)* | *(none)* | ok |
| P/T | — | `None`/`None` | ok |
| oracle_text | as above | byte-identical | ok |

Note the oracle's order is **"a Swamp or a Forest"** and the mana is **"{B} or {G}"** — black first. The code
matches on both counts; the sister card Hinterland Harbor is the green/blue mirror and it would be easy to
transpose them.

### Code issues
No issues found. The doc comment already reads
`/// This land enters tapped unless you control a Swamp or a Forest.`, matching the current wording.

### Rules check
- **Replacement effect, not a trigger** (CR 614.1d — the plain *"[This permanent] enters …"* form; 614.1c is
  reserved for *"enters with …"*, *"As … enters …"* and *"enters as …"*). Implemented via `replace_event` →
  `helpers::enters_tapped_unless`.
- **CR 616.1**: `move_object` plans the entering event *before* the zone change (`state.rs:704`) and applies it
  after the move but before `EnteredBattlefield` is emitted, so the condition is read against the pre-entry board
  and nothing observes an untapped window.
- **"you control"**: `objects_in_zone(Zone::Battlefield, controller)`, not the whole battlefield.
- **Both halves**: `has_subtype("Swamp") || has_subtype("Forest")`, and `state.has_subtype` consults the object's
  own subtypes as well as the active face's.
- **Self-exclusion** (`o.id == object_id`): unreachable through `move_object` (the land is not yet in the zone),
  live for a token copy, where `create_token` inserts into the battlefield before planning the entry
  (`state.rs:573-576`). Correct either way — CR 616.1 means the entering permanent is not yet one you control.
- **Mana**: two `ManaAbilityDef`s producing `{B}` and `{G}`, both tap-only and free. "Add {B} or {G}" is one
  ability with a choice; exposing two selectable abilities gives the player that choice and no extra mana.

### Changes made
This was the last of the five check lands, which retired a scaffold:

- `mtg-engine/tests/enters_tapped_replacement.rs` — collapsed the `AUDITED` table back into `CHECK_LANDS`.
  `AUDITED` existed only because the set was part-audited: the two stricter tests were allowed to judge a check
  land only once someone had actually fetched its oracle text. With all five audited the gate has done its job,
  and two parallel tables listing the same five cards is drift waiting to happen. The single table now carries
  the two satisfying land types, a non-satisfying basic, and the two colours.
- The merge is a strict coverage increase, not just a rename: `check_land_enters_untapped_when_condition_is_met`
  and `check_land_puts_no_trigger_on_the_stack` used to try only *one* satisfying basic per land and now try
  both. That is what caught mutation 2 twice below.
- `an_audited_check_land_taps_for_both_of_its_colours` → `each_check_land_taps_for_both_of_its_colours`.

### Mutation checks (all discriminating)
1. `objects_in_zone(Battlefield, controller)` → `all_objects_in_zone(Battlefield)` →
   `a_check_land_is_not_satisfied_by_an_opponents_land` FAILED
   (`an opponent's Swamp is not one you control`).
2. Dropped the `|| has_subtype("Forest")` half → **two** tests FAILED:
   `check_land_enters_untapped_when_condition_is_met` (`should enter untapped while you control a Forest`) and
   `a_check_land_is_not_satisfied_by_an_opponents_land` (`your own Forest does satisfy it`). Before the table
   merge only the second would have caught it.
3. Second mana ability produces `{B}` instead of `{G}` → `each_check_land_taps_for_both_of_its_colours` FAILED
   (`should add Green; got [[(Black, 1)], [(Black, 1)]]`).

### Tricky interactions checked
- Opponent controls the only Swamp/Forest → enters tapped: **pass**.
- Another check land is neither a Swamp nor a Forest → enters tapped: **pass**
  (`check_lands_do_not_satisfy_each_other`).
- Qualifying land removed after entry → cannot retroactively tap: **pass**
  (`condition_is_evaluated_at_entry_not_later`).
- Tapping for mana in response to its own ETB trigger: **impossible**, no trigger exists
  (`check_land_puts_no_trigger_on_the_stack`).

### Test coverage
- untapped with either satisfying land type: `enters_tapped_replacement.rs:56`
- tapped without one: `enters_tapped_replacement.rs:73`
- already tapped before priority: `enters_tapped_replacement.rs:85`
- no ETB trigger on the stack: `enters_tapped_replacement.rs:100`
- condition not re-read after entry: `enters_tapped_replacement.rs:122`
- check lands do not satisfy each other: `enters_tapped_replacement.rs:144`
- an opponent's land does not satisfy it: `enters_tapped_replacement.rs:161`
- taps for both {B} and {G}: `enters_tapped_replacement.rs:191`

All eight now cover all five check lands with no audit gate.

### Suite
`cargo check --workspace --all-targets` clean, zero warnings. Full suite exit 0, 1389 passing.

