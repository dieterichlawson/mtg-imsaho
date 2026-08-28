## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/92/bump-in-the-night?utm_source=api
**Type line**: `Sorcery` — {B}
**Oracle text**:
```
Target opponent loses 3 life.
Flashback {5}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Status**: ISSUE

### Code issues
See below.


- The life change was written out by hand — read `life`, write `life`, push
  `LifeChanged` — rather than going through `GameState::change_life`, whose own
  doc says why it exists: "Every caller used to hand-roll this ... which meant a
  site that forgot the event silently broke any 'whenever you gain life'
  watcher." Twelve cards were still hand-rolling it. Collapsed onto the helper,
  with a guard to keep it that way.

### Tricky interactions checked
- "Target **opponent** loses 3 life" — `is_valid_target` rejects the caster, so
  you cannot point it at yourself: PASS
- Life **loss**, not damage: it bypasses protection, prevention and damage
  triggers, which is why it does not go through `deal_damage`: PASS
- Flashback {5}{R} is a different colour from the {B} front cost, and the card
  is exiled after the flashback resolution (CR 702.33a): PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The life loss and the opponent-only restriction: `cards_burn_and_damage.rs`
- Flashback from the graveyard and exile: `cards_flashback.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/92/bump-in-the-night?utm_source=api
**Type line**: `Sorcery` — {B}
**Oracle text**:
```
Target opponent loses 3 life.
Flashback {5}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```

**Rulings fetched**:
- [2021-03-19] “Flashback [cost]” means “You may cast this card from your graveyard by paying [cost] rather than paying its mana cost” and “If the flashback cost was paid, exile this card instead of putting it anywhere else any time it would leave the stack.”
- [2021-03-19] You must still follow any timing restrictions and permissions, including those based on the card’s type. For instance, you can cast a sorcery using flashback only when you could normally cast a sorcery.
- [2021-03-19] To determine the total cost of a spell, start with the mana cost or alternative cost (such as a flashback cost) you’re paying, add any cost increases, then apply any cost reductions. The mana value of the spell is determined only by its mana cost, no matter what the total cost to cast the spell was.
- [2021-03-19] A spell cast using flashback will always be exiled afterward, whether it resolves, is countered, or leaves the stack in some other way.
- [2021-03-19] You can cast a spell using flashback even if it was somehow put into your graveyard without having been cast.
- [2021-03-19] If a card with flashback is put into your graveyard during your turn, you can cast it if it’s legal to do so before any other player can take any actions.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/92/bump-in-the-night
**Oracle text**:
```
Target opponent loses 3 life.
Flashback {5}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
```
**Type line**: Sorcery
**Mana cost**: {B} — **Keywords**: Flashback
**Rulings** (6, all 2021-03-19, all generic flashback rulings):
1. "Flashback [cost]" means "You may cast this card from your graveyard by paying [cost] rather than paying its mana cost" and "If the flashback cost was paid, exile this card instead of putting it anywhere else any time it would leave the stack."
2. You must still follow any timing restrictions and permissions, including those based on the card's type. For instance, you can cast a sorcery using flashback only when you could normally cast a sorcery.
3. To determine the total cost of a spell, start with the mana cost or alternative cost (such as a flashback cost) you're paying, add any cost increases, then apply any cost reductions. The mana value of the spell is determined only by its mana cost, no matter what the total cost to cast the spell was.
4. A spell cast using flashback will always be exiled afterward, whether it resolves, is countered, or leaves the stack in some other way.
5. You can cast a spell using flashback even if it was somehow put into your graveyard without having been cast.

**Status**: ISSUE (fixed)

### Card data
Matches the fetched text: `{B}`, `card_types: [Sorcery]`, oracle text verbatim
including the flashback reminder, `flashback_cost: {5}{R}`. No P/T, no
subtypes, no continuous effects, no triggered abilities.

### Code issues

1. "Target opponent" was written out twice, per card
   (`bump_in_the_night.rs`, `tribute_to_hunger.rs`, `cards/mod.rs`,
   `targeting.rs`, `stack.rs` — new `TargetRequirement::OpponentOnly`).
   - Oracle text says: `Target opponent loses 3 life.`
   - The card said: `TargetRequirement::PlayerOnly`, plus
     `fn is_valid_target(..) { Target::Player(pid) => *pid != caster, .. }`
   - CR 102.1 makes "target opponent" a targeting restriction like any other,
     and the engine had no way to say it — so the two cards in the set that use
     it (this one and Tribute to Hunger) each subtracted the caster by hand. I
     noted this during the Victim of Night audit and left it for whichever card
     came first; this is that card.
   - Added `TargetRequirement::OpponentOnly`, enumerated in **both** targeting
     arms — measured from the caster for a spell, and from the *activator* for
     an ability (CR 602.2a), not from whoever holds the source — and re-checked
     in `stack::is_target_legal` for CR 608.2b. Both cards now declare it and
     have no `is_valid_target` at all.
   - The resolution-time half is new ground rather than a move: previously the
     restriction reached resolution only because `stack::resolve_spell` also
     consults `is_valid_target`, and the ability path checks it differently.
     Removing the new `is_target_legal` branch now fails
     `bump_in_the_night_cannot_be_pointed_at_its_own_caster`.

2. The card was tested as a damage spell (`spells.rs:45`, split out).
   - Oracle text says: `Target opponent loses 3 life.`
   - It was a row in `direct_damage_spells_drain_player_life`, whose assertion
     message is `"{name} should deal {damage} damage to the targeted player"`.
   - Life loss is not damage: no `NonCombatDamageDealt`, so nothing that
     watches for damage sees it, and none of the damage pipeline's prevention or
     protection applies. The implementation was right — `state.change_life(*player_id, -3)`
     — but nothing held it to that; making the card deal 3 non-combat damage
     instead would have passed the row it was in.
   - Split into `bump_in_the_night_makes_an_opponent_lose_life_rather_than_dealing_damage`,
     which asserts the life total **and** that no damage event was emitted. The
     damage mutation now fails it.
   - The card also now calls `state.lose_life(player, 3)` rather than
     `change_life(player, -3)`. Same arithmetic; `lose_life`'s own doc comment
     is "this is life LOSS, which is not damage — it bypasses protection,
     prevention and damage triggers", which is the distinction at issue.

3. Two comments quoted the card as saying something it does not
   (`cards_lands_and_mana_sources.rs:719`, `hexproof_filter.rs:737`, corrected).
   - Oracle text says: `Target opponent loses 3 life.`
   - Both read: `Bump in the Night: "Target player loses 3 life."`
   - Both tests were doing the right thing (P0 casting at P1, an opponent), but
     the comments asserted a permission the card does not grant.

### Tricky interactions checked
- Cannot target yourself, at cast time: PASS — the caster is not offered.
- Cannot target yourself, on resolution: PASS — driven directly onto the stack
  at P0 and countered by game rules, no life lost.
- Hexproof opponent (Witchbane Orb) is not a legal target, both ways: PASS —
  `opponent_cannot_target_hexproof_player`,
  `the_resolution_recheck_uses_the_same_player_targeting_rule`.
- Flashback from the graveyard for {5}{R}, then exiled (rulings 1 and 4): PASS —
  `flashback.rs:380`.
- Sorcery timing applies to the flashback cast too (ruling 2): the engine's
  timing check reads the card's types, not how it is being paid for; covered
  generically by `sorcery_timing_restricts_sorceries_and_leaves_instants_alone`.
  Not re-tested per card.
- Mana value is {B} = 1 regardless of the flashback cost paid (ruling 3): the
  card's `cost` is what `card_data` carries and `flashback_cost` is separate, so
  nothing reads {5}{R} as the mana cost. No in-pool card keys off this spell's
  mana value, so there is nothing to observe; noted rather than tested.
- Flashback from a graveyard it reached without being cast (ruling 5): the
  flashback offer reads the card's zone, not how it got there. Covered by
  `flashback.rs:380`, which puts it in the graveyard directly.
- Multiple copies in the graveyard: `flashback_multiple_instances.rs` uses this
  very card.
- Life loss with an empty library / at 0 life: SBA territory, not this card's.
- Self-cleanup: `on_resolve` moves nothing; the engine owns the spell, and the
  flashback exile is `move_spell_after_resolve`'s job. PASS.

### UI presentation
No choices. The only prompt is the target, and `OpponentOnly` offers exactly the
opponents, so nothing has to be filtered out downstream.

### Test coverage
- Loses 3 life, and it is not damage: `spells.rs`
  (`bump_in_the_night_makes_an_opponent_lose_life_rather_than_dealing_damage`) —
  **added this audit**.
- "Target opponent", both directions: `spells.rs`
  (`bump_in_the_night_cannot_be_pointed_at_its_own_caster`) — **added this audit**;
  and a row in `characteristics_targeting.rs`
  (`a_cards_target_filter_matches_its_wording`) — **added this audit**.
- Flashback resolves and exiles (rulings 1, 4, 5): `flashback.rs:380`.
- Multiple graveyard copies: `flashback_multiple_instances.rs`.
- Hexproof player at cast time: `cards_lands_and_mana_sources.rs:711`.
- Hexproof player at resolution: `hexproof_filter.rs:732`.
- Sorcery timing (ruling 2): generic, `spells.rs`
  (`sorcery_timing_restricts_sorceries_and_leaves_instants_alone`).
- Mana value vs total cost (ruling 3): NOT TESTED — nothing in the pool reads
  this spell's mana value, so there is no observable behaviour to assert.

### Mutations run
| mutation | result |
| --- | --- |
| `targeting.rs`: `OpponentOnly` drops `p.id != caster` in both arms | fails `bump_in_the_night_cannot_be_pointed_at_its_own_caster` and the wording table |
| `stack.rs`: drop the new `OpponentOnly` branch from `is_target_legal` | fails `bump_in_the_night_cannot_be_pointed_at_its_own_caster` |
| card: declare `PlayerOnly` again | fails both of the above |
| card: deal 3 non-combat damage instead of losing 3 life | fails `bump_in_the_night_makes_an_opponent_lose_life_rather_than_dealing_damage` (in the damage table it used to sit in: **would have passed**) |

Suite after: 1442 passing, exit 0, zero warnings.

