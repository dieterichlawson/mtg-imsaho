## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/40/urgent-exorcism?utm_source=api
**Type line**: `Instant` — {1}{W}
**Oracle text**:
```
Destroy target Spirit or enchantment.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "target **Spirit** or **enchantment**" — a subtype on one side and a card type
  on the other, so a Spirit creature and a non-Spirit Aura are both legal: PASS
- `has_subtype` reads the object's granted subtypes as well as the printed ones,
  so a token Spirit qualifies: PASS
- `try_destroy`, so indestructible survives: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Both halves of the filter: `cards_removal.rs`, `subtype.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/40/urgent-exorcism?utm_source=api
**Type line**: `Instant` — {1}{W}
**Oracle text**:
```
Destroy target Spirit or enchantment.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/40/urgent-exorcism
**Oracle text**: Destroy target Spirit or enchantment.
**Type line**: Instant
**Mana cost**: {1}{W}
**Rulings**: none (Scryfall returns no rulings for this card)
**Status**: ISSUE (fixed)

### Card data
Every field matches the fetched text: name, `{1}{W}` as
`[Generic(1), Colored(White)]`, `card_types: [Instant]`, oracle text verbatim,
no P/T, no keywords, no subtypes, no flashback.

### Code issues

1. `is_valid_target` restated `target_requirement` (`urgent_exorcism.rs:29-43`, removed).
   - Oracle text says: `Destroy target Spirit or enchantment.`
   - `target_requirement` already says exactly that:
     `PermanentWithFilter(SubtypeOrCardType { subtypes: ["Spirit"], card_types: [Enchantment] })`
   - The override said:
     `state.has_card_type(obj.id, CardType::Enchantment, registry) || state.has_subtype(obj.id, "Spirit", registry)`
   - `matches_target_filter` says, for that filter (`targeting.rs:490`):
     `subtypes.iter().any(|s| state.has_subtype(obj.id, s, registry)) || card_types.iter().any(|t| state.has_card_type(obj.id, *t, registry))`
     — the same two calls, in the other order.
   - The override's other half was `Some(o) if o.zone == Zone::Battlefield`. The
     spell-cast enumerator already reads `state.all_objects_in_zone(Zone::Battlefield)`
     (`targeting.rs:273`), and `stack::is_target_legal` re-checks
     `obj.zone == Battlefield || obj.zone == Stack` for `PermanentWithFilter`.
     Nothing moves a battlefield permanent to the stack, so the `|| Stack` arm
     cannot admit anything this guard was excluding. Removed; the full suite is
     green with it gone.

   `Target::Player(_) => false` and `Target::Illegal => false` went with it: a
   `PermanentWithFilter` requirement never enumerates a player, and
   `is_target_legal` returns false for `Target::Illegal` on its own.

2. The Bug 31-003 regression test would have gone vacuous (`subtype.rs:446`, rewritten).
   - It called `behavior.is_valid_target(&state, P0, &Target::Object(token), &registry)`
     and asserted true. With the override removed that is the trait default,
     which is `fn is_valid_target(..) -> bool { true }` — the assertion would
     have held no matter what the filter did.
   - Rewritten to cast the spell and read `offered_targets`, which is where a
     target is actually offered (CR 601.2c), then to resolve it and check the
     token is gone. Confirmed live: making `SubtypeOrCardType` subtype-blind
     now fails it; before the rewrite it could not have.

3. The enchantment half of the filter was untested anywhere in the suite
   (`cards_removal_and_bounce.rs:163`, row added).
   - The removal table gave the card one row, `Candidate::Named("Chapel Geist")`,
     which is a Spirit. Slayer of the Wicked is the only other
     `SubtypeOrCardType` user and passes `card_types: []`.
   - Verified by mutation against the *pre-change* table: deleting the
     `card_types` arm from `matches_target_filter` produced zero failures across
     the whole workspace. Added a `Candidate::Enchantment` (Pacifism) row; the
     same mutation now fails `targeted_removal_offers_the_targets_its_text_allows`
     and nothing else.

### Tricky interactions checked
- Spirit **token** (Midnight Haunting, Doomed Traveler, Mausoleum Guard,
  Geist-Honored Monk): legal. `has_subtype` unions `obj.subtypes` with the
  active face, so a `card_id: CardId(0)` token is seen. PASS.
- A Spirit that is *also* an enchantment, or an enchantment that is not a
  creature (Pacifism, Curses, Bonds of Faith): legal — the filter is a
  disjunction and `has_card_type` reads the characteristics layer. PASS.
- A **creature** enchanted by an Aura is not itself an enchantment: the new
  Pacifism row destroys the Aura, and the 2/2 it enchants is not offered
  (it is neither a Spirit nor an enchantment). PASS.
- Indestructible: `resolve_destroy` routes to `destruction::try_destroy`, which
  is the "destroy" pipeline (CR 701.7b), not a sacrifice. PASS.
- Hexproof/protection: `can_be_targeted_by` at enumeration,
  `is_target_legal` at resolution — neither was ever the card's job. PASS.
- Target becomes illegal in response (Ranger's Guile on a Chapel Geist, or the
  Spirit leaving the battlefield): sole target illegal → the spell is countered
  by game rules (CR 608.2b) in `stack::resolve_spell`. PASS.
- Self-cleanup: `on_resolve` moves nothing; the engine owns the spell
  (CR 608.2m). PASS.
- A transformed DFC that gained/lost the Spirit subtype: `has_subtype` reads
  `face_data`, so the live face decides. PASS (same accessor that Bug 31-002
  fixed for Avacynian Priest).

### Test coverage
- Spirit half offered and destroyed: `cards_removal_and_bounce.rs:163`
  (`targeted_removal_offers_the_targets_its_text_allows`, Chapel Geist row).
- Enchantment half offered and destroyed: `cards_removal_and_bounce.rs:166`
  (same test, Pacifism row) — **added this audit**.
- A non-Spirit creature is not offered: both rows above.
- Spirit token is targetable and dies: `subtype.rs:446`
  (`bug_31_003_urgent_exorcism_targets_spirit_token`) — **rewritten this audit**
  to go through `legal_actions` instead of calling `is_valid_target`.
- Fizzle when the sole target goes away: covered generically for
  `PermanentWithFilter` spells in `fizzle.rs`; not card-specific, and the
  card contributes no fizzle logic of its own.
- No rulings exist for this card, so there is no per-ruling row to fill.

### Mutations run
| mutation | result |
| --- | --- |
| `matches_target_filter`: drop the `card_types` arm | fails only `targeted_removal_offers_the_targets_its_text_allows` (and, against the old table, **nothing at all**) |
| `matches_target_filter`: drop the `subtypes` arm | fails `bug_31_003_urgent_exorcism_targets_spirit_token`, the removal table, and the Slayer of the Wicked tests |
| `has_subtype`: drop the `obj.subtypes` half (the original Bug 31-003 shape) | fails 20+ tests including `bug_31_003_urgent_exorcism_targets_spirit_token` |

Suite after: 1439 passing, exit 0, zero warnings.

