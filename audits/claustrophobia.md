# Audit: Claustrophobia

## Oracle Text (Scryfall, cached 2026-04-01)
- **Name:** Claustrophobia
- **Mana Cost:** {1}{U}{U}
- **Type Line:** Enchantment — Aura
- **Oracle Text:**
  > Enchant creature
  > When this Aura enters, tap enchanted creature.
  > Enchanted creature doesn't untap during its controller's untap step.
- **Keywords:** Enchant

## Implementation: `mtg-engine/src/cards/isd/claustrophobia.rs`
- **Name:** `"Claustrophobia"` — CORRECT
- **Cost:** `Generic(1), Blue, Blue` — CORRECT
- **Card types:** `[Enchantment]` — CORRECT
- **Subtypes:** `["Aura"]` — CORRECT
- **Supertypes:** `[]` — CORRECT
- **P/T:** `None / None` — CORRECT
- **Keywords:** `vec![]` — ACCEPTABLE (engine has no `Keyword::Enchant` variant; "Enchant" is modeled structurally via `TargetRequirement::Creature` and `resolve_aura`)
- **Target requirement:** `TargetRequirement::Creature` — CORRECT (implements "Enchant creature")
- **Continuous effects:** `PreventUntap { scope: Attached }` — CORRECT (implements "Enchanted creature doesn't untap during its controller's untap step")
- **On resolve:** Taps target creature, then calls `resolve_aura` — See Issue 1

## Issues

### Issue 1: Oracle text field omits "Enchant creature"

**Oracle text** (Scryfall):
> Enchant creature
> When this Aura enters, tap enchanted creature.
> Enchanted creature doesn't untap during its controller's untap step.

**Code** (`claustrophobia.rs`, line 25):
```rust
oracle_text: "When Claustrophobia enters the battlefield, tap enchanted creature. Enchanted creature doesn't untap during its controller's untap step.".into(),
```

The `oracle_text` field is missing the first line, "Enchant creature". This text is surfaced to the LLM player via the game view (`view.rs` line 223), so the omission could affect the LLM's understanding of the card. Additionally, the implementation uses older templating ("When Claustrophobia enters the battlefield") rather than current oracle wording ("When this Aura enters").

**Severity:** Low. Cosmetic/display only; no gameplay impact since "Enchant creature" is enforced structurally by `TargetRequirement::Creature` and `resolve_aura`.

### Issue 2: ETB tap is not a triggered ability

**Oracle text:**
> When this Aura enters, tap enchanted creature.

This is a triggered ability ("When ... enters") that should go on the stack after Claustrophobia resolves and enters the battlefield.

**Code** (`claustrophobia.rs`, lines 39-46):
```rust
fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
    if let Some(Target::Object(target_id)) = targets.first() {
        if let Some(target) = state.get_object_mut(*target_id) {
            target.tapped = true;
        }
    }
    crate::cards::helpers::resolve_aura(state, object_id, targets);
}
```

The tap is performed directly during spell resolution, before `resolve_aura` even moves the aura to the battlefield. Per MTG rules, this should be a triggered ability that triggers when the Aura enters the battlefield and then goes on the stack (allowing responses). The `triggered_abilities` field is `vec![]`.

**Severity:** Medium. In most game situations the result is the same, but the implementation differs from correct rules behavior in the following ways:
1. The tap cannot be responded to (no trigger on the stack).
2. A Stifle-like effect could not counter the tap.
3. If a replacement effect prevented the Aura from entering the battlefield, the tap would still occur (since it happens before `resolve_aura`).

## Tests

Two tests cover Claustrophobia:

1. **`innistrad_cards.rs::claustrophobia_taps_creature`** — Verifies that casting Claustrophobia on a creature taps it and that the aura ends up on the battlefield attached to the creature. Passes.

2. **`card_mechanics.rs::claustrophobia_prevents_untap`** — Verifies that a creature with Claustrophobia attached remains tapped after the untap step, while a normal creature untaps. Passes.

No test covers the ETB tap as a triggered ability (e.g., testing that the tap can be responded to or countered).

## LLM Player (`mtg-player/src/llm.rs`)

No Claustrophobia-specific logic found.

## Verdict

Two issues found, both low-to-medium severity. The core gameplay behavior (tap on entry, prevent untap) is functionally correct for typical gameplay scenarios. The triggered-ability timing is incorrect but unlikely to matter in the Innistrad-only card pool unless trigger-interaction cards are present.
