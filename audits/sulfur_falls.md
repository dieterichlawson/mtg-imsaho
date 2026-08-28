## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/248/sulfur-falls?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
This land enters tapped unless you control an Island or a Mountain.
{T}: Add {U} or {R}.
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

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/248/sulfur-falls?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
This land enters tapped unless you control an Island or a Mountain.
{T}: Add {U} or {R}.
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
- The two subtypes it checks are **Island** and **Mountain**, matching the fetched
  oracle text exactly — verified per card rather than assumed from the cycle:
  PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Entering tapped or untapped by the condition: `cards_lands_and_mana_sources.rs`, `enters_tapped.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/248/sulfur-falls?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
This land enters tapped unless you control an Island or a Mountain.
{T}: Add {U} or {R}.
```

**Rulings fetched**: none published for this card.

**Status**: PASS

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/248/sulfur-falls
**Oracle text**:
```
This land enters tapped unless you control an Island or a Mountain.
{T}: Add {U} or {R}.
```
**Type line**: `Land`
**Mana cost**: none
**Rulings**: none published for this card (`data/oracle_cache.json` has no `rulings` entry for it).
**Status**: PASS (test coverage extended)

### Card data
| field | oracle | `sulfur_falls.rs` | |
|---|---|---|---|
| name | Sulfur Falls | `"Sulfur Falls"` | ok |
| cost | *(none)* | `cost` left at `Default` = `None` | ok |
| types | Land | `vec![CardType::Land]` | ok |
| supertypes | *(none)* | *(none)* | ok |
| subtypes | *(none)* | *(none)* | ok |
| P/T | — | `None`/`None` | ok |
| oracle_text | as above | byte-identical | ok |

### Code issues
No issues found.

The doc comment already reads `/// This land enters tapped unless you control an Island or a Mountain.`, matching the
current (post-errata) wording — it was aligned along with the other four check lands.

### Rules check
- **"enters tapped unless" is a replacement effect, not a trigger.** Modelled through `replace_event` →
  `helpers::enters_tapped_unless`, which returns `Replacement::Modified(EntersBattlefield { tapped: true, .. })`.
  The CR citation in the file is **614.1d** and that is the right one: 614.1c covers
  *"[This permanent] enters with …"*, *"As [this permanent] enters …"* and *"[This permanent] enters as …"*;
  614.1d covers the plain *"[This permanent] enters …"* form, which is what this card uses.
- **CR 616.1 — the condition is read before the event.** `state.move_object` calls `plan_entering` *above* the
  zone change (`state.rs:704`), so the board is the pre-entry board, and `apply_entering` runs after the move but
  before `EnteredBattlefield` is emitted. There is no window in which the land is observably untapped.
- **"you control".** `controller_has_matching_land` scans `objects_in_zone(Zone::Battlefield, controller)`, not the
  whole battlefield. An opponent's Island does not satisfy it.
- **Both halves.** `has_subtype("Island") || has_subtype("Mountain")` — matches the oracle's two subtypes, and
  `state.has_subtype` reads the object's own subtypes *and* the active face's, so a granted subtype counts.
- **Self-exclusion (`o.id == object_id`).** Correct and not dead code: `create_token` inserts the token into the
  battlefield zone *before* calling `plan_entering` (`state.rs:573-576`), so a token copy of this land would see
  itself. Via `move_object` — every real land drop — the object is not yet in the zone, so the guard never fires.
  Either way the answer is the one CR 616.1 wants: the entering permanent is not yet something you control.
- **Mana.** Two separate `ManaAbilityDef`s producing `{U}` and `{R}`, both `requires_tap: true`, free cost, no side
  effects — "Add {U} or {R}" is one ability with a choice, and modelling it as two selectable abilities gives the
  player exactly that choice with no extra mana.

### Tricky interactions checked
- Opponent controls the only Island/Mountain → land enters tapped: **pass** (now covered for this card).
- Another check land is not an Island or a Mountain → enters tapped: **pass** (`check_lands_do_not_satisfy_each_other`).
- Qualifying land destroyed in response → cannot retroactively tap: **pass** (no trigger exists to respond to;
  `condition_is_evaluated_at_entry_not_later`).
- Tapping the land for mana in response to its own "trigger" for free mana: **impossible** — no trigger goes on the
  stack (`check_land_puts_no_trigger_on_the_stack`).

### Changes made
- `mtg-engine/tests/enters_tapped_replacement.rs` — added the Sulfur Falls row to `AUDITED`, so the two
  audit-gated tests now cover it: `("Sulfur Falls", ["Island", "Mountain"], [ManaType::Blue, ManaType::Red])`.

### Mutation checks (all discriminating)
1. `objects_in_zone(Battlefield, controller)` → `all_objects_in_zone(Battlefield)` →
   `a_check_land_is_not_satisfied_by_an_opponents_land` FAILED.
2. Dropped the `|| has_subtype("Mountain")` half → same test FAILED
   (`Sulfur Falls: your own Mountain does satisfy it`).
3. Second mana ability produces `{U}` instead of `{R}` → `an_audited_check_land_taps_for_both_of_its_colours`
   FAILED (`should add Red; got [[(Blue, 1)], [(Blue, 1)]]`).

### Test coverage
- enters untapped with a qualifying land: `enters_tapped_replacement.rs:47` (all five lands)
- enters tapped without one: `enters_tapped_replacement.rs:60`
- no ETB trigger on the stack: `enters_tapped_replacement.rs:95`
- condition not re-read after entry: `enters_tapped_replacement.rs:117`
- other check lands do not satisfy it: `enters_tapped_replacement.rs:139`
- an opponent's land does not satisfy it: `enters_tapped_replacement.rs:165` (now includes Sulfur Falls)
- taps for both {U} and {R}: `enters_tapped_replacement.rs:192` (now includes Sulfur Falls)

### Suite
`cargo check --workspace --all-targets` clean, zero warnings. Full suite exit 0, 1389 passing.

