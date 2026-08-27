## Audit — 2026-08-27 (Tier C — one behaviour hook: replacement effect)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/121/unbreathing-horde?utm_source=api
**Type line**: `Creature — Zombie` — {2}{B}, 0/0
**Oracle text**:
```
This creature enters with a +1/+1 counter on it for each other Zombie you control and each Zombie card in your graveyard.
If this creature would be dealt damage, prevent that damage and remove a +1/+1 counter from it.
```
**Status**: PASS

### Code issues
No issues found.

### What was checked
Card data was verified exact set-wide (see `ISD_AUDIT_PROGRESS.md`). This card's
one hook is `replace_event`, so the audit centres on CR 614 — whether the effect
applies to the right events, exactly once, and modifies rather than replaces
where the oracle says "instead".

- The enters-with count splits the oracle's two halves exactly where CR 109.1
  does: "each other **Zombie** you control" counts tokens, "each Zombie **card**
  in your graveyard" must not, and the code filters the graveyard half through
  `state.is_card`. Getting this backwards is the obvious mistake and it does not
  make it.
- The Horde counting *itself* when it enters from a graveyard is correct per the
  Scryfall ruling, and falls out of the callback running before the zone change.
- The second oracle clause, "If this creature would be dealt damage, prevent
  that damage and remove a +1/+1 counter from it", is not in `replace_event` at
  all — it is a declarative `ContinuousEffect::PreventDamageRemoveCounter` with
  `EffectScope::OnSelf`, which is the right shape for a static replacement.

### Test coverage
`damage_pipeline.rs` (the prevent-and-remove-counter path), `cards_complex_creatures.rs` (enters-with count), `token_is_not_a_card.rs` (the CR 109.1 split)
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/121/unbreathing-horde?utm_source=api
**Type line**: `Creature — Zombie` — {2}{B}, 0/0
**Oracle text**:
```
This creature enters with a +1/+1 counter on it for each other Zombie you control and each Zombie card in your graveyard.
If this creature would be dealt damage, prevent that damage and remove a +1/+1 counter from it.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "If Unbreathing Horde enters **from a graveyard**, it will count
  itself." The counter calculation runs in `replace_event` — *before* the zone
  change (CR 616.1) — so the Horde is still in the graveyard and is counted:
  PASS
- "each **other** Zombie you control" counts tokens (it says Zombies, not
  cards); "each Zombie **card** in your graveyard" does not (CR 109.1). The two
  halves are filtered differently, which is the whole subtlety: PASS
- Ruling: "**Only one** +1/+1 counter will be removed, no matter how much damage
  is prevented": PASS
- Ruling: "If Unbreathing Horde has **no** +1/+1 counters on it (but its
  toughness is raised above 0 by another effect), any damage dealt to it will
  **still be prevented**, even though no counter will be removed." The
  prevention returns true whenever the effect is present, whether or not a
  counter came off: PASS
- "enters with ... counters" is a replacement effect (CR 614.1c), so the Horde
  never exists as a 0/0 that state-based actions could kill: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The entering count from either zone, and the prevention: `cards_complex_creatures.rs`, `token_is_not_a_card.rs:zombie_token_in_graveyard_not_counted`, `:zombie_card_in_graveyard_still_counted`, `:zombie_token_on_the_battlefield_is_still_counted`
