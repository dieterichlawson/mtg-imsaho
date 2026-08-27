## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/4/avacynian-priest?utm_source=api
**Type line**: `Creature — Human Cleric` — {1}{W}, 1/2
**Oracle text**:
```
{1}, {T}: Tap target non-Human creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "target **non-Human** creature" — `is_valid_target` excludes Humans, checking
  `state.has_subtype` so a token or a granted Human subtype counts too: PASS
- A creature that *becomes* a Human in response is no longer a legal target
  (CR 608.2b). The ability arm of `resolve_top_of_stack` checked only whether
  the target could still be targeted at all, not whether it still satisfied the
  card. Fixed by also consulting the granting behavior's `is_valid_target`.
- Tapping is the effect, not the cost — the Priest's own {T} is the cost: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Tapping a non-Human, and the ability not being payable twice: `activated_abilities.rs:avacynian_priest_taps_a_non_human_and_then_cannot_be_paid_again`
