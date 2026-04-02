# Audit: Curse of the Nightly Hunt

## Scryfall Reference
- **Name:** Curse of the Nightly Hunt
- **Cost:** {2}{R}
- **Type:** Enchantment -- Aura Curse
- **Oracle:** Enchant player. Creatures enchanted player controls attack each combat if able.
- **P/T:** N/A
- **Keywords:** Enchant

## Implementation: `curse_of_the_nightly_hunt.rs`
- **Name:** Curse of the Nightly Hunt -- CORRECT
- **Cost:** {2}{R} -- CORRECT
- **Type:** Enchantment -- CORRECT
- **Subtypes:** ["Aura", "Curse"] -- CORRECT
- **P/T:** N/A -- CORRECT
- **Target:** TargetRequirement::PlayerOnly -- CORRECT
- **Continuous effect:** ForceAttack { scope: Global(AttachedPlayer) } -- CORRECT

## Issues
None

## Audit (2026-04-02)

### Oracle Text (Scryfall, verbatim)
```
Enchant player
Creatures enchanted player controls attack each combat if able.
```

### Rulings
1. "The enchanted player still chooses which player or planeswalker each creature they control attacks."
2. "If, during the enchanted player's declare attackers step, a creature they control is tapped, is affected by a spell or ability that says it can't attack, or hasn't been under that player's control continuously since the turn began (and doesn't have haste), then it doesn't attack. If there's a cost associated with having the creature attack, the player isn't forced to pay that cost, so it doesn't have to attack in that case either."

### Implementation Checklist
| Field | Oracle | Code | Match |
|---|---|---|---|
| Name | Curse of the Nightly Hunt | `"Curse of the Nightly Hunt"` | CORRECT |
| Mana cost | {2}{R} | `Generic(2), Colored(Red)` | CORRECT |
| Type | Enchantment | `CardType::Enchantment` | CORRECT |
| Subtypes | Aura Curse | `["Aura", "Curse"]` | CORRECT |
| P/T | N/A | `None, None` | CORRECT |
| Target | Enchant player | `TargetRequirement::PlayerOnly` | CORRECT |
| Resolve | Attach to player | `helpers::resolve_curse` (sets `attached_to_player`) | CORRECT |
| Effect | Creatures attack if able | `ForceAttack { scope: Global(AttachedPlayer) }` | CORRECT |

### Mechanics Verification
- **AttachedPlayer scope** (state.rs:633-643): Looks up `source.attached_to_player`, matches creatures with `controller == player`. Correct.
- **Forced attack enforcement** (engine.rs:1624-1650): Skips tapped, summoning-sick, Defender, and already-attacking creatures before checking ForceAttack effect. Correctly implements ruling #2.
- **Eligible attackers** (combat.rs:569-585): Filters tapped, summoning-sick (unless haste), and Defender creatures. The `must_attack` list (engine.rs:130-140) is derived from eligible creatures only.
- **LLM player** (llm.rs:897-924): Displays `[MUST]` markers for forced attackers, auto-includes them. Correct.

### Test Coverage
- `curse_of_nightly_hunt_forces_attack` (tier7_cards.rs:323): Verifies that P1's creature is forced to attack when cursed, and P0's creature is not. Correct.

### Verdict
No issues found. The implementation correctly matches the oracle text, handles all ruling edge cases (tapped, summoning-sick, Defender), and has adequate test coverage.

## Audit — 2026-04-02 (final)

**Oracle text source**: Oracle cache (Scryfall API)
**Oracle text**: Enchant player\nCreatures enchanted player controls attack each combat if able.
**Type line**: Enchantment — Aura Curse
**Status**: PASS

### Code issues
No issues found. Card data matches: name, cost {2}{R}, subtypes Aura Curse, oracle text. Continuous effect ForceAttack with scope Global(CreatureFilter::AttachedPlayer) correctly forces creatures the cursed player controls to attack. Target requirement is PlayerOnly. Resolves via resolve_curse helper.
