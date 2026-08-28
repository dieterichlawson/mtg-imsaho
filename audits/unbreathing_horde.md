## Audit — 2026-08-27 (Tier C — one behaviour hook: replacement effect)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/121/unbreathing-horde?utm_source=api
**Type line**: `Creature — Zombie` — {2}{B}, 0/0
**Oracle text**:
```
This creature enters with a +1/+1 counter on it for each other Zombie you control and each Zombie card in your graveyard.
If this creature would be dealt damage, prevent that damage and remove a +1/+1 counter from it.
```
**Status**: PASS

### Code issues
No issues found.

### What was checked
Card data was verified exact set-wide (see `ISD_AUDIT_PROGRESS.md`). This card's
one hook is `replace_event`, so the audit centres on CR 614 — whether the effect
applies to the right events, exactly once, and modifies rather than replaces
where the oracle says "instead".

- The enters-with count splits the oracle's two halves exactly where CR 109.1
  does: "each other **Zombie** you control" counts tokens, "each Zombie **card**
  in your graveyard" must not, and the code filters the graveyard half through
  `state.is_card`. Getting this backwards is the obvious mistake and it does not
  make it.
- The Horde counting *itself* when it enters from a graveyard is correct per the
  Scryfall ruling, and falls out of the callback running before the zone change.
- The second oracle clause, "If this creature would be dealt damage, prevent
  that damage and remove a +1/+1 counter from it", is not in `replace_event` at
  all — it is a declarative `ContinuousEffect::PreventDamageRemoveCounter` with
  `EffectScope::OnSelf`, which is the right shape for a static replacement.

### Test coverage
`damage_pipeline.rs` (the prevent-and-remove-counter path), `cards_complex_creatures.rs` (enters-with count), `token_is_not_a_card.rs` (the CR 109.1 split)
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/121/unbreathing-horde?utm_source=api
**Type line**: `Creature — Zombie` — {2}{B}, 0/0
**Oracle text**:
```
This creature enters with a +1/+1 counter on it for each other Zombie you control and each Zombie card in your graveyard.
If this creature would be dealt damage, prevent that damage and remove a +1/+1 counter from it.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "If Unbreathing Horde enters **from a graveyard**, it will count
  itself." The counter calculation runs in `replace_event` — *before* the zone
  change (CR 616.1) — so the Horde is still in the graveyard and is counted:
  PASS
- "each **other** Zombie you control" counts tokens (it says Zombies, not
  cards); "each Zombie **card** in your graveyard" does not (CR 109.1). The two
  halves are filtered differently, which is the whole subtlety: PASS
- Ruling: "**Only one** +1/+1 counter will be removed, no matter how much damage
  is prevented": PASS
- Ruling: "If Unbreathing Horde has **no** +1/+1 counters on it (but its
  toughness is raised above 0 by another effect), any damage dealt to it will
  **still be prevented**, even though no counter will be removed." The
  prevention returns true whenever the effect is present, whether or not a
  counter came off: PASS
- "enters with ... counters" is a replacement effect (CR 614.1c), so the Horde
  never exists as a 0/0 that state-based actions could kill: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The entering count from either zone, and the prevention: `cards_complex_creatures.rs`, `token_is_not_a_card.rs:zombie_token_in_graveyard_not_counted`, `:zombie_card_in_graveyard_still_counted`, `:zombie_token_on_the_battlefield_is_still_counted`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/121/unbreathing-horde?utm_source=api
**Type line**: `Creature — Zombie` — {2}{B}, 0/0
**Oracle text**:
```
This creature enters with a +1/+1 counter on it for each other Zombie you control and each Zombie card in your graveyard.
If this creature would be dealt damage, prevent that damage and remove a +1/+1 counter from it.
```

**Rulings fetched**:
- [2011-09-22] Only one +1/+1 counter will be removed, no matter how much damage is prevented.
- [2011-09-22] If Unbreathing Horde has no +1/+1 counters on it (but its toughness is raised above 0 by another effect), any damage dealt to it will still be prevented, even though no counter will be removed.
- [2011-09-22] If Unbreathing Horde enters from a graveyard, it will count itself when determining how many +1/+1 counters it enters with.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/121/unbreathing-horde
**Oracle text**:
```
This creature enters with a +1/+1 counter on it for each other Zombie you control and each Zombie card in your graveyard.
If this creature would be dealt damage, prevent that damage and remove a +1/+1 counter from it.
```
**Type line**: Creature — Zombie
**Mana cost**: {2}{B} — **P/T**: 0/0
**Rulings** (3, all 2011-09-22):
1. "Only one +1/+1 counter will be removed, no matter how much damage is prevented."
2. "If Unbreathing Horde has no +1/+1 counters on it (but its toughness is raised above 0 by another effect), any damage dealt to it will still be prevented, even though no counter will be removed."
3. "If Unbreathing Horde enters from a graveyard, it will count itself when determining how many +1/+1 counters it enters with."

**Status**: ISSUE (fixed)

### Card data
Matches the fetched text field for field: `{2}{B}`, `card_types: [Creature]`,
`subtypes: ["Zombie"]`, 0/0, oracle text verbatim (the current "This creature
enters with…" errata wording, not the old "enters the battlefield with"), and
`continuous_effects: [PreventDamageRemoveCounter { scope: OnSelf }]` for the
second line. No keywords, no triggered abilities declared — correct, the card
has neither.

### Code issues

1. Ruling 2 was untested, and nothing held the implementation to it
   (`inline_damage.rs`, test added).
   - Ruling says: `any damage dealt to it will still be prevented, even though no counter will be removed`
   - `damage.rs:238` gets this right — `if counter_count > 0 { ..remove one.. }`
     then `true` unconditionally — but making it
     `if counter_count == 0 { return false; }` broke **nothing** in the whole
     workspace. The counter is what the prevention *does*, not a condition on
     doing it, and that distinction had no test.
   - Added `the_damage_is_prevented_even_with_no_counter_left_to_remove`. The
     Horde has zero counters and a toughness raised above 0, which is the
     situation the ruling itself describes. That mutation now fails it.

2. The graveyard count said the same thing twice
   (`unbreathing_horde.rs:63-77`, collapsed).
   - Oracle text says: `each Zombie card in your graveyard` — no "other", and
     ruling 3 makes the self-inclusion explicit.
   - The code filtered `o.id != self_id` out of the count and then added
     `let self_count = u32::from(self_in_gy);` back on. Subtracting the Horde
     and adding it again is the same number by a longer road.
   - Now one plain count. Verified equivalent-and-covered: re-introducing the
     `o.id != self_id` exclusion (without the re-add) fails
     `bug_ac_unbreathing_horde_counts_itself_when_reanimated` and
     `a_replacement_about_your_own_arrival_applies_from_any_zone`.

3. The reanimation test called a hook that no longer exists
   (`cards_complex_creatures.rs`, repaired).
   - It did `state.move_object(horde, Zone::Battlefield, &registry);` and then
     `behavior.on_enter_battlefield(&mut state, horde, &[], &registry);`.
     That was right when the count lived in an ETB hook. The count is a
     `replace_event` now, so the call reached the trait default and did nothing;
     `move_object` applies the entering replacement itself, via `plan_entering`
     and `replacement::for_entering`.
   - Its doc comment still described the old failure mode as current, and the
     assertion was `counters >= 3` where the answer is exactly 3. Dropped the
     dead call, rewrote the comment, tightened to `== 3`.

4. Two ETB tests were one test twice (`cards_complex_creatures.rs`, merged).
   - `unbreathing_horde_enters_with_counters_for_zombies` (line ~1010) and
     `enters_with_correct_counter_count` (line ~2630) sat fifteen hundred lines
     apart in the same file with the same shape — two battlefield Zombies, one
     graveyard Zombie, assert 3 — and nothing said what made them different.
     One builds its Zombies as tokens (subtypes on the object), the other as
     cards (subtypes on the registry face); both paths matter, and both were
     invisible.
   - Merged into `enters_with_a_counter_per_zombie_however_the_zombie_is_a_zombie`,
     which runs both and names the difference.

### Tricky interactions checked
- **Ruling 1** (one counter per damage event, whatever the amount): 13 damage
  to a Horde with 3 counters leaves 2. PASS —
  `prevent_and_remove_a_counter_replaces_the_damage`.
- **Ruling 2** (no counters, damage still prevented): PASS — new test.
- **Ruling 3** (counts itself from a graveyard): PASS —
  `bug_ac_unbreathing_horde_counts_itself_when_reanimated`, and the engine's
  own `a_replacement_about_your_own_arrival_applies_from_any_zone`.
- Zombie **tokens** on the battlefield count ("each other Zombie you control",
  not "card"): PASS.
- Zombie **tokens in a graveyard** do not count ("each Zombie card",
  CR 109.1 — a token is not a card; it sits there until the next SBA pass):
  PASS — dropping `state.is_card` fails `zombie_token_in_graveyard_not_counted`
  and `a_card_enumerating_a_graveyard_excludes_tokens`.
- Non-combat damage is prevented too — it is a replacement effect, not a combat
  one: PASS. Gating it on `kind == Combat` fails five tests including
  `prevent_and_remove_a_counter_replaces_the_damage` and
  `fight_damage_respects_prevent_damage_remove_counter`.
- The Horde still *deals* its damage normally: PASS —
  `still_deals_damage_to_others`.
- "your graveyard" is the controller's, read by owner:
  `objects_in_zone(Graveyard, controller)` filters `obj.owner == player`
  (`state.rs:1066`), which is the right reading — a graveyard is a player's
  zone. The battlefield half filters `obj.controller == player`
  (`state.rs:1067`), matching "you control". PASS.
- "each **other** Zombie you control": unreachable, and now documented as such.
  CR 616.1 works the entering event out before the zone change, so the Horde is
  never among the battlefield objects at that moment. Dropping the `o.id !=
  self_id` filter fails no test and cannot be made to. Kept because it is the
  card's own word; the comment says it is not load-bearing so nobody reads its
  lack of coverage as a gap.
- Entering with 0 counters (no Zombies anywhere): `enters_with_counters` returns
  `None` for an empty list rather than a zero-counter modification, so the
  entering event is untouched. PASS.
- Damage to a Horde that has become a planeswalker-like loyalty holder: not
  applicable, but the prevention runs before the loyalty branch in
  `deal_damage_to_object`, which is what
  `a_planeswalker_keeps_its_loyalty_when_the_damage_is_prevented` checks. PASS.

### UI presentation
No choices, no triggered ability on the stack. The prevention logs
`"{name}: damage prevented, removed a +1/+1 counter"` (`damage.rs:254`) — and
only when a counter actually came off, which matches ruling 2.

### Test coverage
- Ruling 1: `inline_damage.rs` (`prevent_and_remove_a_counter_replaces_the_damage`)
  and `cards_complex_creatures.rs` (`prevents_combat_damage_removes_counter`).
- Ruling 2: `inline_damage.rs`
  (`the_damage_is_prevented_even_with_no_counter_left_to_remove`) —
  **added this audit**.
- Ruling 3: `cards_complex_creatures.rs`
  (`bug_ac_unbreathing_horde_counts_itself_when_reanimated`) —
  **repaired this audit**.
- Counter count, token and card Zombies: `cards_complex_creatures.rs`
  (`enters_with_a_counter_per_zombie_however_the_zombie_is_a_zombie`) —
  **merged this audit**.
- Graveyard tokens excluded: `zombie_token_in_graveyard_not_counted`,
  `a_card_enumerating_a_graveyard_excludes_tokens`.
- Non-combat damage prevented: `prevent_and_remove_a_counter_replaces_the_damage`,
  `fight_damage_respects_prevent_damage_remove_counter`,
  `harvest_pyres_chosen_x_still_goes_through_the_pipeline`,
  `blasphemous_act_consults_each_creature_separately`.
- Combat damage prevented while its own damage lands:
  `prevents_combat_damage_removes_counter`, `still_deals_damage_to_others`.

### Mutations run
| mutation | result |
| --- | --- |
| `damage.rs`: `return false` when no counters | fails only the new ruling-2 test (before it: **nothing at all**) |
| `damage.rs`: prevention only for `DamageKind::Combat` | fails 5 tests |
| card: exclude `self_id` from the graveyard count | fails the reanimation test and `a_replacement_about_your_own_arrival_applies_from_any_zone` |
| card: drop `state.is_card` from the graveyard count | fails `zombie_token_in_graveyard_not_counted` and `a_card_enumerating_a_graveyard_excludes_tokens` |
| card: drop `o.id != self_id` from the **battlefield** count | **nothing** — unreachable by CR 616.1, now documented in the card |

Suite after: 1440 passing, exit 0, zero warnings. (1441 mid-audit; the merge of
two duplicate tests into one parametric took it back to 1440.)

