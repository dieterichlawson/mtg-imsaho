## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/239/gavony-township?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
{T}: Add {C}.
{2}{G}{W}, {T}: Put a +1/+1 counter on each creature you control.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "+1/+1 **counter** on each creature you control" — counters, not a continuous
  effect, so CR 611.2c's snapshot rule does not apply and they persist past end
  of turn: PASS
- "each creature **you control**" — no targeting, so it cannot be responded to
  by making a creature untargetable: PASS
- The set of creatures is read when the ability resolves: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Counters on each creature: `cards_activated_abilities.rs`
- The {T} cost's legality: `tap_cost_legality.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/239/gavony-township?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
{T}: Add {C}.
{2}{G}{W}, {T}: Put a +1/+1 counter on each creature you control.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/239/gavony-township
**Oracle text**:
```
{T}: Add {C}.
{2}{G}{W}, {T}: Put a +1/+1 counter on each creature you control.
```
**Type line**: `Land`
**Mana cost**: none
**Rulings**: none published for this card.
**Status**: ISSUE (fixed) — a redundant tap/zone guard in `activated_abilities`, shared with nine other cards.

### Card data
| field | oracle | `gavony_township.rs` | |
|---|---|---|---|
| name | Gavony Township | `"Gavony Township"` | ok |
| cost | *(none)* | `None` | ok |
| types | Land | `vec![CardType::Land]` | ok |
| supertypes / subtypes | *(none)* | *(none)* | ok |
| oracle_text | as above | byte-identical | ok |
| mana ability | `{T}: Add {C}` | `Colorless`, `requires_tap: true`, free | ok |
| activated ability | `{2}{G}{W}, {T}` | `Generic(2) + Green + White`, `requires_tap: true` | ok |
| targeting | "each creature you control" — untargeted | `target_requirement: None` | ok |
| timing | no restriction | `sorcery_speed_only: false` | ok |

### Code issues

**A redundant guard re-deciding an engine rule.** Fixed, and in the nine other cards carrying it.

- Code was: `if obj.zone == Zone::Battlefield && !obj.tapped {` at the top of `activated_abilities`, with the
  whole ability wrapped in the `if` and `vec![]` in the `else`.
- `engine/legal/abilities.rs` already iterates `state.objects_in_zone(Zone::Battlefield, player)` and, at
  `if ab.requires_tap { if obj_tapped { continue; } ... }`, rejects a tapped source — and additionally applies
  summoning sickness with the haste exception (CR 302.6), which the card-level guard omitted.

Two things are wrong with the copy beyond duplication:

1. It is written per **card**, not per **ability**. Every one of the ten wraps its entire ability list, so the
   first of them to gain a second ability without `{T}` in its cost would have had it silently hidden while the
   permanent was tapped.
2. It is strictly weaker than the check it shadows. On Tree of Redemption — a *creature* with `{T}:` — the
   card-level version omits summoning sickness entirely.

The module header of `tap_cost_legality.rs` records exactly this cleanup having been done for `mana_abilities`,
where two of twenty-odd cards spelled out the summoning-sickness half and both forgot haste. The
`activated_abilities` half outlived it.

**Ten cards carried it**: Cellar Door, Gavony Township, Ghost Quarter, Ghoulcaller's Bell, Graveyard Shovel,
Kessig Wolf Run, Moorland Haunt, Nephalia Drownyard, Stensia Bloodhall, Tree of Redemption. (Two of them,
Graveyard Shovel and Moorland Haunt, wrote the same test inside out as an early return.) Each has exactly one
non-mana activated ability and all ten are `requires_tap: true`, which is what makes the removal provably
behavior-preserving rather than merely plausible. Their real conditions — Graveyard Shovel's non-empty
graveyard, Moorland Haunt's creature card in the graveyard, Skirsdag High Priest's morbid — are untouched.

### Rules check
- **CR 602.2h** (one tap pays one cost): already handled, and specifically for this card —
  `legal/abilities.rs` excludes the source from its own autotap pool when the ability `requires_tap`, so the
  Township's `{T}: Add {C}` cannot fund its own `{2}{G}{W}`. Tested at `tap_cost_legality.rs:185`.
- **"each creature you control"** is not a targeted effect: `target_requirement: None`, and resolution sweeps
  `objects_in_zone(Battlefield, controller)`. An opponent's creatures get nothing.
- **CR 602.2a / 608.2g**: resolution uses `helpers::ability_controller`, which reads
  `state.resolving_ability_activator` — the player who activated it — rather than the source's current
  controller. Right answer if the land is destroyed in response (CR 400.7 would otherwise reset `controller`
  to `owner`).
- **CR 302.6**: irrelevant to a land, but the engine applies it and the card should not be the place that
  decides. Said so in the comment left behind.

### Changes made
- The ten card files above: guard removed, replaced by a comment naming the rule and where it lives.
  `moorland_haunt.rs` also needed `controller` re-derived, now via `helpers::controller_of` (last known
  information, CR 608.2g) rather than the deleted binding's `obj.controller`.
- `mtg-engine/tests/tap_cost_legality.rs` — `a_tapped_permanent_offers_none_of_its_tap_abilities`, a table over
  all ten. Nothing asserted the engine's half for these cards on its own, so deleting the copies without this
  would have left the rule untested for them. Each row is checked untapped first, so "not offered" is about the
  tap and not an unpayable cost or a missing target.
- `mtg-engine/tests/card_data_invariants.rs` —
  `no_card_re_decides_the_tap_cost_rules_in_activated_abilities`, so it cannot creep back.

**A correction during the work.** The first cut of the static guard flagged Skirsdag High Priest, whose cost is
"Tap two untapped creatures you control" and which filters on `!o.tapped` over *other* creatures — legitimate,
and not this pattern. The second cut keyed on `object_id` appearing on the line, which flagged it again, because
its filter names `object_id` precisely to exclude the source. The guard now matches only `obj.tapped`, the
binding all ten copies used, and its doc comment says plainly that it is a tripwire for the shape that existed
and not a proof that no card could express the idea another way.

### Mutation checks (all discriminating)
1. Removed `if obj_tapped { continue; }` from `legal/abilities.rs` →
   `a_tapped_permanent_offers_none_of_its_tap_abilities` FAILED.
2. Reintroduced the gate on Gavony Township →
   `no_card_re_decides_the_tap_cost_rules_in_activated_abilities` FAILED.
3. Resolution sweeping `all_objects_in_zone(Battlefield)` instead of the controller's →
   `gavony_township_counters_all_creatures` FAILED.

### Tricky interactions checked
- Opponent's creatures get no counter: **pass** (`gavony_township_counters_all_creatures`).
- The land's own `{T}: Add {C}` cannot pay toward its `{2}{G}{W}`: **pass** (`tap_cost_legality.rs:185`, and the
  four-utility-land table below it).
- Tapped → ability not offered: **pass** (new).
- Land destroyed in response → the ability still resolves for its activator: covered by
  `helpers::ability_controller`; not separately tested for this card, and not newly at risk from this change.

### Test coverage
- counters on each creature you control, and not the opponent's: `cards_activated_abilities.rs:281`
- cannot fund its own tap ability (CR 602.2h): `tap_cost_legality.rs:185`, `tap_cost_legality.rs:214`
- tapped source offers no tap ability: `tap_cost_legality.rs:190` (new, all ten cards)
- no card re-decides the rule: `card_data_invariants.rs` (new)

### Suite
`cargo check --workspace --all-targets` clean, zero warnings. Full suite exit 0, 1393 passing.

