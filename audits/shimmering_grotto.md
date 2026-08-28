## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/246/shimmering-grotto?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
{T}: Add {C}.
{1}, {T}: Add one mana of any color.
```
**Status**: PASS

### Code issues
No issues found.

- Both abilities are mana abilities under CR 605.1a, so both are visible to the
  auto-tap planner; the colored one carries its `{1}` in `ManaAbilityDef::cost`
  rather than being hidden in `activated_abilities`.
- "one mana of any color" is five entries, one per color, so the player picks a
  colour by picking an ability.

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/246/shimmering-grotto?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
{T}: Add {C}.
{1}, {T}: Add one mana of any color.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Two separate mana abilities: "{T}: Add {C}" and "{1}, {T}: Add one mana of any
  colour" — the second costs mana as well as the tap, so it is a filter rather
  than a ramp: PASS
- Both use the tap, so only one can be activated: PASS
- "one mana of **any color**" presents a colour choice rather than assuming one:
  PASS
- A mana ability does not use the stack (CR 605.1a): PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both abilities and the colour choice: `mana_ability_offers.rs`, `cards_lands_and_mana_sources.rs`

## Audit — 2026-08-28 18:34

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Shimmering Grotto"`, https://scryfall.com/card/isd/246/shimmering-grotto
**Oracle text**:
```
{T}: Add {C}.
{1}, {T}: Add one mana of any color.
```
**Type line**: Land
**Mana cost**: none
**Rulings**: none on Scryfall for this card.
**Status**: PASS (one silent-default removed; one rule found untested)

### Code issues
No behavioural issues. `CardType::Land`, no mana cost, oracle text verbatim.

Both abilities live in `mana_abilities`, which is right and load-bearing: a filter is a mana
ability under CR 605.1a — an activated ability with no target that could add mana — and only
mana abilities are visible to the auto-tap planner. The card's own doc records what happened
when it was exposed through `activated_abilities` instead: a hand needing the Grotto for its one
green source produced no `CastSpell` action at all.

"One mana of any color" is five `ManaAbilityDef` entries rather than a choice prompt, indexed
1..=5 so the indices match what a player is shown, each costing `{1}` and each `requires_tap`.

**One silent default removed.** The colour label was
`ManaType::Red => "R", _ => "G"` — so any mana type other than the four named would have been
labelled green. The array two lines above holds exactly the five colours, so it was unreachable;
it is spelled out now, because a `_` arm that gives a wrong answer quietly is the shape of half
the bugs this audit pass has found.

### Tricky interactions checked
- **The filter is net zero, not ramp**: PASS — two Plains and a Grotto is three mana, and
  `{2}{G}` stays out of reach.
- **The Grotto cannot filter its own mana**: PASS. Both abilities need `{T}`, so with an empty
  pool only the free `{C}` one is offered.
- **The planner sees the coloured abilities**: PASS, and this is the bug the file exists for.
- **The `{1}` is really spent and the colour really produced, for all five colours**: PASS.
- **A mana ability does not use the stack (CR 605.3a)**: PASS, and it was untested — see below.
- **Tapping for `{C}` costs nothing**: PASS.
- **It enters untapped and is an ordinary land drop**: engine-side.

### Test coverage
`mana_filters.rs` is this card's file and covers it thoroughly:
- the planner funds `{2}{G}` through it: `:31 grotto_color_ability_funds_spell_in_tap_plan`
- and cannot without it: `:52 without_the_grotto_there_is_no_green`
- a filter does not ramp: `:66 a_filter_does_not_add_mana`
- the free `{C}`, and the colours gated on paying `{1}`: `:109 the_grotto_still_taps_for_colorless_for_free`
- every colour spends and produces: `:137 activating_the_filter_spends_and_produces`
- the solver's plans really pay: `:164 every_tap_plan_the_solver_returns_actually_pays_the_cost`
- **NEW** — it does not use the stack: `:~190 a_mana_ability_does_not_use_the_stack`

**CR 605.3a had no test anywhere.** `activated_no_stack.rs` is the file for the opposite rule —
every *ordinary* activated ability goes on the stack first — and nothing asserted the exception.
The Grotto is where it shows: the set's only mana ability with a cost, so there is a moment
(the `{1}` paid, the colour not yet added) that a stack would make visible. Mutation-checked by
pushing the ability onto the stack, which fails the new test.

### Changes made
- `shimmering_grotto.rs`: the colour label match is exhaustive. No behavioural change.
- `mana_filters.rs`: the CR 605.3a test.
