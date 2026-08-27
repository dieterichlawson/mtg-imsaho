## Audit — 2026-08-27 — CR 603.2 trigger scope

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/114/screeching-bat-stalking-vampire?utm_source=api
**Type line**: `Creature — Bat` — {2}{B}, 2/2
**Oracle text**:
```
Flying
At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
```
**Back face**: Stalking Vampire, `Creature — Vampire`
```
At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
```

**Status**: ISSUE (fixed) — duplication, not a rules defect

### Code issue
- Oracle text says the trigger happens at **your** upkeep / **your** end step.
- Code did: declared `step_trigger_scope` → `TriggerScope::Your`, which is
  correct and sufficient, and then re-derived the same thing inside the handler
  as `state.active_player != controller`.
- The engine's gate is not taken on trust: `your_upkeep_scope.rs` sweeps the
  registry for every card with a controller-scoped step trigger and checks both
  directions — fires on the controller's step, silent on the opponent's. The
  handler check was provably dead.
- Fixed: removed, with a comment saying where the scoping actually lives.
The trigger is on the back face (Stalking Vampire's upkeep "you may pay
{2}{B}{B}").

### What else was checked
- Card data verified exact set-wide (see `ISD_AUDIT_PROGRESS.md`): cost, types,
  subtypes, supertypes, P/T, oracle text, keywords on both faces, flashback
  cost, and trigger kinds against the oracle phrasing.
- Step 9 anti-patterns: clean after this change.

### Test coverage
`your_upkeep_scope.rs::a_your_step_trigger_fires_on_its_controllers_step_and_no_one_elses`
covers this card by registry sweep, in both directions.
## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/114/screeching-bat-stalking-vampire?utm_source=api
**Type line**: `Creature — Bat` — {2}{B}, 2/2
**Oracle text**:
```
Flying
At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
```
**Back face**: Stalking Vampire — `Creature — Vampire`, 5/5
```
At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "you **may** pay {2}{B}{B}. If you do, transform" is on **both** faces, so the
  prompt is offered whichever way round it is and it can flip back and forth —
  the handler has no `is_transformed` gate: PASS
- CR 106.4: mana pools empty between steps, so the pool is normally empty at
  upkeep. The prompt is offered when the player has enough *untapped sources*,
  planned through the engine's autotap, rather than only when mana is already
  floating: PASS
- Declining costs nothing: PASS
- The tap plan is recomputed when the answer comes back, so it cannot pay with
  sources that have since been tapped: PASS
- Flying is on the front face only: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Paying and transforming from either face: `cards_transforming_permanents.rs:screeching_bat_transforms_at_upkeep_when_player_pays`, `transform_dfc.rs`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/114/screeching-bat-stalking-vampire?utm_source=api
**Type line**: `Creature — Bat` — {2}{B}, 2/2
**Oracle text**:
```
Flying
At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
```
**Back face**: Stalking Vampire — `Creature — Vampire`, 5/5
```
At the beginning of your upkeep, you may pay {2}{B}{B}. If you do, transform this creature.
```

**Rulings fetched**:
- [2016-07-13] For more information on double-faced cards, see the Shadows over Innistrad mechanics article (http://magic.wizards.com/en/articles/archive/feature/shadows-over-innistrad-mechanics).

**Status**: PASS

### Code issues

No issues found.

### Rulings checked

The only published ruling is a link to a mechanics article, with no rules
content. Both faces' text is a plain may-pay upkeep trigger.

### Tricky interactions checked

- **The back face loses flying.** `Stalking Vampire` is `Creature — Vampire`
  with no keywords; the front face is a `Bat` with flying. Verified that
  `has_keyword` reads the *active face* from the registry and deliberately does
  not union `obj.keywords` for a registry-backed card — the code comments the
  reason exactly ("Unioning would resurrect a stale front-face keyword on a
  transformed DFC"). So the Vampire does not keep flying. PASS, and tested both
  ways (loses it, regains it on the flip back).
- **Floating mana pays the cost.** The prompt is gated on
  `plan_autotap_for_cost`, which looks at untapped sources — my first reading was
  that a player who had already tapped lands in response to the trigger would be
  refused. Checked it: `plan_autotap_for_cost` passes the player's pool into
  `compute_autotap`, whose "Phase 0: Deduct floating mana from the cost" spends
  the pool first and returns an empty tap plan when the pool alone covers it. So
  mana floated during the upkeep is used. PASS.
- **Mana pools empty between steps (CR 106.4)**, which is why the card checks
  autotap reachability rather than the pool alone — the pool is normally empty
  when an upkeep trigger resolves. The card comments this.
- **"You may pay"** is a genuine choice, presented as `YesNo` to the controller,
  not auto-taken; declining costs nothing. Both branches tested.
- **The prompt is not offered when the cost cannot be paid.** The choice would
  have no reachable "yes", so nothing is lost. PASS.
- **Trigger scope** is `Your` on both faces ("your upkeep"), so an opponent's Bat
  does not prompt on your turn. PASS.
- **Both faces declare the ability**, so it keeps working after the flip and can
  flip back on a later upkeep. Tested.
- **`should_transform` returns false** — this is not a Werewolf and never flips
  on a board condition. PASS.
- **One flip per upkeep**: the trigger fires once, and the transform happens
  inside its resolution. PASS.

### Recorded, not changed

The prompt commits the player to one autotap plan — they answer yes/no, and the
engine picks which lands to tap. A player who wanted to keep a particular Swamp
untapped has no way to say so. This is the engine's established pattern for
every may-pay cost (flashback, Geistcatcher's Rig), not something specific to
this card, so changing it here would make this one card inconsistent with the
rest. Not a rules defect: the mana is fungible in this card pool.

### Test coverage

- pays and transforms, mana spent, back face is 5/5: `cards_transforming_permanents.rs::screeching_bat_transforms_at_upkeep_when_player_pays`.
- declines: `::screeching_bat_does_not_transform_when_player_declines`.
- no prompt without mana: `::screeching_bat_no_choice_without_mana`.
- back face has no flying: `::stalking_vampire_does_not_have_flying`.
- flying returns on the flip back: `::screeching_bat_regains_flying_on_transform_back`.
- transforms back from the back face: `::stalking_vampire_transforms_back_when_player_pays`.

