## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Counter target spell. Return target permanent to its owner's hand.
**Type line**: Instant
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked

- **Both targets required at cast time**: `TwoTargets(Spell, PermanentWithFilter(Any))` generates a Cartesian product in `engine.rs` (line 986–1004). If either valid-target list is empty, the product is empty and the spell cannot be cast. This correctly implements "You can only cast it if you can choose legal targets for both parts." PASS

- **Partial resolution — spell target becomes illegal**: If the spell target leaves the stack (countered or resolved) before Lost in the Mist resolves, the fizzle check in `stack.rs` (`any_legal`) still finds Target 2 legal and proceeds. In `on_resolve` the counter effect checks `obj.zone == Zone::Stack` (line 53), which is false, so it skips. The bounce still applies. Matches ruling: "If one of Lost in the Mist's targets is illegal by the time it resolves, Lost in the Mist will still affect the remaining legal target." PASS

- **Partial resolution — permanent target becomes illegal**: If the permanent leaves the battlefield before resolution, `any_legal` finds Target 1 (spell still on stack) legal. In `on_resolve` the bounce checks `obj.zone == Zone::Battlefield` (line 64), which is false, so it skips. Counter still applies. PASS

- **Full fizzle — both targets illegal**: `stack.rs` `is_target_legal` checks `Zone::Battlefield || Zone::Stack` (the wildcard case) for both targets via the `TwoTargets` inner_req. If the spell left the stack (zone ≠ Stack, ≠ Battlefield) AND the permanent left the battlefield (zone ≠ Battlefield, ≠ Stack), `any_legal` is false, fizzle occurs before `on_resolve` is called. Matches ruling: "If both targets are illegal at this time, Lost in the Mist won't resolve." PASS

- **`TwoTargets` fizzle check uses req1 for all targets**: `is_target_legal` in `stack.rs` extracts `inner` from `TwoTargets(inner, _)` and applies it to every target (line 19). For Lost in the Mist, req1 = `Spell` which falls through to the wildcard `Zone::Battlefield || Zone::Stack`. This happens to be the correct zone check for both targets (spells on Stack, permanents on Battlefield), so the bug in the general function does not affect this card's behavior. PASS

- **Cannot target itself as the spell**: `valid_targets_for_req` for `TargetRequirement::Spell` filters `id != spell_id` (line 1062), preventing Lost in the Mist from targeting itself. PASS

- **Counter removes from both stack Vec and object zone**: `on_resolve` calls `state.stack.retain(|e| e.as_spell() != Some(*spell_id))` to remove the stack entry, then `state.move_spell_after_resolve(*spell_id)` to change the object's zone. Both are necessary and both happen. PASS

- **Countered flashback spell goes to exile**: `move_spell_after_resolve` checks `cast_with_flashback` on the countered spell's object and sends it to `Zone::Exile` if true (state.rs lines 1132–1141), correctly implementing rule 702.33a ("exile instead of putting it into your graveyard as it resolves or is countered"). PASS

- **Bounce returns to owner's hand, not controller's**: `state.move_object(*perm_id, Zone::Hand)` sets zone to Hand while leaving the object's `owner` field unchanged. The object correctly tracks back to its owner. PASS

- **Token bounced — ceases to exist**: `sba.rs` implements rule 704.5d: tokens not on the battlefield are removed from `state.objects`. After a bounce via `move_object(token, Zone::Hand)`, the next SBA check removes the token. PASS

- **Order of effects (counter then bounce)**: `on_resolve` applies counter first (lines 51–59), then bounce (lines 61–69), matching oracle text order. PASS

- **No summoning sickness or "as long as" continuous effects**: Card has no continuous effects or triggered abilities. PASS

### Test coverage

For each ruling and tricky interaction:

- "You can only cast it if you can choose legal targets for both parts": NOT TESTED (no Lost in the Mist tests exist)
- "If one of Lost in the Mist's targets is illegal by the time it resolves, Lost in the Mist will still affect the remaining legal target" (spell target illegal): NOT TESTED
- "If one of Lost in the Mist's targets is illegal by the time it resolves, Lost in the Mist will still affect the remaining legal target" (permanent target illegal): NOT TESTED
- "If both targets are illegal at this time, Lost in the Mist won't resolve": NOT TESTED (general two-target all-illegal fizzle is tested for Feeling of Dread in `spell_fizzle.rs:265–296`)
- Basic counter + bounce resolving normally: NOT TESTED
- Countered flashback spell goes to exile: NOT TESTED
- Bounce returns to owner's hand (not controller's): NOT TESTED
- Bounce of a token (ceases to exist): NOT TESTED
