## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/138/curse-of-the-pierced-heart?utm_source=api
**Type line**: `Enchantment — Aura Curse` — {1}{R}
**Oracle text**:
```
Enchant player
At the beginning of enchanted player's upkeep, this Aura deals 1 damage to that player or a planeswalker that player controls.
```

**Status**: ISSUE

### Code issues
See below.


- The ordinary case — no planeswalkers — wrote the life total by hand instead of
  dealing damage.
  - Oracle text says: `this Aura deals 1 damage to that player or a planeswalker that player controls`
  - Code did: `let new_life = old - 1; state.get_player_mut(cursed_player).life = new_life;`
    followed by its own `NonCombatDamageDealt` and `LifeChanged` events
  - So the case that happens every game skipped `damage::deal_damage` and
    everything it applies: protection, prevention, damage multipliers, and the
    watchers that key on the pipeline. The planeswalker branch right beside it
    already used `PendingEffect::DealDamage`. Both branches now build one
    effect and differ only in whether there is a choice to present. The
    `only_the_damage_pipeline_marks_damage` guard, which watched `damage_marked`
    and so could not see a hand-written *player* life change, was extended to
    catch this shape.

### Tricky interactions checked
- "At the beginning of **enchanted player's** upkeep" — CR 603.2: the trigger
  event is that player's upkeep beginning, so `TriggerScope::AttachedPlayer`
  keeps it off the stack during anyone else's: PASS
- CR 113.7a: destroying the Curse in response does not counter its trigger, and
  `attached_player` still knows whom it cursed: PASS
- Enchant **player**, so `TargetRequirement::PlayerOnly` and the Curse subtype:
  PASS
- "…to that player **or a planeswalker that player controls**" — the choice is
  the *Curse controller's*, not the cursed player's: PASS
- `obj.card_types` is empty for a non-token permanent, so the planeswalker scan
  goes through `has_card_type`, which reads the active face: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The damage and the planeswalker choice: `cards_auras.rs`, `curse_and_equip_scope.rs`
- The pipeline guard: `test_suite_guards.rs:only_the_damage_pipeline_marks_damage`
