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
