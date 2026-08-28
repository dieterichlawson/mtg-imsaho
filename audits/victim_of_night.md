## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/124/victim-of-night?utm_source=api
**Type line**: `Instant` — {B}{B}
**Oracle text**:
```
Destroy target non-Vampire, non-Werewolf, non-Zombie creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "non-Vampire, non-Werewolf, non-Zombie" — all three excluded, and a creature
  with any one of them is not a legal target: PASS
- A Human Werewolf's *front* face is a Werewolf, so it is excluded on both
  faces: PASS
- `has_subtype` covers granted subtypes, so a creature Olivia Voldaren turned
  into a Vampire becomes an illegal target (CR 608.2b): PASS
- `try_destroy`, so indestructible survives: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The three exclusions: `cards_removal.rs`, `subtype.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/124/victim-of-night?utm_source=api
**Type line**: `Instant` — {B}{B}
**Oracle text**:
```
Destroy target non-Vampire, non-Werewolf, non-Zombie creature.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/124/victim-of-night
**Oracle text**: Destroy target non-Vampire, non-Werewolf, non-Zombie creature.
**Type line**: Instant
**Mana cost**: {B}{B}
**Rulings**: none (Scryfall returns no rulings for this card)
**Status**: ISSUE (fixed)

### Card data
Matches the fetched text field for field: name, `{B}{B}` as
`[Colored(Black), Colored(Black)]` (two coloured symbols, not `Generic(2)`),
`card_types: [Instant]`, oracle text verbatim, no P/T, no keywords, no
subtypes, no flashback.

### Code issues

1. `is_valid_target` restated `target_requirement` (`victim_of_night.rs:28-40`, removed).
   - Oracle text says: `Destroy target non-Vampire, non-Werewolf, non-Zombie creature.`
   - `target_requirement` says:
     `CreatureWithFilter(TargetFilter::NotSubtypes(["Vampire", "Werewolf", "Zombie"]))`
   - The override said:
     `!["Vampire", "Werewolf", "Zombie"].iter().any(|st| state.has_subtype(obj.id, st, registry))`
   - `matches_target_filter` says, for that filter (`targeting.rs:476`):
     `let subtypes = state.subtypes_of(obj.id, registry); !types.iter().any(|t| subtypes.contains(t))`
   - Those are the same question. `subtypes_of` is
     `obj.subtypes` then any face subtype not already present (`state.rs:2141`),
     and `has_subtype` is
     `obj.subtypes.iter().any(..) || face_data(..).subtypes.iter().any(..)`
     (`state.rs:2155`) — membership in exactly that union.
   - The preamble `if obj.zone != Zone::Battlefield || !state.is_creature(obj.id, registry) { return false; }`
     is the callers' job: the enumerator reads
     `all_objects_in_zone(Zone::Battlefield)` (`targeting.rs:273`), and
     `stack::is_target_legal` re-checks the zone and creature-ness for
     `CreatureWithFilter` (`stack.rs:77-81`). Second of the six cards the
     Ranger's Guile audit flagged for that preamble; like Rebuke, nothing was
     left underneath, so the whole override went.

2. Removing it would have made `a_cards_target_filter_matches_its_wording` go
   vacuous, and the test was mis-aimed to begin with
   (`characteristics_targeting.rs:85`, reworked).
   - Its two rows called `behavior.is_valid_target(&state, P0, &good, &reg)`
     directly. With the override gone that is the trait default,
     `fn is_valid_target(..) -> bool { true }` — the accept assertion would hold
     for anything and the reject assertion would fail loudly rather than
     silently, but for the accept half it is the same vacuity that bit the
     Urgent Exorcism regression test one card earlier.
   - Reworked to ask `legal_actions`, which is where CR 601.2c actually offers a
     target, so the row tests the card's wording regardless of which layer
     implements it. Confirmed live in both directions: `NotSubtypes => true`
     (an engine-layer mutation the old test could not have seen) now fails it,
     and Tribute to Hunger accepting any player (a card-layer mutation) still
     does.

3. Two of the card's three exclusions were untested anywhere in the suite
   (`cards_removal_and_bounce.rs:155`, two rows added).
   - The removal table gave the card one illegal candidate, `Named("Markov
     Patrician")`, a Vampire. Verified by mutation against the *pre-change*
     card: dropping `"Werewolf"` from the filter produced zero failures across
     the workspace, and so did dropping `"Zombie"`. Only `"Vampire"` was held
     down.
   - Added `Named("Gatstaf Shepherd")` (front face `Creature — Human Werewolf`)
     and `Named("Walking Corpse")` (`Creature — Zombie`). Each of the two
     mutations now fails `targeted_removal_offers_the_targets_its_text_allows`.

### Tricky interactions checked
- A **Vampire token** (Bloodline Keeper's): rejected. Its subtypes are on the
  object, and `subtypes_of` starts from `obj.subtypes`. PASS — covered by the
  reworked wording table.
- A **registry-subtype** Vampire/Werewolf/Zombie cast from hand, whose
  `obj.subtypes` is empty: rejected via the face half of `subtypes_of`. PASS —
  covered by the three removal-table rows.
- A Human that Olivia Voldaren has made a Vampire: rejected, because the
  granted subtype lands on `obj.subtypes` and `subtypes_of` reads it. PASS
  (same accessor as `olivia_makes_a_creature_a_vampire_before_it_dies`).
- A **transformed** werewolf whose back face is `Werewolf` with no Human:
  rejected — `face_data` follows the active face. A DFC that *lost* Werewolf on
  transforming would become legal, which is the same rule. PASS.
- The exclusions are subtypes, not colours or names: a black non-Vampire
  creature is a legal target. PASS — the legal candidate in all three rows is a
  vanilla creature.
- Indestructible: `resolve_destroy` routes to `destruction::try_destroy`, the
  "destroy" pipeline (CR 701.7b). PASS.
- Target changes subtype in response (gains Vampire before resolution):
  countered by game rules (CR 608.2b) — `is_target_legal` re-runs the filter.
  PASS at the mechanism level; the generic case is
  `a_target_that_stops_qualifying_makes_the_spell_fizzle`
  (`resolution_time_checks.rs:267`), no Victim-specific row.
- Self-cleanup: `on_resolve` moves nothing; the engine owns the spell
  (CR 608.2m). PASS.

### UI presentation
`TargetFilter::NotSubtypes` renders as `"non-Vampire/Werewolf/Zombie"`
(`cards/mod.rs:323`). Not the printed wording, but unambiguous, and it is
shared by every `NotSubtypes` card rather than being this card's business.

### Test coverage
- Vanilla creature is offered and destroyed: `cards_removal_and_bounce.rs:155`
  (`targeted_removal_offers_the_targets_its_text_allows`).
- Vampire excluded (registry face): same test, Markov Patrician row.
- Werewolf excluded (registry face): same test, Gatstaf Shepherd row —
  **added this audit**.
- Zombie excluded (registry face): same test, Walking Corpse row —
  **added this audit**.
- Vampire excluded (token, object subtypes): `characteristics_targeting.rs:85`
  (`a_cards_target_filter_matches_its_wording`) — **rewired this audit** to go
  through `legal_actions`.
- Target leaves the battlefield → countered: covered generically in `fizzle.rs`.
- Target stops being a creature → not legal: `fizzle.rs:495`.
- No rulings exist for this card, so there is no per-ruling row to fill.

### Noted, not this card's audit
Tribute to Hunger reaches "target opponent" as `TargetRequirement::PlayerOnly`
plus an `is_valid_target` of `*pid != caster`. That is a real restriction the
requirement does not express, so the override is load-bearing — but it is the
kind of thing that belongs in the requirement (an `OpponentOnly`, or a filter
on `PlayerOnly`) rather than in each card. Left for that card's own audit.

### Mutations run
| mutation | result |
| --- | --- |
| `matches_target_filter`: `NotSubtypes => true` | fails the removal table **and** the reworked wording table (which, before the rework, could not have seen it) |
| card filter: drop `"Vampire"` | fails both tables |
| card filter: drop `"Werewolf"` | fails the removal table (before the new rows: **nothing at all**) |
| card filter: drop `"Zombie"` | fails the removal table (before the new rows: **nothing at all**) |
| `tribute_to_hunger.rs`: `is_valid_target` accepts any player | fails the reworked wording table, confirming that row is still live |

Suite after: 1440 passing, exit 0, zero warnings.

