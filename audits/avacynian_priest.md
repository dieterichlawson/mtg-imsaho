## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/4/avacynian-priest?utm_source=api
**Type line**: `Creature — Human Cleric` — {1}{W}, 1/2
**Oracle text**:
```
{1}, {T}: Tap target non-Human creature.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "target **non-Human** creature" — `is_valid_target` excludes Humans, checking
  `state.has_subtype` so a token or a granted Human subtype counts too: PASS
- A creature that *becomes* a Human in response is no longer a legal target
  (CR 608.2b). The ability arm of `resolve_top_of_stack` checked only whether
  the target could still be targeted at all, not whether it still satisfied the
  card. Fixed by also consulting the granting behavior's `is_valid_target`.
- Tapping is the effect, not the cost — the Priest's own {T} is the cost: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Tapping a non-Human, and the ability not being payable twice: `activated_abilities.rs:avacynian_priest_taps_a_non_human_and_then_cannot_be_paid_again`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/4/avacynian-priest?utm_source=api
**Type line**: `Creature — Human Cleric` — {1}{W}, 1/2
**Oracle text**:
```
{1}, {T}: Tap target non-Human creature.
```

**Rulings fetched**: none published for this card.

**Status**: PASS


No rulings are cached for this card and none surfaced.

### Code issues
No issues found.

- Card data matches exactly: {1}{W}, Creature — Human Cleric (both subtypes),
  1/2, oracle text verbatim, `{1}` plus `requires_tap`.
- "target **non-Human** creature" lives in `is_valid_target`, and reads
  `has_subtype`, which is the active face — so a transformed werewolf is a
  legal target and a Human is not.
- The zone check is in `is_valid_target` too, which is the hook the engine's
  ability arm calls. This card is one of the ones already doing it the right
  way round; Heretic's Punishment and Kessig Wolf Run were not, and were moved
  to match during their own audits.

### What was untested
`stack.rs` names this card as the motivating example for one half of the
CR 608.2b re-check:

> "Two ways a target stops being legal: it can stop being targetable at all
> (hexproof, protection), and it can stop satisfying what the ability asks of
> it — Avacynian Priest's 'target non-Human creature' is not a legal target
> once it has become a Human."

Nothing tested it. The engine comment asserted the behaviour and no test held
it in place, so the card's-own-restriction half of the re-check could have been
removed without anything failing.

A werewolf transforming back to its front face is how a creature becomes a
Human in this set — Villagers of Estwald is a Human Werewolf, and its back
face, Howlpack of Estwald, is not a Human. The new test targets the back face,
transforms it back in response, and checks the ability is countered. It also
checks the Priest stays tapped: costs are not refunded when an ability is
countered.

Mutation confirms it is precisely aimed — removing the card's own restriction
from the ability arm's re-check fails this test and *only* this test, which is
exactly the half the comment describes.

### Not changed, and why
`resolve_activated_ability` taps by writing `obj.tapped = true` rather than
through a helper, as five other ISD cards do. `GameEvent::Tapped` exists and is
emitted when a permanent taps for mana or as an attacker, but nothing in the
engine or the card pool watches it — no trigger kind consumes it. So unlike the
`CreatureCardMilled` case, where a live watcher (Undead Alchemist) made the
inconsistency real, there is nothing here to miss. Recorded rather than turned
into a tap pipeline for a dead event.

### Tricky interactions checked
- A Human is not offered as a target; a non-Human is: pass
- The Priest is a Human, so it cannot target itself: pass
  (`ability_target_protection.rs:49`)
- A transformed werewolf's live face decides, not its front face: pass
  (`subtype.rs:493`)
- The target becomes a Human in response — ability countered (CR 608.2b): pass
- The Priest taps for its own cost and cannot be used twice: pass
- Summoning sickness: it cannot pay {T} the turn it arrives without haste
  (CR 302.6): pass (`tap_cost_legality.rs`)
- The prompt does not offer illegal targets: pass (`hexproof_filter.rs:425`)

### Test coverage
- Main effect, legal/illegal targets, tapping, reuse:
  `activated_abilities.rs:167`
- Cannot target itself: `ability_target_protection.rs:49`
- Live face decides Human-ness: `subtype.rs:493`
- Summoning sickness and {T} legality: `tap_cost_legality.rs`
- Prompt filtering: `hexproof_filter.rs:425`
- **NEW** a target that becomes a Human is illegal on resolution:
  `activated_abilities.rs:196`

