# TODO — ISD Audit Issues

## Real Bugs (6)

### ~~Charmbreaker Devils~~ — FALSE POSITIVE (triggers.rs already gates SpellCast on instant/sorcery)

### ~~Back from the Brink~~ — FIXED (exile now happens before token creation)

### ~~Bitterheart Witch~~ — FIXED (player now chooses which Curse via ChooseTarget chain)

### Mirror-Mad Phantasm
- Doesn't shuffle before reveal. The implementation appends the card to the bottom of the library (`push`) instead of shuffling it into a random position. Oracle says "shuffles it into their library" — the card should be at a random position before the reveal loop begins.

### Snapcaster Mage
- Auto-selects highest-MV instant/sorcery in graveyard instead of presenting a target choice to the player. Oracle says "target instant or sorcery card in your graveyard" which requires player selection.

### Into the Maw of Hell
- Missing `damaged_by` tracking when dealing 13 damage to the target creature. Other non-combat damage sources (e.g., Heretic's Punishment) correctly push to `damaged_by`. This means death triggers that check `damaged_by` will not correctly identify Into the Maw of Hell as the damage source.

## Behavioral Edge Cases (3)

### Festerhide Boar
- "Enters with" replacement effect modeled as ETB trigger. Current oracle says "This creature enters with two +1/+1 counters on it if a creature died this turn" (replacement effect), but code implements it as `TriggerKind::EntersBattlefield` / `on_enter_battlefield` (triggered ability). Counters should be on the creature as it enters, not added after.

### Splinterfright
- `*/*` P/T uses `Some(0)` base instead of `None`. Card has `*/*` P/T defined by a characteristic-defining ability. Base should be `None` to indicate the CDA defines P/T entirely, not `Some(0)`.

### Unbreathing Horde
- Doesn't count itself when entering from graveyard. Per ruling: "If Unbreathing Horde enters from a graveyard, it will count itself when determining how many +1/+1 counters it enters with." The ETB handler counts graveyard Zombies after the card has already moved to battlefield, so it misses itself.
