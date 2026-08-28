## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/79/spectral-flight?utm_source=api
**Type line**: `Enchantment — Aura` — {1}{U}
**Oracle text**:
```
Enchant creature
Enchanted creature gets +2/+2 and has flying.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- +2/+2 and flying from one Aura, both scoped `Attached` so both end together:
  PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The pump and the keyword: `enchantments.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/79/spectral-flight?utm_source=api
**Type line**: `Enchantment — Aura` — {1}{U}
**Oracle text**:
```
Enchant creature
Enchanted creature gets +2/+2 and has flying.
```

**Rulings fetched**: none published for this card.

**Status**: PASS

### Code issues

No issues found. The card needed no change.

### Card data

`{1}{U}` Enchantment — Aura, `TargetRequirement::Creature` for "Enchant
creature", and the one sentence as two continuous effects:

- `ModifyPT { power: 2, toughness: 2, scope: Attached }`
- `GrantKeyword { keyword: Flying, scope: Attached }`

Cost, type line and subtypes pinned pool-wide by `card_data_invariants.rs`;
`Enchant` is one of the keywords the keyword invariant deliberately does not
model. `resolve_aura` for attachment, no `is_valid_target` override (the card
restricts nothing beyond "creature", which CR 608.2b re-checks at the engine
level), and no card-side cleanup. Both effects `Attached` rather than `OnSelf`
or `Global`, which is what "enchanted creature" means.

### Tricky interactions checked

- +2/+2 and flying together: pass.
- Both end when the Aura leaves the battlefield: pass, and untested until now.
  The suite covers the other direction thoroughly — an Aura falling off a
  creature that died, CR 704.5m, twice over in `enchantments.rs` — but not
  what the creature is left with when the Aura is the one that goes.
- CR 704.5m itself: engine-level, covered by `enchantments.rs` with Holy
  Strength, and not this card's to implement.
- Fizzle when the target leaves in response: `fizzle.rs` covers the Aura shape
  with Pacifism, and notes that an Aura in particular has somewhere else it
  could wrongly end up.
- Layers: flying is 6, +2/+2 is 7c, and a modifier is added to whatever base
  the creature has — the Curse of Death's Hold audit pinned the 7a/7c pairing
  on the other side of the same arithmetic.
- Enchanting an opponent's creature: "Enchant creature" is unrestricted, so
  there is nothing here to get wrong.

### Test coverage

- +2/+2 and flying, and both ending with the Aura:
  `cards_vanilla_and_keywords.rs::spectral_flight_gives_plus_two_and_flying`
  (extended)
- the keyword accessor resolving an aura grant: `keywords.rs::aura_grants_keyword`

### Mutations run

- `toughness: 2` → `0`: **fails**.
- `scope: Attached` → `OnSelf` on the P/T effect: **fails**.
- `walk_effects` drops its `source.zone != Zone::Battlefield` gate: **passes**.
  As with Curse of Death's Hold, two independent guards hold this up —
  `move_object` also clears `attached_to`, so `EffectScope::Attached` matches
  nothing once the Aura has left. Removing **both** fails the new assertions
  (4 vs 2). Recorded as measured: the assertions are real, and neither
  mechanism is isolated by them.

### Note, not a finding

`keywords.rs::aura_grants_keyword` and
`cards_vanilla_and_keywords.rs::spectral_flight_gives_plus_two_and_flying`
assert nearly the same thing about this card, as do
`enchantments.rs::aura_falls_off_when_creature_dies` and
`…::aura_goes_to_graveyard_when_creature_dies` about Holy Strength. Each pair
has a defensible split — the accessor versus the card, and two different ways
of building the board — and none of it is wrong, so I have left it. Noted so
the next reader knows it was seen rather than missed.
