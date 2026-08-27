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
