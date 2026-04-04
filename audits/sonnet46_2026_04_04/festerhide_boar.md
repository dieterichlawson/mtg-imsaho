## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Trample
Morbid — This creature enters with two +1/+1 counters on it if a creature died this turn.
**Type line**: Creature — Boar
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- `creature_died_this_turn` set by all death paths: Both the `destruction.rs` pipeline (`destroy()` at line 100) and `sba.rs` zero-toughness path (line 96) and lethal-damage fallback (line 144) all set `state.creature_died_this_turn = true`. All creature death paths correctly flag morbid. PASS
- `creature_died_this_turn` reset timing: The flag is cleared in `advance_step` (`engine.rs:2888`) inside the `None` branch of `state.step.next()`, which fires when transitioning from `Step::Cleanup` to the next player's `Untap` step. It persists for the entire current player's turn and is cleared at turn boundary. This is correct for morbid ("a creature died this turn"). PASS
- Counter addition order vs. replacement effect: The oracle text says "enters WITH two +1/+1 counters" (a replacement effect per CR 614.1c). The code calls `state.move_object(object_id, Zone::Battlefield)` first (which fires the `EnteredBattlefield` event), then adds counters via `state.add_counters`. Counters are thus placed after the ETB event is emitted rather than as part of entering. In this engine, `collect_triggers` reads the current object state when it processes events later (not snapshot-at-event-time counter state), so triggers that subsequently resolve see the counters correctly. No currently-implemented card in this engine triggers on "entered with a counter," so no behavioral difference results. PASS
- Mana cost, types, P/T, keywords: `ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::Green)])`, `vec![CardType::Creature]`, subtypes `vec!["Boar".into()]`, `power: Some(3), toughness: Some(3)`, `keywords: vec![Keyword::Trample]` all match the oracle text exactly. PASS
- Counter type and count: `state.add_counters(object_id, CounterType::PlusOnePlusOne, 2)` correctly adds exactly 2 +1/+1 counters. `effective_power` and `effective_toughness` in `state.rs` include `PlusOnePlusOne` counters, so a 3/3 base becomes 5/5 when morbid applies. PASS
- Creature permanent not moved with `move_spell_after_resolve`: The card correctly moves itself to the battlefield in `on_resolve` via `state.move_object(object_id, Zone::Battlefield)`. After `on_resolve` returns, `stack.rs:107-111` checks whether the object is still on the stack and only then calls `move_spell_after_resolve`; since the creature is already on the battlefield it is not double-moved. PASS

### Test coverage
For each ruling and tricky interaction, list whether it is tested and where:
- Morbid path (creature_died_this_turn = true → 2 counters, 5/5): `mtg-engine/tests/tier5_cards.rs:217` (`festerhide_boar_morbid`) — TESTED
- No-morbid path (creature_died_this_turn = false → 0 counters, 3/3): `mtg-engine/tests/tier5_cards.rs:234` (`festerhide_boar_no_morbid`) — TESTED
- creature_died_this_turn reset at turn boundary: NOT TESTED (engine-level turn structure; covered indirectly by turn_structure tests)
- creature_died_this_turn set by SBA lethal damage path: NOT TESTED for this card specifically
- creature_died_this_turn set by SBA zero-toughness path: NOT TESTED for this card specifically
- Trample keyword present and functional: NOT TESTED for this card specifically (Trample is tested generically in keywords.rs)
