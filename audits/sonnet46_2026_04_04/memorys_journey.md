## Audit — 2026-04-04 09:14

**Oracle text source**: Scryfall API (pre-fetched)
**Oracle text**: Target player shuffles up to three target cards from their graveyard into their library.
Flashback {G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
**Type line**: Instant
**Status**: ISSUE

### Code issues

- **Missing mandatory `Target::Player` target — opponent cannot be targeted with 0 card targets, and player hexproof is never checked** (`mtg-engine/src/cards/isd/memorys_journey.rs` lines 39–43, 49–55)
  - Oracle text says: `"Target player shuffles up to three target cards from their graveyard into their library."` and ruling: `"You don't have to target any cards when you cast Memory's Journey, but you must target a player."`
  - Code does: `target_requirement` returns `TargetRequirement::ModalChoice(vec![TargetRequirement::UpToTargets(3, Box::new(TargetRequirement::GraveyardCardOwnedByCaster)), TargetRequirement::UpToTargets(3, Box::new(TargetRequirement::GraveyardCardOwnedByOpponent))])`. This structure generates only `Target::Object` actions for graveyard cards — it never generates a `Target::Player` target. The "target player" from the oracle is not an explicit targeting choice at all.

  The consequences are:

  1. **0-card case always shuffles controller's library.** The engine generates `CastSpell { targets: vec![] }` for the k=0 combination in both UpToTargets modes. On resolve, `on_resolve` runs `unwrap_or(controller)` (line 55): `").unwrap_or(controller)`. There is no mechanism to choose an opponent as the "target player" when selecting zero cards — casting with 0 targets is permanently wired to shuffle the controller's library. The oracle/ruling explicitly allows targeting an opponent with 0 cards.

  2. **Player hexproof not checked.** `can_target_player` (which gates on Witchbane Orb hexproof) is only called when a `Target::Player` flows through a `PlayerOnly` / `AnyTarget` requirement. Because `ModalChoice(UpToTargets(GraveyardCardOwnedByOpponent))` never produces a `Target::Player`, `can_target_player` is never invoked for the targeted player. An opponent with Witchbane Orb can be freely "targeted" (their library shuffled) by choosing their graveyard cards.

  3. **Player legality not checked on resolution.** Ruling: `"If the player is an illegal target by the time Memory's Journey resolves, the spell will have no effect, even if the cards are still legal targets."` Because there is no `Target::Player` in the stored targets, `on_resolve` cannot detect that the player became illegal; it always proceeds.

### Tricky interactions checked

- **"Must target a player" — player as mandatory required target**: FAIL. The `target_requirement` contains no `Target::Player`; the "target player" is inferred from card ownership during resolution, not declared as a target at cast time.
- **0-card cast targeting opponent (valid per ruling)**: FAIL. With 0 card targets, `on_resolve` `unwrap_or(controller)` forces shuffle of controller's library; there is no way to produce a `CastSpell` action that targets an opponent's library with 0 graveyard cards.
- **Player hexproof (Witchbane Orb) blocks targeting**: FAIL. `can_target_player` is never called because no `Target::Player` is generated in the `ModalChoice` path.
- **Player becomes illegal target before resolution**: FAIL. No player legality re-check occurs at resolution because there is no stored `Target::Player` to re-validate.
- **Cards from illegal graveyard skipped while player still shuffles (ruling: "Any of the targeted cards that are illegal targets... aren't shuffled")**: PASS. `on_resolve` checks `in_gy` before moving each card, and shuffles unconditionally afterward.
- **"Up to three" — 0 to 3 cards from a single player's graveyard**: PASS for card-count mechanics when ≥1 card is targeted; FAIL for the 0-card opponent-targeting case.
- **Cards cannot come from two different players' graveyards simultaneously**: PASS. `ModalChoice` with separate modes (`GraveyardCardOwnedByCaster` vs. `GraveyardCardOwnedByOpponent`) prevents mixed-graveyard combinations.
- **Flashback exiles instead of going to graveyard**: PASS. `move_spell_after_resolve` checks `cast_with_flashback` and calls `move_object(Exile)` accordingly (`state.rs` line 1136).
- **Flashback cost is {G}**: PASS. `flashback_cost: Some(ManaCost::new(vec![ManaSymbol::Colored(Color::Green)]))` at line 27.
- **Mana cost {1}{U}**: PASS. `cost: Some(ManaCost::new(vec![ManaSymbol::Generic(1), ManaSymbol::Colored(Color::Blue)]))` matches oracle.
- **Card type Instant**: PASS. `card_types: vec![CardType::Instant]`.

### Test coverage

- Basic shuffle of own graveyard card into library: `memorys_journey.rs:21` TESTED
- Shuffle of opponent's graveyard card: `memorys_journey.rs:37` TESTED
- Up to 3 cards from same graveyard: `memorys_journey.rs:53` TESTED
- Legal actions do not mix cards from different graveyards: `memorys_journey.rs:78` TESTED
- Flashback cost exists and is {G}: `memorys_journey.rs:121`, `tier11_cards.rs:364` TESTED
- **0-card cast targeting opponent shuffles opponent's library**: NOT TESTED
- **Player hexproof (Witchbane Orb) prevents targeting**: NOT TESTED
- **Player becomes illegal target before resolution — spell has no effect**: NOT TESTED
- **All card targets become illegal but player still shuffles**: NOT TESTED
