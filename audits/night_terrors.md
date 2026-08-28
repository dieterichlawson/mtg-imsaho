## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/111/night-terrors?utm_source=api
**Type line**: `Sorcery` — {2}{B}
**Oracle text**:
```
Target player reveals their hand. You choose a nonland card from it. Exile that card.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**You** choose a nonland card from it" — the chooser is the spell's
  controller, not the targeted player: PASS
- "a **nonland** card" — lands in hand are not offered, read from the card's
  active face rather than the object's empty `card_types`: PASS
- Exile, not discard, so it does not trigger discard watchers (Murder of Crows
  is in this set): PASS
- A hand with no nonland card resolves with no effect: PASS
- Ruling: "If you target yourself with this spell, you must reveal your entire
  hand" — targeting yourself is legal: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The choice and the exile: `cards_discard_and_hand.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/111/night-terrors?utm_source=api
**Type line**: `Sorcery` — {2}{B}
**Oracle text**:
```
Target player reveals their hand. You choose a nonland card from it. Exile that card.
```

**Rulings fetched**:
- [2014-02-01] If you target yourself with this spell, you must reveal your entire hand to the other players just as any other player would.

**Status**: ISSUE


One ruling: "If you target yourself with this spell, you must reveal your
entire hand to the other players just as any other player would."

### Code issues
No behavioural bug. Card data matches exactly — {2}{B}, Sorcery, oracle text
verbatim, `PlayerOnly` for "target player".

The semantics that are easy to get wrong are all right here:
- **"You choose"** means the spell's controller, not the revealing player.
  `present_target_choice` is handed `controller`.
- **"a nonland card"** filters lands out of the options entirely, so they are
  never offered and an all-land hand loses nothing.
- **Exile**, not discard — `move_object(.., Zone::Exile, ..)`, so nothing that
  watches the graveyard sees it.
- One nonland card is auto-selected rather than prompted, which is the
  codebase's consistent reading of "a choice with one option is not a choice".

**Two comments corrected.** The doc comment on `resolve_card_effect` read:

> "Moving the chosen card to exile and finishing this spell's own resolution is
> Night Terrors' business."

The function does not finish the spell's resolution, and must not: the engine
owns that (`engine::finish_spell_resolution_if_idle`, called from
`engine.rs:410` once the choice chain empties, CR 608.2m). The audit
procedure names card self-cleanup as an anti-pattern in as many words, so a
comment claiming it as the card's business is the kind that invites someone to
add it. Rewritten to say where the cleanup actually lives.

The second, `return; // Don't move spell yet — awaiting choice.`, implied the
card would otherwise move the spell. Reworded.

### Not changed, and why
"Target player reveals their hand" is a no-op in this engine, which has no
hidden-information model to reveal into, and Night Terrors logs nothing for it.
Mulch does log its reveal, so there is a small inconsistency, but inventing a
reveal mechanism for one card is not this audit's business and no rule turns on
it here. Recorded.

### Tricky interactions checked
- The controller chooses, not the revealing player: pass
  (`spell_cleanup.rs:136`, asserted on the prompt's `player`)
- Only nonland cards are offered; the land is not: pass
  (`spell_cleanup.rs:131`, compared by name rather than taking `options.first()`)
- An all-land hand loses nothing: pass
- The chosen card is exiled, not discarded: pass
- Non-chosen cards stay in hand: pass
- The spell stays on the stack while its choice is pending, and reaches the
  graveyard after (CR 608.2m): pass
- One nonland and zero nonland both resolve in a single pass: pass
- The ruling — you can target yourself: pass
- A player with hexproof cannot be targeted by an opponent: covered by the
  shared `can_target_player` rule and its tests

### Test coverage
- Takes the nonland, leaves the land: `cards_graveyard_interaction.rs:272`
- A hand of lands loses nothing: `cards_graveyard_interaction.rs:287`
- Controller chooses; only nonlands offered; spell cleanup timing:
  `spell_cleanup.rs:112`
- One/zero nonland resolve in one pass: `spell_cleanup.rs:155`
- **NEW** the ruling — targeting yourself: `cards_graveyard_interaction.rs:302`

Mutation-checked: making the controller an illegal target for their own spell,
and letting lands through the filter, each fail the new test.

