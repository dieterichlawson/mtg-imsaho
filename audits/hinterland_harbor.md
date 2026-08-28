## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/241/hinterland-harbor?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
This land enters tapped unless you control a Forest or an Island.
{T}: Add {G} or {U}.
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

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/241/hinterland-harbor?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
This land enters tapped unless you control a Forest or an Island.
{T}: Add {G} or {U}.
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
- The two subtypes it checks are **Forest** and **Island**, matching the fetched
  oracle text exactly — verified per card rather than assumed from the cycle:
  PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Entering tapped or untapped by the condition: `cards_lands_and_mana_sources.rs`, `enters_tapped.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/241/hinterland-harbor?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
This land enters tapped unless you control a Forest or an Island.
{T}: Add {G} or {U}.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE


No rulings are cached for this card and none surfaced.

### Code issues
No behavioural bug. Card data matches exactly — Land with no mana cost, oracle
text verbatim, `has_subtype("Forest") || has_subtype("Island")`, and mana
abilities adding {G} and {U}. Structurally identical to Clifftop Retreat: the
enters-tapped clause is a replacement effect through `replace_event` and
`helpers::enters_tapped_unless` (CR 614.1c/d), and the condition is scoped to
"you control".

**A comment naming an API that has never existed.** All five check lands
carried:

```rust
// "Enters tapped unless ..." is a replacement effect (CR 614.1d),
// declared via `enters_tapped` — not a triggered ability.,
```

There is no `enters_tapped` field or method anywhere in the crate; the
declaration is `replace_event`, which hands the condition to
`helpers::enters_tapped_unless`. Same class as Undead Alchemist's
`replace_combat_damage_to_player`: a comment pointing at a mechanism that
isn't there sends the next reader looking for it.

Fixed in all five. This is a claim about the codebase's own API rather than
about any card's rules text, so correcting it everywhere does not require
having fetched the other three cards' oracle text. (The trailing `.,` typo went
with it. My first replacement quoted the dead name to preserve the history and
would have left five grep hits for a symbol that does not exist — trimmed.)

This card's own doc comment also still used the pre-errata templating,
"Hinterland Harbor enters the battlefield tapped unless…", while its
`oracle_text` field carries the current "This land enters tapped unless…".
Aligned.

### Tests generalised rather than duplicated
The two tests I added for Clifftop Retreat — "an opponent's land does not
satisfy *you control*" and "it taps for both of its colours" — were written
scoped to that one card because it was the only check land whose text I had
fetched. Now that Hinterland Harbor's is fetched too, they are table-driven
over an `AUDITED` list, one row per check land that has actually been audited.
The remaining three get their rows in their own audits.

That keeps the rule the procedure exists to protect: a card is only ever judged
against wording someone has read.

### Tricky interactions checked
- Enters untapped with your Forest, and with your Island: pass
  (`cards_lands_and_mana_sources.rs:94`, which sweeps both companions)
- Enters tapped with neither: pass
- An opponent's Forest or Island does not satisfy it: pass
- Another check land does not satisfy it: pass
- No untapped window before priority; no ETB trigger on the stack: pass
- The condition is read at entry and not re-read: pass
- Taps for {G} or {U}, not two of the same: pass

### Test coverage
- Untapped with either companion / tapped with neither:
  `cards_lands_and_mana_sources.rs:86`, `:94`; `enters_tapped_replacement.rs:48`,
  `:61`
- Two mana abilities exposed: `cards_lands_and_mana_sources.rs:107`
- No untapped window; no stack entry; condition fixed at entry:
  `enters_tapped_replacement.rs:79`, `:93`, `:117`
- **NEW ROW** opponent's land does not satisfy "you control":
  `enters_tapped_replacement.rs::a_check_land_is_not_satisfied_by_an_opponents_land`
- **NEW ROW** taps for both of its colours:
  `enters_tapped_replacement.rs::an_audited_check_land_taps_for_both_of_its_colours`

Mutation-checked against this card: scanning the whole battlefield, dropping
the Island half of the condition, and making both abilities produce green each
fail the test that should catch them.

