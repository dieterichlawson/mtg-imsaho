## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/68/mirror-mad-phantasm?utm_source=api
**Type line**: `Creature — Spirit` — {3}{U}{U}, 5/1
**Oracle text**:
```
Flying
{1}{U}: This creature's owner shuffles it into their library. If that player does, they reveal cards from the top of that library until a card named Mirror-Mad Phantasm is revealed. The player puts that card onto the battlefield and all other cards revealed this way into their graveyard.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**This creature's owner** shuffles it into their library" — the owner, not
  the controller, so a stolen Phantasm goes to its owner's library: PASS
- "**If that player does**, they reveal cards ... until a card named Mirror-Mad
  Phantasm is revealed" — the reveal is conditional on the shuffle happening:
  PASS
- The revealed copy goes to the battlefield and *all other revealed cards* to
  the graveyard — usually the rest of the library, which is the card's whole
  point: PASS
- A library with no other copy mills itself entirely: PASS
- The ability resolves from the stack, so removing the Phantasm in response
  leaves nothing to shuffle: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The shuffle-and-dig, and being removed in response: `activated_no_stack.rs:mirror_mad_phantasm_shuffles_and_digs_itself_back_out_on_resolution`, `:mirror_mad_phantasm_source_removed_before_resolution`
