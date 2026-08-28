## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/82/sturmgeist?utm_source=api
**Type line**: `Creature — Spirit` — {3}{U}{U}, */*
**Oracle text**:
```
Flying
Sturmgeist's power and toughness are each equal to the number of cards in your hand.
Whenever this creature deals combat damage to a player, draw a card.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "power and toughness are each equal to the number of cards in your hand" is a
  characteristic-defining ability — `dynamic_pt`, recomputed live, so casting a
  card shrinks it mid-combat: PASS
- "Whenever **this creature** deals combat damage to a player, draw a card" —
  its own damage only, and the draw then grows it: PASS
- Flying: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The CDA and the draw trigger: `cards_complex_creatures.rs`, `combat_rules.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/82/sturmgeist?utm_source=api
**Type line**: `Creature — Spirit` — {3}{U}{U}, */*
**Oracle text**:
```
Flying
Sturmgeist's power and toughness are each equal to the number of cards in your hand.
Whenever this creature deals combat damage to a player, draw a card.
```

**Rulings fetched**:
- [2011-09-22] The ability that defines Sturmgeist's power and toughness works in all zones, not just the battlefield.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/82/sturmgeist
**Oracle text**:
```
Flying
Sturmgeist's power and toughness are each equal to the number of cards in your hand.
Whenever this creature deals combat damage to a player, draw a card.
```
**Type line**: Creature — Spirit
**Mana cost**: {3}{U}{U} — **P/T**: */* — **Keywords**: Flying
**Rulings** (1, 2011-09-22): "The ability that defines Sturmgeist's power and toughness works in all zones, not just the battlefield."

**Status**: ISSUE (fixed) — the card code is correct; its ruling had no test and its draw count was uncounted.

### Card data
Matches the fetched text: `{3}{U}{U}`, `card_types: [Creature]`,
`subtypes: ["Spirit"]`, `keywords: [Flying]`, oracle text verbatim in the
current "Whenever this creature enters"-style errata wording, and one
`TriggeredAbilityDef` of kind `CombatDamageToPlayer` matching the one
implemented hook.

`*/*` is carried as `power: Some(0), toughness: Some(0)` — the CDA sentinel
`effective_power` documents ("CDA creatures use the `Some(0)` sentinel"), which
is what makes the engine consult `dynamic_pt` at all.

### Code issues

No issue in `sturmgeist.rs`. Two in what held it up; both mutations passed the
entire workspace.

1. **The ruling had no test** (`cards_combat_damage_triggers.rs:304`, test added).
   - Ruling says: `The ability that defines Sturmgeist's power and toughness works in all zones, not just the battlefield.`
   - That is CR 604.3, and the card is right — `dynamic_pt` reads
     `state.get_object(object_id)?.controller` with no zone gate.
   - Verified: adding `if obj.zone != Zone::Battlefield { return None; }`
     produced zero failures. Geist-Honored Monk has exactly this ruling and a
     test for it (`cards_evasion_and_graveyard_pt.rs:142`); Sturmgeist did not.
   - Added `sturmgeists_defining_ability_works_outside_the_battlefield`, which
     puts the card in a graveyard and watches its P/T follow the hand size.
     CR 109.5 makes the owner act as controller off the battlefield, which
     `move_object` already arranges, so "your hand" keeps its meaning there.

2. **"draw a card" was uncounted**
   (`trigger_source_independence.rs:536`, library restocked).
   - Oracle text says: `draw a card` — one.
   - `sturmgeist_draws_after_dying` asserts `hand.len() == before + 1`, but
     pushed exactly **one** card into the library. A Sturmgeist that drew two
     would draw one and stop, and the assertion would still hold.
   - Verified: `draw_cards(state, controller, 2, registry)` produced zero
     failures. Three cards in the library now, and that mutation fails. Same
     shape as the Mindshrieker mill-count gap found earlier in this audit run.

3. **"in *your* hand" was not a claim the test could fail** (same file as 1).
   - The P/T test gave only P0 a hand, so reading the opponent's hand size
     instead would have been caught only by accident (it happens to be caught,
     because an empty opponent hand gives 0 ≠ 4). Gave P1 two cards, so the two
     readings now differ by a number rather than by "something vs nothing".

### Tricky interactions checked
- P/T tracks the controller's hand, live: PASS — `sturmgeist_pt_equals_hand_size`.
- "your hand", not the opponent's: PASS — reading `state.opponent(..)`'s hand
  fails that test.
- The ruling, all zones: PASS — new test.
- The trigger draws exactly one: PASS — now that the library has three cards.
- The trigger still resolves after Sturmgeist dies (CR 113.7a): PASS —
  `trigger_source_independence.rs:534`, driven through the dispatcher rather
  than by calling the hook, which is that file's whole point.
- "combat damage **to a player**": the `TriggerKind::CombatDamageToPlayer`
  dispatch is the engine's; damage to a blocking creature does not raise it.
  Not re-tested per card.
- Flying: `keywords: [Flying]`, read through `has_keyword` like every other
  keyword. Covered by the evasion tests generically.
- A Sturmgeist **in your hand** counts itself, since it is a card in your hand —
  follows from the ruling; noted rather than tested, as nothing in the pool
  reads a hand card's P/T.
- `on_combat_damage_to_player` reads `helpers::controller_of` rather than
  `o.controller`, so "you" survives the source leaving the battlefield
  (CR 608.2g).
- Self-cleanup: none; this is a permanent.

### UI presentation
Trigger description: "draw a card". Nothing to choose. The P/T shows through
`effective_power`/`effective_toughness`, so the board displays the live value.

### Test coverage
- P/T equals your hand size, not the opponent's:
  `cards_combat_damage_triggers.rs` (`sturmgeist_pt_equals_hand_size`) —
  **tightened this audit**.
- The ruling (works in all zones):
  (`sturmgeists_defining_ability_works_outside_the_battlefield`) —
  **added this audit**.
- Draws exactly one card, and still does so after dying:
  `trigger_source_independence.rs` (`sturmgeist_draws_after_dying`) —
  **the count made real this audit**.

### Mutations run
| mutation | result |
| --- | --- |
| `dynamic_pt` gated on `zone == Battlefield` | fails the new ruling test (before: **nothing at all**) |
| `draw_cards(.., 2, ..)` | fails `sturmgeist_draws_after_dying` (before the restock: **nothing at all**) |
| read the opponent's hand size | fails `sturmgeist_pt_equals_hand_size` |
| remove `dynamic_pt` entirely (a 0/0) | fails `sturmgeist_pt_equals_hand_size` |

Suite after: 1456 passing, exit 0, zero warnings.

