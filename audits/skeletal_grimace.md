## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/116/skeletal-grimace?utm_source=api
**Type line**: `Enchantment — Aura` — {1}{B}
**Oracle text**:
```
Enchant creature
Enchanted creature gets +1/+1 and has "{B}: Regenerate this creature."
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- The Aura grants an *activated ability* to the enchanted creature, so the
  ability is activated on the creature but dispatched to this card's behavior —
  which is exactly the `behavior_card_id` the engine resolves through the
  native → copy-grantor → attached walk. A card cannot work that out for itself,
  which is why the stack push is the engine's: PASS
- "{B}: Regenerate this creature" is a shield: it taps the creature, removes its
  damage and removes it from combat when it applies (CR 701.15): PASS
- Shields stack, and one is consumed per lethal event: PASS
- Regeneration does not save it from "destroy ... can't be regenerated", from
  exile, or from lethal -X/-X: PASS
- +1/+1 and the granted ability end together when the Aura leaves: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The granted regenerate ability and its limits: `cards_morbid_and_ltb.rs:skeletal_grimace_grants_regenerate_ability`, `:skeletal_grimace_regeneration_saves_from_lethal`, `:skeletal_grimace_regeneration_vs_deathtouch`, `:skeletal_grimace_regeneration_vs_doom_blade`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/116/skeletal-grimace?utm_source=api
**Type line**: `Enchantment — Aura` — {1}{B}
**Oracle text**:
```
Enchant creature
Enchanted creature gets +1/+1 and has "{B}: Regenerate this creature."
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (1; shared with three other cards, all fixed)

**Rulings fetched**: none are published for this card.

### Code issues found and fixed

**One, shared with three other cards: a regeneration shield could land on a
creature that had already left the battlefield, and then survive the turn.**

- Oracle text says: `Enchanted creature gets +1/+1 and has "{B}: Regenerate
  this creature."`
- Code did:
  ```rust
  if let Some(obj) = state.get_object_mut(object_id) {
      obj.regeneration_shields += 1;
  }
  ```
  with no check that the creature is still on the battlefield.

Regeneration replaces a destruction (CR 701.15), and a permanent that has left
the battlefield is a different object (CR 400.7) that cannot be destroyed — so
a shield aimed at one should land nowhere. It landed on the graveyard object
instead, and that is where it got interesting: the cleanup step clears unused
shields only from permanents **on the battlefield**, so the shield survived the
turn sitting on a card in the graveyard.

Confirmed before changing anything. Destroying the enchanted creature with its
own "{B}: Regenerate" still on the stack, then letting the turn end and
reanimating it:

```
shield after resolve=1, after cleanup=1, after reanimation=1
```

— a free regeneration the creature never earned, on a board where Grimoire of
the Dead, Unburial Rites and Moldgraf Monstrosity can all bring it back.

Fixed with `state.add_regeneration_shield`, which refuses anything off the
battlefield exactly as `add_counters` does for CR 121.1. All four cards that
create shields now use it — Skeletal Grimace, Full Moon's Rise, Manor Skeleton,
Ulvenwald Mystics — and a guard
(`card_data_invariants.rs::no_card_creates_a_regeneration_shield_by_hand`)
fails the build on a card touching `regeneration_shields` directly.

### Card data checked against the fetched text

| field | oracle | code |
|---|---|---|
| cost | `{1}{B}` | `Generic(1), Colored(Black)` OK |
| type | `Enchantment - Aura` | `Enchantment`, `["Aura"]` OK |
| enchant | "Enchant creature" | `TargetRequirement::Creature` OK |
| static | "+1/+1" | `ContinuousEffect::ModifyPT { power: 1, toughness: 1, scope: Attached }` OK |
| granted ability | `"{B}: Regenerate this creature."` | an `ActivatedAbilityDef` costing `{B}`, offered on the enchanted creature OK |
| oracle text | verbatim, including the quoted ability | OK |

### Tricky interactions checked

- **The granted ability belongs to the creature, not the Aura.** **Pass** —
  `legal/abilities.rs` collects abilities from attached permanents and passes
  the *creature's* id, and the card's guard (`is_creature`) keeps the Aura from
  offering it to itself.
- **"this creature" is the enchanted creature.** **Pass** — the shield goes on
  the id the ability was offered for.
- **A shield aimed at a creature that has already left.** **Was broken, now
  fixed.**
- **Shields expire at cleanup** (CR 701.15). **Pass**, tested generally in
  `regeneration.rs` — and the bug above was precisely that the expiry only
  reaches permanents on the battlefield.
- **Regeneration does not save from zero toughness** (CR 704.5f is not a
  destruction) or from sacrifice (CR 701.17a). **Pass**, both tested generally.
- **Regeneration saves from deathtouch and from a destroy effect.** **Pass**,
  tested.
- **The Aura leaving takes the granted ability with it.** **Pass** — the
  ability is only collected from attachments, so nothing lingers.
- **The +1/+1 is a continuous effect from the Aura**, so it goes when the Aura
  does. **Pass**, tested.
- **The enchanted creature stops being a creature.** The card's own guard asks
  `is_creature` each time abilities are collected, so the granted ability stops
  being offered.

### Test coverage

- grants the ability and the +1/+1:
  `cards_morbid_and_ltb.rs::skeletal_grimace_grants_regenerate`,
  `cards_vanilla_and_keywords.rs::skeletal_grimace_gives_plus_one_plus_one`
- regeneration saves from lethal damage and from a destroy effect:
  `cards_morbid_and_ltb.rs` (two tests)
- what a shield does, expires, and does not save from:
  `regeneration.rs` (eight tests)
- **a shield does not land on something that has left the battlefield**:
  `regeneration.rs::a_shield_does_not_land_on_something_that_has_left_the_battlefield` (new)
- **and the card-level version: no free regeneration comes back with a
  reanimated creature**:
  `cards_morbid_and_ltb.rs::skeletal_grimaces_regeneration_leaves_nothing_on_a_dead_creature` (new)

Mutation-checked: dropping the battlefield guard fails both new tests.
