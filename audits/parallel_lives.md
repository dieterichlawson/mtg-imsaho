## Audit — 2026-08-27 (Tier C — one behaviour hook: replacement effect)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/199/parallel-lives?utm_source=api
**Type line**: `Enchantment` — {3}{G}
**Oracle text**:
```
If an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.
```
**Status**: PASS

### Code issues
No issues found.

### What was checked
Card data was verified exact set-wide (see `ISD_AUDIT_PROGRESS.md`). This card's
one hook is `replace_event`, so the audit centres on CR 614 — whether the effect
applies to the right events, exactly once, and modifies rather than replaces
where the oracle says "instead".

- "under **your** control" — gated on the token controller matching this
  permanent's controller, so an opponent's token-maker is unaffected even when
  it would put tokens under their own control.
- Doubles the count rather than creating a second batch, which is what "creates
  twice that many of those tokens instead" means: one event, modified.

### Test coverage
`token_copy.rs` / `cards_death_triggers_and_tokens.rs` exercise token creation; the doubling itself is NOT TESTED
## Audit — 2026-08-27 (Tier A)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/199/parallel-lives?utm_source=api
**Type line**: `Enchantment` — {3}{G}
**Oracle text**:
```
If an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.
```

**Status**: PASS

### Code issues
No issues found.

Ruling 1: "If you control two Parallel Lives, then the number of tokens
created is four times the original number." `replace_event` returns
`count * 2`, and `replacement::apply` walks each candidate object in turn
feeding the previous result forward — so two copies compose to 4x, three to 8x,
which falls out of the loop rather than being special-cased. CR 614.5 (an
effect applies at most once per event) holds because each candidate object is
asked exactly once.

Ruling 2: "Everything that is specified by the effect creating the original
token ... will also be true about the additional token" — e.g. tapped and
attacking. `create_token_with_subtypes` returns *all* the ids, and every caller
that specifies more than the base token loops over the whole vector: Geist of
Saint Traft (tapped, attacking, end-of-combat exile), Kessig Cagebreakers
(tapped and attacking), Army of the Damned (tapped), Gutter Grime (P/T linked
to slime counters). Verified across all twenty ISD token creators.

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
`replacement_effects.rs` — doubling, and the doubled token inheriting tapped-and-attacking.

## Audit — 2026-08-28 19:12

**Oracle text source**: Oracle cache (Scryfall API) — `scripts/oracle_lookup.py lookup "Parallel Lives"`, https://scryfall.com/card/isd/199/parallel-lives
**Oracle text**:
```
If an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.
```
**Type line**: Enchantment
**Mana cost**: {3}{G}
**Rulings** (2, 2023-09-01): two copies quadruple; and everything the original effect specifies
("tapped and attacking") is true of the doubled tokens too.
**Status**: PASS

### Code issues
No issues found in `mtg-engine/src/cards/isd/parallel_lives.rs`.

`{3}{G}`, `CardType::Enchantment`, oracle text verbatim. The whole card is one `replace_event`
arm: a `CreatesTokens` event whose controller matches the Lives' controller has its count
doubled, returned as `Modified` — which is what makes ruling 1 true for free, since a modifying
replacement leaves the event live for the next replacement to see (contrast `Replaced`, which
`replacement_effects.rs` covers with the one-of-each pair).

Ruling 2 falls out of where the doubling sits: `create_token_with_subtypes` runs the event
through the replacement layer and then builds *all* the resulting tokens from the same
definition and returns all their ids — so a caller that then taps them (Army of the Damned)
taps the doubled ones too.

### Tricky interactions checked
- **Your tokens doubled, an opponent's not, with a no-Lives baseline**: PASS.
- **Two copies quadruple** (ruling 1): PASS.
- **The doubled tokens share the originals' specifics** (ruling 2): PASS — Army of the Damned's
  26 tapped Zombies, and a token copy's doubled twin keeping the copied identity.
- **Tokens from any source** — an ability (Gutter Grime), a spell, a copy effect: the
  replacement sits in the one token-construction helper, so every source goes through it.
- **A modeling note, recorded**: the engine emits `CreatesTokens { count: 1 }` once per token —
  "create two tokens" is two events of one, not one event of two. For doubling the arithmetic is
  identical (2×1 twice = 2×2), and nothing in this pool can see the difference; it is why the
  `count+1` mutation below is only visible to the compound test.

### Test coverage
- doubles yours, not theirs, with baseline:
  `cards_rule_modifiers.rs:70 parallel_lives_doubles_only_its_controllers_tokens`
- two copies compound: `replacement_effects.rs:186 modifying_replacements_compound`
- doubled tokens are tapped too: `token_copy.rs:78 a_caller_that_mutates_the_returned_tokens_reaches_the_doubled_ones`
- a doubled token copy keeps the copied identity: `token_copy.rs:47`

Mutation-checked: dropping the controller check fails the scope test and only it; `count+1`
instead of `count*2` fails the compound test and only it (see the modeling note for why the
others cannot see that one).

### Changes made
None — code and coverage were both right.
