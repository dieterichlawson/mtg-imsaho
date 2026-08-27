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
