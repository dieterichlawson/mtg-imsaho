## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/133/burning-vengeance?utm_source=api
**Type line**: `Enchantment` — {2}{R}
**Oracle text**:
```
Whenever you cast a spell from your graveyard, this enchantment deals 2 damage to any target.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Whenever **you** cast a spell **from your graveyard**" — the caster must be
  the enchantment's controller, and the spell must have been cast from a
  graveyard, which in this set means flashback: PASS
- Ruling: "Burning Vengeance doesn't trigger when you **activate an ability** of
  a card in your graveyard" — only casting, not activating: PASS
- "deals 2 damage to **any target**", chosen when the trigger goes on the stack
  (CR 603.3d): PASS
- The trigger resolves before the spell that caused it, since it went on the
  stack on top: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Triggering on flashback casts: `cards_flashback.rs`, `trigger_dispatch.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/133/burning-vengeance?utm_source=api
**Type line**: `Enchantment` — {2}{R}
**Oracle text**:
```
Whenever you cast a spell from your graveyard, this enchantment deals 2 damage to any target.
```

**Rulings fetched**:
- [2025-01-24] Burning Vengeance doesn’t trigger when you activate an ability of a card in your graveyard, such as unearth or the ability of Reassembling Skeleton.
- [2025-01-24] Burning Vengeance’s triggered ability will resolve before the spell you cast from your graveyard.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/133/burning-vengeance
**Oracle text**:
```
Whenever you cast a spell from your graveyard, this enchantment deals 2 damage to any target.
```
**Type line**: `Enchantment` · **Mana cost**: `{2}{R}`
**Rulings** (2, both 2025-01-24, https://api.scryfall.com/cards/fd403810-840b-46ac-ae6e-5df23ce16fec/rulings):
1. "Burning Vengeance doesn't trigger when you activate an ability of a card in your graveyard, such as unearth
   or the ability of Reassembling Skeleton."
2. "Burning Vengeance's triggered ability will resolve before the spell you cast from your graveyard."

**Status**: ISSUE (fixed) — a redundant, wrong-in-principle re-check, and two negative tests that passed for the
wrong reason.

### Card data
| field | oracle | `burning_vengeance.rs` | |
|---|---|---|---|
| cost | `{2}{R}` | `Generic(2) + Red` | ok |
| types | Enchantment | `vec![CardType::Enchantment]` | ok |
| oracle_text | as above | byte-identical | ok |
| trigger | on casting a spell from your graveyard | `SpellCast` + `should_trigger_on_spell_cast` | ok |
| targeting | "any target" | `TargetRequirement::AnyTarget` on the trigger (CR 603.3d) | ok |

**On "from your graveyard" vs `cast_with_flashback`.** The code's condition is
`state.get_object(spell_id).is_some_and(|o| o.cast_with_flashback)`. The engine tracks no cast-from zone, and in
this set flashback is the only way to cast a spell from a graveyard — including Snapcaster Mage, which grants
flashback rather than inventing a second route. So the proxy is exact here. Recorded as a proxy rather than
flagged as a mismatch, because there is no case in this pool where the two answers differ. Ruling 1 falls out of
it: activating an ability of a graveyard card is not a cast, and `TriggerKind::SpellCast` only fires on casts.

### Code issues

**The handler re-asked the trigger condition.** Removed.

`on_spell_cast` opened by re-deriving the controller and re-testing both halves — `caster != controller` and
`cast_with_flashback` — that `should_trigger_on_spell_cast` had already gated. CR 603.2 checks a trigger's
condition when the event happens, not when the ability resolves; the one shape that gets a second look is an
intervening-if (CR 603.4), and this ability has no such clause.

Worse than duplication: it re-derived the controller from the source. CR 603.3d fixes a trigger's controller
when it goes on the stack. Nothing in this set steals an enchantment, so this was unreachable — but had anything
done so, the re-check would have suppressed a trigger that should still resolve. CR 113.7a says the same from
the other side, and the deleted code's own comment cited it while doing the opposite.

### The real finding: two negative tests passing for the wrong reason
Removing the handler re-check should have made the card depend entirely on `should_trigger_on_spell_cast`. To
confirm that, I mutated each half of it away. **Neither mutation failed anything** — including the test I had
just written for the "you cast" half.

The cause is the same in both tests, and it is a trap worth writing down. This ability targets, so a trigger
that fires stops on `awaiting_action` waiting for a target to be chosen. `triggers::process_triggers` leaves it
there. A test that fires the event and then asserts "nobody lost life" therefore cannot tell a trigger that
never fired from one that fired and stalled — both leave the life totals alone. The pre-existing
`burning_vengeance_ignores_non_flashback` had exactly this hole, and I reproduced it in
`burning_vengeance_ignores_an_opponents_flashback_cast`.

Both now use `process_triggers_auto_target_opponent`, which answers the prompt, so a trigger that fires resolves
and deals its damage. Both also assert `awaiting_action.is_none()`, naming the failure mode. Each half of the
condition now fails its own test when removed.

### Changes made
- `mtg-engine/src/cards/isd/burning_vengeance.rs` — handler re-check removed, with a comment on why the
  condition is not re-asked.
- `mtg-engine/tests/cards_spells_and_enchantments.rs`:
  - `burning_vengeance_ignores_an_opponents_flashback_cast` — "whenever **you** cast". Neither existing test
    varied the caster, so an implementation ignoring it passed both.
  - `burning_vengeances_trigger_sits_above_the_spell_that_caused_it` — ruling 2. Asserted as the 2 damage having
    landed while the flashback spell is still on the stack, which is what the ruling is about; my first version
    tried to inspect stack order after `collect_triggers` and found nothing there, because a targeting trigger
    does not reach the stack until its target is chosen.
  - `burning_vengeance_ignores_non_flashback` — repaired as above.

### Mutation checks
1. `caster == controller` dropped from `should_trigger_on_spell_cast` → **vacuous** first time; discriminating
   after the test repair (`burning_vengeance_ignores_an_opponents_flashback_cast` FAILED).
2. `cast_with_flashback` dropped → **vacuous** first time; discriminating after the repair
   (`burning_vengeance_ignores_non_flashback` FAILED).
3. `amount: 2` → `0` → `burning_vengeance_triggers_on_flashback` and the new ordering test both FAILED.

### Tricky interactions checked
- Flashback cast triggers it: **pass** (`cards_spells_and_enchantments.rs:407`).
- Ordinary cast does not: **pass** (repaired).
- An opponent's flashback cast does not: **pass** (new).
- The trigger resolves before the spell: **pass** (new).
- Destroying Burning Vengeance in response still deals the damage (CR 113.7a): follows from the handler no
  longer consulting the source at all; not separately tested, and the trigger carries its target.
- Ruling 1 (activated abilities of graveyard cards): structurally satisfied — `SpellCast` fires only on casts —
  and unreachable in this set, which has no such ability.

### Test coverage
- triggers on a flashback cast: `cards_spells_and_enchantments.rs:407`
- not on an ordinary cast: `cards_spells_and_enchantments.rs:437` (repaired)
- not on an opponent's flashback cast: `cards_spells_and_enchantments.rs:466` (new)
- resolves before the spell (ruling 2): `cards_spells_and_enchantments.rs:487` (new)
- no stale "to opponent" log before the target is picked: `trigger_dispatch.rs:334`

### Suite
`cargo check --workspace --all-targets` clean, zero warnings. Full suite exit 0, 1424 passing.

