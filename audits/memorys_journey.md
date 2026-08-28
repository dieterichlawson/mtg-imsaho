## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/66/memorys-journey?utm_source=api
**Type line**: `Instant` — {1}{U}
**Oracle text**:
```
Target player shuffles up to three target cards from their graveyard into their library.
Flashback {G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "Target player shuffles **up to three target cards from their graveyard**" —
  two target slots, the second nested `UpToTargets` inside `TwoTargets`, which
  is the shape that made the card uncastable when `valid_targets_for_req` had no
  `UpToTargets` branch: PASS
- "from **their** graveyard" — only the targeted player's, enforced at
  announcement rather than silently discarded at resolution (CR 601.2c): PASS
- CR 109.1 now keeps tokens out of the graveyard enumeration engine-side: PASS
- Castable with zero card targets: PASS
- Flashback {G} is a different colour from the {1}{U} front cost: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Castability, the up-to slot, and the per-player filter: `multi_target_and_mill.rs:memorys_journey_is_castable`, `:memorys_journey_can_be_cast_with_no_card_targets`, `:memorys_journey_only_offers_the_targeted_players_graveyard`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/66/memorys-journey?utm_source=api
**Type line**: `Instant` — {1}{U}
**Oracle text**:
```
Target player shuffles up to three target cards from their graveyard into their library.
Flashback {G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Rulings fetched**:
- [2021-03-19] If a card with flashback is put into your graveyard during your turn, you can cast it if it's legal to do so before any other player can take any actions.
- [2021-03-19] "Flashback [cost]" means "You may cast this card from your graveyard by paying [cost] rather than paying its mana cost" and "If the flashback cost was paid, exile this card instead of putting it anywhere else any time it would leave the stack."
- [2021-03-19] You must still follow any timing restrictions and permissions, including those based on the card's type. For instance, you can cast a sorcery using flashback only when you could normally cast a sorcery.
- [2021-03-19] To determine the total cost of a spell, start with the mana cost or alternative cost (such as a flashback cost) you're paying, add any cost increases, then apply any cost reductions. The mana value of the spell is determined only by its mana cost, no matter what the total cost to cast the spell was.
- [2021-03-19] A spell cast using flashback will always be exiled afterward, whether it resolves, is countered, or leaves the stack in some other way.
- [2021-03-19] You can cast a spell using flashback even if it was somehow put into your graveyard without having been cast.
- [2011-09-22] If the player is an illegal target by the time Memory's Journey resolves, the spell will have no effect, even if the cards are still legal targets. This is because a spell can't make an illegal target (the player) perform any actions (such as shuffling their library).
- [2011-09-22] Any of the targeted cards that are illegal targets by the time Memory's Journey resolves aren't shuffled into their owner's library.
- [2011-09-22] If no cards were targeted by Memory's Journey or if all the targeted cards are illegal targets by the time Memory's Journey resolves, the targeted player will still shuffle their library.
- [2011-09-22] If you cast Memory's Journey with flashback, it won't be in the graveyard when you choose targets. It can't target itself.
- [2011-09-22] You don't have to target any cards when you cast Memory's Journey, but you must target a player.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/66/memorys-journey
**Oracle text**:
```
Target player shuffles up to three target cards from their graveyard into their library.
Flashback {G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Type line**: Instant
**Mana cost**: {1}{U} — **Keywords**: Flashback
**Rulings** (11; the six generic flashback ones are omitted here and covered in the Bump in the Night entry):
7. (2011-09-22) "If the player is an illegal target by the time Memory's Journey resolves, the spell will have no effect, even if the cards are still legal targets. This is because a spell can't make an illegal target (the player) perform any actions (such as shuffling their library)."
8. (2011-09-22) "Any of the targeted cards that are illegal targets by the time Memory's Journey resolves aren't shuffled into their owner's library."
9. (2011-09-22) "If no cards were targeted by Memory's Journey or if all the targeted cards are illegal targets by the time Memory's Journey resolves, the targeted player will still shuffle their library."
10. (2011-09-22) "If you cast Memory's Journey with flashback, it won't be in the graveyard when you choose targets. It can't target itself."

**Status**: ISSUE (fixed)

### Card data
Matches the fetched text: `{1}{U}`, `card_types: [Instant]`, oracle text
verbatim including the flashback reminder, `flashback_cost: {G}`. The
requirement is
`TwoTargets(PlayerOnly, UpToTargets(3, GraveyardCardOwnedByTargetPlayer))`,
which is the sentence's shape: a player target, and card targets constrained by
it (CR 601.2c).

### Code issues

1. **Three cards reached past `helpers::shuffle_library`**
   (`memorys_journey.rs:70`, `mirror_mad_phantasm.rs:80`,
   `bitterheart_witch.rs:89`, plus a new guard).
   - `cards::helpers::shuffle_library` exists (CR 701.20), and Caravan Vigil
     and the search helpers use it. These three each wrote
     `state.get_player_mut(p).library_order.shuffle(&mut rand::thread_rng())`
     themselves.
   - Four copies of one rule, free to disagree about whose library is being
     shuffled — and every one of them in the way of seeding the RNG later: one
     call site can take a seed from the game state, four cannot.
   - All three routed through the helper, and
     `a_card_shuffles_a_library_through_the_helper` added to
     `test_suite_guards.rs` to keep it that way. `mulligan.rs` shuffles as part
     of the opening procedure (CR 103.2), not as a card effect, so the guard
     scans card files only.

2. **The card invented a target when it could not find one**
   (`memorys_journey.rs:41`, removed).
   - Oracle text says: `Target player shuffles`
   - The code said:
     `}).unwrap_or(controller); // If no player target, default to controller`
   - This is the pattern already removed from Tribute to Hunger, whose comment
     records why: "which invents a target the caster never declared; the rule
     for a target that stopped being legal is CR 608.2b, the spell does not
     resolve at all, and `stack::resolve_spell` applies it before this is ever
     called." Now an early return.
   - It is unreachable — the player slot is mandatory and `resolve_spell`
     substitutes `Target::Illegal` only for objects — so this is a shape fix,
     not a behaviour change, and no test can demonstrate it. Said plainly
     rather than dressed up.

3. **`is_target_legal`'s zone table was missing this card's requirement**
   (`stack.rs:51`, added).
   - `GraveyardCardOwnedByTargetPlayer` was not among the requirements that
     read a graveyard, so it fell through to the battlefield-or-stack arm and
     called every legal graveyard card illegal.
   - No observable difference today: `any_legal` is satisfied by the player
     target either way, and it is the card's own `zone == Graveyard` guard that
     implements ruling 8. Added because the table is meant to say which zone
     each requirement reads, not to be right by accident. Stated as such —
     no test demonstrates it.

4. **Rulings 8 and 9 had no test**
   (`cards_graveyard_interaction.rs:745`, two tests added).
   - **Ruling 8**: letting a card that had left the graveyard be moved anyway
     (`if { let _ = in_gy; true } && ..`) produced zero failures — every
     existing test resolves against cards that stayed put. Added
     `memorys_journey_skips_a_card_that_left_the_graveyard`: two targeted cards,
     one exiled in response, only the other moves. That mutation now fails.
   - **Ruling 9**: added `memorys_journey_resolves_on_the_player_alone` — "up
     to three" includes none, so the spell resolves on the player alone rather
     than fizzling for want of a card, and the library's contents are unchanged.

### What is still not covered, and why
Skipping the shuffle entirely when no cards moved **still** passes the suite,
and so does shuffling the wrong player's library. Neither is observable while
the RNG is unseeded: a shuffle rearranges a list, and with no seed a test
cannot say what the rearrangement should be, nor distinguish "shuffled" from
"left alone" without flakiness. The new ruling-9 test pins down the two things
that *are* observable — the spell resolves, and the library gains and loses
nothing — and says so in its own comment rather than implying more. Routing all
three cards through `shuffle_library` is what makes seeding a one-line change
when that infrastructure lands.

### Tricky interactions checked
- Ruling 8 (illegal card targets are skipped): PASS — new test.
- Ruling 9 (resolves on the player alone): PASS for its observable half — new
  test; the shuffle itself is not observable, see above.
- Ruling 7 (illegal *player* target means no effect at all): the code would
  proceed, since `resolve_spell` substitutes `Target::Illegal` only for
  objects. Not reachable in this pool: the only source of player hexproof is
  Witchbane Orb, a static ability on an artifact, so a player who has it was
  never targetable in the first place and one who does not cannot gain it in
  response. Recorded rather than fixed speculatively.
- Ruling 10 (cast with flashback, it cannot target itself): the spell is on the
  stack when targets are chosen, so it is not in the graveyard to be offered.
  Structural.
- Cards come from the **targeted** player's graveyard, not any:
  PASS — `cards_shortcuts_taken.rs:248`, `auto_pick.rs:456`, and
  `resolving_only_moves_the_targeted_players_cards`.
- The requirement's shape (a player, then their cards): PASS —
  `memorys_journey_targets_a_player_and_then_their_cards`, matched structurally.
- Up to three cards: PASS — `shuffles_up_to_three_cards`.
- Own and opponent's graveyard: PASS — two tests.
- Flashback for {G}, then exiled: covered by the flashback suite.
- Self-cleanup: `on_resolve` moves the targeted cards, never itself. PASS.

### Test coverage
- Moves a card from your own / the opponent's graveyard:
  `cards_graveyard_interaction.rs:673`, `:688`.
- Up to three at once: `:702`.
- Only the targeted player's cards: `:727`, plus `cards_shortcuts_taken.rs:248`
  and `auto_pick.rs:456`.
- Requirement shape: `memorys_journey_targets_a_player_and_then_their_cards`.
- Ruling 8: `memorys_journey_skips_a_card_that_left_the_graveyard` —
  **added this audit**.
- Ruling 9 (observable half): `memorys_journey_resolves_on_the_player_alone` —
  **added this audit**.
- Ruling 7: NOT TESTED — unreachable in this pool, see above.
- The shuffle itself: NOT TESTED — unobservable with an unseeded RNG.

### Mutations run
| mutation | result |
| --- | --- |
| move a targeted card that has left the graveyard | fails the new ruling-8 test (before: **nothing at all**) |
| hand-roll the shuffle again instead of calling the helper | fails the new guard |
| skip the shuffle when no cards moved | **nothing** — unobservable, see above |

Suite after: 1459 passing, exit 0, zero warnings.

