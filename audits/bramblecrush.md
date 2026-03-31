# Audit: Bramblecrush

## Oracle (Scryfall/API)
- **Name:** Bramblecrush
- **Cost:** {2}{G}{G}
- **Type:** Sorcery
- **Oracle:** Destroy target noncreature permanent.
- **P/T:** N/A

## Implementation: `bramblecrush.rs`
- **Name:** Bramblecrush -- CORRECT
- **Cost:** {2}{G}{G} -- CORRECT
- **Type:** Sorcery -- CORRECT
- **Target:** PermanentWithFilter(Noncreature) -- CORRECT
- **Target validation:** Checks battlefield zone and !CardType::Creature -- CORRECT
- **Effect:** Uses resolve_destroy (destruction pipeline) -- CORRECT (destroy, not sacrifice)

## Verdict: PASS -- No issues found
