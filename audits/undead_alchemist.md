## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/84/undead-alchemist?utm_source=api
**Type line**: `Creature — Zombie` — {3}{U}, 4/2
**Oracle text**:
```
If a Zombie you control would deal combat damage to a player, instead that player mills that many cards.
Whenever a creature card is put into an opponent's graveyard from their library, exile that card and create a 2/2 black Zombie creature token.
```

**Status**: PASS

### Code issues
No issues found.


### Tricky interactions checked
- "**If** a Zombie you control **would** deal combat damage to a player,
  **instead** that player mills that many cards" — a replacement effect, so no
  damage is dealt and damage triggers do not fire: PASS
- "a Zombie **you control**", including Zombie tokens: PASS
- "Whenever a creature card is put into an **opponent's** graveyard from their
  library" — the opponent filter is the collector's, which is why every mill in
  the set now emits `CreatureCardMilled` and lets the collector decide: PASS
- "exile that card **and** create a 2/2 black Zombie" — both, once per card: PASS
- CR 109.1: a creature *card*, so its own Zombie tokens dying do not feed it: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- The damage replacement and the mill trigger: `token_is_not_a_card.rs:mindshrieker_milled_creature_triggers_undead_alchemist`, `multi_target_and_mill.rs`
## Full audit — 2026-08-28

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/84/undead-alchemist?utm_source=api
**Type line**: `Creature — Zombie` — {3}{U}, 4/2
**Oracle text**:
```
If a Zombie you control would deal combat damage to a player, instead that player mills that many cards.
Whenever a creature card is put into an opponent's graveyard from their library, exile that card and create a 2/2 black Zombie creature token.
```

**Rulings fetched**:
- [2011-09-22] If you control multiple Undead Alchemists, the multiple replacement abilities will have no added effect. Combat damage dealt to a player by a Zombie you control will be replaced only once with cards being put into that player's graveyard.
- [2011-09-22] Whenever a creature card is put into an opponent's graveyard from their library, the triggered ability of each Undead Alchemist you control will trigger. The first such ability to resolve will exile that creature card and create a Zombie token. Subsequent abilities won't exile the creature card, but each will create another Zombie token.

**Status**: ISSUE


Two rulings, both about controlling more than one Alchemist:
1. "If you control multiple Undead Alchemists, the multiple replacement
   abilities will have no added effect. Combat damage dealt to a player by a
   Zombie you control will be replaced only once..."
2. "...the triggered ability of each Undead Alchemist you control will trigger.
   The first such ability to resolve will exile that creature card and create a
   Zombie token. Subsequent abilities won't exile the creature card, but each
   will create another Zombie token."

### Code issues

**1. "exile that card" reached into zones the card had moved to.**

- Oracle text says: `exile that card`
- Code did: `state.move_object(milled_object, Zone::Exile, registry);` —
  unconditionally, wherever the card happened to be.

CR 400.7: a card that changes zones becomes a new object, and an ability that
named the old one can no longer find it. So if the milled card's owner rescued
it in response — Ghoulcaller's Chant, in this very set, returns Zombie creature
cards from a graveyard to hand — the Alchemist's trigger would reach into their
*hand* and exile it from there.

Now it exiles only a card still in a graveyard. The token is created either
way, because the token is not conditional on the exile.

**2. The same line made ruling 2's log false.** With two Alchemists the second
trigger found the card already in exile and still announced "exiled milled X".
Same shape as the destruction-log finding earlier in this audit: a line that
claims an action the code did not verify. It now says which of the two things
happened. (The card's *behaviour* under ruling 2 was already correct — both
Alchemists made their token — so this half is a logging fix, not a rules fix.)

**3. The doc comment described an implementation that does not exist.** It said:

> Ability 1 is a replacement effect: combat damage from Zombies is replaced
> with milling. Implemented via `replace_combat_damage_to_player`.
>
> Ability 2 (mill-watcher trigger for non-combat mill sources) is not yet
> implemented as a standalone trigger — currently the exile-and-token logic is
> inlined in the replacement effect for the combat mill path only.

Three claims, all false. `replace_combat_damage_to_player` no longer exists —
`replacement.rs`'s own module note records it as one of the per-event hooks
folded into `replace_event`. Ability 2 *is* a standalone trigger: the card
declares a `CreatureCardMilled` `TriggeredAbilityDef` and implements
`on_creature_card_milled`, twenty lines below the comment saying it does not.
And the logic is not inlined in the replacement effect, which only calls
`mill_cards`.

This is the most misleading kind of stale comment: it tells a reader a feature
is missing when it is implemented, inviting them to build it a second time.
Rewritten to describe what is there, including why `_milled_player` is unused
— the opponent scoping is the collector's (`watcher_controller == milled_player
{ continue }`), which is where a general rule belongs.

### Not a bug, checked
- Ruling 1 holds: `replacement::apply` returns on the first
  `Replacement::Replaced`, so a second Alchemist never sees the event.
- The replacement correctly requires the Alchemist on the battlefield
  (CR 113.6), checks `combat: true` and a player target, and checks the damage
  source is a Zombie its controller controls — including the Alchemist itself.
- Scryfall lists `Keywords: Mill`; that is the keyword *action*, not a keyword
  ability, and the codebase has no `Keyword::Mill`. Not flagged.

### Tricky interactions checked
- Multiple Alchemists mill once, not once each (ruling 1): pass
- Multiple Alchemists each make a token, only the first exiles (ruling 2): pass
- A milled card rescued from the graveyard in response is not exiled from its
  new zone (CR 400.7): pass, after the fix
- The trigger fires only for cards put into an *opponent's* graveyard: pass
- Non-combat mill sources feed the trigger too: pass
- A token milled into a graveyard is not "a creature card": pass
- The trigger resolves after the Alchemist dies (CR 113.7a): pass

### Test coverage
- Mill-instead-of-damage and the exile + token: `token_is_not_a_card.rs:187`
- Ruling 1, replaced once: `replacement_effects.rs:130`
- Only an opponent's graveyard: `multi_target_and_mill.rs:142`, `:197`
- Non-combat mill sources reach it: `multi_target_and_mill.rs:119`
- Fires per milled card in a multi-card mill: `trigger_dispatch.rs:457`
- Resolves after the source dies: `trigger_source_independence.rs:464`
- **NEW** a card that left the graveyard is not exiled from its new zone:
  `token_is_not_a_card.rs:216`
- **NEW** ruling 2, two Alchemists, two tokens, one exile:
  `token_is_not_a_card.rs:245`

Mutation-checked: restoring the unconditional `move_object` fails the first
new test — it is the original bug, reproduced exactly — and making the token
conditional on the exile fails both.

