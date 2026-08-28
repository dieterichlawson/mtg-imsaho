## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/234/travelers-amulet?utm_source=api
**Type line**: `Artifact` — {1}
**Oracle text**:
```
{1}, Sacrifice this artifact: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Search your library for a basic land card" — a Basic supertype *and* the Land
  card type, so a nonbasic land is not offered: PASS
- Every basic in the library is offered, not the first one found — a B/R deck
  splashing green must be able to tutor the Forest specifically: PASS
- "...then shuffle": PASS
- Sacrificing the Amulet is a cost, paid on activation: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Every basic offered: `auto_pick.rs:travelers_amulet_offers_every_basic_land_in_the_library`
- The library is shuffled after the search: `auto_pick.rs:bug_bf_travelers_amulet_shuffles_library_after_search`
- The basic reaches hand: `cards_equipment_and_artifacts.rs:travelers_amulet_finds_basic_land`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/234/travelers-amulet?utm_source=api
**Type line**: `Artifact` — {1}
**Oracle text**:
```
{1}, Sacrifice this artifact: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/234/travelers-amulet
**Oracle text**: {1}, Sacrifice this artifact: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.
**Type line**: Artifact
**Mana cost**: {1}
**Rulings**: none (Scryfall returns no rulings for this card)
**Status**: ISSUE (fixed) — the card code is correct; what counts as a find was never a testable claim.

### Card data
Matches the fetched text: `{1}`, `card_types: [Artifact]`, oracle text verbatim
in the current "Sacrifice this artifact" errata wording, no subtypes, no P/T.
The ability is `{1}` plus `SacrificeCost::SacrificeThis` — the whole printed
cost — with `requires_tap: false`, which is right: the card has no tap symbol.

The search goes through `helpers::search_library` with `optional: false`. That
is the correct reading: "Search your library" is mandatory, and the helper's own
comment records why it still asks with exactly one candidate — CR 701.19b, a
player searching a hidden zone "isn't required to find some or all of those
cards even if they're present".

### Code issues

No issue in `travelers_amulet.rs`. One in its tests.

1. **"a *basic land* card" was never a claim the test could fail**
   (`auto_pick.rs:270`, test extended).
   - Oracle text says: `Search your library for a basic land card`
   - Code says:
     `state.has_card_type(id, CardType::Land, registry) && state.face_data(id, registry).is_some_and(|d| d.supertypes.contains(&Supertype::Basic))`
   - `travelers_amulet_offers_every_basic_land_in_the_library` stocked the
     library with a Forest and a Swamp and nothing else. Two mutations produced
     zero failures across the whole workspace:
     - dropping the `Basic` supertype check — search for **any land**;
     - returning `true` for everything — search for **any card**.
   - A Ghost Quarter (a land, not a basic one) and a Grizzly Bears now sit in
     the library alongside the basics, and neither may be offered. Both
     mutations now fail.

### Tricky interactions checked
- Every basic in the library is offered, not just the first: PASS —
  `travelers_amulet_offers_every_basic_land_in_the_library`, which is the Bug
  76-003 regression (the card used to `find()` the first match and take it).
- A nonbasic land is not a legal find: PASS — new assertion.
- A non-land card is not a legal find: PASS — new assertion.
- Goes to **hand**, not the battlefield: PASS — the destination is asserted, and
  `Zone::Battlefield` fails three tests.
- The library is shuffled afterwards: PASS —
  `bug_bf_travelers_amulet_shuffles_library_after_search`, over 20 cards.
- The shuffle happens even when nothing is found (CR 701.19a): the helper's
  job, and its doc says so; not re-tested per card.
- CR 701.19b — the player may decline even with one candidate: the helper asks
  rather than auto-picking, and `bug_bf_...` answers that prompt explicitly
  with the comment "the search stops and asks, even with one candidate".
- **Your** library: `ability_controller` (CR 602.2a) rather than the source's
  current controller, so the search follows the activator even though the
  Amulet is already in the graveyard by then (its own cost sacrificed it).
- The sacrifice is a cost, paid on activation: `SacrificeCost::SacrificeThis`,
  which `engine/actions/abilities.rs` pays before the ability goes on the
  stack. The card's comment ("The artifact was already sacrificed by the
  engine") records this. Note this is the same cost path that
  `activate_via_hooks` was not paying until the Full Moon's Rise audit two
  cards ago — these tests go through that helper, so they only became honest
  about the cost then.
- Empty library / no basics: the helper's zero-candidate branch shuffles and
  moves on. Not re-tested per card.
- Self-cleanup: none — the Amulet is removed by its own cost, not by the
  resolution.

### UI presentation
Ability description: "{1}, Sacrifice: Search library for a basic land, put it
into your hand". The prompt reads "Traveler's Amulet: choose a basic land card",
naming the source.

### Test coverage
- Every basic offered, none auto-picked: `auto_pick.rs:270`
  (`travelers_amulet_offers_every_basic_land_in_the_library`).
- A nonbasic land and a non-land are not offered: same test —
  **added this audit**.
- Found card goes to hand: same test.
- The library is shuffled after the search: `auto_pick.rs:600`
  (`bug_bf_travelers_amulet_shuffles_library_after_search`).
- No rulings exist for this card, so there is no per-ruling row to fill.

### Mutations run
| mutation | result |
| --- | --- |
| drop the `Basic` supertype check (any land) | fails the extended test (before: **nothing at all**) |
| `is_basic_land` returns `true` (any card) | fails the extended test (before: **nothing at all**) |
| destination `Zone::Battlefield` instead of `Zone::Hand` | fails three tests |

Suite after: 1460 passing, exit 0, zero warnings.

