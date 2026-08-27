## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/212/evil-twin?utm_source=api
**Type line**: `Creature — Shapeshifter` — {2}{U}{B}, 0/0
**Oracle text**:
```
You may have this creature enter as a copy of any creature on the battlefield, except it has "{U}{B}, {T}: Destroy target creature with the same name as this creature."
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "You **may** have this creature enter as a copy" — declining leaves a
  0/0 that dies to state-based actions: PASS
- Ruling: "Evil Twin copies exactly what was printed on the original creature
  ... It doesn't copy whether that creature is tapped or untapped, whether it has
  any counters on it or any Auras and Equipment attached to it, or any non-copy
  effects that have changed its power, toughness, types, color": PASS
- Ruling: "If the chosen creature is copying something else ... your Evil Twin
  enters the battlefield as whatever the chosen creature copied": PASS
- Ruling: "The activated ability that Evil Twin gains as part of its copy effect
  is a copiable value" — the granted ability is dispatched through
  `copy_grantor` (CR 706.2), which is also how the engine resolves whose
  behavior an ability belongs to: PASS
- "Destroy target creature **with the same name as this creature**" — the name
  comes from the active face, not `obj.name`'s display cache: PASS
- Ruling: "If Evil Twin somehow enters the battlefield at the same time as
  another creature, Evil Twin can't become a copy of that creature": PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The copy choice, declining, and the granted ability: `cards_complex_creatures.rs`, `subtype.rs`, `characteristics_targeting.rs`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/212/evil-twin?utm_source=api
**Type line**: `Creature — Shapeshifter` — {2}{U}{B}, 0/0
**Oracle text**:
```
You may have this creature enter as a copy of any creature on the battlefield, except it has "{U}{B}, {T}: Destroy target creature with the same name as this creature."
```

**Rulings fetched**:
- [2022-06-10] Unless a token is a copy of another permanent or was explicitly given a name by the effect that created it, its name is the subtypes it was given when it was created plus the word "Token." For example, if an effect creates a 1/1 Soldier creature token, that token is named "Soldier Token."
- [2017-03-14] The activated ability that Evil Twin gains as part of its copy effect is a copiable value that other effects may copy.
- [2017-03-14] If the chosen creature has {X} in its mana cost, X is considered to be 0.
- [2017-03-14] If the chosen creature is copying something else (for example, if the chosen creature is another Evil Twin), then your Evil Twin enters the battlefield as whatever the chosen creature copied.
- [2017-03-14] Evil Twin copies exactly what was printed on the original creature (unless that creature is copying something else or is a token; see below) and it gains the activated ability. It doesn't copy whether that creature is tapped or untapped, whether it has any counters on it or any Auras and Equipment attached to it, or any non-copy effects that have changed its power, toughness, types, color, or so on.
- [2017-03-14] Any enters-the-battlefield abilities of the copied creature will trigger when Evil Twin enters the battlefield. Any "as [this creature] enters the battlefield" or "[this creature] enters the battlefield with" abilities of the chosen creature will also work.
- [2017-03-14] If Evil Twin somehow enters the battlefield at the same time as another creature, Evil Twin can't become a copy of that creature. You may choose only a creature that's already on the battlefield.
- [2017-03-14] You can choose not to copy anything. In that case, Evil Twin enters the battlefield as a 0/0 creature, and is probably put into the graveyard immediately.
- [2017-03-14] If the chosen creature is a token, Evil Twin copies the original characteristics of that token as stated by the effect that created the token. Evil Twin is not a token in this case.

**Status**: ISSUE (fixed)

### Code issues

**A copy picked up subtypes and colors that non-copy effects had granted to the original.**

- Ruling says: `Evil Twin copies exactly what was printed on the original creature ... It doesn't copy whether that creature is tapped or untapped, whether it has any counters on it or any Auras and Equipment attached to it, or any non-copy effects that have changed its power, toughness, types, color, or so on.`
- Code did, in `engine/effects.rs`'s `CopyCreature` handler:
  ```rust
  (o.name.clone(), o.power, o.toughness, o.card_id,
   o.card_types.clone(), o.subtypes.clone(), kw, o.colors.clone(), legendary)
  ```

`obj.subtypes` and `obj.colors` are the *runtime grant* vectors for a real card
— that is the codebase's own rule, stated at the top of the characteristics
module. They double as printed values only for a token, which has no registry
face. Reading them directly conflated the two.

Reachable in this set, by two cards that write exactly those vectors:

- `olivia_voldaren.rs:115` — `obj.subtypes.push("Vampire".to_string())` for
  "That creature becomes a Vampire in addition to its other types."
- `grimoire_of_the_dead.rs:145,148` — `obj.subtypes.push("Zombie".into())` and
  `obj.colors.push(Color::Black)`.

So Olivia turns an opponent's creature into a Vampire, Evil Twin copies that
creature, and the copy came out a Vampire too — copying a non-copy effect, which
CR 707.2 and this card's ruling both forbid.

Fixed where the distinction lives rather than in the card: added
`printed_card_types_of`, `printed_subtypes_of`, `printed_colors_of` and
`printed_pt_of` alongside the `printed_keywords_of` that already existed for
exactly this reason (a comment there records a copy of a Spirit token losing its
flying). Each takes the active face when there is one and falls back to the
object only for a faceless token. The copy handler now uses them, and takes the
name from `name_of` rather than `obj.name`, which is a display cache.

This is one fix for every copy effect, not just Evil Twin.

### Rulings checked

- **"It doesn't copy ... any non-copy effects that have changed its power,
  toughness, types, color."** Fixed above; tested for subtypes and colors.
- **"If the chosen creature is a token, Evil Twin copies the original
  characteristics of that token as stated by the effect that created the token.
  Evil Twin is not a token in this case."** Both halves hold — the `printed_*`
  accessors fall back to the object vectors when there is no face, which is
  exactly the token case, and nothing sets `is_token` on the copy. Tested.
- **"You can choose not to copy anything. In that case, Evil Twin enters the
  battlefield as a 0/0 creature, and is probably put into the graveyard
  immediately."** The copy choice is optional, and declining disarms
  `entering_copy_source` at `choices.rs:76` so state-based actions can then bury
  the 0/0. Without that the declined Evil Twin would have been permanently
  immune to SBAs. PASS, tested.
- **"If Evil Twin somehow enters the battlefield at the same time as another
  creature, Evil Twin can't become a copy of that creature. You may choose only
  a creature that's already on the battlefield."** The choice list is
  `creature_choices_except(state, object_id, registry)`, which excludes Evil
  Twin itself and is built from creatures already on the battlefield. PASS.
- **"Any enters-the-battlefield abilities of the copied creature will trigger
  when Evil Twin enters the battlefield."** The handler queues the copied card's
  ETB trigger after the copy resolves, with a comment explaining that the copy
  is modelled as a choice resolving after entry (CR 614.12). PASS.
- **"The activated ability that Evil Twin gains as part of its copy effect is a
  copiable value."** Modelled generically as `copy_grantor` rather than by name,
  so a copy of an Evil Twin keeps the granted ability. PASS.
- **"If the chosen creature is copying something else ... your Evil Twin enters
  the battlefield as whatever the chosen creature copied."** Falls out of
  copying `card_id`: a creature that is already a copy has the copied card's
  `card_id`, so the second copy lands on the same face. PASS.
- **"If the chosen creature has {X} in its mana cost, X is considered to be 0."**
  Not reachable — no ISD creature has {X} in its cost — and mana value is not
  read by the copy. Noted, not tested.
- **Token naming ruling** (a token is named after its subtypes plus "Token") is
  about what "same name" matches; the set's tokens are created with explicit
  names, so the activated ability compares those. Noted.

### Tricky interactions checked

- **The copy choice is a choice, not a target** (CR 115.1), so an opponent's
  hexproof creature may be copied. The card uses `creature_choices_except`
  rather than the targeting helper, with a comment recording that using the
  targeting helper had hidden exactly that. PASS, tested.
- **The granted ability exists only on a permanent that actually copied
  something** — gated on `copy_grantor.is_some()`, so a declined Evil Twin is a
  plain 0/0 with no ability. PASS, tested.
- **Legendary is copiable** (CR 707.2), so copying a legend triggers the legend
  rule. PASS, tested.
- **`{U}{B}, {T}` is a real cost** — `requires_tap: true`, so the ability is
  subject to summoning sickness. PASS.

### Test coverage

- no granted subtype copied: `copy_effects.rs::a_copy_does_not_pick_up_a_subtype_granted_by_a_non_copy_effect` (new, mutation-checked).
- no granted colour copied: `::a_copy_does_not_pick_up_a_color_granted_by_a_non_copy_effect` (new, mutation-checked).
- a token's printed characteristics still copied, and the copy is not a token: `::a_copy_of_a_token_still_takes_the_tokens_printed_characteristics` (new).
- hexproof creature may be copied: `::evil_twin_may_copy_a_hexproof_creature_it_could_not_target`.
- not marked a copy until the choice is made: `::evil_twin_is_not_marked_as_a_copy_until_the_choice_is_made`.
- keeps the granted ability after copying: `::evil_twin_keeps_its_granted_ability_after_copying`.
- survives SBAs while the choice is pending: `::evil_twin_survives_state_based_actions_while_its_copy_choice_is_pending`.
- copies on ETB, and the legend rule: `cards_complex_creatures.rs:1139`, `:1171`.

