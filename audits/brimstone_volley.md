# Audit: Brimstone Volley

## Oracle (Scryfall/API)
- **Name:** Brimstone Volley
- **Cost:** {2}{R}
- **Type:** Instant
- **Oracle:** Brimstone Volley deals 3 damage to any target. Morbid -- Brimstone Volley deals 5 damage instead if a creature died this turn.
- **P/T:** N/A

## Implementation: `brimstone_volley.rs`
- **Name:** Brimstone Volley -- CORRECT
- **Cost:** {2}{R} -- CORRECT
- **Type:** Instant -- CORRECT
- **Target:** AnyTarget -- CORRECT
- **Effect:** 3 damage normally, 5 if morbid (`state.creature_died_this_turn`) -- CORRECT
- **Damage resolution:** Uses `resolve_damage` helper -- CORRECT

## Verdict: PASS -- No issues found
