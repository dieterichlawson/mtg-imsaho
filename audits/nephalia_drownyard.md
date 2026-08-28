## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/245/nephalia-drownyard?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
{T}: Add {C}.
{1}{U}{B}, {T}: Target player mills three cards.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "{1}{U}{B}, {T}: Target player mills three cards" — through `mill_cards`, so
  creature cards among them emit `CreatureCardMilled`: PASS
- The mill happens on resolution, not on activation (CR 602.2a): PASS
- Its mana ability and its activated ability are both offered while it is
  untapped, and neither after: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The mill on resolution: `activated_no_stack.rs:nephalia_drownyard_mills_on_resolution`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/245/nephalia-drownyard?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
{T}: Add {C}.
{1}{U}{B}, {T}: Target player mills three cards.
```

**Rulings fetched**: none published for this card.

**Status**: PASS

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/245/nephalia-drownyard
**Oracle text**:
```
{T}: Add {C}.
{1}{U}{B}, {T}: Target player mills three cards.
```
**Type line**: `Land` · **Mana cost**: none · **Keywords**: Mill
**Rulings**: none published for this card.
**Status**: PASS (test coverage extended)

### Card data
| field | oracle | `nephalia_drownyard.rs` | |
|---|---|---|---|
| name / cost / types | Nephalia Drownyard, none, Land | matching | ok |
| oracle_text | as above | byte-identical | ok |
| mana ability | `{T}: Add {C}` | `Colorless`, tap, free | ok |
| activation cost | `{1}{U}{B}, {T}` | `Generic(1) + Blue + Black`, `requires_tap: true` | ok |
| targeting | "target player" | `TargetRequirement::PlayerOnly` | ok |
| timing | unrestricted | `sorcery_speed_only: false` | ok |

Scryfall lists `Keywords: ["Mill"]`. Mill is an action word (CR 701.13a), not a static keyword, and has no
counterpart in `types::Keyword` — correctly, since the card's ability is what performs it.

### Code issues
No issues found. The whole resolution is one call:
`crate::engine::mill_cards(state, *player_id, 3, "Nephalia Drownyard", registry)`. That matters beyond
tidiness: `mill_cards` → `mill_one` → `move_object`, and `move_object` is what emits `CreatureCardMilled`, which
is how Undead Alchemist sees an opponent's creature card go from library to graveyard. A card milling by hand
would bypass that watcher — the engine comment at `state.rs` records four cards having done exactly that.

### Rules check
- **CR 701.13a** — "If a library has fewer cards in it than the number of cards the player is instructed to
  mill, that player mills as many cards as they can." `mill_cards` breaks out of its loop on an empty library
  and reports the shortfall.
- **CR 704.5b** — losing is for *attempting to draw* from an empty library. Milling one is not that, and
  `sba.rs` keys the loss on `has_drawn_from_empty`, which `mill_cards` never sets.
- **CR 702.11b / 104.3a** — `PlayerOnly` filters through `can_target_player`, so a hexproofed opponent and a
  player who has lost are both excluded, while the activator can still target themselves.
- **CR 602.2h** — the Drownyard's own `{T}: Add {C}` cannot fund its `{1}{U}{B}` (utility-land table in
  `tap_cost_legality.rs`).
- **CR 602.2a** — the ability uses the stack; nothing is milled at activation (`activated_no_stack.rs:103`).

### Changes made
Nothing in the card. `mtg-engine/tests/cards_activated_abilities.rs`:

- `nephalia_drownyard_may_target_either_player_but_not_a_hexproof_one` — "target player" has no restriction, so
  the ability must offer its own controller too; milling yourself is a real play in a set built on flashback and
  graveyard-counting creatures. The Witchbane Orb half asserts the Orb removes *only* its controller.
- `nephalia_drownyard_mills_as_many_as_it_can_and_no_one_loses` — a one-card library and an empty one, with
  state-based actions run afterwards so the "not a loss" half is actually checked rather than assumed.
- Folded the `bloodhall_targets` helper added during the Stensia Bloodhall audit into a shared
  `ability_targets`, instead of adding a second copy for this card. I had in fact written the duplicate first
  and then removed it.

### Mutation checks
1. `mill_cards` setting `has_drawn_from_empty` when the library runs out (milling treated as a failed draw) →
   `nephalia_drownyard_mills_as_many_as_it_can_and_no_one_loses` FAILED. **Discriminating.**
2. `PlayerOnly` excluding the caster → **vacuous on the first attempt.** There are two `PlayerOnly` enumeration
   arms — one for casting spells, one for activated abilities — and I mutated the spell one. Re-run against the
   ability arm (`targeting.rs:543`), `nephalia_drownyard_may_target_either_player_but_not_a_hexproof_one`
   FAILED. Recorded because the vacuous first result is the useful part: a reader mutating this file needs to
   know the two arms exist.
3. `mill_cards(.., 2, ..)` instead of 3 → `nephalia_drownyard_mills_three` FAILED.
4. Hexproof check dropped from `can_target_player` → this card's new test FAILED, and
   `stensia_bloodhall_cannot_target_a_player_with_hexproof` with it.

### Tricky interactions checked
- Mills exactly three from the chosen player: **pass** (`cards_activated_abilities.rs:307`).
- Nothing happens until the ability resolves: **pass** (`activated_no_stack.rs:103`).
- May target yourself: **pass** (new).
- Cannot target a hexproofed opponent: **pass** (new).
- Fewer than three cards → mills what there is, no loss: **pass** (new).
- Milled creature cards reach Undead Alchemist: covered at the pipeline level
  (`trigger_dispatch.rs:454`, which calls `mill_cards` directly). Not re-tested through this card, since it is
  the same function call — recorded rather than claimed.
- Cannot fund its own tap ability; tapped → not offered: **pass** (`tap_cost_legality.rs`).

### Test coverage
- mills three: `cards_activated_abilities.rs:307`
- either player, but not a hexproof one: `cards_activated_abilities.rs:326` (new)
- short and empty libraries, and no loss: `cards_activated_abilities.rs:361` (new)
- stack timing: `activated_no_stack.rs:103`
- tap-cost legality and tapped-source: `tap_cost_legality.rs:200`, `tap_cost_legality.rs:270`

### Suite
`cargo check --workspace --all-targets` clean, zero warnings. Full suite exit 0, 1416 passing.

