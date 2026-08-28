## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/18/ghostly-possession?utm_source=api
**Type line**: `Enchantment — Aura` — {2}{W}
**Oracle text**:
```
Enchant creature
Enchanted creature has flying.
Prevent all combat damage that would be dealt to and dealt by enchanted creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Prevent all combat damage that would be dealt **to and dealt by** enchanted
  creature" — both directions, and combat damage only, so a Geistflame still
  kills it: PASS
- Prevention, not a P/T change, so the creature still deals its damage for
  purposes of "deals damage" triggers being *prevented*: PASS
- Flying is granted, so it can block fliers it otherwise could not: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Damage prevention in both directions: `enchantments.rs`, `combat_rules.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/18/ghostly-possession?utm_source=api
**Type line**: `Enchantment — Aura` — {2}{W}
**Oracle text**:
```
Enchant creature
Enchanted creature has flying.
Prevent all combat damage that would be dealt to and dealt by enchanted creature.
```

**Rulings fetched**: none published for this card.

**Status**: PASS

### Code issues

No issues found. The card needed no change; the whole of this audit is
coverage.

### Card data

`{2}{W}` Enchantment — Aura, `TargetRequirement::Creature` for "Enchant
creature", and the two static abilities as two continuous effects:

- `GrantKeyword { keyword: Flying, scope: Attached }` for "Enchanted creature
  has flying"
- `PreventCombatDamage { scope: Attached }` for "Prevent all combat damage
  that would be dealt to and dealt by enchanted creature"

Cost, type line and subtypes pinned pool-wide by `card_data_invariants.rs`;
`Enchant` is one of the keywords the keyword invariant deliberately does not
model. `resolve_aura` for attachment, no `is_valid_target` override (the card
restricts nothing beyond "creature", which CR 608.2b now re-checks at the
engine level), and no card-side cleanup.

`EffectScope::Attached` on both is right: the effects are the Aura's and apply
to what it is attached to. Both directions of the prevention fall out of the
damage pipeline asking `has_combat_damage_prevention` of the source *and* the
target.

### Tricky interactions checked

- Damage to the enchanted creature: pass.
- Damage by the enchanted creature to another creature: pass.
- Damage by the enchanted creature **to a player**: pass, and a separate path
  in the pipeline (`deal_damage_to_player`) that no test reached. For a card
  whose point is that an enchanted attacker gets through and does nothing,
  this was the gap that mattered.
- **Combat** damage only — a Geistflame or a fight still lands: pass, and
  untested. The word is doing work: `deal_damage_to_object` consults the
  prevention only when `kind == DamageKind::Combat`.
- The prevention ends when the Aura leaves the battlefield: pass.
- Flying granted: pass.
- CR 704.5m, an Aura attached to nothing goes to the graveyard: handled in
  `sba.rs`, and not this card's to implement.
- CR 303.4h, an Aura that cannot legally enchant its target: the target
  requirement is bare "creature" with no further restriction, so there is no
  case here beyond the engine's own re-check.

### Test coverage

- prevents damage to and from the creature (creature vs creature):
  `cards_morbid_and_ltb.rs::ghostly_possession_prevents_damage`
- prevents the creature's damage to a player:
  `cards_morbid_and_ltb.rs::ghostly_possession_prevents_the_creatures_damage_to_a_player` (new)
- does not prevent noncombat damage:
  `cards_morbid_and_ltb.rs::ghostly_possession_does_not_prevent_noncombat_damage` (new)
- the prevention ends with the Aura:
  `cards_morbid_and_ltb.rs::ghostly_possessions_prevention_ends_with_the_aura` (new)
- grants flying: `cards_vanilla_and_keywords.rs::ghostly_possession_grants_flying`

### Mutations run

- `deal_damage_to_player` drops its `has_combat_damage_prevention(source)`
  check: **fails** the new player test, and nothing else — which is the hole
  that test was written for.
- The prevention applies to noncombat damage as well: **fails** the new
  noncombat test, and nothing else.
- The card's prevention scope `Attached` → `OnSelf`: **fails** two tests.
- The card grants Vigilance instead of Flying: **fails** the flying test.
  (Deleting the grant outright did not compile — it left `Keyword` unused —
  and proved nothing; redone as a substitution.)

Suite: 1534 passing, exit 0, `cargo check --workspace --all-targets` clean.
