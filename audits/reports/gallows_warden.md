# Audit: Gallows Warden

## Reference (Scryfall)
- **Name:** Gallows Warden
- **Cost:** {4}{W}
- **Type:** Creature -- Spirit
- **Oracle:** Flying. Other Spirit creatures you control get +0/+1.
- **P/T:** 3/3

## Implementation vs Reference
- Name: CORRECT
- Cost: CORRECT ({4}{W})
- Type: CORRECT (Creature)
- Subtypes: CORRECT (Spirit)
- Oracle text: CORRECT
- P/T: CORRECT (3/3)
- Keywords: CORRECT (Flying)
- +0/+1 to other Spirit creatures you control: CORRECT (ModifyPT power:0, toughness:1, scope: GlobalOther with You + HasSubtype("Spirit"))

## Issues
None found.

## Audit — 2026-04-02

### Oracle Text (Scryfall)
```
Flying
Other Spirit creatures you control get +0/+1.
```

### Card Data Review

| Field          | Oracle / Expected         | Implementation            | Status |
|----------------|---------------------------|---------------------------|--------|
| Name           | Gallows Warden            | `"Gallows Warden"`        | OK     |
| Mana Cost      | {4}{W}                    | `Generic(4), White`       | OK     |
| Type           | Creature — Spirit         | `Creature`, sub `"Spirit"`| OK     |
| P/T            | 3/3                       | `3/3`                     | OK     |
| Keywords       | Flying                    | `Keyword::Flying`         | OK     |
| Oracle Text    | (see above)               | matches                   | OK     |

### Continuous Effect Audit

- **Effect**: `ModifyPT { power: 0, toughness: 1 }` — matches oracle "+0/+1". **OK**
- **Scope**: `GlobalOther(And(You, HasSubtype("Spirit")))` — correctly uses `GlobalOther` to exclude self, filters to Spirits you control. **OK**
- **"Other" exclusion**: `GlobalOther` variant in `state.rs` checks `creature_id != source_id`, so Gallows Warden does not buff itself. **OK**
- **Controller restriction**: `CreatureFilter::You` ensures only your Spirits are buffed, not opponent's. **OK**

### Test Coverage

- `gallows_warden_buffs_other_spirits`: Verifies warden's own toughness stays 3 (no self-buff), Chapel Geist goes from 2/3 to 2/4. **OK**
- `spirit_lord_doesnt_buff_opponent`: Verifies opponent's spirits are unaffected. **OK**

### Verdict

**PASS** — No issues found. Implementation matches oracle text exactly. Continuous effect scope correctly handles "other" exclusion and controller restriction. Test coverage is adequate.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying\nOther Spirit creatures you control get +0/+1.
**Type line**: Creature — Spirit
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 21:03

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Flying\nOther Spirit creatures you control get +0/+1.
**Type line**: Creature — Spirit
**Status**: PASS

### Code issues
None. All card data fields match oracle exactly. Continuous effect correctly implements +0/+1 with `GlobalOther(And(You, HasSubtype("Spirit")))`.

### Tricky interactions checked (min 3)
1. **Self-exclusion**: `GlobalOther` uses `creature_id != source_id` in `state.rs:720`, so Gallows Warden does not buff itself. Confirmed by test (warden stays 3/3).
2. **Opponent's Spirits not buffed**: `CreatureFilter::You` checks `creature.controller == source_controller`. Confirmed by `spirit_lord_doesnt_buff_opponent` test.
3. **Stacking with Battleground Geist**: Both lords use `GlobalOther` with the same filter pattern. Engine aggregates via `continuous_pt_mods`, so a Spirit with both lords gets +1/+1 total. No conflict.
4. **Transformed DFC Spirits**: `HasSubtype` filter in `matches_filter` checks back face subtypes for transformed creatures, so transformed Spirits correctly receive the buff.

### Test coverage
- `gallows_warden_buffs_other_spirits` (tier5_cards.rs): Verifies warden own toughness=3 (no self-buff), Chapel Geist toughness 3->4 and power stays 2.
- `spirit_lord_doesnt_buff_opponent` (tier5_cards.rs): Verifies opponent Chapel Geist power stays 2 (no cross-player buff).
