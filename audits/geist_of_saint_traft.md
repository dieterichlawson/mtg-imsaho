## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/213/geist-of-saint-traft?utm_source=api
**Type line**: `Legendary Creature — Spirit Cleric` — {1}{W}{U}, 2/2
**Oracle text**:
```
Hexproof (This creature can't be the target of spells or abilities your opponents control.)
Whenever Geist of Saint Traft attacks, create a 4/4 white Angel creature token with flying that's tapped and attacking. Exile that token at end of combat.
```

**Status**: ISSUE

### Code issues
See below.


- Its `on_resolve` override was `state.move_object(object_id, Zone::Battlefield,
  registry)` plus `obj.is_legendary = true` — exactly the trait default, written
  out again. Deleted; a guard now fails the build on a card that moves itself
  onto the battlefield.

### Tricky interactions checked
- "create a 4/4 white Angel creature token with flying **that's tapped and
  attacking**" — it does not trigger attack triggers, because it was never
  declared as an attacker (CR 508.4): PASS
- "**Exile** that token at end of combat" — a delayed trigger, and exiling a
  token means it ceases to exist either way: PASS
- Hexproof stops opponents targeting the Geist but not blocking it, and not
  board wipes: PASS
- Legendary: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The Angel token and its exile: `cards_complex_creatures.rs`, `trigger_dispatch.rs`
- Hexproof: `hexproof_filter.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/213/geist-of-saint-traft?utm_source=api
**Type line**: `Legendary Creature — Spirit Cleric` — {1}{W}{U}, 2/2
**Oracle text**:
```
Hexproof (This creature can't be the target of spells or abilities your opponents control.)
Whenever Geist of Saint Traft attacks, create a 4/4 white Angel creature token with flying that's tapped and attacking. Exile that token at end of combat.
```

**Rulings fetched**:
- [2017-07-17] Geist of Saint Traft is banned as a commander in Duel Commander format, but it may be part of your deck.
- [2020-08-07] You choose which player or planeswalker the Angel token is attacking. It doesn't have to be attacking the same player or planeswalker that Geist of Saint Traft is attacking.
- [2020-08-07] Although the Angel is an attacking creature, it was never declared as an attacking creature. This means that abilities that trigger whenever a creature attacks won't trigger when it enters the battlefield attacking.
- [2020-08-07] Any effects that say that the Angel can't attack (such as that of Propaganda) affect only the declaration of attackers. They won't stop the Angel token from entering the battlefield attacking.
- [2020-08-07] If you create more than one Angel token (most likely due to Doubling Season), both are exiled at end of combat. On the other hand, if something else becomes a copy of the Angel token, the copy isn't exiled.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/213/geist-of-saint-traft
**Oracle text**:
```
Hexproof (This creature can't be the target of spells or abilities your opponents control.)
Whenever Geist of Saint Traft attacks, create a 4/4 white Angel creature token with flying that's tapped and attacking. Exile that token at end of combat.
```
**Type line**: Legendary Creature — Spirit Cleric
**Mana cost**: {1}{W}{U} — **P/T**: 2/2 — **Keywords**: Hexproof
**Rulings** (5):
1. (2017-07-17) Banned as a commander in Duel Commander — a format rule, nothing to implement.
2. (2020-08-07) "You choose which player or planeswalker the Angel token is attacking. It doesn't have to be attacking the same player or planeswalker that Geist of Saint Traft is attacking."
3. (2020-08-07) "Although the Angel is an attacking creature, it was never declared as an attacking creature. This means that abilities that trigger whenever a creature attacks won't trigger when it enters the battlefield attacking."
4. (2020-08-07) "Any effects that say that the Angel can't attack (such as that of Propaganda) affect only the declaration of attackers. They won't stop the Angel token from entering the battlefield attacking."
5. (2020-08-07) "If you create more than one Angel token (most likely due to Doubling Season), both are exiled at end of combat. On the other hand, if something else becomes a copy of the Angel token, the copy isn't exiled."

**Status**: ISSUE (fixed) — the card code is correct; two of the token's printed adjectives and ruling 3 had no test.

### Card data
Matches the fetched text: `{1}{W}{U}`, `card_types: [Creature]`,
`supertypes: [Legendary]`, `subtypes: ["Spirit", "Cleric"]` (both), 2/2,
`keywords: [Hexproof]`, oracle text verbatim, and one `TriggeredAbilityDef` of
kind `Attacks` matching the one implemented hook.

### Code issues

No issue in `geist_of_saint_traft.rs`. Three mutations passed the entire
workspace.

1. **The token's colour and flying had no assertion**
   (`geist_of_saint_traft.rs` test file, `geist_creates_angel_on_attack`, extended).
   - Oracle text says: `a 4/4 white Angel creature token with flying`
   - The test read `angel.power`, `angel.toughness` and `angel.tapped` off the
     raw object and nothing else.
   - Verified: creating it `Color::Black`, and creating it with no keywords,
     each produced zero failures.
   - Every adjective is now asserted through the accessors — `effective_power`,
     `effective_toughness`, `colors_of`, `has_subtype`, `is_creature`,
     `has_keyword` — plus that it is in `combat.attackers`.

2. **Ruling 3 had no test** (same file, test added).
   - Ruling says: `Although the Angel is an attacking creature, it was never declared as an attacking creature.`
   - No card in this pool triggers on *another* creature attacking (the five
     `TriggerKind::Attacks` users are Geist, Kessig Cagebreakers, Hamlet
     Captain, Trepanation Blade and Grimgrin, all of which watch themselves or
     their equipped creature), so the ruling's stated consequence has no
     in-pool observer. What **is** observable is the CR 508.1 stamp the engine
     keeps for "was declared as an attacker" — `attacked_on_turn`, read by
     `state.attacked_this_turn` and depended on by Civilized Scholar's end-step
     trigger.
   - `combat::declare_attackers` sets it; the card, which inserts the Angel
     into `combat.attackers` directly, does not — correctly. Verified: adding
     `obj.attacked_on_turn = Some(turn)` beside the insert produced zero
     failures before the new test, and fails it now.
   - The test drives Geist through `submit_declare_attackers` rather than the
     `attacks_unblocked` helper, because that helper builds a `CombatState` by
     hand and stamps neither creature — which would have made the comparison
     between them meaningless.

### Tricky interactions checked
- **Ruling 2** (the Angel attacks whoever Geist attacks — read, not
  re-derived): PASS, and covered twice —
  `the_angel_token_attacks_whoever_geist_is_attacking` in a two-player game and
  `the_angel_attacks_geists_defender_and_not_just_the_next_player` in a
  three-player one, where "the opponent" and "the defender" differ. The card
  reads `attack.defending_player` from the trigger rather than
  `state.opponent(controller)`, and the comment records why.
  (The ruling's first sentence — *you choose* which player the Angel attacks —
  is not modelled: the card sends it at Geist's defender. That is the ruling's
  own default and the only choice available in a two-player game; a chooser
  would be new plumbing for a case this pool cannot reach. Recorded, not built.)
- **Ruling 3**: PASS — new test.
- Token is 4/4, white, an Angel, a creature, flying, tapped, attacking: PASS —
  extended test.
- Exiled at end of combat: PASS — `angel_exiled_at_end_of_combat`.
- Exiled even if Geist has died first (CR 603.7d, a delayed trigger recorded at
  attack time): PASS — `angel_exiled_even_if_geist_dies`.
- The exile is a *triggered ability* on the stack, not a turn-based action that
  bypasses priority: PASS — `geist_angel_exile_is_triggered_not_turn_based`.
- No spurious end-of-combat trigger on a turn Geist did not attack: PASS —
  `geist_no_spurious_end_combat_trigger_when_did_not_attack`.
- **Ruling 5** (more than one Angel: all exiled; a copy of the Angel: not):
  the card pushes one `EndOfCombatExileEntry` per token it created, keyed on
  that token's id, so both halves fall out of the design — a copy made
  elsewhere has a different id and no entry. Nothing in this pool doubles
  tokens or copies one, so there is nothing to observe; structural, not tested.
- **Ruling 4** (Propaganda-style "can't attack" doesn't stop it entering
  attacking): no such effect exists in this pool. The card does not consult any
  attack restriction, which is the correct behaviour; nothing to test.
- Hexproof: a keyword read through `has_keyword`, covered by the hexproof
  suite.
- Legendary: the supertype is declared; the legend rule is `sba.rs`'s.
- Self-cleanup: none; this is a permanent.

### UI presentation
Trigger description: "create a 4/4 Angel token tapped and attacking". Log line:
"Geist of Saint Traft: created a 4/4 Angel token tapped and attacking". The
delayed exile carries its own description, "exile the Angel token".

### Test coverage
- Token is a 4/4 white flying Angel creature, tapped and attacking:
  `geist_of_saint_traft.rs` (`geist_creates_angel_on_attack`) —
  **colour, flying, subtype, creature-ness and attacking added this audit**.
- Ruling 3 (attacking but never declared):
  (`the_angel_is_attacking_but_was_never_declared_an_attacker`) —
  **added this audit**.
- Ruling 2 (the right defender), two- and three-player: two existing tests.
- Exile at end of combat, after Geist dies, on the stack rather than turn-based,
  and no spurious trigger: four existing tests.
- Rulings 1, 4, 5: NOT TESTED — nothing in this pool can reach them; see above.

### Mutations run
| mutation | result |
| --- | --- |
| token created without flying | fails the extended test (before: **nothing at all**) |
| token created `Color::Black` | fails the extended test (before: **nothing at all**) |
| stamp the Angel `attacked_on_turn = Some(turn)` | fails the new ruling-3 test (before: **nothing at all**) |
| token not tapped | fails the extended test |

Suite after: 1461 passing, exit 0, zero warnings.


## Follow-up — 2026-08-28 — the deferred token attack-target choice, implemented

**Status**: ISSUE (fixed) — the deferral recorded under ruling 2 is closed.

The full audit recorded: "The ruling's first sentence — *you choose* which
player or planeswalker the Angel attacks — is not modelled: the card sends it
at Geist's defender... a chooser would be new plumbing for a case this pool
cannot reach. Recorded, not built." Planeswalker combat has since been
implemented (Garruk Relentless and Liliana of the Veil are attackable), so the
case is reachable and the plumbing now exists.

### What changed
- `helpers::tokens_enter_combat_attacking` — the shared CR 508.4b mechanism
  for a token put onto the battlefield tapped and attacking: it arrives
  tapped, is never declared (no `attacked_on_turn` stamp, no attack triggers),
  and its controller chooses which player or planeswalker it is attacking.
  The legal options are every surviving opponent plus every planeswalker an
  opponent controls (`helpers::token_attack_options`); with exactly one option
  — a two-player board with no walkers — `present_target_choice`'s mandatory
  single-option auto-apply asks nothing, so the pre-existing behaviour is the
  degenerate case of the rule rather than a special case beside it.
- `PendingEffect::TokenAttacks` carries the chain: one choice per token, the
  next raised as each is answered, with no priority window in between.
- A choice of a planeswalker inserts the attacker against the walker's
  controller (CR 508.1a) and records the walker in `planeswalker_defenders`,
  which is where the combat damage step already looks.
- Geist's `on_attacks` no longer reads `attack.defending_player` at all: the
  answer comes from the choice (or the sole option), not from an assumed
  default.

### Test coverage
- The Angel sent at Liliana, loyalty damage, walker dies to the SBA:
  `planeswalker_combat.rs::geists_angel_can_be_sent_at_a_planeswalker` (new)
- Two live opponents prompt the controller, who may pick either:
  `geist_of_saint_traft.rs::the_angel_attacks_geists_defender_and_not_just_the_next_player`
  (rewritten — it now answers the prompt the ruling requires instead of
  asserting a default)
- Two-player, no walkers: no prompt, Angel at the only opponent — the
  pre-existing tests pass unchanged through the auto-apply path.

### Mutations run
- Walker choice recorded without the `planeswalker_defenders` entry: fails
  both new walker tests.
