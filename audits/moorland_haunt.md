## Audit — 2026-08-27 (Tier B)

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/244/moorland-haunt?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
{T}: Add {C}.
{W}{U}, {T}, Exile a creature card from your graveyard: Create a 1/1 white Spirit creature token with flying.
```

**Status**: ISSUE

### Code issues
See below.


- "{1}{W}, {T}, **Exile a creature card from your graveyard**:" — everything
  before the colon is a cost, and the whole ability (cost *and* token) was in
  `on_activate_ability`, which is the hook whose trait default was the CR 602.2a
  stack push. So the token appeared the instant the ability was activated.
  Fixed in the set-wide CR 602.2a conversion: the exile is now the one thing
  `pay_activation_cost` is for, and the token is
  `resolve_activated_ability`'s. See
  `reports/ISD_AUDIT_CR6022a_ACTIVATED_ABILITIES.md`.

### Tricky interactions checked
- CR 601.2h: the cost is paid on activation, so the creature card is already in
  exile while the ability is on the stack and countering it does not give the
  card back: PASS
- CR 109.1: "a creature **card** from your graveyard", so a token there is not
  one: PASS
- Which card to exile is the player's choice when there is more than one, and
  the choice completes the cost before the ability resolves — CR 602.2b puts
  activation through the casting steps, where the ability is on the stack
  (CR 602.2a) before costs are paid: PASS
- The Spirit token carries its colour, subtype and flying: PASS

### What else was checked
Card data verified exact set-wide — cost, card types, supertypes, subtypes, P/T,
oracle text, keywords on both faces, flashback cost, and trigger kinds against
the oracle phrasing (see `ISD_AUDIT_PROGRESS.md`). Step 9 anti-patterns: clean.

### Test coverage
- Cost at activation, token at resolution: `activated_no_stack.rs`, `cards_lands_and_mana_sources.rs:moorland_haunt_creates_spirit_token`
## Full audit — 2026-08-27

**Oracle text source**: Oracle cache (Scryfall API) — https://scryfall.com/card/isd/244/moorland-haunt?utm_source=api
**Type line**: `Land` — no mana cost
**Oracle text**:
```
{T}: Add {C}.
{W}{U}, {T}, Exile a creature card from your graveyard: Create a 1/1 white Spirit creature token with flying.
```

**Rulings fetched**: none published for this card.

**Status**: ISSUE (fixed, set-wide)

### Code issues

Moorland Haunt itself is correct — including the one thing the rest of the set
got wrong.

**The set disagreed with itself about what a token is called.**

- CR 111.4: "A spell or ability that creates a token sets both its name and its
  subtype(s). If the spell or ability doesn't specify the name of the token, its
  name is the same as its subtype(s) plus the word 'Token.'"
- Moorland Haunt did: `create_token_with_subtypes("Spirit Token", ...)` — right.
- Four other cards did: `create_token_with_subtypes("Spirit", ...)` — wrong.

Five cards in this set make a 1/1 white flying Spirit — Doomed Traveler,
Mausoleum Guard, Geist-Honored Monk, Midnight Haunting and Moorland Haunt — and
they did not agree what to call it. Twenty-six token creation sites in total,
with the name hardcoded at each one alongside the subtypes that determine it.

That is reachable, not cosmetic, because two cards in the set match creatures
**by name**: Sever the Bloodline ("Exile target creature and all other creatures
with the same name as that creature") and Evil Twin's granted ability ("Destroy
target creature with the same name as this creature"). A Sever aimed at a
Doomed Traveler's Spirit would have missed a Moorland Haunt's, and vice versa.

Fixed at the source: `create_token_with_subtypes` derives the name from the
subtypes per CR 111.4 when the effect does not give one, and every one of the
twenty-six call sites now passes `""`. The name is no longer written down twice
per token, so the two copies cannot drift again.

The rule's other half still works: a token the effect *does* name keeps that
name. No card in this set names one, but the branch exists and is tested.

I checked CR 111.4 against a source rather than trusting the shape of the
existing code — the Evil Twin ruling I had fetched earlier states the same rule,
which is what made the mismatch visible in the first place.

### Rulings checked

Moorland Haunt has no published rulings.

### Tricky interactions checked

- **"Exile a creature card from your graveyard" is a cost, not an effect.**
  Paid in `pay_activation_cost` (CR 601.2h via 602.2b), so it happens on
  activation and cannot be responded to, and the ability is not offered at all
  without a creature card to exile. PASS.
- **"a creature **card**"** — `is_card` excludes a token sitting in the
  graveyard (CR 109.1), both when deciding whether the ability is available and
  when paying. PASS.
- **Choosing which card to exile** is presented when there is more than one, and
  taken automatically when there is exactly one — a cost with a single legal
  payment has no choice in it. PASS.
- **The `{T}` is shared between both abilities**, so tapping for mana and making
  a Spirit are mutually exclusive; `activated_abilities` returns nothing while
  tapped. PASS.
- **The token is a 1/1 white Spirit with flying**, created with its subtype so
  it counts for anything that cares about Spirits. PASS.
- **No mana cost** — it is a land, and `cost` is absent rather than zero. PASS.

### Test coverage

- token naming follows CR 111.4, including the named-token branch: `card_data_invariants.rs::tokens_are_named_after_their_subtypes` (new).
- no card hardcodes a derivable token name: `card_data_invariants.rs::no_card_hardcodes_a_derivable_token_name` (new, mutation-checked by putting `"Spirit"` back into Doomed Traveler).
- the ability exiles from the graveyard and makes a Spirit: `cards_lands_and_mana_sources.rs:172`.
- the exile choice when several creature cards are available: `auto_pick.rs:117`.

