## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/218/cellar-door?utm_source=api
**Type line**: `Artifact` — {2}
**Oracle text**:
```
{3}, {T}: Target player puts the bottom card of their library into their graveyard. If it's a creature card, you create a 2/2 black Zombie creature token.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "the **bottom** card of their library" — `library_order[len - 1]`, and the
  bottom is what `mill_cards` cannot express, which is why this goes through
  `mill_one` directly: PASS
- "If it's a creature card, **you** create" — the token goes to Cellar Door's
  controller, not the milled player: PASS
- The Zombie token carries its subtype via `create_token_with_subtypes`: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Milling from the bottom still emits CreatureCardMilled: `multi_target_and_mill.rs:cellar_door_emits_creature_card_milled`
- The Zombie is created for a creature card: `cards_complex_creatures.rs:cellar_door_creates_zombie_when_milling_creature`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/218/cellar-door?utm_source=api
**Type line**: `Artifact` — {2}
**Oracle text**:
```
{3}, {T}: Target player puts the bottom card of their library into their graveyard. If it's a creature card, you create a 2/2 black Zombie creature token.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/218/cellar-door
**Oracle text**: {3}, {T}: Target player puts the bottom card of their library into their graveyard. If it's a creature card, you create a 2/2 black Zombie creature token.
**Type line**: Artifact
**Mana cost**: {2}
**Rulings**: none (Scryfall returns no rulings for this card)
**Status**: ISSUE (fixed) — the card code is correct; its tests could not see any of it.

### Card data
Matches the fetched text: `{2}`, `card_types: [Artifact]`, oracle text
verbatim, no subtypes, no P/T, no keywords. The ability is `{3}` plus
`requires_tap: true`, and `activated_abilities` carries no zone-or-tapped guard
— the comment already there explains why, and it is the right call.

### Code issues

No issue in `cellar_door.rs`. Everything below is what was holding it up.

1. "the **bottom** card of their library" was untested
   (`cards_complex_creatures.rs:755`, rebuilt).
   - Oracle text says: `Target player puts the bottom card of their library into their graveyard`
   - The card does:
     `let last_idx = player.library_order.len() - 1; let milled_id = player.library_order[last_idx];`
   - The test put **one** creature card in P1's library — where the bottom card
     and the top card are the same object — and its comment even read "Put a
     creature card on top of P1's library", the opposite of what the card does.
   - Verified: replacing that with `player.library_order[0]` — mill from the
     **top** — produced zero failures across the whole workspace.

2. "**If it's a creature card**" was untested (same test, same cause).
   - Verified: `if is_creature || true`, so the token is created whatever was
     milled, produced zero failures across the whole workspace. The `else`
     branch had never been reached by a test.

3. "**you** create a 2/2 black Zombie creature token" was untested
   (`cards_complex_creatures.rs`, test added).
   - The card reads `helpers::ability_controller(state, object_id)` — the
     activator (CR 602.2a), with CR 608.2g's last-known-controller fallback.
     That is right, and it is not the player whose library was milled.
   - Verified: creating the token under `*player_id` instead produced zero
     failures across the whole workspace. The only assertion was
     `count_tokens_named(&state, "Zombie Token") == 1`, which counts both
     players' tokens and checks nothing about the token but its name.

Rebuilt as a parametric test with a decoy at the other end of the library:
one row with `Walking Corpse` on the bottom and `Forest` on top, one reversed.
Each row asserts which card moved, that the other stayed in the library, and
whether a Zombie appeared — so rows 1 and 2 above both fail now. A second test
checks the token is a 2/2 **black Zombie** controlled by the **activating**
player and not by the target; a third checks an empty library.

### Tricky interactions checked
- Milling from the bottom still announces the mill, so an opponent's Undead
  Alchemist sees it: PASS — `multi_target_and_mill.rs:127`
  (`cellar_door_emits_creature_card_milled`). The card routes through
  `mill_one` precisely because `mill_cards` cannot express "from the bottom".
- Empty library: nothing milled, no token, no stall. PASS — new test.
- The source leaves the battlefield or changes hands in response: the card
  reads `ability_controller` rather than `o.controller`, so the activator keeps
  the token (CR 602.2a) and a destroyed Cellar Door still resolves its ability
  (CR 113.7a). Covered structurally by the helper, which exists for this; the
  comment in the card records the bug it fixed. Not re-tested per card.
- Target player becomes untargetable before resolution: the ability path
  re-checks targets and fizzles. Generic, covered by
  `an_activated_abilitys_targets_are_rechecked_when_it_resolves`.
- Targeting yourself: legal — "target player", not opponent. `PlayerOnly` is
  right.
- A creature **token** milled: impossible, a library holds only cards, so the
  absence of an `is_card` check here is correct rather than an oversight. The
  graveyard-enumeration guard does not apply — this reads one known object, not
  a graveyard.
- The token's name: the card passes `""` and `create_token_with_subtypes`
  derives "Zombie Token" from the subtypes (CR 111.4), which is what all
  fourteen token-making cards in the set do. Not a missing name.
- `{T}` cost legality and summoning sickness: the engine's, and the card is in
  the `tap_cost_legality.rs:198` list that checks it does not re-decide them.

### UI presentation
Ability description: `"{3}, {T}: Target player mills a card, maybe create Zombie"`
— loose next to the printed text ("the bottom card", "if it's a creature card"),
but unambiguous about cost and target. Log lines name the source in both
branches: `"Cellar Door milled a creature, created a 2/2 Zombie token"` and
`"Cellar Door milled a non-creature card"`.

### Test coverage
- Bottom card is the one milled, top card stays:
  `cards_complex_creatures.rs`
  (`cellar_door_mills_the_bottom_card_and_zombies_only_for_a_creature`) —
  **added this audit**.
- Zombie only for a creature card, both directions: same test — **added this audit**.
- Token is a 2/2 black Zombie under the activator:
  `cellar_doors_zombie_is_a_two_two_black_zombie_for_the_activating_player` —
  **added this audit**.
- Empty library: `cellar_door_does_nothing_to_an_empty_library` — **added this audit**.
- The mill is announced (`CreatureCardMilled`): `multi_target_and_mill.rs:127`.
- Tap-cost legality is not re-decided by the card: `tap_cost_legality.rs:198`.
- No rulings exist for this card, so there is no per-ruling row to fill.

### Mutations run
| mutation | result |
| --- | --- |
| mill `library_order[0]` (the top) instead of the bottom | fails the rebuilt test (before: **nothing at all**) |
| `if is_creature \|\| true` — always make the token | fails the rebuilt test (before: **nothing at all**) |
| token created under the target player instead of the activator | fails the new ownership test (before: **nothing at all**) |

Suite after: 1448 passing, exit 0, zero warnings.

