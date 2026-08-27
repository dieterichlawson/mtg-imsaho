## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/215/olivia-voldaren?utm_source=api
**Type line**: `Legendary Creature — Vampire` — {2}{B}{R}, 3/3
**Oracle text**:
```
Flying
{1}{R}: Olivia Voldaren deals 1 damage to another target creature. That creature becomes a Vampire in addition to its other types. Put a +1/+1 counter on Olivia Voldaren.
{3}{B}{B}: Gain control of target Vampire for as long as you control Olivia Voldaren.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "If Olivia Voldaren deals lethal damage to a creature with its first
  activated ability, that creature will become a Vampire before dying." Damage
  is marked and the subtype added inside one resolution; state-based actions run
  afterwards: PASS
- "**another** target creature" — `TargetFilter::Another`: PASS
- The Vampire subtype is an object-level grant, so the second ability recognises
  both printed Vampires and ones Olivia made: PASS
- Ruling: "If you activate Olivia Voldaren's last ability, and before that
  ability resolves you lose control of Olivia Voldaren, the ability will resolve
  with no effect." The control effect's duration is the engine's, keyed on
  "for as long as you control Olivia" — it ends both when she leaves and when
  someone else takes control of her (CR 611.2b): PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The damage/subtype/counter ability: `olivia_voldaren.rs`
- The control duration ends on a control change, not only on leaving: `control_durations.rs`, `control_change.rs`
