## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/176/daybreak-ranger-nightfall-predator?utm_source=api
**Type line**: `Creature — Human Archer Ranger Werewolf` — {2}{G}, 2/2
**Oracle text**:
```
{T}: This creature deals 2 damage to target creature with flying.
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Nightfall Predator — `Creature — Werewolf`, 4/4
```
{R}, {T}: This creature fights target creature. (Each deals damage equal to its power to the other.)
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
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
- Front: "{T}: This creature deals 2 damage to target creature **with flying**"
  — `TargetFilter::HasKeyword(Flying)`, and a creature that loses flying in
  response is no longer a legal target (CR 608.2b), now re-checked for abilities:
  PASS
- Back: "{R}, {T}: This creature **fights** target creature" — the fight
  pipeline, so both deal damage equal to power simultaneously: PASS
- The damage source is the Ranger, so protection from green stops it: PASS
- "At the beginning of **each** upkeep, **if** no spells were cast last turn" —
  an intervening-if checked both when the trigger would go on the stack and
  again on resolution (CR 603.4), via the shared werewolf helpers: PASS
- Both faces' upkeep triggers are declared, and the transform is the effect: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both faces' abilities and the flip conditions: `werewolf_cards.rs`, `transform_dfc.rs`
- Intervening-if on both directions: `intervening_if.rs`
- The back face's size: `cards_transforming_permanents.rs:every_transformed_dfc_is_its_back_faces_printed_size`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/176/daybreak-ranger-nightfall-predator?utm_source=api
**Type line**: `Creature — Human Archer Ranger Werewolf` — {2}{G}, 2/2
**Oracle text**:
```
{T}: This creature deals 2 damage to target creature with flying.
At the beginning of each upkeep, if no spells were cast last turn, transform this creature.
```
**Back face**: Nightfall Predator — `Creature — Werewolf`, 4/4
```
{R}, {T}: This creature fights target creature. (Each deals damage equal to its power to the other.)
At the beginning of each upkeep, if a player cast two or more spells last turn, transform this creature.
```

**Rulings fetched**:
- [2016-07-13] For more information on double-faced cards, see the Shadows over Innistrad mechanics article (http://magic.wizards.com/en/articles/archive/feature/shadows-over-innistrad-mechanics).

**Status**: ISSUE (fixed)

### Code issues

**A fight dealt damage even when one of the fighters was gone.**

- CR 701.12b: "If one or both creatures instructed to fight are no longer on the
  battlefield or are no longer creatures, neither of them fights or deals
  damage."
- Code did, in `combat::fight`:
  ```rust
  let power_a = u32::try_from(state.effective_power(a, registry).unwrap_or(0).max(0)).unwrap_or(0);
  let power_b = u32::try_from(state.effective_power(b, registry).unwrap_or(0).max(0)).unwrap_or(0);
  crate::damage::deal_damage(state, a, DamageTarget::Object(b), power_a, ...);
  crate::damage::deal_damage(state, b, DamageTarget::Object(a), power_b, ...);
  ```

No presence check on either side. Killing Nightfall Predator in response to its
own fight ability should spare the target entirely — the ability still resolves
(CR 113.7a), it just does nothing — but `effective_power` happily read the dead
Predator's printed 4 off its face and dealt it. The mirror case is the same: a
target that left the battlefield still sent its damage back into the fighter.

Fixed in `combat::fight`, which is the shared helper, so **Prey Upon** gets the
same fix. Also documented two things the code was already doing right, because
neither is obvious from reading it: both powers are read before either damage is
dealt (a fight is one simultaneous exchange, CR 701.12a), and a creature that
fights itself deals damage to itself twice, which falls out of `a == b` rather
than needing a case.

I checked the self-fight case against a source rather than changing it on a
hunch — a creature that fights itself does deal damage to itself equal to twice
its power, which is what the existing code produces.

### Rulings checked

The only published ruling is a link to a mechanics article, with no rules
content. The rules work here is CR 701.12 (fight) and CR 603.4 (the werewolf
transform condition), both checked below.

### Tricky interactions checked

- **"target creature with flying"** is `CreatureWithFilter(HasKeyword(Flying))`,
  and CR 608.2b re-legality is real: `stack.rs:51-62` re-runs the same filter on
  resolution, so a target that loses flying in response makes the ability fizzle.
  PASS.
- **The fight ability has no controller restriction** — "This creature fights
  target creature", not "target creature an opponent controls" — so it may fight
  your own creature. `TargetRequirement::Creature` is unrestricted. PASS, and
  tested.
- **The damage source is the Ranger itself** (`source_id: object_id`), so
  protection and prevention keyed on the source apply, and fight damage is
  `DamageKind::NonCombat` — deathtouch and lifelink still work, but it is not
  combat damage. PASS.
- **Subtypes.** Front is `Human Archer Ranger Werewolf` — all four present,
  which matters for Human-matters cards and for Moonmist. Back is `Werewolf`
  alone. PASS.
- **Scryfall lists Keywords "Transform, Fight"** and the card declares neither.
  That is right: those are Scryfall's tags for text patterns, not keyword
  abilities that `has_keyword` should answer to. Adding them would make the
  Ranger register as having a keyword it does not. PASS.
- **The upkeep transform** goes through the shared werewolf helpers, so it
  inherits the CR 603.4 fix made for Mayor of Avabruck: the condition re-checked
  on resolution is the one belonging to the face that triggered. PASS.
- **`on_upkeep`'s battlefield guard is correct here** — unlike the end-step
  handlers fixed in Cloistered Youth and Bloodgift Demon, "transform this
  creature" genuinely needs the permanent. PASS.

### Test coverage

- fight does nothing when the fighter has left: `werewolf_cards.rs::nightfall_predators_fight_does_nothing_if_the_predator_dies_in_response` (new, mutation-checked).
- and nothing when the target has left: `::a_fight_deals_no_damage_when_the_target_has_left_the_battlefield` (new, mutation-checked).
- fighting your own creature, both directions of damage: `::nightfall_predator_can_fight_own_creature`.
- front-face ability exists and targets fliers: `::daybreak_ranger_has_activated_ability_on_front_face`.
- back-face ability is the fight: `::nightfall_predator_has_fight_ability`.
- transform conditions and the intervening-if gate: `intervening_if.rs:129`, `trigger_snapshots.rs:132`.
- the ability uses the stack: `activated_no_stack.rs:163`.

