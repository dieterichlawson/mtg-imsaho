## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/47/civilized-scholar-homicidal-brute
**Oracle text**: `{T}: Draw a card, then discard a card. If a creature card is discarded this way, untap this creature, then transform it.`
**Type line**: `Creature — Human Advisor` — {2}{U}, 0/1, Keywords: Transform
**Back face**: Homicidal Brute, `Creature — Human Mutant`, 5/1 — `At the beginning of your end step, if this creature didn't attack this turn, tap this creature, then transform it.`
**Rulings (3, all 2011-09-22)**:
1. You don't have priority between untapping Civilized Scholar and transforming it.
2. If Civilized Scholar attacks and later transforms that turn, Homicidal Brute's last ability won't trigger — the creature attacked that turn, even with its other face up.
3. You'll tap and transform Homicidal Brute even if it couldn't attack.

**Status**: ISSUE (2 found, both fixed)

### Code issues

1. **CR 603.4 intervening-if not checked at dispatch time** — `civilized_scholar.rs`, back-face EndStep trigger.
   - Oracle text says: `At the beginning of your end step, if this creature didn't attack this turn, tap this creature, then transform it.`
   - Code did: implemented the condition only in `on_end_step` (the resolution handler) and did not override `should_trigger`. The trait doc for that hook states the requirement directly: *"Cards whose trigger text reads 'At the beginning of ..., **if** X' ... must override this to mirror X."*
   - Effect: the trigger went on the stack even when the Brute had attacked this turn, creating a stack entry the rules say never exists and a priority window with it.
   - Reproduction: `cards_transforming_permanents.rs::homicidal_brute_that_attacked_this_turn_puts_no_trigger_on_the_stack` — failed with `left: 1, right: 0` before the fix.
   - Fixed: added `should_trigger`, factored the condition into `CivilizedScholar::attacked_this_turn` so dispatch and resolution cannot drift apart.

2. **Anti-pattern: `obj.power` consulted ahead of the card's face** — the `is_creature` check, both call sites.
   - Oracle text says: `If a creature card is discarded this way`
   - Code did: `o.power.is_some() || state.face_data(o.id, registry).is_some_and(|d| d.card_types.contains(&CardType::Creature))`
   - `obj.power` holds runtime grants only for a registry-backed card, so the short-circuit meant the authoritative face data was consulted only when the object field happened to be unset. Listed under step 9's known anti-patterns (`obj.power` instead of the registry).
   - Fixed: the check is now face data alone, factored into `is_creature_card` and shared by both call sites. `cards_shortcuts_taken.rs::civilized_scholar_detects_creature_via_registry` still passes, confirming the dropped clause was dead.

### Also changed (not a rules defect)
- `on_end_step` re-derived `state.active_player == controller`, which `step_trigger_scope` → `TriggerScope::Your` already guarantees. Removed as duplication.

### Tricky interactions checked
- **Ruling 2** (attacked as Scholar, then transformed, same turn — no transform-back): PASS. `on_attacks` is declared on *both* faces and stamps `attacked_on_turn` with the turn number, so the permanent's attack carries across the flip (CR 711.5 — transforming makes no new object).
- **Stale marker across turns**: PASS. The marker is a turn stamp compared against `state.turn_number`, not a bare flag.
- **Ruling 3** (taps and transforms even if it couldn't attack): PASS. The condition tests only `!attacked`, never "could have attacked".
- **Ruling 1** (no priority between untap and transform): PASS in effect. The code transforms then untaps — the reverse of the oracle's written order — but `apply_transform` neither reads `tapped` nor emits an event, and no player receives priority in between, so the end state is identical. Not flagged: no exact quote supports a behavioural difference.
- **Back-face P/T via `dynamic_pt`**: PASS. `dynamic_pt` sets *base* P/T in `effective_power`; counters and anthems still layer on top, so a Homicidal Brute with a +1/+1 counter is 6/2. Cross-checked all 19 ISD back-face DFCs: `back_face_data` P/T and `dynamic_pt` agree in every case (0 mismatches).
- **Draw-then-discard ordering**: PASS. The hand is collected *after* the draw, so the drawn card is a legal discard.
- **Single-card hand**: PASS. Auto-discards without a prompt — there is no choice to present.
- **Transform on a token copy**: PASS (engine-side). `apply_transform` refuses on `is_token` per CR 111.7.

### Test coverage
- Discarding a creature transforms: `cards_transforming_permanents.rs:849`
- Discarding a non-creature does not: `cards_transforming_permanents.rs:895`
- Front face has no EndStep trigger: `cards_transforming_permanents.rs:1036`
- Back face does have one: `cards_transforming_permanents.rs:1054`
- Ruling 2 (attack carries across the flip), at resolution: `cards_transforming_permanents.rs:1089`
- Ruling 2 at *dispatch* (CR 603.4): `cards_transforming_permanents.rs`, `homicidal_brute_that_attacked_this_turn_puts_no_trigger_on_the_stack` — **added by this audit**
- Creature detection via face data, not `obj.power`: `cards_shortcuts_taken.rs:462`
- Ruling 1 (untap/transform atomicity): NOT TESTED — unobservable in this engine (no priority window exists between the two writes).
- Ruling 3 (transforms even if it couldn't attack): NOT TESTED.
- Draw-then-discard lets you discard the drawn card: NOT TESTED.

### Notes for the sweep
- Research step 3 used the three Scryfall rulings as the authoritative community-knowledge source rather than WebSearch; they cover the card's known corner cases directly.
- Cross-cutting guard candidate: nothing checks that a DFC's `back_face_data` P/T agrees with its `dynamic_pt`. Verified by hand here (19/19 agree); worth a test so it stays true.
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/47/civilized-scholar-homicidal-brute?utm_source=api
**Type line**: `Creature — Human Advisor` — {2}{U}, 0/1
**Oracle text**:
```
{T}: Draw a card, then discard a card. If a creature card is discarded this way, untap this creature, then transform it.
```
**Back face**: Homicidal Brute — `Creature — Human Mutant`, 5/1
```
At the beginning of your end step, if this creature didn't attack this turn, tap this creature, then transform it.
```

**Rulings fetched**:
- [2011-09-22] You don't have priority between untapping Civilized Scholar and transforming it. You can't activate the draw-and-discard ability again, for example.
- [2011-09-22] If Civilized Scholar attacks, and later in the turn (but before the beginning of your end step), it transforms, Homicidal Brute's last ability won't trigger. This is because the creature attacked that turn, even if had its other face up at the time.
- [2011-09-22] You'll tap and transform Homicidal Brute even if it couldn't attack.

**Status**: ISSUE (fixed)

### Code issues

**The card fabricated two triggered abilities to do its own bookkeeping.**

- Oracle text (front) says: `{T}: Draw a card, then discard a card. If a creature card is discarded this way, untap this creature, then transform it.` — one activated ability, no triggered ability.
- Code declared: `TriggeredAbilityDef { kind: TriggerKind::Attacks, description: "mark as attacked this turn" }` on the front face, and the same again on the back face.

Triggered abilities go on the stack (`triggers::collect_triggers` pushes them,
`resolve_next_trigger` pops them). So every time Civilized Scholar attacked, a
stack entry reading "mark as attacked this turn" appeared — visible to both
players, with a priority window attached, for an ability the card does not
have. Step 8 and step 9 both.

Underneath it: the engine had no notion of whether a creature had attacked this
turn. `grep` for it found only this card. So the card invented the fact, and
had to store it in `card_state: HashMap<String, ObjectId>` — a map with no room
for a number — by writing `ObjectId(u64::from(state.turn_number))`, a turn
count wearing an object id's type.

Fixed in the engine, where the fact lives:
- `GameObject::attacked_on_turn: Option<u32>`, set in `combat::declare_attackers`
  and in the forced-attacker path in `engine/actions/combat.rs` — the two places
  CR 508.1 says a creature becomes an attacker.
- Cleared when the permanent leaves the battlefield (CR 400.7 — what comes back
  is a new object and has not attacked).
- `GameState::attacked_this_turn(id)`.
- Both fabricated `TriggeredAbilityDef`s and the `on_attacks` hook deleted from
  this card; `should_trigger` and `on_end_step` now read the engine's answer.

**Two smaller things, fixed inline:**

- Oracle order. `"untap this creature, then transform it"` — the code transformed
  first and untapped second. No player has priority in between (ruling
  2011-09-22), so nothing observes the difference, but the code now reads in the
  order the card is written.
- `activated_abilities` began `if obj.tapped { return vec![]; }`. The engine
  already refuses a `requires_tap` ability on a tapped permanent
  (`engine/legal/abilities.rs:146`), so this was the card re-deriving a cost
  check it does not own. Removed.

### Rulings checked

- **"You don't have priority between untapping Civilized Scholar and
  transforming it."** Both happen inside one `resolve_activated_ability` (or one
  `on_discard_choice`) with no `awaiting_action` between them, so no priority is
  offered. PASS.
- **"If Civilized Scholar attacks, and later in the turn it transforms,
  Homicidal Brute's last ability won't trigger."** The stamp is on the object,
  and CR 712.8 transforming does not make a new object, so it survives the flip.
  `should_trigger` returns false and the trigger never reaches the stack (CR
  603.4, intervening-if). PASS.
- **"You'll tap and transform Homicidal Brute even if it couldn't attack."**
  `on_end_step` asks only whether it *did* attack — there is no "could have
  attacked" check anywhere in the path. PASS.

### Tricky interactions checked

- Empty hand after the draw (library was empty): `resolve_activated_ability`
  returns before discarding. "Discard a card" with no cards does nothing. PASS.
- The just-drawn card is in the discard pool: the hand is read *after*
  `draw_cards`. PASS.
- "If a creature **card** is discarded" — asked of the card's face
  (`face_data(...).card_types`), not `obj.power`, so it is the printed type and
  a DFC in hand answers with its front face (CR 712.8a). Checked both the
  auto-discard path and the chosen-discard path; the chosen path evaluates after
  the card has moved to the graveyard, where `face_data` still answers. PASS.
- Summoning sickness on the `{T}` ability: the engine gates it
  (`legal/abilities.rs:148`, CR 302.6). PASS.
- Brute already tapped at end step: "tap this creature" on a tapped creature
  does nothing, then it transforms. Unconditional assignment. PASS.
- `should_transform` returns `false` — this is not a Werewolf; it never flips on
  its own condition. PASS.

### Test coverage

- attack-then-transform still counts: `cards_transforming_permanents.rs::an_attack_before_transforming_still_counts_for_the_back_face` — rewritten to declare the attack through `combat::declare_attackers` rather than poking the marker, since whether the engine records the attack at all is now the thing under test.
- no fabricated ability on the stack: `::civilized_scholar_attacking_puts_nothing_on_the_stack` (new).
- re-entry clears it (CR 400.7): `::returning_to_the_battlefield_clears_the_attack` (new).
- intervening-if, no phantom stack entry: `::homicidal_brute_that_attacked_this_turn_puts_no_trigger_on_the_stack`.
- transform on creature discard: `::civilized_scholar_transforms_on_creature_discard`; declining/non-creature: `::civilized_scholar_stays_front_face_on_noncreature_discard`.
- creature detection via the face, not `obj.power`: `cards_shortcuts_taken.rs:457`.
- All three new tests mutation-checked (`attacked_on_turn` stamped with a wrong turn ⇒ all three fail).

