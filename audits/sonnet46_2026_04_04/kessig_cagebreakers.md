## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Whenever this creature attacks, create a 2/2 green Wolf creature token that's tapped and attacking for each creature card in your graveyard.
**Type line**: Creature — Human Rogue
**Status**: ISSUE

### Code issues

- **Attack trigger silently discarded if Kessig is destroyed before resolution** (`mtg-engine/src/triggers.rs:980-985` and `mtg-engine/src/cards/isd/kessig_cagebreakers.rs:39-42`)

  The engine's `resolve_next_trigger` guards the `AttacksTrigger` resolution with a battlefield check. The card's own `on_attacks` then also early-returns if the source is not on the battlefield:

  - Oracle text says: `"Whenever this creature attacks, create a 2/2 green Wolf creature token that's tapped and attacking for each creature card in your graveyard."`
  - Code does (engine, `triggers.rs:980-985`):
    ```rust
    PendingTrigger::AttacksTrigger { object_id, card_id, .. } => {
        if state.get_object(object_id).map(|o| o.zone == Zone::Battlefield).unwrap_or(false) {
            if let Some(behavior) = registry.get(card_id) {
                behavior.on_attacks(state, object_id, registry);
            }
        }
    }
    ```
  - Code does (card, `kessig_cagebreakers.rs:39-42`):
    ```rust
    let controller = match state.get_object(self_id) {
        Some(o) if o.zone == Zone::Battlefield => o.controller,
        _ => return,
    };
    ```

  The effect "create Wolf tokens" does not reference or depend on the source creature being present at resolution. Per MTG rules (CR 112.7a), once a triggered ability is on the stack it exists independently of its source. If an opponent casts a removal spell in the priority window after Kessig's attack trigger is placed on the stack but before it resolves, the trigger silently does nothing instead of creating wolves. Both the engine-level guard and the card-level early-return must be fixed; the card-level one is the more fundamental problem because even if the engine guard were removed, `on_attacks` would still bail before counting the graveyard.

- **Parallel Lives doubled tokens are not set as tapped and attacking** (`mtg-engine/src/cards/isd/kessig_cagebreakers.rs:61-76` and `mtg-engine/src/state.rs:314-348`)

  - Oracle text says: `"create a 2/2 green Wolf creature token that's tapped and attacking for each creature card in your graveyard"`
  - Code does (`kessig_cagebreakers.rs:61-76`):
    ```rust
    for _ in 0..creature_count {
        let token_id = state.create_token_with_subtypes(...);
        if let Some(obj) = state.get_object_mut(token_id) {
            obj.tapped = true;
            obj.summoning_sick = false;
        }
        if let Some(combat) = &mut state.combat {
            combat.attackers.insert(token_id, defending_player);
        }
    }
    ```
  - `create_token_with_subtypes` (`state.rs:314-348`) creates the primary token and then creates extra copies for Parallel Lives, but returns only the primary token's `ObjectId`; the extra copies' IDs are discarded:
    ```rust
    let id = self.create_token_internal(...);           // primary
    for _ in 0..extra_copies {
        self.create_token_internal(...);                // extra copies — IDs discarded
    }
    id   // only primary returned
    ```

  With Parallel Lives on the battlefield (e.g., 3 creature cards in graveyard → 6 Wolf tokens expected), each iteration returns only one ID and only that token is tapped and inserted into `combat.attackers`. The three doubled tokens enter the battlefield untapped and not attacking, violating the oracle text.

### Tricky interactions checked

- **Trigger fires from `AttackersDeclared` event**: `triggers.rs:677-721` iterates every attacker in the event, calls `trigger_description` for `TriggerKind::Attacks`, and creates `PendingTrigger::AttacksTrigger` for Kessig. Kessig declares `TriggerKind::Attacks` with a non-empty description, so the trigger is collected. **PASS**
- **Count happens at resolution time (ruling 1 — "count when trigger resolves")**: `on_attacks` is called from `resolve_next_trigger` at resolution time; the graveyard count (`state.objects_in_zone(Zone::Graveyard, controller)`) is evaluated inside that call, not at trigger-placement time. **PASS** (when Kessig survives to resolution)
- **Kessig dies in response before trigger resolves**: Engine-level guard (`triggers.rs:981`) and card-level early-return (`kessig_cagebreakers.rs:40`) both prevent any tokens from being created, even though the effect is independent of the source. **FAIL** (see Issue 1)
- **Token stats (2/2 green Wolf)**: `create_token_with_subtypes("Wolf", controller, 2, 2, vec![Color::Green], vec![CardType::Creature], vec![], vec!["Wolf".into()])` — power, toughness, color, type, and subtype all match oracle. **PASS**
- **Tokens tapped and attacking (no Parallel Lives)**: Code correctly sets `obj.tapped = true`, `obj.summoning_sick = false`, and inserts into `combat.attackers`. **PASS**
- **Parallel Lives doubles tokens, all should be tapped/attacking**: Only the primary token per iteration gets tapped and inserted into `combat.attackers`; extra copies' IDs are discarded by `create_token_with_subtypes`. **FAIL** (see Issue 2)
- **Tokens not declared as attackers (ruling 3)**: Tokens are added directly to `combat.attackers`; no `AttackersDeclared` event is emitted for them. `collect_triggers` only fires `AttacksTrigger` from `AttackersDeclared` events, so "whenever a creature attacks" effects do NOT trigger from the wolf tokens. **PASS**
- **`objects_in_zone(Graveyard, controller)` uses owner**: `state.rs:601-608` filters graveyard by `obj.owner == player`, which is correct — cards in the graveyard belong to their owner's graveyard per MTG rules. **PASS**
- **Fallback for anonymous objects in graveyard count**: `registry.card_data(o.card_id).unwrap_or(o.power.is_some())` — tokens (card_id = 0) fall back to `power.is_some()`, which would incorrectly count creature tokens that briefly pass through the graveyard before SBA removal. However, tokens are removed by SBAs before any triggered ability can resolve, so this is not observable in practice. **PASS** (not practically reachable)
- **0 creatures in graveyard → no tokens created**: `if creature_count == 0 { return; }` at `kessig_cagebreakers.rs:52-54`. **PASS**
- **Graveyard controller lookup**: `controller` is obtained from the live object before the count. If Kessig is alive this is correct. If Kessig is dead, the early-return prevents reaching the count entirely (see Issue 1). **PASS** (when alive)

### Test coverage

- Basic token creation (3 wolves when 3 creature cards in graveyard): `tier15_cards.rs:104` TESTED
- Wolves tapped at creation: `tier15_cards.rs:132` TESTED
- Wolves inserted into `combat.attackers`: `tier15_cards.rs:135` TESTED
- Ruling 1 — count happens at trigger resolution time, not placement: NOT TESTED
- Ruling 2 — each token can attack a different player/planeswalker than Kessig: NOT TESTED
- Ruling 3 — tokens do not trigger "whenever a creature attacks" effects: NOT TESTED
- Kessig destroyed before trigger resolves, tokens still created: NOT TESTED
- Parallel Lives doubled tokens are tapped and attacking: NOT TESTED
- 0 creature cards in graveyard, 0 tokens created: NOT TESTED
- Full trigger system flow (AttackersDeclared → AttacksTrigger → on_attacks via engine): NOT TESTED (test calls `on_attacks` directly)
