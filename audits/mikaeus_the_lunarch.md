# Audit: Mikaeus, the Lunarch

## Official Oracle
- **Name:** Mikaeus, the Lunarch
- **Cost:** {X}{W}
- **Type:** Legendary Creature — Human Cleric
- **Oracle:** Mikaeus, the Lunarch enters the battlefield with X +1/+1 counters on it. {T}: Put a +1/+1 counter on Mikaeus. {T}, Remove a +1/+1 counter from Mikaeus: Put a +1/+1 counter on each other creature you control.
- **P/T:** 0/0

## Implementation: `mtg-engine/src/cards/mikaeus_the_lunarch.rs`
- **Name:** Mikaeus, the Lunarch -- CORRECT
- **Cost:** {X}{W} -- CORRECT
- **Type:** Creature, Legendary -- CORRECT
- **Subtypes:** Human, Cleric -- CORRECT
- **P/T:** 0/0 -- CORRECT
- **on_resolve:** Enters with X +1/+1 counters -- CORRECT
- **Ability 0:** {T}: +1/+1 counter on self -- CORRECT
- **Ability 1:** {T}, remove counter: +1/+1 on each other creature you control -- CORRECT
- **Ability 1 gate:** Only available if has a +1/+1 counter -- CORRECT

## Verdict
**PASS** -- No issues found.
