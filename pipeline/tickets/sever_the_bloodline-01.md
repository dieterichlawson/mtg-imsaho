---
id: sever_the_bloodline-01
status: new
card: Sever the Bloodline
card_file: mtg-engine/src/cards/isd/sever_the_bloodline.rs
created: 2026-04-15T03:47:14Z
audit_run_id: 2026-04-14-sever_the_bloodline-audit
audit_model: opus
audit_tokens: 20080
audit_duration: 488
---

## Audit Finding

**Oracle text:**
> Exile target creature

**Code:**
> `is_target_legal` in stack.rs:37-41 checks hexproof but not protection or shroud:
> ```
> if obj.zone == Zone::Battlefield && obj.controller != caster
>     && state.has_keyword(*id, Keyword::Hexproof, registry) {
>     return false;
> }
> ```

**Description:**
Per CR 608.2b, a spell re-checks target legality on resolution. A target is illegal if it has protection from the spell's qualities (CR 702.16c — protection prevents targeting). The resolution-time check `is_target_legal` (stack.rs:8-56) only checks hexproof, not protection or shroud. If the target creature gains protection from black (Sever is a black spell) between casting and resolution — e.g., via Brave the Elements or Faith's Shield — the spell should fizzle per CR 608.2b, but instead resolves and exiles the protected creature along with all same-name creatures. The cast-time check (`can_be_targeted_by` at engine.rs:1452-1467) correctly checks both hexproof and protection, so only the resolution-time re-check is incomplete.

**Engine path:**
- stack.rs:8-56 (`is_target_legal` — missing protection/shroud check)
- stack.rs:88-110 (`resolve_spell` — calls `is_target_legal` for fizzle check)
- engine.rs:1452-1467 (`can_be_targeted_by` — correct cast-time check, for comparison)

**Required check:** 8f

**Affected cards:**
- Sever the Bloodline
- All spells with targeted creatures (engine-wide: `is_target_legal` is shared)

## Tests

### sever_target_gains_protection_should_fizzle
Source ticket: (new)
Implementation: (not yet written)
Scenario: Cast Sever the Bloodline targeting a creature. Before resolution, give the target protection from black (e.g., via a continuous effect or card_state flag). Verify the spell fizzles (does not resolve), no creatures are exiled, and the spell goes to the graveyard (or exile if flashback).

