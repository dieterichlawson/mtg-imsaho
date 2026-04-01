## Audit — 2026-04-01

**Scryfall Oracle text**: Whenever another Human creature enters the battlefield under your control, put a +1/+1 counter on Champion of the Parish.
**Scryfall type line**: Creature — Human Soldier
**Status**: ISSUE

### Findings

1. **Self-exclusion check missing (ISSUE)**: Oracle says "another Human creature." The `on_any_creature_enters` hook does not check `entered_id != self_id`. If Champion of the Parish itself enters the battlefield, it would match as a Human under its own control and incorrectly add a +1/+1 counter to itself. The check `entered_id != self_id` is needed.

2. **Card data correct**: Name, cost ({W}), type (Creature), subtypes (Human, Soldier), P/T (1/1) all correct.

3. **Trigger kind**: Uses `TriggerKind::AnyCreatureEnters` which is appropriate.

4. **Human detection**: Checks both registry card data and object subtypes, which is thorough.

5. **Tests**: No dedicated tests found.
