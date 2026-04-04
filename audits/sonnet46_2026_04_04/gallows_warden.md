## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Flying
Other Spirit creatures you control get +0/+1.
**Type line**: Creature — Spirit
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Self-exclusion ("other")**: `EffectScope::GlobalOther` checks `creature_id != source_id` in `state.rs:720` before applying the filter. Gallows Warden does not buff itself. Confirmed by test `gallows_warden_buffs_other_spirits` (warden toughness stays 3).
- **Controller restriction ("you control")**: `CreatureFilter::You` in `matches_filter` checks `creature.controller == source_controller`. Opponent's Spirits receive no buff. Confirmed by test `spirit_lord_doesnt_buff_opponent`.
- **Spirit token coverage**: `matches_filter` for `HasSubtype` first checks `registry.card_data()` for regular cards, then falls back to `creature.subtypes` (the object-level field) for tokens. Spirit tokens with subtypes stored in `obj.subtypes` are correctly included in the buff.
- **Transformed DFC Spirits**: `matches_filter` for `HasSubtype` checks the back-face subtypes via `behavior.back_face_data()` when `creature.is_transformed` is true. Transformed Spirits correctly receive the buff.
- **Continuous re-evaluation ("as long as" equivalent)**: `continuous_pt_mods` is called at evaluation time (inside `effective_toughness`/`effective_power`), not cached. The effect is always current; if a creature loses the Spirit subtype mid-game, it would stop receiving the buff. Correct behavior.
- **Power unchanged (+0)**: `ModifyPT { power: 0, toughness: 1 }` — power modifier is 0. Spirits get no power boost. Correct per oracle "+0/+1".
- **Stacking with Battleground Geist**: Both lords register independent `ModifyPT` entries in `continuous_pt_mods`. A Spirit controlled by the same player under both lords gets +1/+1 total (Battleground +1/+0, Gallows +0/+1). Correct additive behavior.

### Test coverage
- **Gallows Warden buffs other Spirits (+0/+1), does not buff self**: `tier5_cards.rs:42` — TESTED
- **Opponent's Spirits not buffed**: `tier5_cards.rs:57` (uses Battleground Geist, not Gallows Warden specifically, but same engine path) — TESTED via shared test
- **Spirit tokens receiving the buff**: NOT TESTED
- **Transformed DFC Spirits receiving the buff**: NOT TESTED
- **Flying keyword on Gallows Warden itself**: NOT TESTED (no combat test for the card)
