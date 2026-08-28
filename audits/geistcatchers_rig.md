## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/223/geistcatchers-rig?utm_source=api
**Type line**: `Artifact Creature — Construct` — {6}, 4/5
**Oracle text**:
```
When this creature enters, you may have it deal 4 damage to target creature with flying.
```

**Status**: PASS

### Code issues
No issues found.

- "**you may** have it deal 4 damage to **target** creature with flying" — both
  halves handled: the target is locked when the trigger goes on the stack
  (CR 603.3d) and only the may-decision is presented at resolution, through
  `present_optional_target_choice` offering the locked target rather than a fresh
  pick. Re-picking at resolution is the tempting shortcut and would let a player
  dodge a removal spell aimed at the original target.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`cards_death_triggers_and_tokens.rs`, `trigger_targets_declared.rs` (targets locked at trigger time), `intervening_if.rs` (the morbid pair), `auto_pick.rs` (choices the engine must not make for a player).
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/223/geistcatchers-rig?utm_source=api
**Type line**: `Artifact Creature — Construct` — {6}, 4/5
**Oracle text**:
```
When this creature enters, you may have it deal 4 damage to target creature with flying.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "When this artifact enters, you **may** have it deal 4 damage to target
  creature with flying" — optional, and targeted at CR 603.3d time so the
  target is chosen as the trigger goes on the stack: PASS
- The trigger declares a `target_requirement`, which routes the enumeration
  through the engine — where hexproof is filtered once for every card, rather
  than by this card walking the battlefield itself: PASS
- "with **flying**" — the filter is re-checked on resolution, so a creature that
  lost flying makes it fizzle: PASS
- Damage through `deal_damage`: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Targeting only fliers, and hexproof: `hexproof_filter.rs:an_etb_trigger_does_not_offer_an_opponents_hexproof_creature`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/223/geistcatchers-rig?utm_source=api
**Type line**: `Artifact Creature — Construct` — {6}, 4/5
**Oracle text**:
```
When this creature enters, you may have it deal 4 damage to target creature with flying.
```

**Rulings fetched**:
- [2011-09-22] The target creature with flying is chosen when the ability triggers and goes on the stack. You choose whether or not Geistcatcher’s Rig will deal 4 damage to it when then ability resolves.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/223/geistcatchers-rig
**Oracle text**: When this creature enters, you may have it deal 4 damage to target creature with flying.
**Type line**: Artifact Creature — Construct
**Mana cost**: {6} — **P/T**: 4/5
**Rulings** (1, 2011-09-22): "The target creature with flying is chosen when the ability triggers and goes on the stack. You choose whether or not Geistcatcher's Rig will deal 4 damage to it when the ability resolves."

**Status**: ISSUE (fixed) — the card code is correct; nothing tested what it does.

### Card data
Matches the fetched text: `{6}`, `card_types: [Artifact, Creature]` (both),
`subtypes: ["Construct"]`, 4/5, oracle text verbatim in the current "When this
creature enters" errata wording, no keywords. One `TriggeredAbilityDef` of kind
`EntersBattlefield`, matching the one implemented hook, and `has_etb_handler()`
returns true so the engine actually puts it on the stack.

### The ruling, and how the card meets it
The ruling splits the card into two moments, and the implementation splits the
same way:
- **Target chosen when the trigger goes on the stack** — declared as
  `target_requirement: CreatureWithFilter(HasKeyword(Flying))` on the
  `TriggeredAbilityDef`, so the engine locks it there (CR 603.3d) and applies
  hexproof and protection while doing so.
- **You choose whether to deal the damage on resolution** — `on_enter_battlefield`
  takes `chosen_targets.first()` and offers only the may-decision through
  `present_optional_target_choice`. It never re-enumerates, which is what the
  comment in the card says and what the ruling requires.

Damage goes through `PendingEffect::DealDamage`, which `engine::effects` deals
as `DamageKind::NonCombat` — correct for a non-combat source.

### Code issues

No issue in `geistcatchers_rig.rs`. The card had no test of its behaviour at
all. Its only coverage was `hexproof_filter.rs:638`
(`an_etb_trigger_does_not_offer_an_opponents_hexproof_creature`), which checks
that an opponent's hexproof flyer is not offered — a question that needs none
of the card's three claims to be right. All three of these mutations passed the
entire workspace:

1. **"4 damage"**
   - Oracle text says: `deal 4 damage to target creature with flying`
   - Code says: `PendingEffect::DealDamage { amount: 4, .. }`
   - Changing it to `amount: 3` produced zero failures.

2. **"you may"**
   - Oracle text says: `you may have it deal 4 damage`
   - Code says: `helpers::present_optional_target_choice(..)`
   - Swapping to `present_target_choice(.., optional: false, ..)` produced zero
     failures — and that is not a small difference: with exactly one target the
     helper *auto-applies* the effect rather than prompting, so no choice would
     ever have been offered.

3. **"target creature with flying"**
   - Oracle text says: `target creature with flying`
   - Code says: `CreatureWithFilter(TargetFilter::HasKeyword(Keyword::Flying))`
   - Replacing it with `TargetRequirement::Creature` produced zero failures.
     The hexproof test asserts an Abbey Griffin **is** offered; it never asserts
     that a creature without flying is **not** — the same positive-half-only
     shape found in the Naturalize, Urgent Exorcism and Victim of Night audits.

Added `geistcatchers_rig_deals_four_to_the_flyer_when_you_say_yes` and
`geistcatchers_rig_deals_nothing_when_you_decline`, sharing one board run both
ways: an opponent's Abbey Griffin and an opponent's vanilla 3/3. The helper
checks that the trigger locks the flyer and **not** the 3/3, that resolution
raises an `optional: true` prompt with no damage dealt yet, and then each test
takes its branch.

While adding the card to the file's module-doc index,
`a_cards_file_covers_exactly_the_cards_its_module_doc_lists` caught the stale
"Cards covered (20)" count — the guard working as intended.

### Tricky interactions checked
- Only a creature with flying may be targeted: PASS — new test's negative half.
- Granted flying counts (`has_keyword` reads the object's granted keywords, the
  active face, and `until_end_of_turn` grants): correct by the accessor, which
  is shared. Not re-tested per card.
- An opponent's hexproof flyer is not offered: PASS — `hexproof_filter.rs:638`.
- No legal target at all: the trigger is not put on the stack (CR 603.3c),
  which is the engine's; `chosen_targets.first()` returning `None` makes the
  hook a no-op as a second guard.
- Declining deals nothing and leaves nothing pending: PASS — new test asserts
  `awaiting_action.is_none()` afterwards.
- The Rig leaves the battlefield before the trigger resolves: the damage is
  still dealt with the Rig as its source (CR 608.2g, last known information).
  `source_id: object_id` is captured when the trigger is set up, and
  `deal_damage` does not require the source to still be there. Noted; not
  tested, and the `hexproof_filter` and new tests both keep it around.
- Target becomes illegal between trigger and resolution: CR 608.2b, the
  engine's re-check, covered generically.
- Self-cleanup: none — this is a permanent, not a spell moving itself.

### UI presentation
The prompt reads "Geistcatcher's Rig: you may deal 4 damage to the targeted
creature", and the trigger's stack description is "deal 4 damage to target
creature with flying". Both name the source and the amount.

### Test coverage
- 4 damage to the targeted flyer: `cards_complex_creatures.rs`
  (`geistcatchers_rig_deals_four_to_the_flyer_when_you_say_yes`) —
  **added this audit**.
- Declining deals nothing:
  (`geistcatchers_rig_deals_nothing_when_you_decline`) — **added this audit**.
- A creature without flying is not a legal target: the shared helper of those
  two — **added this audit**.
- The ruling's two moments (target locked on the stack, decision on
  resolution): same helper — **added this audit**.
- Hexproof flyer not offered: `hexproof_filter.rs:638`.

### Mutations run
| mutation | result |
| --- | --- |
| `amount: 3` instead of 4 | fails the "yes" test (before: **nothing at all**) |
| mandatory instead of `you may` (auto-applies, never prompts) | fails both new tests (before: **nothing at all**) |
| `TargetRequirement::Creature` instead of the Flying filter | fails both new tests (before: **nothing at all**) |

Suite after: 1450 passing, exit 0, zero warnings.

