---
id: mirror_mad_phantasm-01
status: closed-duplicate
card: Mirror-Mad Phantasm
card_file: mtg-engine/src/cards/isd/mirror_mad_phantasm.rs
created: 2026-04-15T03:36:03Z
audit_run_id: 2026-04-14-mirror_mad_phantasm-audit
audit_model: opus
audit_tokens: 26524
audit_duration: 621
duplicate_of: merged-activated-no-stack-02
---

## Audit Finding

**Oracle text:**
> {1}{U}: This creature's owner shuffles it into their library. If that player does, they reveal cards from the top of that library until a card named Mirror-Mad Phantasm is revealed. The player puts that card onto the battlefield and all other cards revealed this way into their graveyard.

**Code:**
> engine.rs:2716-2719: `} else { // Non-X ability: fire the effect immediately. if let Some(behavior) = registry.get(behavior_card_id) { behavior.on_activate_ability(&mut new_state, *object_id, *ability_index, targets, registry); } }`

**Description:**
Per CR 602.2, activated abilities use the stack: the player announces the ability and pays costs, the ability is placed on the stack, all players receive priority, and the ability resolves only when it's the topmost stack object and all players pass. In this engine, all activated abilities resolve immediately via a direct call to `on_activate_ability` — no stack entry is created, no priority is granted, and opponents cannot respond. For Mirror-Mad Phantasm, this means: (1) opponents cannot counter the ability (e.g., Stifle, Disallow); (2) opponents cannot remove the Phantasm from the battlefield between ability activation and resolution, which would cause the "If that player does" conditional to fail (the creature is already in another zone and cannot be shuffled into the library); (3) no other triggered abilities that would fire in response to the ability being put on the stack can interleave. The "If that player does" conditional in the oracle text exists precisely because the creature can leave the battlefield while the ability is on the stack — but with immediate resolution, this conditional is dead code.

**Engine path:**
- engine.rs:2716-2719 (direct `on_activate_ability` call for non-X abilities)
- engine.rs:2559-2725 (entire `Action::ActivateAbility` handler — no `StackEntry` created)
- state.rs:10-14 (`StackEntry` enum has no `ActivatedAbility` variant)

**Required check:** 8c

**Affected cards:**
- Mirror-Mad Phantasm
- All cards with activated abilities in this engine

## Tests

### activated_ability_opponent_response
Source ticket: (new)
Implementation: (not yet written)
Scenario: Player A controls Mirror-Mad Phantasm on the battlefield. Player B has mana available and a "counter target activated ability" effect (e.g., Stifle). Player A activates Mirror-Mad Phantasm's ability. Verify that Player B receives priority and can respond before the ability resolves. With correct stack-based resolution: the ability goes on the stack, Player B counters it with Stifle, the ability is removed from the stack without effect, and Mirror-Mad Phantasm remains on the battlefield unchanged.

### activated_ability_source_removed_before_resolution
Source ticket: (new)
Implementation: (not yet written)
Scenario: Player A controls Mirror-Mad Phantasm. Player A activates the ability (pays {1}{U}). Before the ability resolves, Player B casts an instant that exiles Mirror-Mad Phantasm. When the ability resolves, "This creature's owner shuffles it into their library" fails because the creature is in exile (not on the battlefield). The "If that player does" conditional is false, so no reveal/mill occurs. Verify the library is unchanged and no cards are milled.
