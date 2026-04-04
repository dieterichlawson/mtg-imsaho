## Audit — 2026-04-03 23:01

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Grimgrin enters tapped and doesn't untap during your untap step.
Sacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.
Whenever Grimgrin attacks, destroy target creature defending player controls, then put a +1/+1 counter on Grimgrin.
**Type line**: Legendary Creature — Zombie Warrior
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- "Another creature" restriction: PASS - `SacrificeAnotherCreature` correctly excludes source permanent via `o.id != obj_id` check in engine
- Target choice presentation: PASS - Uses `present_target_choice` which auto-applies when 1 target, presents choice when multiple
- "Defending player controls" targeting: PASS - Gets defending player from combat state, not just opponent, and filters creatures by defender ownership
- "Destroy target creature defending player controls, then put a +1/+1 counter": PASS - `DestroyThenCounter` effect implements correct order and adds counter regardless of destruction success per ruling
- "Doesn't untap during your untap step": PASS - `PreventUntap { scope: EffectScope::OnSelf }` continuous effect verified in engine untap step code
- Manual untap via activated ability: PASS - `obj.tapped = false` directly overrides continuous effect as expected
- Attack trigger timing: PASS - `TriggerKind::Attacks` properly dispatched when creature is declared as attacker
- No targets scenario: PASS - Returns early when defender has no creatures, no counter added per ruling

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Basic attack trigger (1 target): `/Users/dlaw/mtg/mtg-engine/tests/tier15_cards.rs:1568-1578`
- Attack trigger with multiple targets: `/Users/dlaw/mtg/mtg-engine/tests/tier15_cards.rs:1598-1618`
- No defender creatures (no effect): `/Users/dlaw/mtg/mtg-engine/tests/tier15_cards.rs:1637-1641`
- Indestructible target still gives counter: `/Users/dlaw/mtg/mtg-engine/tests/tier15_cards.rs:1664-1671`
- Targets defending player's creatures not controller's: `/Users/dlaw/mtg/mtg-engine/tests/tier15_cards.rs:1694-1701`
- Activated ability (sacrifice another creature for untap/counter): NOT TESTED - no direct test found
- Enters tapped functionality: NOT TESTED - no direct test found
- Continuous untap prevention: NOT TESTED - no direct test found