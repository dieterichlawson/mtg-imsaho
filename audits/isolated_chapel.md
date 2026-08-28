## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/242/isolated-chapel?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
This land enters tapped unless you control a Plains or a Swamp.
{T}: Add {W} or {B}.
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

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/242/isolated-chapel?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
This land enters tapped unless you control a Plains or a Swamp.
{T}: Add {W} or {B}.
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
- The two subtypes it checks are **Plains** and **Swamp**, matching the fetched
  oracle text exactly — verified per card rather than assumed from the cycle:
  PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Entering tapped or untapped by the condition: `cards_lands_and_mana_sources.rs`, `enters_tapped.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/242/isolated-chapel?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
This land enters tapped unless you control a Plains or a Swamp.
{T}: Add {W} or {B}.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE


No rulings are cached for this card and none surfaced.

### Code issues
No behavioural bug. Card data matches exactly — Land with no mana cost, oracle
text verbatim, `has_subtype("Plains") || has_subtype("Swamp")`, and mana
abilities adding {W} and {B}. The enters-tapped clause is a replacement effect
through `replace_event` and `helpers::enters_tapped_unless` (CR 614.1c/d), and
the condition is scoped to "you control".

**One comment corrected.** The doc comment used the pre-errata templating —
"Isolated Chapel enters the battlefield tapped unless you control a Plains or a
Swamp" — while the card's own `oracle_text` field carries the current wording,
"This land enters tapped unless…". The same drift I found on Hinterland Harbor.
Aligning a doc comment to the file's own data is internal consistency rather
than a rules judgement, so it is safe to do for a card whose text I have
fetched; Sulfur Falls and Woodland Cemetery still have it and get it in their
own audits.

The `enters_tapped` phantom-API comment was already corrected across all five
check lands during the Hinterland Harbor audit.

### Tests
Added this card's row to the `AUDITED` table in
`enters_tapped_replacement.rs`, which is how each check land picks up the two
tests the shared sweeps do not cover:

- an opponent's Plains or Swamp does not satisfy "you control" — every older
  test put the qualifying basic on the land's own side, so a condition that
  scanned the whole battlefield would have passed all of them;
- it taps for {W} *and* {B}, not two of the same, which the shared
  "two mana abilities" count would not catch.

Three rows now: Clifftop Retreat, Hinterland Harbor, Isolated Chapel. Sulfur
Falls and Woodland Cemetery join as they are audited — the table only holds
cards whose oracle text someone has actually read.

### Tricky interactions checked
- Enters untapped with your Plains, and with your Swamp: pass
  (`cards_lands_and_mana_sources.rs:94`, which sweeps both companions)
- Enters tapped with neither: pass
- An opponent's Plains or Swamp does not satisfy it: pass
- Another check land does not satisfy it — none has a basic land subtype: pass
- No untapped window before anyone gets priority: pass
- No ETB trigger on the stack: pass
- The condition is read at entry and not re-read afterwards: pass
- Taps for {W} or {B}: pass

### Test coverage
- Untapped with either companion / tapped with neither:
  `cards_lands_and_mana_sources.rs:86`, `:94`; `enters_tapped_replacement.rs:48`,
  `:61`
- Two mana abilities exposed: `cards_lands_and_mana_sources.rs:107`
- No untapped window; no stack entry; condition fixed at entry; check lands do
  not satisfy each other: `enters_tapped_replacement.rs:79`, `:93`, `:117`,
  `:139`
- **NEW ROW** opponent's land does not satisfy "you control":
  `enters_tapped_replacement.rs::a_check_land_is_not_satisfied_by_an_opponents_land`
- **NEW ROW** taps for both of its colours:
  `enters_tapped_replacement.rs::an_audited_check_land_taps_for_both_of_its_colours`

Mutation-checked against this card: scanning the whole battlefield, dropping
the Swamp half of the condition, and making both abilities produce white each
fail the test that should catch them.

