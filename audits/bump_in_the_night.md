# Audit: Bump in the Night

## Oracle (Scryfall/API)
- **Name:** Bump in the Night
- **Cost:** {B}
- **Type:** Sorcery
- **Oracle:** Target opponent loses 3 life. Flashback {5}{R}
- **P/T:** N/A

## Implementation: `bump_in_the_night.rs`
- **Name:** Bump in the Night -- CORRECT
- **Cost:** {B} -- CORRECT
- **Type:** Sorcery -- CORRECT
- **Flashback:** {5}{R} -- CORRECT
- **Target:** PlayerOnly, validated to exclude self (opponent only) -- CORRECT
- **Effect:** Target opponent loses 3 life -- CORRECT
- **Life loss:** Directly modifies life and emits LifeChanged event -- CORRECT

## Issues
1. **ISSUE (minor):** This is life loss, not damage. The implementation correctly does NOT emit NonCombatDamageDealt, which is correct -- life loss is distinct from damage in MTG rules.

## Verdict: PASS -- No issues found

---

# Re-Audit: Bump in the Night (2026-04-02)

## Oracle Text (Scryfall, cached 2026-04-01)

> Target opponent loses 3 life.
> Flashback {5}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)

## Card Data

| Field           | Oracle                | Implementation             | Match |
|-----------------|-----------------------|----------------------------|-------|
| Name            | Bump in the Night     | "Bump in the Night"        | OK    |
| Mana cost       | {B}                   | `[Colored(Black)]`         | OK    |
| Type            | Sorcery               | `[CardType::Sorcery]`      | OK    |
| Supertypes      | (none)                | `[]`                       | OK    |
| Subtypes        | (none)                | `[]`                       | OK    |
| Flashback cost  | {5}{R}                | `[Generic(5), Colored(Red)]` | OK  |
| Keywords        | Flashback             | `[]`                       | **MINOR** |

### Keyword note

The `keywords` vec is empty, but Flashback is declared via `flashback_cost: Some(...)`. This is consistent with other flashback cards in the codebase (e.g., Think Twice, Unburial Rites) which also leave `keywords` empty and rely on `flashback_cost` for the mechanic. No functional issue.

## Targeting

- `target_requirement` returns `PlayerOnly` -- correct, the spell targets an opponent (a player).
- `is_valid_target` rejects `Target::Player(pid)` where `pid == caster`, allowing only opponents. This correctly implements "target opponent."

No issues.

## Effect: Life Loss

Oracle: "Target opponent loses 3 life."

Implementation (lines 42-54):
```rust
fn on_resolve(&self, state: &mut GameState, object_id: ObjectId, targets: &[Target], _registry: &CardRegistry) {
    if let Some(Target::Player(player_id)) = targets.first() {
        let old_life = state.get_player(*player_id).life;
        let new_life = old_life - 3;
        state.get_player_mut(*player_id).life = new_life;
        state.events.push(GameEvent::LifeChanged {
            player: *player_id,
            old: old_life,
            new_life,
        });
    }
    state.move_spell_after_resolve(object_id);
}
```

- Subtracts exactly 3 life: correct.
- This is life loss, not damage. The implementation correctly does NOT emit a `NonCombatDamageDealt` event. Life loss bypasses damage prevention, indestructible, etc.
- A `LifeChanged` event is emitted. This is consistent with other life-loss cards in the engine (e.g., Falkenrath Noble at `falkenrath_noble.rs:64`). This event is used by the engine for state-based action checks (player at 0 or less life loses), so emitting it is correct and necessary.

No issues.

## Flashback / Zone Movement

`state.move_spell_after_resolve(object_id)` is called unconditionally at line 53. This helper checks `cast_with_flashback` on the object: if true, the spell is exiled; otherwise it goes to the graveyard. This correctly handles both normal casts and flashback casts.

No issues.

## Test Coverage

1. `tests/tier2_spells.rs::bump_in_the_night_drains_3` -- Verifies opponent goes from 20 to 17 life after resolution.
2. `tests/flashback.rs::bump_in_the_night_flashback_exiles` -- Verifies flashback cast causes 3 life loss and the card ends up in exile.
3. `tests/witchbane_orb.rs` -- Uses Bump in the Night to verify hexproof-for-players (Witchbane Orb) blocks targeting.

Coverage is adequate for the card's functionality.

## Verdict

**PASS** -- No mismatches found. The implementation faithfully represents the oracle text. Card data, targeting, life loss effect, LifeChanged event, and flashback handling are all correct and consistent with engine conventions.
