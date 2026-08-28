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
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/147/heretics-punishment?utm_source=api
**Type line**: `Enchantment` — {4}{R}
**Oracle text**:
```
{3}{R}: Choose any target, then mill three cards. This enchantment deals damage to that permanent or player equal to the greatest mana value among the milled cards.
```

**Rulings fetched**:
- [2011-09-22] If the targeted permanent or player is an illegal target by the time the ability resolves, the entire ability won't resolve. No cards will be put into your graveyard, and no damage will be dealt.
- [2011-09-22] If you have two or fewer cards in your library when the ability resolves, all of them will be put into your graveyard. Heretic's Punishment will still deal damage equal to the highest mana value among those cards.
- [2011-09-22] The mana value of a double-faced card in your graveyard is the mana value of the front face.
- [2011-09-22] If all three cards have a mana value of 0, no damage will be dealt.

**Status**: ISSUE


Four rulings:
1. "If the targeted permanent or player is an illegal target by the time the
   ability resolves, the entire ability won't resolve. No cards will be put
   into your graveyard, and no damage will be dealt."
2. "If you have two or fewer cards in your library when the ability resolves,
   all of them will be put into your graveyard. Heretic's Punishment will still
   deal damage equal to the highest mana value among those cards."
3. "The mana value of a double-faced card in your graveyard is the mana value
   of the front face."
4. "If all three cards have a mana value of 0, no damage will be dealt."

### Code issues
No behavioural bug. All four rulings were already satisfied. One structural
change and a lot of missing tests.

**The card enforced target legality from inside its own resolution.** It opened
`resolve_activated_ability` with a hand-rolled check — quoting ruling 1 in the
comment — and returned early if the target had left the battlefield.

That is the only card in the set doing it that way, and it is a step too late.
`stack.rs`'s ability arm asks each card's `is_valid_target`, substitutes
`Target::Illegal`, and counters the ability outright when every target is
illegal — which *is* ruling 1. Every other targeted activated ability in the
set relies on that. Heretic's Punishment does not define `is_valid_target` at
all, so its in-resolve check was load-bearing rather than redundant: removing
it mills three cards against a dead target, which I confirmed by mutation
before touching anything.

Moved into `is_valid_target`, where the engine asks. Same answer, one mechanism
instead of two, and it would have been the wrong place the moment the ability
gained a second target: with two targets, one illegal, the engine calls the
hook and the card's early `return` would have thrown away the *legal* half.

### What I checked and did not change
- `library_order[0]` is the top of the library — confirmed against
  `engine::mill_cards`, which takes index 0. The card's `[..mill_count]` and
  `drain(..mill_count)` agree with it.
- `ability_controller` (CR 602.2a) is the miller, so an opponent taking the
  enchantment in response does not get to mill.
- The mill routes through `mill_one`, so a creature card among the three emits
  `CreatureCardMilled` and an opponent's Undead Alchemist sees it.
- Damage is `NonCombat` and records `damaged_by`.
- Ruling 3 falls out of `face_data`, which reads the front face for a card in a
  library.

I started down a wider path and stopped: the ability arm's re-check calls
`can_be_targeted_by`, which tests hexproof and protection but *not* zone, so I
suspected a general engine gap affecting the 21 targeted activated abilities
with no zone guard of their own. Two probes — Skirsdag Cultist's damage and
Avacynian Priest's tap, against a target moved to the graveyard in response —
both behaved correctly. The zone rule is enforced through each card's
`is_valid_target`, which the arm does call. So there is no engine bug here, and
I have not invented one; the finding is only that this card was the outlier.

### Tricky interactions checked
- Ruling 1, illegal target: no mill and no damage: pass
- Ruling 2, two-card library: both milled, damage from the higher: pass
- Ruling 3, a double-faced card counts its front face: pass
- Ruling 4, all mana value 0: no damage and no damage event: pass
- Main effect: mills three, damage equals the greatest mana value: pass
- `damaged_by` records the enchantment: pass
- A creature card among the milled three is visible to mill watchers: pass
  (`multi_target_and_mill.rs:149`)

### Test coverage
- Mills three then deals damage: `cards_complex_creatures.rs:400`
- `damaged_by` tracking: `cards_complex_creatures.rs:433`
- Ruling 1 via the hook path: `cards_complex_creatures.rs:459`
- Mill watchers see the milled cards: `multi_target_and_mill.rs:149`
- **NEW** ruling 1 driven through the real activation and stack, asserting the
  library is untouched: `fizzle.rs:400`
- **NEW** ruling 2, short library: `cards_complex_creatures.rs:497`
- **NEW** ruling 4, all mana value 0: `cards_complex_creatures.rs:518`
- **NEW** ruling 3, double-faced front face: `cards_complex_creatures.rs:542`

### One test that cannot discriminate, and why
Ruling 4's is satisfied twice over: the card guards with `if max_mv > 0`, and
`damage::deal_damage` returns immediately on `amount == 0`. Removing the card's
guard changes nothing observable, so the test pins the *rule* but cannot tell
which layer enforces it. Recorded rather than dressed up as a stronger result
than it is. The other three are mutation-checked and discriminate: milling a
short library as zero, reading the back face's mana value, and neutralizing the
new `is_valid_target` each fail their test.

