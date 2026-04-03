## Audit — 2026-04-02 (final)
**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Flying
Other Spirit creatures you control get +1/+0.
**Type line**: Creature — Spirit
**Status**: PASS
### Code issues
No issues found.

## Audit — 2026-04-02 (full-reaudit)

**Oracle text source**: Oracle cache (Scryfall API)
**Status**: PASS

### Code issues
No issues found.

## Audit — 2026-04-02 20:33

**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: Flying
Other Spirit creatures you control get +1/+0.
**Type line**: Creature — Spirit
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- Buff excludes self (GlobalOther scope): pass — test confirms Battleground Geist's own power stays at 3
- Buff does not apply to opponent's Spirits (You filter): pass — test confirms opponent's Chapel Geist stays at 2 power
- Buff does not apply to non-Spirit creatures you control: pass — test confirms generic 2/2 creature stays at 2 power
- Buff stacks with other Spirit lords (e.g., Gallows Warden): pass — engine iterates all battlefield sources independently; each lord's GlobalOther effect is applied separately
- Transformed creatures checked against back face subtypes for Spirit filter: pass — HasSubtype filter in matches_filter checks back_face_data when is_transformed is true

### Test coverage
- Self-exclusion (does not buff itself): `tier5_cards.rs:32`
- Buffs other friendly Spirits (+1/+0): `tier5_cards.rs:34-35`
- Does not buff non-Spirits: `tier5_cards.rs:37`
- Does not buff opponent's Spirits: `tier5_cards.rs:66`
