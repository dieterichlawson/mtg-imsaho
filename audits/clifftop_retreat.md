## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/238/clifftop-retreat?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
This land enters tapped unless you control a Mountain or a Plains.
{T}: Add {R} or {W}.
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

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/238/clifftop-retreat?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
This land enters tapped unless you control a Mountain or a Plains.
{T}: Add {R} or {W}.
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
- The two subtypes it checks are **Mountain** and **Plains**, matching the fetched
  oracle text exactly — verified per card rather than assumed from the cycle:
  PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Entering tapped or untapped by the condition: `cards_lands_and_mana_sources.rs`, `enters_tapped.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/238/clifftop-retreat?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
This land enters tapped unless you control a Mountain or a Plains.
{T}: Add {R} or {W}.
```

**Rulings fetched**: none published for this card.

**Status**: PASS


No rulings are cached for this card and none surfaced.

### Code issues
No issues found. Card data matches exactly — Land with no mana cost, oracle
text verbatim, two mana abilities.

- "enters tapped unless…" goes through `replace_event` and
  `helpers::enters_tapped_unless`, i.e. a replacement effect (CR 614.1c/d), not
  a triggered ability. That is the difference between a land that is never
  observable untapped and one that enters untapped and then taps.
- The condition reads **subtypes** — `has_subtype("Mountain") ||
  has_subtype("Plains")` — rather than basic-ness, which is what the text says
  and would still be right in a pool with nonbasic Mountains.
- `objects_in_zone(Battlefield, controller)` scopes it to "you control".

The `o.id == object_id` self-exclusion in the condition is redundant twice
over: the land has not entered the battlefield yet when a replacement effect is
evaluated (CR 616.1), and Clifftop Retreat has no basic land subtype of its own
so it could never satisfy its own condition. Harmless, and I left it.

### What was untested
Coverage for the check lands is otherwise thorough — both companion basics,
the tapped case, no untapped window before priority, no stack entry, the
condition locked in at entry, and the check lands not satisfying each other.
Two things were missing:

- **"you control".** Every existing test put the qualifying basic on the
  Retreat's own side, so a condition that scanned the whole battlefield would
  have passed all of them. The new test puts a Mountain and a Plains under the
  opponent and asserts the Retreat still enters tapped — then repeats with the
  same basics on your own side, so the assertion is about *whose* land it is
  and not about the land being absent.
- **Which mana it makes.** The shared sweep asserts the land exposes two mana
  abilities, which a land exposing "Add {R}" twice would also satisfy. The new
  test pins {R} and {W}.

Both new tests are scoped to Clifftop Retreat rather than swept across all five
check lands, deliberately: it is the only one whose oracle text this audit
fetched, and the procedure is explicit that a card must not be judged against
wording I have not read. The other four get the same treatment in their own
audits.

### Tricky interactions checked
- Enters untapped with your Mountain, and with your Plains: pass
  (`cards_lands_and_mana_sources.rs:94`, which sweeps both companions)
- Enters tapped with neither: pass
- An opponent's Mountain or Plains does not satisfy it: pass
- Another check land does not satisfy it — no basic land subtypes: pass
- No untapped window before anyone gets priority: pass
- No ETB trigger on the stack — it is a replacement, not an ability: pass
- The condition is read at entry and not re-read afterwards: pass
- Taps for {R} or {W}: pass

### Test coverage
- Untapped with either companion / tapped with neither:
  `cards_lands_and_mana_sources.rs:86`, `:94`; `enters_tapped_replacement.rs:48`,
  `:61`
- Two mana abilities exposed: `cards_lands_and_mana_sources.rs:107`
- No untapped window; no stack entry; condition fixed at entry; check lands do
  not satisfy each other: `enters_tapped_replacement.rs:79`, `:93`, `:117`,
  `:139`
- **NEW** an opponent's land does not satisfy "you control":
  `enters_tapped_replacement.rs:152`
- **NEW** it taps for {R} or {W}, not two of the same:
  `enters_tapped_replacement.rs:184`

Mutation-checked: scanning the whole battlefield instead of your own, dropping
the Plains half of the condition, and making both abilities produce red each
fail the test that should catch them.

