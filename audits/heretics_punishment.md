## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/147/heretics-punishment?utm_source=api
**Type line**: `Enchantment` — {4}{R}
**Oracle text**:
```
{3}{R}: Choose any target, then mill three cards. This enchantment deals damage to that permanent or player equal to the greatest mana value among the milled cards.
```

**Status**: ISSUE

### Code issues
See below.

- The mill bypassed the mill pipeline, so no `CreatureCardMilled` event fired.
  - Oracle text says: `then mill three cards`
  - Code did: `let milled: Vec<ObjectId> = state.get_player_mut(controller)
    .library_order.drain(..mill_count).collect();` then `move_object(card_id,
    Zone::Graveyard, registry)` per card
  - `mill_one`'s contract is that every library-to-graveyard move goes through
    it. An opponent's Undead Alchemist — "whenever a creature card is put into
    an opponent's graveyard from their library" — saw nothing. Whether a
    watcher cares is the collector's decision (it skips watchers controlled by
    the milled player), not the miller's. Fixed.

- The ability's effect lived in `on_activate_ability`, whose trait default *was*
  the CR 602.2a stack push, so the effect happened the instant the ability was
  activated and no opponent ever received priority.
  - CR 602.2a says: `the ability goes on the stack`
  - Code did: `fn on_activate_ability(&self, ...) { <the effect> }` — overriding
    the push away
  - Fixed set-wide: the hook is gone, the engine owns the push
    (`engine::actions::abilities::put_ability_on_stack`), and the effect moved to
    `resolve_activated_ability`. See
    `reports/ISD_AUDIT_CR6022a_ACTIVATED_ABILITIES.md`.
  The card also hand-rolled its own fizzle check inside the activation hook,
  because the engine's could never run there. Removed with the conversion.

### Tricky interactions checked
- Ruling: "If you have two or fewer cards in your library when the ability
  resolves, all of them will be put into your graveyard" — `min(3, len)`: PASS
- Ruling: "If all three cards have a mana value of 0, no damage will be dealt" —
  guarded by `if max_mv > 0`: PASS
- Ruling: "The mana value of a double-faced card in your graveyard is the mana
  value of the front face" — read via `face_data` while still in the library,
  which is the front face for an untransformed DFC: PASS
- Non-combat damage emits `NonCombatDamageDealt`, via `damage::deal_damage`
  with `DamageKind::NonCombat`: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- CR 602.2a (the ability waits on the stack): `activated_no_stack.rs:activating_through_the_engine_leaves_the_ability_on_the_stack`
- CR 608.2b (targets re-checked on resolution): `fizzle.rs:an_activated_abilitys_targets_are_rechecked_when_it_resolves`
- Guards: `test_suite_guards.rs:no_card_or_test_names_the_removed_activation_hook`, `test_suite_guards.rs:only_the_engine_puts_an_ability_on_the_stack`
- The mill emits CreatureCardMilled: `multi_target_and_mill.rs:heretics_punishment_emits_creature_card_milled`
- Mill then damage: `cards_complex_creatures.rs:heretics_punishment_mills_then_deals_damage`
- damaged_by tracked on the target: `cards_complex_creatures.rs:heretics_punishment_tracks_damaged_by_on_creature`
