## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/inr/239/grimgrin-corpse-born?utm_source=api
**Type line**: `Legendary Creature — Zombie Warrior` — {3}{U}{B}, 5/5
**Oracle text**:
```
Grimgrin enters tapped and doesn't untap during your untap step.
Sacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.
Whenever Grimgrin attacks, destroy target creature defending player controls, then put a +1/+1 counter on Grimgrin.
```

**Status**: ISSUE

### Code issues
See below.


- It tapped itself *after* arriving instead of entering tapped.
  - Oracle text says: `Grimgrin enters tapped and doesn't untap during your untap step.`
  - Code did: `state.move_object(object_id, Zone::Battlefield, registry);` then
    `if let Some(obj) = state.get_object_mut(object_id) { obj.tapped = true; ... }`
  - `move_object` emits `EnteredBattlefield` as part of the move, so every ETB
    watcher saw an untapped Grimgrin and the tap landed afterwards — the
    ordering CR 614.1c exists to prevent. The same override also re-did the
    trait default's "move a permanent to the battlefield" and its
    `is_legendary` stamping. Replaced with `enters_tapped_unless`, the
    replacement effect the ISD lands already use.

### Tricky interactions checked
- "and **doesn't untap during your untap step**" — a `PreventUntap` continuous
  effect on itself, which is why the sacrifice ability exists: PASS
- "Sacrifice **another** creature" — `SacrificeAnotherCreature`, so Grimgrin
  cannot eat itself: PASS
- Ruling: "If Grimgrin's last ability resolves, but the targeted creature isn't
  destroyed (perhaps because it regenerated or has indestructible), you'll still
  put a +1/+1 counter on Grimgrin" — the counter is added after `try_destroy`
  regardless of its result: PASS
- Ruling: "If the targeted creature is an illegal target by the time Grimgrin's
  last ability resolves, the entire ability doesn't resolve and none of its
  effects will occur. You won't put a +1/+1 counter on Grimgrin": PASS
- Ruling: "If the defending player controls no creatures when Grimgrin attacks,
  the last ability will be removed from the stack and have no effect" — the
  trigger is not pushed when it has no legal target (CR 603.3d): PASS
- "target creature **defending player controls**" — `is_valid_target` reads the
  defending player from combat state rather than assuming the opponent: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Entering tapped as a replacement: `cards_complex_creatures.rs:grimgrin_enters_tapped`
- The counter lands even against an indestructible target: `cards_complex_creatures.rs:grimgrin_attack_indestructible_target_still_gets_counter`
- No creatures means no trigger: `cards_complex_creatures.rs:grimgrin_attack_no_targets_no_counter`
- The defending player comes from combat: `cards_complex_creatures.rs:grimgrin_attack_uses_defending_player_from_combat`
- Target re-checked on resolution: `trigger_target_recheck.rs`, `ability_target_protection.rs`
