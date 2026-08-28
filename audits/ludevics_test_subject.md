## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/64/ludevics-test-subject-ludevics-abomination?utm_source=api
**Type line**: `Creature — Lizard Egg` — {1}{U}, 0/3
**Oracle text**:
```
Defender
{1}{U}: Put a hatchling counter on this creature. Then if there are five or more hatchling counters on it, remove all of them and transform it.
```
**Back face**: Ludevic's Abomination — `Creature — Lizard Horror`, 13/13
```
Trample
```

**Status**: ISSUE

### Code issues
See below.


- The back face's printed P/T came from a `dynamic_pt` override that did nothing
  but restate this card's own `back_face_data` — one derived fact written twice,
  in two places free to disagree, and every test that covered a flip asserted the
  *hook* rather than `effective_power`. CR 712.8: a transformed permanent has its
  back face's characteristics. `effective_power`/`effective_toughness` now read
  the back face directly when `is_transformed`, the nineteen echoes are deleted,
  and a guard fails the build on a new one.

### Tricky interactions checked
- "{1}{U}: Put a hatching counter on this creature. Then if there are five or
  more hatching counters on it, remove them and transform it." — the removal and
  the flip happen together, and only at five: PASS
- The front face has defender and the back face trample; the back face's
  keywords come from the active face rather than being granted: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The counter loop and the flip at five: `cards_transforming_permanents.rs:ludevics_test_subject_transforms_at_five_counters`
- Counter costs: `counter_costs.rs`
- The back face's size: `cards_transforming_permanents.rs:every_transformed_dfc_is_its_back_faces_printed_size`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/64/ludevics-test-subject-ludevics-abomination?utm_source=api
**Type line**: `Creature — Lizard Egg` — {1}{U}, 0/3
**Oracle text**:
```
Defender
{1}{U}: Put a hatchling counter on this creature. Then if there are five or more hatchling counters on it, remove all of them and transform it.
```
**Back face**: Ludevic's Abomination — `Creature — Lizard Horror`, 13/13
```
Trample
```

**Rulings fetched**:
- [2016-07-13] For more information on double-faced cards, see the Shadows over Innistrad mechanics article (http://magic.wizards.com/en/articles/archive/feature/shadows-over-innistrad-mechanics).

**Status**: ISSUE (5 found, all fixed)

### Code issues found and fixed

1. **The back face had no colour indicator, so Ludevic's Abomination was colourless.**
   - CR 204.2: a face with no mana cost takes its colour from its printed
     colour indicator. `back_face_data` declared none, and `colors_of` falls
     back to the mana cost — which the back face does not have.
   - Colour established externally (Scryfall and Gatherer results via
     WebSearch; direct fetches to scryfall.com, mtg.wtf, gatherer.wizards.com,
     aetherhub.com and playingmtg.com are all blocked by the egress proxy in
     this environment): **blue**.
   - Fixed: `color_indicator: vec![Color::Blue]` on the back face.
     Tested by `cards_transforming_permanents.rs::ludevics_abomination_is_blue`.

2. **The ability refused to do anything once the permanent had transformed.**
   - Oracle text says: `{1}{U}: Put a hatchling counter on this creature. Then
     if there are five or more hatchling counters on it, remove all of them and
     transform it.`
   - Code did: `if state.get_object(object_id).is_some_and(|o| o.is_transformed) { return; }`
     at the top of `resolve_activated_ability`, commented "back face has no
     activated abilities".
   - Nothing in the ability's text asks which face is up. The comment's premise
     is true and irrelevant: once activated, the ability is on the stack as its
     own object and resolves on its own terms even if its source is gone
     entirely (CR 113.7a). Transforming is not a zone change, so the permanent
     is the same object before and after (CR 400.7, CR 712.8), and "this
     creature" is a self-reference to that object, not to a permanent with a
     particular name.
   - Consequence: five extra activations held on the stack across the flip
     should stack hatchling counters onto Ludevic's Abomination and, at the
     fifth, remove them and flip it back to Ludevic's Test Subject. They did
     nothing.
   - Note on the contrary reading: 2011-era forum threads argue the surplus
     abilities do nothing "because after it transforms it is no longer
     Ludevic's Test Subject". That argument is against the *printed* text,
     which read "Put a hatchling counter on Ludevic's Test Subject". The
     current Oracle text has no name in it, so there is no name left to fail to
     match. No official ruling addresses this; the card's only published ruling
     is the generic Shadows over Innistrad DFC article pointer.
   - Fixed: the guard is gone. Tested by
     `cards_transforming_permanents.rs::surplus_activations_keep_working_after_the_flip`.

3. **The card reached into the counter map by hand instead of using the
   engine's counter pipeline** — `o.counters.get(&CounterType::Hatchling)` to
   read and `obj.counters.remove(&CounterType::Hatchling)` to clear. Now
   `state.get_counter_count` and `state.remove_counters`. See the sweep below.

4. **Dead binding.** `activated_abilities` bound `obj` from the match and then
   discarded it with `let _ = obj;`. Replaced with a plain predicate.

5. **`should_transform` override restated the trait default** (`false`) and no
   engine code calls it for this card; the transform is the ability's doing.
   Removed.

### The sweep this turned into: card code hand-rolling the counter pipeline

Ludevic was one of four cards reaching into `obj.counters` directly. Two of the
three others were harmless style, the fourth was a real bug:

| card | what it did | outcome |
|---|---|---|
| `gutter_grime.rs:57` | `o.counters.get(&Slime).unwrap_or(&0)`, with a nonsense `map_or(1, ..)` default | -> `state.get_counter_count` |
| `garruk_relentless.rs:251` | `o.counters.get(&Loyalty).unwrap_or(&0) <= 2` | -> `state.get_counter_count` |
| `mikaeus_the_lunarch.rs:111` | `obj.counters.entry(PlusOnePlusOne).or_insert(0)` then `-= 1`, at resolution | see below |

**Mikaeus, the Lunarch** — "{T}, Remove a +1/+1 counter from Mikaeus, the
Lunarch: Put a +1/+1 counter on each other creature you control." Everything
before the colon is cost (CR 602.2b), so the removal is paid on activation
(CR 601.2h). Mikaeus removed it at *resolution*, which is the same bug already
fixed on Grimoire of the Dead's discard: an opponent responding to the ability
still saw the counter on Mikaeus, and countering the ability handed the counter
back. The engine already has the general mechanism —
`ActivatedAbilityDef::counter_cost`, paid in `engine/actions/abilities.rs` and
checked for payability in `engine/legal/abilities.rs` — so the fix is to
declare the cost and delete the hand-rolled removal. The card's own
`has_counter` gate went with it: affordability is the engine's question, and
two sources of truth for it is one too many.

The `entry().or_insert(0)` form was also leaving a zero-valued `PlusOnePlusOne`
key behind where `remove_counters` drops the key. `format_counters` in the LLM
view filters on `n > 0`, so that one never reached a player.

New guard: `card_data_invariants.rs::no_card_reaches_into_the_counter_map_by_hand`
fails the build on `.counters.get(` / `.entry(` / `.remove(` / `.insert(` /
`.contains_key(` / `.get_mut(` anywhere under `src/cards`.

### Test-helper gap found on the way

`tests/common/mod.rs::activate_via_hooks` paid only `pay_activation_cost` — the
*card-level* hook — and then pushed the ability on the stack. Any cost the
ability *declares* rather than hand-writes (a tap, a `counter_cost`) went
unpaid, so every test through that path measured an ability's effect without
its cost. Declaring Mikaeus's counter cost is what surfaced it:
`mikaeus_distributes_counters` started failing because the counter was no
longer being removed anywhere. The helper now pays the declared tap and counter
costs the way `submit_action` does. Mana is still not paid — it comes from a
pool these tests do not fill.

### Card data checked against the fetched text

| field | oracle | code |
|---|---|---|
| cost | `{1}{U}` | `Generic(1), Colored(Blue)` OK |
| front type | `Creature - Lizard Egg` | `Creature`, `["Lizard", "Egg"]` OK |
| front P/T | 0/3 | `Some(0)/Some(3)` OK |
| front keywords | Defender | `vec![Keyword::Defender]` OK |
| back type | `Creature - Lizard Horror` | `Creature`, `["Lizard", "Horror"]` OK |
| back P/T | 13/13 | `Some(13)/Some(13)` OK |
| back keywords | Trample | `vec![Keyword::Trample]` OK |
| back colour | blue (colour indicator) | now `color_indicator: vec![Color::Blue]` |
| oracle text | both faces | verbatim; `oracle_text_says_what_scryfall_says` covers it |

### Tricky interactions checked

- Counters put on it by something other than its own ability (proliferate,
  Doubling Season) count toward the five: **pass** — the check reads the
  counter pool, not an activation tally.
- "remove all of them" removes the surplus above five too: **pass**.
- Five counters alone do not transform it; only the ability's resolution does
  (the card has no upkeep trigger and no state trigger): **pass** —
  `should_transform` is the default `false` and nothing calls it.
- The ability is not offered on the back face (CR 712.8a): **pass**.
- Surplus activations resolving after the flip: **was broken, now fixed** (2).
- Dies and is reanimated -> comes back front-face-up with no counters
  (CR 400.7, CR 121.2): **pass** — handled generally in `state.rs`, not by this
  card.
- A token copy of it cannot transform (CR 111.7): **pass** — `apply_transform`
  refuses tokens.

### Test coverage

- transforms on the fifth activation: `cards_transforming_permanents.rs:445`
- back face is blue: `cards_transforming_permanents.rs::ludevics_abomination_is_blue` (new)
- counters from elsewhere count: `::ludevic_counts_hatchling_counters_it_did_not_put_there_itself` (new)
- "remove all of them": `::ludevic_transforming_removes_every_hatchling_counter` (new)
- no activated ability on the back face: `::ludevics_abomination_offers_no_activated_ability` (new)
- surplus activations after the flip: `::surplus_activations_keep_working_after_the_flip` (new)
- counters live in the engine pipeline: `counter_costs.rs::ludevic_hatchling_counters_live_in_the_counter_pipeline` (rewritten from a
  fingerprint assertion about a `card_state` key that no longer exists)
- reanimation resets counters and card_state:
  `zone_change_resets_object.rs::a_reanimated_permanent_brings_back_neither_counters_nor_card_state` (rewritten, same reason)

All four fixes mutation-checked: reverting each one fails the test named for it.
