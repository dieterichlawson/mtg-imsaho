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

### ~~Splinterfright~~ — NOT A BUG (Some(0) is engine convention for */* creatures; needed for power.is_some() creature detection)

### ~~Unbreathing Horde~~ — FIXED (counts graveyard before moving to battlefield in on_resolve)
