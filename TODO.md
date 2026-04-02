# TODO — ISD Audit Issues

## Real Bugs (6)

### ~~Charmbreaker Devils~~ — FALSE POSITIVE (triggers.rs already gates SpellCast on instant/sorcery)

### ~~Back from the Brink~~ — FIXED (exile now happens before token creation)

### ~~Bitterheart Witch~~ — FIXED (player now chooses which Curse via ChooseTarget chain)

### ~~Mirror-Mad Phantasm~~ — FIXED (now shuffles library after inserting card)

### ~~Snapcaster Mage~~ — FIXED (player choice via ChooseTarget + GrantFlashback effect)

### ~~Into the Maw of Hell~~ — FIXED (added damaged_by.push)

## Behavioral Edge Cases (3)

### ~~Festerhide Boar~~ — FIXED (moved to on_resolve, counters added as part of entering)

### Splinterfright
- `*/*` P/T uses `Some(0)` base instead of `None`. Card has `*/*` P/T defined by a characteristic-defining ability. Base should be `None` to indicate the CDA defines P/T entirely, not `Some(0)`.

### Unbreathing Horde
- Doesn't count itself when entering from graveyard. Per ruling: "If Unbreathing Horde enters from a graveyard, it will count itself when determining how many +1/+1 counters it enters with." The ETB handler counts graveyard Zombies after the card has already moved to battlefield, so it misses itself.
