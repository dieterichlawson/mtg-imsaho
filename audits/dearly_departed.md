## Audit — 2026-04-01

**Scryfall Oracle text**: Flying
As long as Dearly Departed is in your graveyard, Human creatures you control enter the battlefield with an additional +1/+1 counter on them.
**Scryfall type line**: Creature — Spirit
**Status**: ISSUE

### Findings

1. **P/T incorrect (ISSUE)**: Implementation has `power: Some(5), toughness: Some(5)` (lines 23-24). Oracle for Dearly Departed is **5/5**. This is actually correct — 5/5 is right.

2. **Cost**: {4}{W}{W} matches the six-mana cost. Correct.

3. **Graveyard ability implementation**: Uses `on_any_creature_enters` and checks `self_obj.zone == Zone::Graveyard` (line 42). This is correct — the ability functions from the graveyard.

4. **Self-exclusion not checked (potential ISSUE)**: If Dearly Departed somehow enters the battlefield while a copy is in the graveyard, it would correctly get a counter (Dearly Departed is a Spirit, not a Human). But if it were given the Human type, there could be an issue. As a Spirit, this is fine.

5. **Owner vs controller (minor ISSUE)**: Line 47 checks `entered_controller != owner`. This should check against the controller's identity, not the owner. In most cases owner == controller, but if Dearly Departed is in an opponent's graveyard due to a control-changing effect, the check against `owner` is technically more correct for "your graveyard" semantics. This is acceptable.

6. **Tests**: No dedicated tests found.
