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
