## Audit — 2026-08-27 — CR 109.1: a token in a graveyard is not a card

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/226/grimoire-of-the-dead?utm_source=api
**Oracle text**:
```
{1}, {T}, Discard a card: Put a study counter on Grimoire of the Dead.
{T}, Remove three study counters from Grimoire of the Dead and sacrifice it: Put all creature cards from all graveyards onto the battlefield under your control. They're black Zombies in addition to their other colors and types.
```
**Status**: ISSUE (fixed)

### Code issue
- Oracle text says: a **card** in a graveyard (`{T}, Remove three study counters from Grimoire of the Dead and sacrifice it: Put all creature cards from all graveyards onto the battlefield under your control. They're black Zombies in addition to their other colors and types.`)
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
## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/226/grimoire-of-the-dead?utm_source=api
**Type line**: `Legendary Artifact` — {4}
**Oracle text**:
```
{1}, {T}, Discard a card: Put a study counter on Grimoire of the Dead.
{T}, Remove three study counters from Grimoire of the Dead and sacrifice it: Put all creature cards from all graveyards onto the battlefield under your control. They're black Zombies in addition to their other colors and types.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "{1}, {T}, Discard a card: Put a study counter on this artifact" — the discard
  is a cost, paid on activation: PASS
- "{T}, Remove three study counters ... Sacrifice this artifact: Put all
  creature cards from all graveyards onto the battlefield under your control" —
  removing exactly three leaves any surplus to be lost to the zone change rather
  than swallowed by the sacrifice, which is why the counter cost is paid before
  the sacrifice: PASS
- "all creature **cards** from all graveyards" — CR 109.1, `state.is_card`: PASS
- "under **your** control" — the Grimoire's controller, not each card's owner:
  PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The discard/counter loop and the reanimation: `cards_complex_creatures.rs:grimoire_discard_presents_choice_and_adds_study_counter`, `:grimoire_accumulates_three_study_counters`, `:grimoire_reanimates_all_graveyard_creatures`, `:grimoire_single_card_in_hand_auto_discards`
## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/226/grimoire-of-the-dead?utm_source=api
**Type line**: `Legendary Artifact` — {4}
**Oracle text**:
```
{1}, {T}, Discard a card: Put a study counter on Grimoire of the Dead.
{T}, Remove three study counters from Grimoire of the Dead and sacrifice it: Put all creature cards from all graveyards onto the battlefield under your control. They're black Zombies in addition to their other colors and types.
```

**Status**: ISSUE

### Code issues
See below.


- Same dead `on_resolve` override as Geist of Saint Traft — move plus
  `is_legendary`, both the trait default's job. Deleted.

### Tricky interactions checked
- Covered in full in the previous entry; nothing else changed.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The discard/counter loop and the reanimation: `cards_complex_creatures.rs`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/226/grimoire-of-the-dead?utm_source=api
**Type line**: `Legendary Artifact` — {4}
**Oracle text**:
```
{1}, {T}, Discard a card: Put a study counter on Grimoire of the Dead.
{T}, Remove three study counters from Grimoire of the Dead and sacrifice it: Put all creature cards from all graveyards onto the battlefield under your control. They're black Zombies in addition to their other colors and types.
```

**Rulings fetched**:
- [2011-09-22] “Creature cards” includes each card with the type creature, even if it has additional types, such as artifact.

**Status**: ISSUE (fixed)

### Code issues

Two found, both fixed.

1. **The discard happened on resolution, but it is a cost.** `grimoire_of_the_dead.rs:74` (before the fix)
   - Oracle text says: `{1}, {T}, Discard a card: Put a study counter on Grimoire of the Dead.`
   - Code did the discard inside `resolve_activated_ability` — `state.discard_card(card_id, registry)` for the one-card case, and a `ChooseCardFromHand` prompt for the rest, both on the resolution side.
   - Everything before the colon is cost, paid on activation (CR 601.2h via 602.2b). Doing it on resolution puts it on the far side of the priority window: an opponent responding to the ability still saw the card in hand, and countering the ability would have taken the discard back with it.
   - Now in `pay_activation_cost`, where Moorland Haunt and Blazing Torch already pay theirs. The study counter — the part after the colon — stays on resolution. The choice of which card to discard is made while paying (CR 601.2b), which that hook supports.

2. **The card was stamping `is_legendary` on the creatures it reanimated.** `grimoire_of_the_dead.rs:142` (before the fix)
   - Code did: `obj.is_legendary = is_legendary;` after `move_object_under_control`, computing it from the face itself.
   - That is the card compensating for an engine gap, and it is the shape worth chasing: a derivable property cached on the object, which every non-standard way onto the battlefield has to remember to recompute. Removed — the legend rule reads the face now.

### The engine gap behind issue 2

`obj.is_legendary` is a cache filled in by exactly one path: the default `on_resolve` for a permanent spell (`cards/mod.rs:747`). Every other way onto the battlefield — reanimation, Fiend Hunter's return, a blink, a copy — had to stamp it or CR 704.5j silently skipped that permanent. Grimoire remembered; nothing made anything else, and nothing would have noticed.

`GameState::is_legendary` reads the active face, with the flag as fallback for objects that have no face — a token copy of a legend is legendary and has no card behind it. Both halves of the legend rule now ask it.

They were previously reading **different** sources, which the fix exposed: the SBA raised the choice from one and the resolution filtered on the other, so with the SBA fixed the prompt appeared and then removed nothing. That is a latent inconsistency the old code hid by having both halves wrong in the same way. Also sorted the legend groups, so which duplicate a player is asked about first is not HashMap order.

Worth naming: the test covering this asserted `is_legendary == true` on both objects as a *precondition*, with the comment "Both Grimgrins must have is_legendary=true for SBA to detect them" — requiring the cache rather than the property, and so requiring every reanimator to keep stamping it. Rewritten to ask `state.is_legendary`.

### Checked against the ruling

- `"Creature cards" includes each card with the type creature, even if it has additional types, such as artifact.` — PASS. The filter is `state.is_creature(o.id, registry)`, which asks whether the card types include Creature, not whether they are exactly Creature. An artifact creature in a graveyard is returned.

### Checked and correct

- Cost `{4}`, `Legendary Artifact`, `Supertype::Legendary`, oracle text verbatim.
- Ability 0: `{1}` plus `requires_tap`, and it is not offered at all with an empty hand — a cost that cannot be paid cannot be chosen (CR 601.2h).
- Ability 1: no mana, `requires_tap`, `SacrificeCost::SacrificeThis`, `counter_cost: Some((Study, 3))`. The engine checks the counters are present and removes exactly three before the sacrifice, which would otherwise clear all of them.
- "from **all** graveyards" — `all_objects_in_zone(Zone::Graveyard)`, not just the controller's.
- "creature **cards**" — `state.is_card`, so a token in a graveyard is not returned (CR 109.1).
- "under **your** control" — `move_object_under_control(..., controller, ...)`, so the ETB fires with the right controller rather than being corrected after (CR 110.2).
- "They're black Zombies **in addition to** their other colors and types" — pushed onto `obj.subtypes` and `obj.colors`, the runtime-grant vectors, not replacing what is there.
- The card does not clean up its own spell; it has none.

### Tricky interactions checked

- Discard visible to an opponent responding to the ability: PASS (after fix).
- Countering the ability does not undo the discard: holds by construction now.
- Reanimating a legend when its twin is already out: the legend rule fires. PASS (after fix, and the fix is what makes it general).
- Artifact creature in a graveyard is a creature card: PASS.
- Token in a graveyard is not: PASS.
- Zombie and black are additions, not replacements: PASS (`characteristics_card_sweep.rs`, `copy_effects.rs`).
- Ability 1 unavailable below three counters: PASS.

### Test coverage

- discard prompt and study counter: `cards_complex_creatures.rs:1770`
- single card in hand auto-discards: `cards_complex_creatures.rs:1808`
- three counters accumulate: `cards_complex_creatures.rs:1834`
- ability 1 unavailable below three counters: `cards_complex_creatures.rs`
- reanimates all graveyard creatures: `cards_complex_creatures.rs`
- counter cost removes exactly three: `counter_costs.rs:24`
- black and Zombie are grants, not replacements: `characteristics_card_sweep.rs:141`, `copy_effects.rs:324`
- reanimated legend hits the legend rule: `state_based_actions.rs:145` (precondition rewritten to ask the property)
- the discard is paid on activation, the counter on resolution: `cards_complex_creatures.rs` `grimoire_discards_when_the_ability_is_activated_not_when_it_resolves` (NEW, mutation-checked)

