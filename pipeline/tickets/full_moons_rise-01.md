---
id: full_moons_rise-01
status: closed-duplicate
card: Full Moon's Rise
card_file: mtg-engine/src/cards/isd/full_moons_rise.rs
created: 2026-04-15T03:51:13Z
audit_run_id: 2026-04-14-full_moons_rise-audit
audit_model: opus
audit_tokens: 21104
audit_duration: 485
duplicate_of: merged-activated-no-stack-02
---

## Audit Finding

**Oracle text:**
> Sacrifice this enchantment: Regenerate all Werewolf creatures you control.

**Code:**
> engine.rs:2717-2719: `behavior.on_activate_ability(&mut new_state, *object_id, *ability_index, targets, registry);` — the ability effect fires immediately after costs are paid, without being placed on the stack.

**Description:**
Per CR 602.2, activated abilities are placed on the stack and players receive priority before they resolve. The engine executes `on_activate_ability` immediately after paying costs (sacrifice at engine.rs:2662, then effect at engine.rs:2718), bypassing the stack entirely. This means opponents cannot respond to the regeneration ability — they have no window to destroy/exile the Werewolves before regeneration shields are applied, or to counter the ability with Stifle-type effects. This is an engine-wide issue affecting all activated abilities.

**Engine path:**
- engine.rs:2633-2723 (ActivateAbility action handler — costs paid then effect fired inline)

**Required check:** 8c

**Affected cards:**
- Full Moon's Rise
- All cards with activated abilities (engine-wide)

## Tests

### activated_ability_goes_on_stack
Source ticket: (new)
Implementation: (not yet written)
Scenario: Player A controls Full Moon's Rise and a Werewolf creature. Player B has priority. Player A activates "Sacrifice this enchantment: Regenerate all Werewolf creatures you control." Verify the ability goes on the stack, Player B receives priority to respond (e.g., by casting an exile spell on the Werewolf), and the regeneration shields are not applied until the ability resolves.
