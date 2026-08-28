## Audit — 2026-08-27 (Tier C)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/26/paraselene?utm_source=api
**Type line**: `Sorcery` — {2}{W}
**Oracle text**:
```
Destroy all enchantments. You gain 1 life for each enchantment destroyed this way.
```
**Status**: PASS

### Code issues
No issues found.

- "Destroy all enchantments" goes through `try_destroy_all`, one event
  (CR 700.2c), so each indestructible check is made against the battlefield as it
  stood before any of them died.
- "You gain 1 life for each enchantment destroyed **this way**" counts only
  `DestroyResult::Died`, so a regenerated or indestructible enchantment does not
  pay out. That distinction is why the card says "this way".

### What else was checked
Card data verified exact set-wide (cost, types, subtypes, supertypes, P/T,
oracle text, keywords, flashback cost, trigger kinds) — see
`ISD_AUDIT_PROGRESS.md`. Step 9 anti-patterns: clean.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/26/paraselene?utm_source=api
**Type line**: `Sorcery` — {2}{W}
**Oracle text**:
```
Destroy all enchantments. You gain 1 life for each enchantment destroyed this way.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "You gain 1 life **for each enchantment destroyed this way**" — only the ones
  that actually died, counted from `DestroyResult::Died`, so an indestructible
  enchantment neither dies nor pays: PASS
- "Destroy **all** enchantments" — both players', and Auras and Curses count: PASS
- `try_destroy_all`, so they die simultaneously: PASS
- The life gain goes through `change_life`: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The count of what actually died: `cards_removal.rs`

## Audit — 2026-08-28 18:31

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Paraselene"`, https://scryfall.com/card/isd/26/paraselene
**Oracle text**:
```
Destroy all enchantments. You gain 1 life for each enchantment destroyed this way.
```
**Type line**: Sorcery
**Mana cost**: {2}{W}
**Rulings**: none on Scryfall for this card.
**Status**: PASS (one accessor made consistent; two test gaps closed)

### Code issues
No behavioural issues. `{2}{W}`, `CardType::Sorcery`, oracle text verbatim, no target
requirement ("all enchantments" targets nothing).

The two things this card has to get right, it does:
- **Simultaneity.** `destruction::try_destroy_all` rather than a loop, so every indestructible
  check is made against the battlefield as it was before any of them died (CR 700.2c).
- **"Destroyed this way."** The life is the count of results that came back `Died`, not the
  count of enchantments found — so an indestructible or regenerated one gives nothing. This is
  the only reason the card reads `try_destroy_all`'s results at all.

**One accessor changed for consistency**, not for a bug: the filter read
`state.face_data(o.id, registry).is_some_and(|d| d.card_types.contains(&Enchantment))`, and
`state.has_card_type(..)` is the accessor every other card asks through — it unions the object's
own types with its active face's, so a token or a granted type counts. Nothing in this set is an
enchantment either of those ways, so behaviour is unchanged.

### Tricky interactions checked
- **All enchantments, including your own**: PASS — no controller filter, and the first test has
  one on each side.
- **Only enchantments**: PASS after the new test; the old one had nothing else on the board.
- **An enchantment that survives gives no life**: PASS.
- **Auras are enchantments**: they are destroyed, and the creature is freed. Covered by the type
  check; not separately tested.
- **Curses are enchantments**: same.
- **Sorcery timing**: engine-side.
- **No enchantments at all**: no life, and a log line saying so.

### Test coverage
- two enchantments, one per side, both destroyed, two life:
  `cards_lands_and_mana_sources.rs:407 paraselene_destroys_enchantments_and_gains_life`
- creatures, lands and artifacts are untouched:
  `cards_lands_and_mana_sources.rs:~430 paraselene_leaves_everything_that_is_not_an_enchantment` (NEW)
- an indestructible enchantment survives and gives no life:
  `cards_lands_and_mana_sources.rs:~455 paraselene_gains_no_life_for_an_enchantment_it_could_not_destroy` (NEW)
- simultaneous destruction as one event: `destruction::try_destroy_all`'s own tests

Mutation-checked: treating every permanent as an enchantment fails the second test; counting the
enchantments found instead of the ones that died fails the third. The original test passed both
mutations — it put two enchantments on an otherwise empty battlefield, so "destroy everything"
looks the same, and neither of them could survive, so "count what you found" looks the same too.

### Changes made
- `paraselene.rs`: the type filter goes through `has_card_type`.
- `cards_lands_and_mana_sources.rs`: two new tests.
