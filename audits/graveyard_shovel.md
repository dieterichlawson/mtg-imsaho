## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/225/graveyard-shovel?utm_source=api
**Type line**: `Artifact` — {2}
**Oracle text**:
```
{2}, {T}: Target player exiles a card from their graveyard. If it's a creature card, you gain 2 life.
```

**Status**: ISSUE

### Code issues
See below.


- Three sites counted tokens as cards.
  - Oracle text says: `Target player exiles a card from their graveyard.`
  - Code did: `state.objects.values().filter(|o| o.zone == Zone::Graveyard && o.owner == *target_player)` — no `is_card`
  - CR 109.1: a token is not a card, and CR 704.5e leaves one in a graveyard
    until the next state-based-action pass, so a choice list built
    mid-resolution could offer one. The availability check and
    `is_valid_target` had the same gap. All three now ask `state.is_card`.

### Tricky interactions checked
- Ruling: "The targeted player chooses which card to exile when the ability
  resolves" — the `ResolutionChoice` is presented to the *targeted* player, not
  the Shovel's controller: PASS
- "If it's a creature card, **you** gain 2 life" — the life goes to the Shovel's
  controller, not the targeted player, and emits `LifeChanged`: PASS
- With exactly one card there is no choice to present: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- A token is not offered as a choice: `token_is_not_a_card.rs:a_token_in_a_graveyard_is_not_offered_as_a_card_to_choose`
- Exile and life gain: `cards_lands_and_mana_sources.rs:graveyard_shovel_exiles_and_gains_life`, `graveyard_shovel.rs`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/225/graveyard-shovel?utm_source=api
**Type line**: `Artifact` — {2}
**Oracle text**:
```
{2}, {T}: Target player exiles a card from their graveyard. If it's a creature card, you gain 2 life.
```

**Rulings fetched**:
- [2011-09-22] The targeted player chooses which card to exile when the ability resolves.

**Status**: ISSUE (fixed)

### Code issues

Two found, both fixed.

1. **"you gain 2 life" paid whoever held the Shovel at resolution, not the player who activated the ability.** `graveyard_shovel.rs:64` and `:124` (before the fix)
   - Oracle text says: `{2}, {T}: Target player exiles a card from their graveyard. If it's a creature card, you gain 2 life.`
   - Code did: `let controller = crate::cards::helpers::controller_of(state, object_id);` — the source's controller, read at resolution.
   - CR 602.2a fixes an ability's controller when it is activated. Take the Shovel in response and the old code paid the thief the 2 life.
   - Now `helpers::ability_controller`, which reads the activator recorded on the stack entry.

2. **The forced case was a second copy of the effect, and the two had drifted.** `graveyard_shovel.rs:77-95` (before the fix)
   - With exactly one card in the graveyard the choice is forced, so the card exiled it inline instead of routing through `resolve_card_effect`. The copy decided "creature card" by reading `face_data(...).card_types` where the other asked `state.is_creature`, and wrote the life total with `change_life` where the other used `gain_life`.
   - Neither difference changes an outcome today (`gain_life` is `change_life`, and a card in a graveyard has no runtime grants to tell the two type checks apart) — which is exactly why it would have gone unnoticed if one of them started to matter. Only one copy was reachable by the test covering the life gain.
   - The forced case now calls the same `resolve_card_effect` the chosen case does.

**Set-wide follow-up.** Issue 1 is not this card's alone. Nineteen cards read `controller_of` inside `resolve_activated_ability`, all with the same meaning and the same bug. `helpers::ability_controller` now answers the question, falling back to the source's last known controller for triggers and for effects reached outside a resolving activated ability, and all nineteen go through it. The activator also had to survive a choice the ability raises — Graveyard Shovel's own targeted-player choice is the case — so `resolving_ability_activator` is held while such a choice is pending and cleared once the chain runs out.

### Checked against the ruling

- `The targeted player chooses which card to exile when the ability resolves.` — PASS. The `ResolutionChoice` is raised with `player: *target_player`, not the Shovel's controller, and the choice is set up during resolution rather than at activation. Already tested.

### Checked and correct

- Cost `{2}`, `Artifact`, no subtypes, oracle text verbatim.
- Ability cost `{2}` plus `requires_tap: true` for `{2}, {T}`.
- `TargetRequirement::PlayerOnly` — "target **player**", and the test that the offered targets are all players is already there.
- `is_valid_target` narrows to players who actually have a card in their graveyard, so a player with an empty graveyard is not a legal target and the ability cannot be activated pointing at them (CR 601.2c).
- "a **card**" is `state.is_card` throughout, so a token sitting in a graveyard until the next state-based action check is not exiled and does not make the ability activatable (CR 109.1).
- The exile is `move_object(..., Zone::Exile, ...)` — not a destroy, not a sacrifice.
- The life gain goes through `gain_life`, which emits `LifeChanged`.
- The 2 life is conditional on the exiled card being a creature card, and that is read before the card moves.
- The card does not clean up its own spell (it has none — it is an activated ability).

### Tricky interactions checked

- Exactly one card in the graveyard: forced, and now on the same code path. PASS.
- Several cards: the targeted player chooses. PASS.
- Non-creature exiled: no life. PASS.
- Token in the graveyard: not a card, not exiled, does not enable the ability. PASS.
- Shovel changes hands in response: the activator gains the life. PASS (after fix).
- Shovel changes hands, and the choice is answered afterwards: still the activator. PASS (after fix) — this is the half that needed the activator to outlive the ability's own resolution.
- Targeted player's graveyard emptied in response: the ability fizzles on the `is_valid_target` re-check (CR 608.2b), and the resolution guard returns if it somehow gets there.

### Test coverage

- targets players, not cards: `graveyard_shovel.rs:15`
- forced single card is exiled and pays 2 life: `graveyard_shovel.rs:43`
- no life for a non-creature: `graveyard_shovel.rs:65`
- the targeted player is the one asked: `graveyard_shovel.rs:85`
- the chosen card is exiled, the other stays, life is paid: `graveyard_shovel.rs:111`
- a player with an empty graveyard cannot be targeted: `graveyard_shovel.rs:144`
- a token in a graveyard is not a card: `token_is_not_a_card.rs:267`
- the life goes to the activator, not the new controller: `graveyard_shovel.rs` `the_life_goes_to_whoever_activated_it_not_whoever_holds_the_shovel` (NEW, mutation-checked)
- and still does when the choice is answered later: `graveyard_shovel.rs` `the_life_still_goes_to_the_activator_after_the_choice` (NEW, mutation-checked against clearing the activator early)

