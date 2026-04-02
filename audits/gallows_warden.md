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
