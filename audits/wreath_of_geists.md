## Audit — 2026-08-27 — CR 109.1: a token in a graveyard is not a card

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/211/wreath-of-geists?utm_source=api
**Oracle text**:
```
Enchant creature
Enchanted creature gets +X/+X, where X is the number of creature cards in your graveyard.
```
**Status**: ISSUE (fixed)

### Code issue
- Oracle text says: a **card** in a graveyard (`Enchanted creature gets +X/+X, where X is the number of creature cards in your graveyard.`)
- Code did: filtered the graveyard by creature-ness alone, with no card/token distinction.
- CR 109.1: a token is not a card. CR 111.7 removes a token from a graveyard as
  a state-based action, so between the moment it dies and the next SBA check it
  really is sitting there — the same window a dies-trigger sees. Measured
  directly on Boneyard Wurm: 2/2 with one creature card and one just-died token
  in the yard, 1/1 the instant SBAs ran. The oracle's answer is 1/1 throughout.
- Fixed: the graveyard filter now goes through `state.is_card`.

### How this was found
A sweep for cards whose oracle says "cards in a graveyard" against code that
never distinguishes tokens. Thirteen cards matched; five already guarded
(Gnaw to the Bone, Moorland Haunt, Past in Flames, Runechanter's Pike,
Splinterfright) and eight did not.

Splinterfright and Boneyard Wurm are the instructive pair — near-identical
text, adjacent in the set. `token_is_not_a_card.rs::cda_does_not_count_tokens_in_graveyard`
covered Splinterfright, which is why Splinterfright alone had the guard.

### Test coverage
`token_is_not_a_card.rs::a_token_in_a_graveyard_is_not_a_creature_card` —
**added by this audit**, covers Boneyard Wurm and Splinterfright together and
fails against the unfixed code.
## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/211/wreath-of-geists?utm_source=api
**Type line**: `Enchantment — Aura` — {G}
**Oracle text**:
```
Enchant creature
Enchanted creature gets +X/+X, where X is the number of creature cards in your graveyard.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- Ruling: "The value of X is **constantly updated** as creature cards are put
  into or removed from your graveyard." `dynamic_pt`, recomputed every time P/T
  is asked for: PASS
- "**your** graveyard" is the *Aura's controller's*, not the enchanted
  creature's — `dynamic_pt` is called with the Aura's own object id, so it reads
  the right player even on a stolen creature: PASS
- CR 109.1: "creature **cards**", so tokens are excluded: PASS
- The bonus ends when the Aura leaves: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The CDA-style bonus: `enchantments.rs`, `token_is_not_a_card.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/211/wreath-of-geists?utm_source=api
**Type line**: `Enchantment — Aura` — {G}
**Oracle text**:
```
Enchant creature
Enchanted creature gets +X/+X, where X is the number of creature cards in your graveyard.
```

**Rulings fetched**:
- [2011-09-22] The value of X is constantly updated as creature cards are put into or removed from your graveyard.

**Status**: ISSUE (fixed)

**Oracle text source**: Oracle cache (Scryfall API), https://scryfall.com/card/isd/211/wreath-of-geists
**Oracle text**:
```
Enchant creature
Enchanted creature gets +X/+X, where X is the number of creature cards in your graveyard.
```
**Type line**: Enchantment — Aura
**Mana cost**: {G} — **Keywords**: Enchant
**Rulings** (1, 2011-09-22): "The value of X is constantly updated as creature cards are put into or removed from your graveyard."

**Status**: ISSUE (fixed) — the card code is correct; its test was not.

### Card data
Matches the fetched text: `{G}`, `card_types: [Enchantment]`,
`subtypes: ["Aura"]`, oracle text verbatim, no P/T. "Enchant creature" is
`TargetRequirement::Creature` plus `helpers::resolve_aura`, the standard Aura
pattern, so `keywords` carrying nothing for "Enchant" is right — it is the
targeting rule, not a keyword ability with its own effect.

### Mechanism
The +X/+X reaches the creature through `dynamic_pt` on the **Aura**, which
`state::continuous_pt_mods` collects by walking every battlefield object whose
`attached_to` is the creature (`state.rs:1219-1229`). The creature's own
`dynamic_pt` is consulted separately and only when it has a base P/T
(`state.rs:1389`), so an Aura's `dynamic_pt` never gets mistaken for the
creature's own. Correct, and it satisfies the ruling for free: the count is
recomputed on every read rather than snapshotted when the Aura resolved.

### Code issues

No issue in `wreath_of_geists.rs`. The problems were in what held it up.

1. The one test could not tell the card's clauses apart
   (`cards_evasion_and_graveyard_pt.rs:158`, rebuilt).
   - Oracle text says: `X is the number of creature cards in your graveyard`
   - The test stocked the graveyard with
     `state.create_object(CardId(9999), P0, Zone::Graveyard, Some(1), Some(1))`
     — an anonymous object with a P/T, which CR 205.1b (and
     `card_types_of`, `state.rs:2039`) makes a creature. Every object it put in
     the graveyard was a creature card, so the test showed the count going up
     and nothing else.
   - Verified: replacing the card's filter
     `state.is_card(o.id) && state.is_creature(o.id, registry)` with
     `state.is_card(o.id)` — count **every** card in the graveyard — produced
     zero failures across the whole workspace.
   - Rebuilt on real cards, a clause at a time: a Walking Corpse raises X, a
     Forest does not, a Zombie token in the graveyard does not (CR 109.1,
     CR 704.5e — it sits there until the next SBA pass), and moving the Walking
     Corpse out lowers X again, which is the ruling's "or removed from your
     graveyard".

2. "in **your** graveyard" was untested (same file, test added).
   - Oracle text says: `creature cards in your graveyard`
   - The card reads `state.get_object(object_id)?.controller` — the **Aura's**
     controller. That is right, and it differs from the enchanted creature's
     controller the moment the Aura goes on an opponent's creature.
   - Verified: making it read the enchanted creature's controller instead
     produced zero failures across the whole workspace.
   - Added `wreath_of_geists_counts_its_own_controllers_graveyard_not_the_creatures`:
     the opponent's creature is enchanted while the opponent's graveyard holds
     three creature cards and the Aura controller's holds one. X is 1.

### Tricky interactions checked
- Count is live, not snapshotted (the ruling): PASS — the rebuilt test both
  adds to and removes from the graveyard after the Aura has resolved.
- A creature **token** in the graveyard does not count: PASS — behaviourally in
  the rebuilt test, and structurally by the
  `a_card_enumerating_a_graveyard_excludes_tokens` source guard, which was the
  only thing covering it before.
- A non-creature card does not count: PASS — new step.
- "your graveyard" is the Aura controller's: PASS — new test.
- Aura on an opponent's creature at all: legal — "Enchant creature" has no "you
  control". `TargetRequirement::Creature` offers either side. PASS.
- The enchanted creature dies / the Aura is destroyed: `continuous_pt_mods`
  only walks sources with `zone == Battlefield` and `attached_to == creature`,
  so both cases stop contributing without the card doing anything. The
  unattached-Aura SBA (CR 704.5m) is `sba.rs`'s. PASS.
- A DFC creature card in the graveyard: counted, and `face_data` gives the
  front face there (CR 712.8a). Every ISD DFC is a creature on both faces, so
  the answer is the same either way; noted rather than tested.
- Self-cleanup: `on_resolve` delegates to `helpers::resolve_aura`; the card
  moves nothing itself. PASS.

### UI presentation
No choices beyond the Aura's target. The P/T shows through
`effective_power`/`effective_toughness` like any other modifier, so logs and
the board display the live value.

### Test coverage
- X counts creature cards, and only those, in the controller's graveyard:
  `cards_evasion_and_graveyard_pt.rs`
  (`wreath_of_geists_counts_the_creature_cards_in_its_controllers_graveyard`) —
  **rebuilt this audit**.
- The ruling (X updated as cards enter and leave): same test — **the "leave"
  half added this audit**.
- Land card excluded, token excluded: same test — **added this audit**.
- "your graveyard" vs the creature's controller's:
  `wreath_of_geists_counts_its_own_controllers_graveyard_not_the_creatures` —
  **added this audit**.
- Token exclusion, structurally: `test_suite_guards.rs:936`.

### Mutations run
| mutation | result |
| --- | --- |
| count every card in the graveyard, not just creature cards | fails the rebuilt test (before: **nothing at all**) |
| read the enchanted creature's controller's graveyard | fails the new "your graveyard" test (before: **nothing at all**) |
| drop `is_card`, so tokens count | fails the rebuilt test **and** the source guard (before: the guard alone) |

Suite after: 1446 passing, exit 0, zero warnings.

