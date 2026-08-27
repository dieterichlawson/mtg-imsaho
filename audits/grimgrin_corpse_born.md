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
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/inr/239/grimgrin-corpse-born?utm_source=api
**Type line**: `Legendary Creature — Zombie Warrior` — {3}{U}{B}, 5/5
**Oracle text**:
```
Grimgrin enters tapped and doesn't untap during your untap step.
Sacrifice another creature: Untap Grimgrin and put a +1/+1 counter on it.
Whenever Grimgrin attacks, destroy target creature defending player controls, then put a +1/+1 counter on Grimgrin.
```

**Rulings fetched**:
- [2013-07-01] If Grimgrin's last ability resolves, but the targeted creature isn't destroyed (perhaps because it regenerated or has indestructible), you'll still put a +1/+1 on Grimgrin.
- [2011-09-22] If the targeted creature is an illegal target by the time Grimgrin's last ability resolves, the entire ability doesn't resolve and none of its effects will occur. You won't put a +1/+1 counter on Grimgrin.
- [2011-09-22] If the defending player controls no creatures when Grimgrin attacks, the last ability will be removed from the stack and have no effect.

**Status**: ISSUE (fixed)

### Code issues

**The log claimed a destruction that the card's own headline ruling says may not happen.**

- Ruling says: `If Grimgrin's last ability resolves, but the targeted creature isn't destroyed (perhaps because it regenerated or has indestructible), you'll still put a +1/+1 on Grimgrin.`
- Code did:
  ```rust
  crate::destruction::try_destroy(state, *id, registry);
  state.log(crate::state::LogLevel::Event, format!("Grimgrin, Corpse-Born destroyed {name}"));
  ```

The *behaviour* was right — the counter is unconditional, which is what the
ruling is about, and there is a test for it. But `try_destroy` returns a
`DestroyResult` that was thrown away, so the log said "destroyed X" when X was
indestructible and still sitting on the battlefield. A player reading the log
back — or an LLM player, which reads `display_log` as its record of the game —
cannot tell the two cases apart, and this is precisely the case the card is most
often asked about. The log now reports which of the three outcomes happened.

**A missing test for the half of the card that defines it.**

"Grimgrin enters tapped **and doesn't untap during your untap step**" — the
second clause is why the sacrifice ability exists at all, and nothing tested it.
`grimgrin_enters_tapped` covers the first clause only. Added a test that rounds
the table back to Grimgrin's controller's untap step with an ordinary tapped
creature alongside, so it also shows the untap step really ran rather than being
skipped. Mutation-checked by re-scoping `PreventUntap` away from `OnSelf`.

### Rulings checked

- **"If Grimgrin's last ability resolves, but the targeted creature isn't
  destroyed ... you'll still put a +1/+1 on Grimgrin."** `resolve_card_effect`
  calls `try_destroy` and then `add_counters` unconditionally — the counter is
  not inside a success branch. PASS, and tested.
- **"If the targeted creature is an illegal target by the time Grimgrin's last
  ability resolves, the entire ability doesn't resolve and none of its effects
  will occur. You won't put a +1/+1 counter on Grimgrin."** This is the engine's
  CR 608.2b re-check in `resolve_next_trigger`, which fizzles the whole trigger
  rather than skipping the destruction — so the counter goes too. PASS, and
  `trigger_target_recheck.rs:174` asserts the *whole* footprint, not just the
  counter, which is what makes it a real test of "none of its effects".
- **"If the defending player controls no creatures when Grimgrin attacks, the
  last ability will be removed from the stack and have no effect."** With no
  legal targets the trigger never reaches the stack (CR 603.3d). PASS, tested by
  `grimgrin_attack_no_targets_no_counter`.

### Tricky interactions checked

- **"destroy target creature defending player controls"** — a genuine
  restriction, not decoration. `TargetRequirement::Creature` plus
  `is_valid_target` narrowing to the defending player, and
  `valid_targets_for_req` applies both that *and* `can_be_targeted_by` when it
  enumerates the trigger's targets — so Grimgrin can neither point at his own
  side nor at a hexproof creature. Verified in the engine rather than assumed.
  PASS.
- **"Sacrifice another creature"** — `SacrificeCost::SacrificeAnotherCreature`,
  so Grimgrin cannot eat himself, and the ability is not offered with no other
  creature. PASS.
- **The ability has no `{T}` and `requires_tap: false`**, which is what lets a
  tapped, summoning-sick Grimgrin untap himself. Getting this wrong would break
  the card's only engine. PASS.
- **"enters tapped" is a replacement effect** (CR 614.1c) via
  `enters_tapped_unless`, not a tap applied after arrival — so ETB watchers see
  a tapped Grimgrin. The card carries a comment about the earlier version that
  moved him and tapped him afterwards. PASS.
- **The counter goes on `source_id`.** If Grimgrin has left the battlefield by
  resolution the ability still resolves and destroys the target (CR 113.7a);
  there is simply no Grimgrin to put a counter on. PASS.
- **Multi-attacker defender derivation.** `is_valid_target` has no source id, so
  it finds the defending player from any attacker the caster controls, falling
  back to `opponent(caster)`. In a two-player game every attacker faces the same
  player, so this cannot pick the wrong one here. Noted as a two-player
  assumption that is correct for this pool, not a defect.

### Test coverage

- doesn't untap during your untap step: `cards_complex_creatures.rs::grimgrin_does_not_untap_during_his_controllers_untap_step` (new, mutation-checked).
- enters tapped: `::grimgrin_enters_tapped`.
- sacrifice untaps and adds a counter: `::grimgrin_sacrifice_untaps_and_counters`.
- not offered without another creature: `::grimgrin_sacrifice_not_available_without_other_creatures`.
- attack trigger destroys and adds a counter: `::grimgrin_attack_trigger_destroys_and_adds_counter`.
- choice among several defending creatures: `::grimgrin_attack_trigger_presents_choice_with_multiple_targets`.
- no defending creatures, no counter: `::grimgrin_attack_no_targets_no_counter`.
- indestructible target still gives the counter: `::grimgrin_attack_indestructible_target_still_gets_counter`.
- target taken from combat's defending player: `::grimgrin_attack_uses_defending_player_from_combat`.
- opponent's hexproof creature excluded: `hexproof_filter.rs:181`.
- illegal target fizzles the entire ability: `trigger_target_recheck.rs:174`.

