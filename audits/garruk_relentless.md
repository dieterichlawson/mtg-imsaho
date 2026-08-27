## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/181/garruk-relentless-garruk-the-veil-cursed
**Type line**: `Legendary Planeswalker — Garruk` — {3}{G}, starting loyalty 3
**Oracle text (front)**:
```
When Garruk has two or fewer loyalty counters on him, transform him.
0: Garruk deals 3 damage to target creature. That creature deals damage equal to its power to him.
0: Create a 2/2 green Wolf creature token.
```
**Back face**: Garruk, the Veil-Cursed, `Legendary Planeswalker — Garruk`
```
+1: Create a 1/1 black Wolf creature token with deathtouch.
−1: Sacrifice a creature. If you do, search your library for a creature card, reveal it, put it into your hand, then shuffle.
−3: Creatures you control gain trample and get +X/+X until end of turn, where X is the number of creature cards in your graveyard.
```
**Rulings (6, all 2011-09-22)**: state-triggered, can't retrigger while on the stack; loyalty is unchanged by the transform; one loyalty ability per turn across both faces; the −1 doesn't target but is mandatory if you control a creature; X is counted on resolution; only creatures you control on resolution get the bonus.

**Status**: ISSUE (1 found, fixed) + 1 cleanup

### Code issues

1. **`on_resolve` re-implemented the engine's permanent resolution, including writing printed characteristics onto the object** — `garruk_relentless.rs`.
   - Code did:
     ```rust
     state.move_object(object_id, Zone::Battlefield, registry);
     state.add_counters(object_id, CounterType::Loyalty, 3);
     obj.card_types = vec![CardType::Planeswalker];
     obj.is_legendary = true;
     ```
   - `obj.card_types` holds runtime *grants* for a registry-backed card; the printed type comes from the active face. Writing it there is the anti-pattern the characteristics model exists to remove. The card also declares `starting_loyalty() -> Some(3)`, so the loyalty line duplicated it — and Liliana of the Veil, the set's other planeswalker, declares `starting_loyalty` and has no `on_resolve` at all.
   - Verified rather than assumed: removing the whole method and casting Garruk gives `zone=Battlefield loyalty=3 legendary=true types=[Planeswalker] transformed=false` — byte-identical to before — with the full suite green.
   - Fixed: method removed. Garruk now resolves down the same path as every other permanent.

2. **Redundant filter clause** (cleanup, not a defect) — the −1 ability filtered `state.has_card_type(o.id, Creature, registry) || state.is_creature(o.id, registry)`. `is_creature` already contains the first clause. Collapsed.

### Tricky interactions checked
- **Ruling 1** (state trigger, CR 603.8): PASS. Implemented via `state_trigger_condition` / `on_state_trigger` rather than a step trigger, and the condition includes `!o.is_transformed`, which is what stops it retriggering.
- **Ruling 2** (transform does not change loyalty): PASS. `on_state_trigger` touches `is_transformed` and the name only, never the counters.
- **Ruling 3** (one loyalty ability per turn *across the transform*): PASS, and now tested. The two faces number their abilities 0/1 and 10/11/12, so a per-index limit would let a player use one on each face; the engine tracks a per-permanent sentinel (`abilities_activated_this_turn.insert(999)`), and transforming does not clear it. CR 606.3 + CR 711.5.
- **Ruling 4** (the −1 does not target): PASS. The candidate list is built directly from the battlefield with no `can_be_targeted_by` filter, so hexproof and protection correctly do not apply — it is a choice, not a target, despite the `ChooseTarget` prompt shape.
- **Ruling 5** (X counted on resolution): PASS. X is computed inside the handler.
- **Ruling 6** (only creatures controlled on resolution): PASS. The list is snapshotted before the effects are pushed, so a creature entering later gets nothing.
- **Front-face 0: the fight-back damage**: PASS. Reads `state.effective_power` (not `obj.power`), captures it before Garruk's damage lands, and skips the return damage at 0 power. Damage to Garruk goes through `apply_pending_effect` → the damage pipeline, so CR 120.3c loyalty removal, protection and prevention all apply.
- **Creature dies to the 3 damage but still deals its own**: PASS by construction — SBAs do not run mid-resolution (CR 704.3), and the power was captured first.
- **Token subtypes**: PASS. Both Wolf abilities use `create_token_with_subtypes` with `["Wolf"]`.

### Test coverage
- Front-face Wolf token: `cards_transforming_permanents.rs`, `garruk_creates_wolf_token`
- Transform at ≤2 loyalty: `garruk_transforms_at_two_or_fewer_loyalty`
- Back-face deathtouch Wolf: `garruk_back_face_creates_deathtouch_wolf`
- Back-face −1 sacrifice + tutor, and its choice prompt: `garruk_back_face_sacrifice_to_tutor`, `garruk_back_face_tutor_presents_sacrifice_choice`, `garruk_back_face_tutor_shuffles_library`
- Back-face −3: `garruk_back_face_overrun`
- Back-face abilities offered only when transformed: `garruk_back_face_loyalty_abilities_shown_when_transformed`
- Damage respects protection: `garruk_damage_respects_protection`
- Legendary on the battlefield: `bug_garruk_relentless_not_legendary_on_battlefield`
- **Ruling 3** (one loyalty ability across the flip): `garruk_cannot_activate_a_loyalty_ability_on_each_face_in_one_turn` — **added by this audit**
- Ruling 5 (X fixed on resolution, not re-read later): NOT TESTED
- Ruling 6 (creature entering after resolution gets no bonus): NOT TESTED
- Ruling 1 (cannot retrigger while the transform trigger is on the stack): NOT TESTED
## Audit — 2026-08-27 — CR 109.1: a token in a graveyard is not a card

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/181/garruk-relentless-garruk-the-veil-cursed?utm_source=api
**Oracle text**:
```
When Garruk has two or fewer loyalty counters on him, transform him.
0: Garruk deals 3 damage to target creature. That creature deals damage equal to its power to him.
0: Create a 2/2 green Wolf creature token.
```
**Status**: ISSUE (fixed)

### Code issue
- Oracle text says: a **card** in a graveyard (`see above`)
- Code did: filtered the graveyard by creature-ness alone, with no card/token distinction.
- CR 109.1: a token is not a card. CR 111.7 removes a token from a graveyard as
  a state-based action, so between the moment it dies and the next SBA check it
  really is sitting there — the same window a dies-trigger sees. Measured
  directly on Boneyard Wurm: 2/2 with one creature card and one just-died token
  in the yard, 1/1 the instant SBAs ran. The oracle's answer is 1/1 throughout.
- Fixed: the graveyard filter now goes through `state.is_card`.

### How this was found
A sweep for cards whose oracle says "cards in a graveyard" against code that
never distinguishes tokens. Thirteen cards matched; five already guarded
(Gnaw to the Bone, Moorland Haunt, Past in Flames, Runechanter's Pike,
Splinterfright) and eight did not.

Splinterfright and Boneyard Wurm are the instructive pair — near-identical
text, adjacent in the set. `token_is_not_a_card.rs::cda_does_not_count_tokens_in_graveyard`
covered Splinterfright, which is why Splinterfright alone had the guard.

### Test coverage
`token_is_not_a_card.rs::a_token_in_a_graveyard_is_not_a_creature_card` —
**added by this audit**, covers Boneyard Wurm and Splinterfright together and
fails against the unfixed code.
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/181/garruk-relentless-garruk-the-veil-cursed?utm_source=api
**Type line**: `Legendary Planeswalker — Garruk` — {3}{G}
**Oracle text**:
```
When Garruk has two or fewer loyalty counters on him, transform him.
0: Garruk deals 3 damage to target creature. That creature deals damage equal to its power to him.
0: Create a 2/2 green Wolf creature token.
```
**Back face**: Garruk, the Veil-Cursed — `Legendary Planeswalker — Garruk`
```
+1: Create a 1/1 black Wolf creature token with deathtouch.
−1: Sacrifice a creature. If you do, search your library for a creature card, reveal it, put it into your hand, then shuffle.
−3: Creatures you control gain trample and get +X/+X until end of turn, where X is the number of creature cards in your graveyard.
```

**Rulings fetched**:
- [2011-09-22] Garruk Relentless's first ability is a state-triggered ability. It triggers once Garruk has two or fewer loyalty counters on him and it can't retrigger while that ability is on the stack.
- [2011-09-22] You don't add or remove loyalty counters from Garruk Relentless when he transforms into Garruk, the Veil-Cursed. In most cases, he'll have one or two loyalty counters on him.
- [2011-09-22] You can't activate a loyalty ability of Garruk Relentless and later that turn after he transforms activate a loyalty ability of Garruk, the Veil-Cursed.
- [2011-09-22] The second ability of Garruk, the Veil-Cursed doesn't target a creature. However, when that ability resolves, you must sacrifice a creature if you control one.
- [2011-09-22] The number of creature cards in your graveyard is counted when the third ability of Garruk, the Veil-Cursed resolves. Once the ability resolves, the bonus doesn't change if that number changes later in the turn.
- [2011-09-22] Only creatures you control when the third ability of Garruk, the Veil-Cursed resolves will receive the bonus. Creatures that enter or that you gain control of later in the turn won't be affected.

**Status**: ISSUE (fixed)

### Code issues

**1. The back face was never declared.**

Garruk was the only double-faced card in the set that modelled its second face
by branching on `is_transformed` instead of declaring `back_face_data()`. The
name was written into `obj.name` by hand on transform:

```rust
obj.is_transformed = true;
obj.name = "Garruk, the Veil-Cursed".into();
```

`obj.name` is a display cache — the doc comment on `GameState::name_of` says so
outright. The authoritative read goes through `face_data`, which falls back to
the front face when a card declares no back one. So a transformed Garruk
answered:

- `state.name_of(garruk, &reg)` → `"Garruk Relentless"`
- `state.face_data(garruk, &reg).oracle_text` → the *front* face's rules text

which reaches the legend rule (CR 704.5j) and anything matching on names.
Verified by a test written to fail first.

Fixed by declaring `back_face_data()` — Garruk, the Veil-Cursed, Legendary
Planeswalker — Garruk, with the back face's oracle text — and replacing the
hand-flip in `on_state_trigger` with `helpers::apply_transform`, so there is one
definition of what transforming does. The five Garruk tests that hand-set
`is_transformed` *and* `obj.name` now transform through that helper; the second
assignment was them encoding the workaround.

Guarded: `card_data_invariants.rs::every_card_with_a_back_face_declares_it`
cross-checks every implemented card against `data/oracle_cache.json` — an
independent, fetched source — for both the presence of the back face and its
name. Mutation-checked.

**2. The −1 ability had two implementations.**

`sacrifice_and_tutor` (taken when you control exactly one creature) and
`resolve_card_effect` (taken when you choose) did the same thing, and had
already drifted: one read the sacrificed creature's name with
`get_object(..).name`, the other with `state.obj_name(..)`. The one-creature
branch now calls `resolve_card_effect` directly — there is no choice to make,
but it is the same effect.

### Rulings checked

- **"Garruk Relentless's first ability is a state-triggered ability. It triggers
  once Garruk has two or fewer loyalty counters on him and it can't retrigger
  while that ability is on the stack."** Implemented as `state_trigger_condition`
  (CR 603.8), not an upkeep or ETB trigger. The no-retrigger half is the
  engine's `state_trigger_on_stack` flag: `sba.rs:226` sets it as the ability
  goes on the stack, `triggers.rs:567` clears it on resolution, and `sba.rs:213`
  filters on it. PASS.
- **"You don't add or remove loyalty counters from Garruk Relentless when he
  transforms."** `apply_transform` touches `is_transformed` and the name cache,
  nothing else — in particular not `counters`. PASS.
- **"You can't activate a loyalty ability of Garruk Relentless and later that
  turn after he transforms activate a loyalty ability of Garruk, the
  Veil-Cursed."** The once-per-turn gate is keyed on the *permanent*
  (`abilities_activated_this_turn.contains(&999)`, a per-object sentinel), not
  on the ability index — so front-face index 0 blocks back-face index 10 for the
  rest of the turn. Had it been keyed per index, the front and back faces would
  have had separate allowances. PASS.
- **"The second ability of Garruk, the Veil-Cursed doesn't target a creature.
  However, when that ability resolves, you must sacrifice a creature if you
  control one."** `target_requirement: None` on ability 11 — it does not target,
  so it cannot be fizzled by removing the creature in response. On resolution
  the choice is `optional: false`. With no creatures it logs and does nothing,
  and — "**If you do**, search your library" — performs no search. PASS.
- **"The number of creature cards in your graveyard is counted when the third
  ability resolves. Once the ability resolves, the bonus doesn't change if that
  number changes later in the turn."** X is computed at resolution and pushed as
  fixed `ModifyPT` values. `is_card` filters tokens out of the count (CR 109.1).
  PASS.
- **"Only creatures you control when the third ability resolves will receive the
  bonus."** The battlefield is enumerated once at resolution (CR 611.2c). PASS.

### Tricky interactions checked

- **The 0 ability is the card's own kill switch.** "That creature deals damage
  equal to its power to him" — damage to a planeswalker removes that many
  loyalty counters (`damage.rs:96`, CR 120.3c), which is what drops Garruk to
  ≤2 and fires the state trigger. Confirmed the planeswalker branch exists and
  is taken by card type. PASS.
- **A creature killed by Garruk's 3 damage still deals its damage back.** Both
  halves happen inside one `on_loyalty_ability` with no priority between them,
  and state-based actions run only when a player would receive priority — so the
  creature is still on the battlefield with lethal damage marked when it deals
  its own. The code does not check whether it survived, which is correct. PASS.
- Power is snapshotted *before* Garruk's damage rather than read at the moment
  the creature deals its own. Nothing in this pool changes a creature's power in
  response to damage during a single resolution, and marked damage does not
  reduce power, so the two readings cannot differ. Noted, not changed.
- **Both front abilities cost `0:`.** That does not make them free of CR 606.3 —
  the once-per-turn gate is on activating *a* loyalty ability, and the sentinel
  is set regardless of the loyalty change. PASS.
- **−1 with no creatures.** Legal to activate (the loyalty cost is payable), and
  resolves to nothing. PASS.
- The back face has no triggered abilities, so declaring `back_face_data()` adds
  none — checked against `triggers.rs:270`/`286`, which take the back face's
  `triggered_abilities` when transformed. Back-face P/T is `None` for a
  planeswalker, so the `effective_power` back-face branch is inert. PASS.

### Test coverage

- back face reported after transform: `cards_transforming_permanents.rs::a_transformed_garruk_reports_his_back_face` (new — written to fail first).
- every DFC declares its back face: `card_data_invariants.rs::every_card_with_a_back_face_declares_it` (new, mutation-checked).
- state trigger at ≤2 loyalty: `cards_transforming_permanents.rs:599-640`.
- back-face +1 deathtouch Wolf, −1 sacrifice-and-tutor (both the single-creature and chosen paths), −3 count and trample: `cards_transforming_permanents.rs:643-860`, now transforming through `helpers::apply_transform` rather than hand-setting the flag and the name.
- front-face 0 fight-back and loyalty loss: `damage_pipeline.rs`, `damage_helper.rs`.

