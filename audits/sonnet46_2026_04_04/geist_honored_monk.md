## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Vigilance
Geist-Honored Monk's power and toughness are each equal to the number of creatures you control.
When this creature enters, create two 1/1 white Spirit creature tokens with flying.
**Type line**: Creature — Human Monk
**Status**: PASS

### Code issues
No issues found.

### Tricky interactions checked
- **Dynamic P/T works in all zones (ruling 2011-09-22)**: PASS. `effective_power`/`effective_toughness` in `state.rs` call `behavior.dynamic_pt(self, id)` without restricting to the battlefield. `dynamic_pt` uses `state.get_object(object_id)?.controller` which succeeds for objects in any zone (graveyard, hand, etc.), then counts `o.zone == Zone::Battlefield` creatures for that controller.
- **Self-counting (ruling 2011-09-22)**: PASS. When on the battlefield, the Monk's stored power is `Some(0)`, so the filter `o.power.is_some()` includes the Monk itself in the creature count. A Monk entering with 2 tokens correctly evaluates to 3/3.
- **ETB trigger dispatch path**: PASS. `collect_triggers` in `triggers.rs` fires `PendingTrigger::EnteredBattlefield` for the Monk when `registry.get(card_id).is_some()` (true). `trigger_description` correctly finds the `TriggerKind::EntersBattlefield` entry and returns "create two 1/1 white Spirit tokens with flying". Resolution calls `on_enter_battlefield` which creates exactly 2 tokens.
- **ETB trigger resolves if source has left battlefield**: Engine resolves all ETB triggers synchronously via `process_triggers` before giving players priority. There is no priority window between trigger creation and resolution in this engine, so the zone-check guard in `resolve_next_trigger` is unreachable in normal gameplay. The trigger always fires in practice.
- **Token specs (1/1, white, Spirit, Creature, Flying)**: PASS. `on_enter_battlefield` calls `create_token_with_subtypes("Spirit", controller, 1, 1, vec![Color::White], vec![CardType::Creature], vec![Keyword::Flying], vec!["Spirit".into()])` — all five characteristics match oracle text.
- **Mandatory (no "you may")**: PASS. `on_enter_battlefield` creates the tokens unconditionally; no choice is presented.
- **Token ETB events don't cause spurious self-triggers**: PASS. Spirit tokens are created with `card_id: CardId(0)`; `registry.get(CardId(0))` returns `None`, so no self-ETB trigger fires for the tokens.
- **No double-counting of dynamic P/T**: PASS. `continuous_pt_mods` only calls `dynamic_pt` for objects where `source.attached_to == Some(creature_id)` (i.e., auras). The Monk is not an aura; its `dynamic_pt` is only called from `effective_power` directly, so there is no double-application.
- **Vigilance keyword**: PASS. `keywords: vec![Keyword::Vigilance]` is declared in `card_data()`.
- **Mana cost {3}{W}{W}**: PASS. `ManaCost::new(vec![ManaSymbol::Generic(3), ManaSymbol::Colored(Color::White), ManaSymbol::Colored(Color::White)])`.
- **Subtypes Human and Monk**: PASS. `subtypes: vec!["Human".into(), "Monk".into()]`.
- **Parallel Lives interaction with token creation**: PASS. `create_token_with_subtypes` checks for Parallel Lives and doubles tokens accordingly; each of the two Spirit tokens is independently doubled.

### Test coverage
- **ETB creates exactly 2 Spirit tokens**: `tier5_cards.rs:73` (`geist_honored_monk_dynamic_pt_and_tokens`) — TESTED (asserts 3 total creatures on battlefield after ETB).
- **Dynamic P/T equals creature count**: `tier5_cards.rs:96-97` — TESTED (asserts 3/3 with 3 creatures).
- **Self-counting on battlefield**: `tier5_cards.rs:73` — TESTED implicitly (3/3 count includes the Monk).
- **P/T works in non-battlefield zones (ruling 2011-09-22)**: NOT TESTED.
- **P/T decreases when a creature dies**: NOT TESTED.
- **Token has Flying keyword**: NOT TESTED.
- **Token is white**: NOT TESTED.
- **Token is 1/1**: NOT TESTED.
- **Vigilance keyword active**: NOT TESTED.
- **Parallel Lives doubling the Spirit tokens**: NOT TESTED.
