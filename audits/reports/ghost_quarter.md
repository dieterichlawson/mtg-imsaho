# Audit: Ghost Quarter

## Oracle Reference (Scryfall)
- Cost: (none, land)
- Type: Land
- Oracle: "{T}: Add {C}.
  {T}, Sacrifice Ghost Quarter: Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle."

## Implementation: ghost_quarter.rs

## Issues Found

No issues found. Name, type (Land), oracle text, mana ability, and activated ability all match. The sacrifice ability correctly requires tap, sacrifice self, and targets a land. The "may search" is auto-resolved (always searches), which is a reasonable AI simplification. Basic land search logic correctly checks for CardType::Land + Supertype::Basic.

## Verdict: PASS

---

## Re-audit: 2026-04-02

### Oracle Text (Scryfall, 2026-04-01 cache)
```
{T}: Add {C}.
{T}, Sacrifice this land: Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle.
```

### Findings
- Name, type (Land), cost (none) all match.
- Oracle text in code matches Scryfall oracle.
- Mana ability: {T} adds {C}, checks untapped + battlefield -- correct.
- Activated ability: {T}, sacrifice self, targets any land -- correct.
- On resolution: destroys target land, then auto-searches controller's library for Basic Land -- correct.
- "May search" is auto-resolved (always searches), acceptable AI simplification.

### Verdict: PASS

---

## Audit — 2026-04-02 21:09
**Oracle text source**: Scryfall API (cached 2026-04-01)
**Oracle text**: `{T}: Add {C}.\n{T}, Sacrifice this land: Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle.`
**Type line**: `Land`
**Status**: ISSUE

### Code issues

1. **Missing library shuffle after search (ISSUE)**: After finding and placing the basic land onto the battlefield, the implementation does not shuffle the target land controller's library. The engine supports shuffling (e.g., `ChooseFromLibrary` resolution in engine.rs:2048 calls `.shuffle()`), so this is not an engine limitation -- it is a missing step. The library order is meaningful in the engine, so this affects gameplay correctness.

2. **Log message says "destroyed" even when land survives (minor)**: Line 79 always logs `"Ghost Quarter destroyed {target_name}"` regardless of the return value of `try_destroy`. If the target land is indestructible or regenerated, the log is misleading. The `try_destroy` return value (`DestroyResult`) is discarded.

3. **Oracle text in `card_data` says "Sacrifice Ghost Quarter" but Scryfall oracle says "Sacrifice this land"**: The code's `oracle_text` field reads `"...Sacrifice Ghost Quarter: Destroy target land..."` while the canonical Scryfall oracle text reads `"...Sacrifice this land: Destroy target land..."`. This is a minor text mismatch.

### Tricky interactions checked (min 3)

1. **Indestructible land**: The code calls `try_destroy` but does not check its return value, so the search still proceeds even if the land survives. This correctly matches the ruling: "The target land's controller gets to search for a basic land card even if that land wasn't destroyed by Ghost Quarter's ability."

2. **Self-targeting**: Ghost Quarter can be chosen as a target since it is on the battlefield when targets are selected. However, it is sacrificed as part of activation cost (SacrificeCost::SacrificeThis). When the ability resolves, lines 71-73 check `o.zone == Zone::Battlefield` and the sacrificed Ghost Quarter is no longer there, so `_ => return` fires and the ability does nothing. This correctly matches the ruling: "If you target Ghost Quarter with its own ability, the ability won't resolve because its target is no longer on the battlefield."

3. **Target removed before resolution**: If the target land leaves the battlefield before resolution (e.g., bounced), lines 71-74 return early with no effect. This matches the ruling: "If the targeted land is an illegal target by the time Ghost Quarter's ability resolves, it won't resolve and none of its effects will happen."

4. **Regenerated land**: If the land has regeneration shields, `try_destroy` returns `Regenerated` (land is tapped, damage removed, but stays on battlefield). The search still proceeds since the return value is not checked. Per the ruling, this is correct -- the controller still gets to search.

### Test coverage

- `ghost_quarter_card_data`: Verifies card type is Land and oracle text contains "Destroy target land". PASS.
- `ghost_quarter_taps_for_colorless`: Verifies the mana ability appears in legal actions. PASS.
- **Missing**: No test for the core activated ability (destroy a land + opponent searches for basic land).
- **Missing**: No test for indestructible interaction (search still happens).
- **Missing**: No test for self-targeting fizzle behavior.
